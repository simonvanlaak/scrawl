//! Desktop adapter for the shared Engine recorder-session owner.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{MAX_CAPTURE_SCALE, platform};
use crate::model::TargetInfo;

use apparent_capture_engine::recording::ScreenCaptureSession as EngineSession;
#[cfg(any(target_os = "macos", test))]
pub(crate) use apparent_capture_engine::recording::sample_real_candidate;
pub use apparent_capture_engine::recording::{
    CANDIDATE_BUDGET_BYTES, CANDIDATE_EDGE, CandidateBuffer, FRAME_SAMPLE_INTERVAL, ScreenRecording,
};
pub(crate) use apparent_capture_engine::recording::{
    CANDIDATE_EDGE_NO_VIDEO, Sampler, push_candidate,
};

/// The capture size for `target` with its long edge at most `long_edge` px (and at most its
/// display's pixels: a 1x display isn't upscaled, capped by `MAX_CAPTURE_SCALE`).
pub fn capture_size(target: &TargetInfo, long_edge: f64) -> (u32, u32) {
    let max_dim = target.frame.width.max(target.frame.height).max(1.0);
    let native = target.frame.scale.clamp(1.0, MAX_CAPTURE_SCALE);
    let scale = (long_edge / max_dim).min(native);
    (
        ((target.frame.width * scale).round() as u32).max(48) & !1,
        ((target.frame.height * scale).round() as u32).max(48) & !1,
    )
}

/// Desktop session: shared orchestration, with root-owned retention protection.
pub struct ScreenCaptureSession {
    // Fields drop in declaration order. Engine Drop wakes and joins the native worker
    // before the retention guard can release, including on an abandoned session.
    session: EngineSession,
    _recording: crate::retention::Recording,
}

impl ScreenCaptureSession {
    pub fn start(
        target: TargetInfo,
        video_output_path: Option<PathBuf>,
        excluded_window_ids: Arc<Mutex<Vec<u32>>>,
    ) -> Self {
        Self::start_with_runner(
            target,
            video_output_path,
            excluded_window_ids,
            platform::run_sampler,
        )
    }

    fn start_with_runner(
        target: TargetInfo,
        video_output_path: Option<PathBuf>,
        excluded_window_ids: Arc<Mutex<Vec<u32>>>,
        run_sampler: impl FnOnce(Sampler) -> Option<PathBuf> + Send + 'static,
    ) -> Self {
        let recording = crate::retention::Recording::start();
        Self {
            session: EngineSession::start(
                target,
                video_output_path,
                excluded_window_ids,
                run_sampler,
            ),
            _recording: recording,
        }
    }

    pub fn request_immediate_frame(&self) {
        self.session.request_immediate_frame();
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.session.elapsed_ms()
    }

    pub fn has_frames(&self) -> bool {
        self.session.has_frames()
    }

    pub fn failed_to_start(&self) -> bool {
        self.session.failed_to_start()
    }

    pub fn stop(self) -> ScreenRecording {
        self.stop_with_readiness().0
    }

    pub fn stop_with_readiness(self) -> (ScreenRecording, bool) {
        let Self {
            session,
            _recording,
        } = self;
        // Keep protection through the real join AND collection, not just Stop intent.
        let result = session.stop_with_readiness();
        drop(_recording);
        result
    }
}

#[cfg(test)]
#[path = "tests_recorder.rs"]
mod tests;
