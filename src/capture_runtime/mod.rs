//! Capture caller ownership. The Engine is the only durable receipt authority.
//! Keep this value with media teardown AND the background finalizer, not the UI.
mod paths;
mod phone;
pub use phone::Phone;
#[cfg(test)]
pub(crate) mod tests;

use crate::{bundle_pipeline::publication, model::ApparentBundle};
use anyhow::{Context, Result, bail, ensure};
use apparent_capture_engine::{Admission, Control, FinalizerSuccess, Runtime, Session, Status};
use std::path::{Path, PathBuf};

type Built = (PathBuf, ApparentBundle, String);

pub struct Capture {
    session: Option<Session>,
    runtime: Runtime,
    request: String,
    token: String,
    path: PathBuf,
    recording: bool,
    _retention: crate::retention::Recording,
}

pub fn runtime() -> Result<Runtime> {
    #[cfg(test)]
    return tests::default_runtime();
    #[cfg(not(test))]
    {
        let path = dirs::data_local_dir()
            .context("no local data directory")?
            .join("apparent/capture-runtime-v1");
        paths::provision(&path, true)?;
        Ok(Runtime::open(&path)?)
    }
}

pub fn maintenance(path: &Path) -> Result<Option<apparent_capture_engine::BundleLease>> {
    maintenance_in(&runtime()?, path)
}

pub(crate) fn maintenance_in(
    runtime: &Runtime,
    path: &Path,
) -> Result<Option<apparent_capture_engine::BundleLease>> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("invalid bundle identity")?;
    Ok(runtime.lease(id, apparent_capture_engine::LeaseKind::Maintenance)?)
}

impl Capture {
    /// Output identity is reserved before playback, microphone or movie startup.
    pub fn start<T>(path: &Path, media: impl FnOnce() -> T) -> Result<(Self, T)> {
        Self::start_in(runtime()?, path, media)
    }

    fn start_in<T>(runtime: Runtime, path: &Path, media: impl FnOnce() -> T) -> Result<(Self, T)> {
        let ownership = Self::admit_in(runtime, path)?;
        let media = media();
        Ok((ownership, media))
    }

    fn admit_in(runtime: Runtime, path: &Path) -> Result<Self> {
        let path = paths::output(path)?;
        let bundle = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("invalid bundle identity")?;
        let request = apparent_capture_engine::new_id()?;
        let token = apparent_capture_engine::new_owner_token()?;
        let session = match runtime.admit(&request, &token, bundle)? {
            Admission::Acquired(session) => session,
            Admission::Existing => bail!("capture admission refused: occupied or bundle busy"),
        };
        Ok(Self {
            session: Some(session),
            runtime,
            request,
            token,
            path,
            recording: false,
            _retention: crate::retention::Recording::start(),
        })
    }

    /// Only the media adapter's real frame/recording acknowledgement calls this.
    pub fn observed_frame(&mut self) -> Result<()> {
        if !self.recording {
            self.session
                .as_mut()
                .context("capture already ended")?
                .recording()?;
            self.recording = true;
        }
        Ok(())
    }

    pub fn processing(&mut self) -> Result<()> {
        ensure!(self.recording, "capture never received a real frame");
        self.session
            .as_mut()
            .context("capture already ended")?
            .processing()?;
        Ok(())
    }

    /// The closure includes ALL whiteboard mutation/fallback, not just core build.
    /// Coordination remains held until exact-byte Engine validation and finish.
    pub fn publish(mut self, build: impl FnOnce() -> Result<Built>) -> Result<Built> {
        let _lease = publication::coordinate(&self.path)?;
        let result = publication::publish_after(&self.path, build).and_then(|built| {
            let checked = publication::checked(&self.path, &built.1, &built.2)?;
            let success = FinalizerSuccess::validate(
                &self.request,
                &checked.bundle_id,
                &checked.path,
                &checked.manifest,
            )?;
            let receipt = self
                .session
                .take()
                .context("capture already ended")?
                .finish(Ok(success))?;
            ensure!(
                receipt.status == Status::Saved,
                "capture publication refused: {:?}",
                receipt.error
            );
            Ok(built)
        });
        if result.is_err() {
            if let Some(session) = self.session.take() {
                session.finish(Err(()))?;
            }
        }
        result
    }

    /// Caller tears down media before consuming ownership here.
    pub fn cancel(mut self) -> Result<()> {
        self.runtime
            .request_control(&self.request, &self.token, Control::Cancel)?;
        let receipt = self
            .session
            .take()
            .context("capture already ended")?
            .finish(Err(()))?;
        ensure!(
            receipt.status == Status::Cancelled,
            "cancellation did not commit"
        );
        Ok(())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        // Start failure/unwind is not success; a hard crash is recovered by Engine inspect.
        if let Some(session) = self.session.take() {
            if let Err(error) = session.finish(Err(())) {
                crate::capture::apparent_log(format!("Capture ownership cleanup failed: {error}"));
            }
        }
    }
}
