//! App-local Swift writer ABI. No candidate is publication authority without native validation.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
use std::ffi::{CString, c_void};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

type Create = unsafe extern "C" fn(*const u8, isize, i32, i32) -> *mut c_void;
type Enqueue = unsafe extern "C" fn(*mut c_void, *const c_void, i64) -> i32;
type Final = unsafe extern "C" fn(*mut c_void, *const c_void, i32, i32, i64) -> i32;
type Finish = unsafe extern "C" fn(*mut c_void) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Validate = unsafe extern "C" fn(*const u8, isize, *const u8, i32, i32, i64) -> i32;

struct WriterApi {
    create: Create,
    enqueue: Enqueue,
    final_frame: Final,
    finish: Finish,
    destroy: Destroy,
    validate: Validate,
}

// One resident dlopen reference, including failed symbol resolution. Native queue
// closures may outlive destroy or a finish timeout, so unloading is not safe.
static API: std::sync::OnceLock<Result<WriterApi, String>> = std::sync::OnceLock::new();

fn writer_api() -> Result<&'static WriterApi> {
    API.get_or_init(|| load_writer_api().map_err(|error| format!("{error:#}")))
        .as_ref().map_err(|error| anyhow::anyhow!(error.clone()))
}

fn load_writer_api() -> Result<WriterApi> {
    use std::os::unix::ffi::OsStrExt;
    let exe = std::env::current_exe()?;
    let parent = exe.parent().context("executable has no directory")?;
    let dylib = if parent.file_name().is_some_and(|name| name == "MacOS") {
        parent.parent().context("app bundle has no Contents")?
            .join("Frameworks/libapparent_realtime_writer.dylib")
    } else {
        parent.join("libapparent_realtime_writer.dylib")
    };
    let name = CString::new(dylib.as_os_str().as_bytes())?;
    // Loading only beside the executable prevents a search-path substituted writer.
    let library = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if library.is_null() { bail!("cannot load app-local writer: {}", dylib.display()); }
    let symbol = |name: &'static [u8]| -> Result<*mut c_void> {
        let ptr = unsafe { libc::dlsym(library, name.as_ptr().cast()) };
        if ptr.is_null() { bail!("writer missing symbol {}", String::from_utf8_lossy(name)); }
        Ok(ptr)
    };
    // SAFETY: resolved symbols have the versioned ABI signatures documented in docs.
    unsafe {
        Ok(WriterApi {
            create: std::mem::transmute::<*mut c_void, Create>(symbol(b"apparent_writer_create_v1\0")?),
            enqueue: std::mem::transmute::<*mut c_void, Enqueue>(symbol(b"apparent_writer_enqueue_sample_v1\0")?),
            final_frame: std::mem::transmute::<*mut c_void, Final>(symbol(b"apparent_writer_enqueue_final_rgba_v1\0")?),
            finish: std::mem::transmute::<*mut c_void, Finish>(symbol(b"apparent_writer_finish_v1\0")?),
            destroy: std::mem::transmute::<*mut c_void, Destroy>(symbol(b"apparent_writer_destroy_v1\0")?),
            validate: std::mem::transmute::<*mut c_void, Validate>(symbol(b"apparent_writer_validate_v1\0")?),
        })
    }
}

pub(super) struct Writer {
    _media_lease: crate::retention::MediaLease,
    handle: *mut c_void,
    enqueue: Enqueue,
    final_frame: Final,
    finish: Finish,
    destroy: Destroy,
    validate: Validate,
    candidate: PathBuf,
}

// Only used on the sampler thread; the callback holds the session's Mutex across each ABI call.
unsafe impl Send for Writer {}

impl Writer {
    pub(super) fn create(original: &Path, width: u32, height: u32) -> Result<Self> {
        use std::os::unix::ffi::OsStrExt;
        let WriterApi { create, enqueue, final_frame, finish, destroy, validate } = *writer_api()?;
        let candidate = original.with_file_name(format!(".candidate-{}.mp4", crate::make_bundle_id()));
        let path = candidate.as_os_str().as_bytes();
        anyhow::ensure!(candidate.is_absolute() && !candidate.exists() && path.len() <= 4096, "invalid candidate path");
        let media_lease = crate::retention::candidate_media_lease(
            original.parent().context("movie has no bundle directory")?)?;
        let handle = unsafe { create(path.as_ptr(), path.len() as isize, width as i32, height as i32) };
        if handle.is_null() { bail!("writer refused candidate"); }
        Ok(Self { _media_lease: media_lease, handle, enqueue, final_frame, finish, destroy, validate, candidate })
    }

    pub(super) fn sample(&mut self, buffer: *const c_void, ms: i64) -> Result<()> {
        let rc = unsafe { (self.enqueue)(self.handle, buffer, ms) };
        anyhow::ensure!(rc == 0, "writer enqueue refused sample: {rc}");
        Ok(())
    }

    pub(super) fn final_frame(&mut self, rgba: &[u8], width: u32, height: u32, ms: i64) -> Result<()> {
        let expected = usize::try_from(width)?.checked_mul(usize::try_from(height)?)
            .and_then(|n| n.checked_mul(4)).context("screenshot dimensions overflow")?;
        anyhow::ensure!(rgba.len() == expected, "screenshot size mismatch");
        let rc = unsafe { (self.final_frame)(self.handle, rgba.as_ptr().cast(), width as i32, height as i32, ms) };
        anyhow::ensure!(rc == 0, "writer final frame refused: {rc}");
        Ok(())
    }

    pub(super) fn finish_and_validate(mut self, screenshot: &[u8], width: u32, height: u32, stop_ms: i64) -> Result<super::ValidatedCandidate> {
        let rc = unsafe { (self.finish)(self.handle) };
        anyhow::ensure!(rc == 0, "writer finish failed: {rc}");
        use std::os::unix::ffi::OsStrExt;
        let bytes = self.candidate.as_os_str().as_bytes();
        let rc = unsafe { (self.validate)(bytes.as_ptr(), bytes.len() as isize,
            screenshot.as_ptr(), width as i32, height as i32, stop_ms) };
        anyhow::ensure!(rc == 0, "native tail validation failed: {rc}");
        let candidate = super::ValidatedCandidate(std::mem::take(&mut self.candidate));
        drop(self); // Destruction may wait; keep it in the bounded worker.
        Ok(candidate)
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.handle);
        }
        if !self.candidate.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.candidate);
        }
    }
}
