#![allow(unsafe_code)]
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

pub struct Target(OwnedFd);
impl Target {
    pub fn open(pid: u32) -> io::Result<Self> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(io::Error::other("invalid process id"));
        }
        // SAFETY: pidfd_open accepts integer arguments and returns a new owned descriptor.
        let fd = unsafe { nix::libc::syscall(nix::libc::SYS_pidfd_open, pid as i32, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a successful pidfd_open returns a descriptor with no existing Rust owner.
        Ok(Self(unsafe { OwnedFd::from_raw_fd(fd as i32) }))
    }
    pub fn hangup(&self) -> io::Result<()> {
        // SAFETY: the owned pidfd is valid; a null siginfo requests the ordinary signal.
        let result = unsafe {
            nix::libc::syscall(
                nix::libc::SYS_pidfd_send_signal,
                self.0.as_raw_fd(),
                nix::libc::SIGHUP,
                std::ptr::null::<nix::libc::siginfo_t>(),
                0,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_pids_and_does_not_retarget_an_exited_child() {
        assert!(Target::open(0).is_err());
        assert!(Target::open(u32::MAX).is_err());
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("child");
        let target = Target::open(child.id()).expect("pidfd");
        target.hangup().expect("signal");
        assert!(!child.wait().expect("reap").success());
        assert!(target.hangup().is_err());
    }
}
