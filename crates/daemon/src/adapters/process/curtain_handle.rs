use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
    process::Child,
    time::{Instant, sleep_until, timeout},
};
use veila_common::ipc::{
    CurtainControlMessage, CurtainControlResponse, CurtainLockState, decode_message, encode_message,
};

use super::CURTAIN_CONTROL_TIMEOUT;
use crate::adapters::{ipc, ownership};

pub(crate) enum CurtainHandle {
    Spawned {
        child: Child,
        owner_path: PathBuf,
    },
    Adopted {
        pid: u32,
        start_ticks: u64,
        owner_path: PathBuf,
        next_check: Instant,
    },
}

#[derive(Debug)]
pub(crate) enum CurtainExit {
    Spawned(ExitStatus),
    Adopted,
}

impl CurtainHandle {
    pub(crate) fn owner_path(&self) -> &Path {
        match self {
            Self::Spawned { owner_path, .. } | Self::Adopted { owner_path, .. } => owner_path,
        }
    }

    pub(crate) async fn wait(&mut self) -> Result<CurtainExit> {
        match self {
            Self::Spawned { child, .. } => Ok(CurtainExit::Spawned(child.wait().await?)),
            Self::Adopted {
                pid,
                start_ticks,
                next_check,
                ..
            } => loop {
                sleep_until(*next_check).await;
                *next_check = Instant::now() + Duration::from_millis(250);
                if !ownership::process_matches(*pid, *start_ticks)? {
                    break Ok(CurtainExit::Adopted);
                }
            },
        }
    }

    pub(crate) async fn signal_if_running(&self, signal: Signal) -> Result<()> {
        match self {
            Self::Spawned { child, .. } => {
                if let Some(pid) = child.id() {
                    kill(Pid::from_raw(pid as i32), signal)?;
                }
            }
            Self::Adopted { .. } => bail!("adopted curtain cannot be signaled without a pidfd"),
        }
        Ok(())
    }
}

pub(crate) async fn probe_curtain(
    control_socket: &Path,
    expected_pid: u32,
) -> Result<CurtainLockState> {
    timeout(CURTAIN_CONTROL_TIMEOUT, async {
        let mut stream = UnixStream::connect(control_socket)
            .await
            .context("failed to connect to curtain probe socket")?;
        ipc::verify_peer_uid(&stream)?;
        if stream.peer_cred()?.pid() != i32::try_from(expected_pid).ok() {
            bail!("curtain probe peer PID does not match ownership record");
        }
        let mut payload = encode_message(&CurtainControlMessage::Probe)?;
        payload.push('\n');
        stream.write_all(payload.as_bytes()).await?;
        let line = ipc::read_ipc_line(&mut stream, "curtain probe reply")
            .await?
            .context("curtain closed probe connection without a reply")?;
        let CurtainControlResponse::Status { state } = decode_message(&line)?;
        Ok(state)
    })
    .await
    .context("timed out probing surviving curtain")?
}

#[cfg(test)]
mod recovery_tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::UnixListener,
    };
    use veila_common::ipc::{CurtainControlResponse, CurtainLockState, encode_message};

    use super::probe_curtain;

    #[tokio::test]
    async fn probe_requires_matching_curtain_pid_and_locked_reply() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("veila-probe-{}-{stamp}.sock", std::process::id()));
        let listener = UnixListener::bind(&path).expect("bind probe fixture");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept probe");
            let mut request = [0_u8; 8];
            stream.read_exact(&mut request).await.expect("read probe");
            assert_eq!(&request, b"\"Probe\"\n");
            let mut reply = encode_message(&CurtainControlResponse::Status {
                state: CurtainLockState::Locked,
            })
            .expect("encode reply");
            reply.push('\n');
            stream
                .write_all(reply.as_bytes())
                .await
                .expect("write reply");
        });
        let state = probe_curtain(&path, std::process::id())
            .await
            .expect("probe curtain");
        assert_eq!(state, CurtainLockState::Locked);
        server.await.expect("server task");
        std::fs::remove_file(&path).expect("remove socket");
    }

    #[tokio::test]
    async fn probe_rejects_another_process_identity() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "veila-wrong-pid-{}-{stamp}.sock",
            std::process::id()
        ));
        let listener = UnixListener::bind(&path).expect("bind probe fixture");
        let server = tokio::spawn(async move {
            listener.accept().await.expect("accept probe");
        });
        let error = probe_curtain(&path, std::process::id() + 1)
            .await
            .expect_err("another PID must be rejected");
        assert!(error.to_string().contains("peer PID does not match"));
        server.await.expect("server task");
        std::fs::remove_file(&path).expect("remove socket");
    }
}
