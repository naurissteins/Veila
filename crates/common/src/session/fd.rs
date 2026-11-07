#![allow(unsafe_code)]

use nix::fcntl::{FcntlArg, FdFlag, fcntl};
use std::{
    io,
    os::fd::{AsFd, FromRawFd, OwnedFd, RawFd},
};

/// takes an inherited descriptor before any threads start; returns a close-on-exec duplicate
pub fn take(fd: RawFd) -> io::Result<OwnedFd> {
    if fd < 3 {
        return Err(io::Error::other(
            "inherited descriptor overlaps standard I/O",
        ));
    }
    // SAFETY: fcntl validates the untrusted number and creates a new descriptor without borrowing it
    let duplicate = unsafe { nix::libc::fcntl(fd, nix::libc::F_DUPFD_CLOEXEC, 3) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl returned a fresh owned descriptor; no other Rust owner has been created for it
    let owned = unsafe { OwnedFd::from_raw_fd(duplicate) };
    nix::unistd::close(fd)?;
    Ok(owned)
}

pub fn inheritable(fd: &impl AsFd, enabled: bool) -> io::Result<()> {
    let flags = if enabled {
        FdFlag::empty()
    } else {
        FdFlag::FD_CLOEXEC
    };
    fcntl(fd, FcntlArg::F_SETFD(flags))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        os::fd::{AsRawFd, IntoRawFd},
        os::unix::net::UnixStream,
    };
    #[test]
    fn takes_a_socket_once_and_sets_close_on_exec() {
        let (socket, mut peer) = UnixStream::pair().expect("pair");
        let raw = socket.into_raw_fd();
        let owned = take(raw).expect("adopt");
        assert_ne!(owned.as_raw_fd(), raw);
        assert!(
            FdFlag::from_bits_truncate(fcntl(&owned, FcntlArg::F_GETFD).expect("flags"))
                .contains(FdFlag::FD_CLOEXEC)
        );
        let mut socket = UnixStream::from(owned);
        peer.write_all(b"x").expect("write");
        let mut byte = [0];
        socket.read_exact(&mut byte).expect("read");
        assert_eq!(byte, [b'x']);
        inheritable(&socket, true).expect("inherit");
        assert!(
            !FdFlag::from_bits_truncate(fcntl(&socket, FcntlArg::F_GETFD).expect("flags"))
                .contains(FdFlag::FD_CLOEXEC)
        );
        inheritable(&socket, false).expect("restore");
    }
    #[test]
    fn rejects_standard_and_missing_descriptors() {
        for fd in [-1, 0, 1, 2, i32::MAX] {
            assert!(take(fd).is_err());
        }
    }
}
