//! Android's std file-lock API can be unsupported; use Bionic's advisory flock.
use std::{fs::File, io};

pub fn exclusive(file: &File) -> io::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        file.lock()
    }
    #[cfg(target_os = "android")]
    {
        loop {
            // The persistent inode is unlocked when File is dropped.
            match rustix::fs::flock(file, rustix::fs::FlockOperation::LockExclusive) {
                Ok(()) => return Ok(()),
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}
