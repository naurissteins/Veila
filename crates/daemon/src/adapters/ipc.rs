use std::{
    ffi::{OsStr, OsString},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use nix::unistd::Uid;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use veila_common::ipc::{
    ClientMessage, DaemonControlMessage, DaemonControlResponse, DaemonMessage, decode_message,
    encode_message,
};

const SOCKET_MODE: u32 = 0o600;
const RUNTIME_DIR_MODE: u32 = 0o700;
const IPC_MAX_LINE_BYTES: usize = 64 * 1024;

pub async fn bind_listener(path: &Path) -> Result<UnixListener> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove stale socket {}", path.display()))?;
    }

    bind_secured(path)
}

pub async fn bind_single_instance_listener(path: &Path) -> Result<UnixListener> {
    if path.exists() {
        match UnixStream::connect(path).await {
            Ok(_) => {
                return Err(anyhow!(
                    "veilad is already running and listening on {}",
                    path.display()
                ));
            }
            Err(_) => {
                std::fs::remove_file(path).with_context(|| {
                    format!("failed to remove stale daemon socket {}", path.display())
                })?;
            }
        }
    }

    bind_secured(path)
}

fn bind_secured(path: &Path) -> Result<UnixListener> {
    let staging = staging_socket_path(path);
    let _ = std::fs::remove_file(&staging);

    let listener = UnixListener::bind(&staging)
        .with_context(|| format!("failed to bind {}", staging.display()))?;
    if let Err(error) = secure_socket_file(&staging) {
        let _ = std::fs::remove_file(&staging);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&staging, path) {
        let _ = std::fs::remove_file(&staging);
        return Err(error)
            .with_context(|| format!("failed to publish socket at {}", path.display()));
    }

    Ok(listener)
}

fn staging_socket_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(path.file_name().unwrap_or_else(|| OsStr::new("socket")));
    name.push(format!(".{}.staging", std::process::id()));
    path.with_file_name(name)
}

pub fn auth_socket_path() -> Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    let runtime_dir = runtime_dir()?;

    Ok(runtime_dir.join(format!("veila-auth-{stamp}.sock")))
}

pub fn daemon_socket_path() -> Result<PathBuf> {
    Ok(runtime_dir()?.join("veilad.sock"))
}

pub fn transient_socket_path(label: &str) -> Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();

    Ok(runtime_dir()?.join(format!("veila-{label}-{stamp}.sock")))
}

pub async fn send_daemon_control_message(
    path: &Path,
    message: &DaemonControlMessage,
) -> Result<DaemonControlResponse> {
    let mut stream = UnixStream::connect(path)
        .await
        .with_context(|| format!("failed to connect to daemon socket {}", path.display()))?;
    verify_peer_uid(&stream).context("daemon control socket peer rejected")?;

    let mut payload = encode_message(message).context("failed to encode daemon control message")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write daemon control message")?;
    stream
        .flush()
        .await
        .context("failed to flush daemon control message")?;

    read_daemon_control_response(&mut stream)
        .await?
        .ok_or_else(|| anyhow!("daemon closed control socket without a response"))
}

pub async fn read_client_message(stream: &mut UnixStream) -> Result<Option<ClientMessage>> {
    let Some(line) = read_bounded_line(stream, "auth request").await? else {
        return Ok(None);
    };

    decode_message(&line)
        .map(Some)
        .context("invalid auth request")
}

pub async fn write_daemon_message(stream: &mut UnixStream, message: &DaemonMessage) -> Result<()> {
    let mut payload = encode_message(message).context("failed to encode auth response")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write auth response")?;
    stream
        .flush()
        .await
        .context("failed to flush auth response")
}

pub async fn read_daemon_control_message(
    stream: &mut UnixStream,
) -> Result<Option<DaemonControlMessage>> {
    let Some(line) = read_bounded_line(stream, "daemon control message").await? else {
        return Ok(None);
    };

    decode_message(&line)
        .map(Some)
        .context("invalid daemon control message")
}

pub async fn write_daemon_control_response(
    stream: &mut UnixStream,
    response: &DaemonControlResponse,
) -> Result<()> {
    let mut payload =
        encode_message(response).context("failed to encode daemon control response")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write daemon control response")?;
    stream
        .flush()
        .await
        .context("failed to flush daemon control response")
}

async fn read_daemon_control_response(
    stream: &mut UnixStream,
) -> Result<Option<DaemonControlResponse>> {
    let Some(line) = read_bounded_line(stream, "daemon control response").await? else {
        return Ok(None);
    };

    decode_message(&line)
        .map(Some)
        .context("invalid daemon control response")
}

pub(crate) async fn accept_verified(listener: &UnixListener, label: &str) -> Result<UnixStream> {
    let (stream, _) = listener
        .accept()
        .await
        .with_context(|| format!("failed to accept {label} connection"))?;
    verify_peer_uid(&stream).with_context(|| format!("{label} socket peer rejected"))?;
    Ok(stream)
}

pub(crate) async fn read_ipc_line(stream: &mut UnixStream, label: &str) -> Result<Option<String>> {
    read_bounded_line(stream, label).await
}

fn runtime_dir() -> Result<PathBuf> {
    let runtime_root = runtime_root_from_env(std::env::var_os("XDG_RUNTIME_DIR"))?;
    let veila_dir = runtime_root.join("veila");
    std::fs::create_dir_all(&veila_dir)
        .with_context(|| format!("failed to create runtime directory {}", veila_dir.display()))?;
    std::fs::set_permissions(
        &veila_dir,
        std::fs::Permissions::from_mode(RUNTIME_DIR_MODE),
    )
    .with_context(|| {
        format!(
            "failed to restrict runtime directory {}",
            veila_dir.display()
        )
    })?;
    validate_private_dir(&veila_dir, "Veila runtime directory")?;
    Ok(veila_dir)
}

fn runtime_root_from_env(value: Option<OsString>) -> Result<PathBuf> {
    let Some(value) = value else {
        bail!(
            "XDG_RUNTIME_DIR is not set; refusing to create Veila IPC sockets in a shared temporary directory"
        );
    };
    let root = PathBuf::from(value);
    validate_private_dir(&root, "XDG_RUNTIME_DIR")?;
    Ok(root)
}

fn validate_private_dir(path: &Path, label: &str) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to inspect {label} {}", path.display()))?;
    if !metadata.is_dir() {
        bail!("{label} {} is not a directory", path.display());
    }
    let current_uid = Uid::effective().as_raw();
    if metadata.uid() != current_uid {
        bail!(
            "{label} {} is owned by uid {}, expected uid {}",
            path.display(),
            metadata.uid(),
            current_uid
        );
    }
    if metadata.mode() & 0o077 != 0 {
        bail!(
            "{label} {} must not be readable, writable, or executable by group/other users",
            path.display()
        );
    }
    Ok(())
}

fn secure_socket_file(path: &Path) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SOCKET_MODE))
        .with_context(|| format!("failed to restrict socket {}", path.display()))?;
    Ok(())
}

fn verify_peer_uid(stream: &UnixStream) -> Result<()> {
    let peer = stream
        .peer_cred()
        .context("failed to read peer credentials")?;
    let expected_uid = Uid::effective().as_raw();
    if peer.uid() != expected_uid {
        bail!(
            "peer uid {} does not match daemon uid {}",
            peer.uid(),
            expected_uid
        );
    }
    Ok(())
}

async fn read_bounded_line(stream: &mut UnixStream, label: &str) -> Result<Option<String>> {
    let mut line = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .with_context(|| format!("failed to read {label}"))?;
        if read == 0 {
            if line.is_empty() {
                return Ok(None);
            }
            bail!("{label} ended before newline");
        }

        let bytes = &chunk[..read];
        let line_end = bytes.iter().position(|byte| *byte == b'\n');
        let consumed = line_end.unwrap_or(bytes.len());
        if line.len() + consumed > IPC_MAX_LINE_BYTES {
            bail!("{label} exceeds {IPC_MAX_LINE_BYTES} bytes");
        }

        line.extend_from_slice(&bytes[..consumed]);
        if line_end.is_some() {
            let line = String::from_utf8(line).with_context(|| format!("{label} is not UTF-8"))?;
            return Ok(Some(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        SOCKET_MODE, bind_secured, runtime_root_from_env, secure_socket_file, staging_socket_path,
        validate_private_dir,
    };

    #[test]
    fn rejects_missing_runtime_dir() {
        let error = runtime_root_from_env(None).expect_err("missing runtime dir should fail");
        assert!(error.to_string().contains("XDG_RUNTIME_DIR is not set"));
    }

    #[test]
    fn rejects_public_runtime_dir() {
        let dir = unique_test_dir("public-runtime");
        fs::create_dir_all(&dir).expect("test dir");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("permissions");

        let error =
            validate_private_dir(&dir, "test runtime").expect_err("public runtime dir should fail");
        assert!(error.to_string().contains("group/other users"));

        fs::remove_dir_all(&dir).ok();
    }

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("veila-{label}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn staging_path_is_a_hidden_sibling_of_the_target() {
        // Same directory keeps the publish step a rename rather than a cross-filesystem copy
        let path = Path::new("/run/user/1000/veila/veilad.sock");
        let staging = staging_socket_path(path);

        assert_eq!(staging.parent(), path.parent());
        assert!(
            staging
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".veilad.sock.")),
            "unexpected staging name: {staging:?}"
        );
    }

    #[tokio::test]
    async fn bind_secured_publishes_an_owner_only_socket() {
        let dir = unique_test_dir("bind-secured");
        fs::create_dir_all(&dir).expect("test dir");
        let path = dir.join("veilad.sock");

        let listener = bind_secured(&path).expect("bind secured");

        let mode = fs::metadata(&path).expect("metadata").mode() & 0o777;
        assert_eq!(mode, SOCKET_MODE);

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("staging"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staging entries left behind: {leftovers:?}"
        );

        drop(listener);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn secure_socket_file_sets_owner_only_mode() {
        let dir = unique_test_dir("socket-mode");
        fs::create_dir_all(&dir).expect("test dir");
        let path = dir.join("placeholder.sock");
        fs::write(&path, b"stub").expect("stub");

        secure_socket_file(Path::new(&path)).expect("secure socket file");

        let metadata = fs::metadata(&path).expect("metadata");
        assert_eq!(metadata.mode() & 0o777, SOCKET_MODE);

        fs::remove_file(&path).ok();
        fs::remove_dir_all(&dir).ok();
    }
}
