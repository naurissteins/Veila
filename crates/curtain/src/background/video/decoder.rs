use std::{
    collections::BTreeSet,
    env,
    fs::OpenOptions,
    io::{self, Read},
    os::{
        fd::AsFd,
        unix::fs::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    poll::{PollFd, PollFlags, poll},
};
use veila_renderer::FrameSize;

use super::reader::ReaderControl;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_PROBE_BYTES: usize = 64 * 1024;

pub(super) struct Decoder(Child);

impl Decoder {
    pub(super) fn spawn(path: &Path, size: FrameSize, control: &ReaderControl) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags((OFlag::O_NONBLOCK | OFlag::O_NOCTTY).bits())
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected a regular video file",
            ));
        }

        let program = executable("ffmpeg")?;
        let sandbox = executable("bwrap")?;
        let mut command = Command::new(executable("prlimit")?);
        command
            .args(["--as=2147483648", "--core=0", "--"])
            .arg(sandbox)
            .args([
                "--unshare-all",
                "--unshare-user",
                "--disable-userns",
                "--as-pid-1",
                "--die-with-parent",
                "--new-session",
                "--cap-drop",
                "ALL",
                "--clearenv",
                "--setenv",
                "LC_ALL",
                "C",
                "--dev",
                "/dev",
                "--remount-ro",
                "/dev",
            ]);
        for runtime in runtime_files(&program, control)? {
            command.arg("--ro-bind").arg(&runtime).arg(&runtime);
        }
        // only this opened inode is visible inside the sandbox, even if its path changes
        command.args([
            "--ro-bind-fd",
            "0",
            "/clip",
            "--remount-ro",
            "/",
            "--chdir",
            "/",
            "--",
        ]);
        command.arg(program).args(ffmpeg_args(size));
        control.wait_if_paused()?;
        let child = command
            .env_clear()
            .stdin(Stdio::from(file))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self(child))
    }

    pub(super) fn stdout(&mut self) -> io::Result<ChildStdout> {
        let stdout = self
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("video worker stdout unavailable"))?;
        fcntl(&stdout, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;
        Ok(stdout)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn ffmpeg_args(size: FrameSize) -> Vec<String> {
    let mut args = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-max_alloc",
        "100663296",
        "-stream_loop",
        "-1",
        "-hwaccel",
        "none",
        "-max_pixels",
        "8294400",
        "-threads",
        "2",
        "-filter_threads",
        "2",
        "-protocol_whitelist",
        "file",
        "-format_whitelist",
        "mov,matroska,webm",
        "-i",
        "/clip",
        "-map",
        "0:v:0",
        "-an",
        "-sn",
        "-dn",
        "-vf",
    ]
    .map(str::to_owned)
    .to_vec();
    args.push(format!(
        "fps=10,scale={}:{}:flags=bilinear,format=bgra,realtime",
        size.width, size.height
    ));
    args.extend(
        [
            "-fps_mode",
            "passthrough",
            "-c:v",
            "rawvideo",
            "-threads",
            "1",
            "-f",
            "rawvideo",
            "pipe:1",
        ]
        .map(str::to_owned),
    );
    args
}

fn executable(name: &str) -> io::Result<PathBuf> {
    let search_path = env::var_os("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin".into());
    env::split_paths(&search_path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(name))
        .find(|candidate| {
            candidate
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{name} is unavailable")))?
        .canonicalize()
}

fn runtime_files(program: &Path, control: &ReaderControl) -> io::Result<BTreeSet<PathBuf>> {
    let child = Command::new(executable("ldd")?)
        .args([Path::new("--"), program])
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut probe = Decoder(child);
    let mut stdout = probe.stdout()?;
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        if control.stopped() || Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "FFmpeg runtime inspection timed out",
            ));
        }
        match stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) if output.len() + count <= MAX_PROBE_BYTES => {
                output.extend_from_slice(&buffer[..count])
            }
            Ok(_) => return Err(io::Error::other("FFmpeg runtime report is too large")),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let mut descriptors = [PollFd::new(stdout.as_fd(), PollFlags::POLLIN)];
                let _ = poll(&mut descriptors, 20_u16);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    let status = loop {
        if let Some(status) = probe.0.try_wait()? {
            break status;
        }
        if control.stopped() || Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "FFmpeg runtime inspection timed out",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = String::from_utf8(output).map_err(io::Error::other)?;
    if output.contains("not found") {
        return Err(io::Error::other("FFmpeg runtime library unavailable"));
    }
    let mut files: BTreeSet<_> = output
        .split_whitespace()
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
        .collect();
    if !status.success() || files.is_empty() {
        return Err(io::Error::other(
            "expected a dynamically linked FFmpeg executable",
        ));
    }
    files.insert(program.to_path_buf());
    Ok(files)
}
