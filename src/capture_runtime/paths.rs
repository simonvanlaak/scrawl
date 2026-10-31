//! Match Engine's strict output ancestor policy before any media is started.
//! Provision missing directories with mkdirat; never repair existing permissions.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::CString,
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
};

pub(super) fn provision(path: &Path, private: bool) -> Result<()> {
    ensure!(path.is_absolute(), "capture storage must be absolute");
    let mut dir = File::open("/")?;
    for part in path.components() {
        let Component::Normal(part) = part else {
            ensure!(
                part == Component::RootDir,
                "capture storage contains traversal"
            );
            continue;
        };
        let name = CString::new(part.as_bytes())?;
        // SAFETY: owned parent descriptor and NUL-free leaf; existing paths are never chmodded.
        let rc = unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) };
        if rc != 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
            return Err(std::io::Error::last_os_error().into());
        }
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        ensure!(
            fd >= 0,
            "capture storage has an inaccessible or symlink ancestor"
        );
        let next = unsafe { File::from_raw_fd(fd) };
        let meta = next.metadata()?;
        let uid = unsafe { libc::geteuid() };
        let sticky_root = meta.uid() == 0 && meta.mode() & libc::S_ISVTX != 0;
        ensure!(
            (meta.uid() == 0 || meta.uid() == uid) && (meta.mode() & 0o022 == 0 || sticky_root),
            "capture storage has an unsafe ancestor: {}",
            path.display()
        );
        next.sync_all()?;
        dir.sync_all()?;
        dir = next;
    }
    let meta = dir.metadata()?;
    ensure!(
        meta.uid() == unsafe { libc::geteuid() } && meta.mode() & 0o022 == 0,
        "capture storage must be owned and not group/world writable"
    );
    if private {
        ensure!(
            meta.mode() & 0o777 == 0o700,
            "runtime directory must already be private (0700)"
        );
    }
    Ok(())
}

pub(super) fn output(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = path.parent().context("capture output has no parent")?;
    provision(parent, false)?;
    ensure!(path.file_name().is_some(), "capture output has no name");
    // Existing output must not redirect the media writers, even before raw protection.
    match std::fs::symlink_metadata(&path) {
        Ok(meta) => ensure!(
            meta.is_dir() && meta.uid() == unsafe { libc::geteuid() } && meta.mode() & 0o022 == 0,
            "unsafe capture output directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    Ok(path)
}
