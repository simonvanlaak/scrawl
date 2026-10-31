//! Shared worker orchestration. Native acquisition and retention stay with the caller.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use super::{CANDIDATE_BUDGET_BYTES, CandidateBuffer, ScreenRecording};
use crate::model::{RgbaFrame, TargetInfo};

/// What the platform's sampler thread records into. It runs until `stopping`, pushes candidate
/// frames (at once when `urgent`), stores when the movie started into `video_offset_ms`, and
/// returns the movie it wrote.
pub struct Sampler {
    pub target: TargetInfo,
    pub video_output_path: Option<PathBuf>,
    /// Apparent's own windows to leave out of the recording.
    pub excluded_window_ids: Arc<Mutex<Vec<u32>>>,
    pub started_at: Instant,
    pub stopping: Arc<AtomicBool>,
    pub urgent: Arc<AtomicBool>,
    pub frames: Arc<Mutex<CandidateBuffer>>,
    pub real_frame_seen: Arc<AtomicBool>,
    /// A terminal producer error, independent of worker lifetime (cleanup may still run).
    pub startup_failed: Arc<AtomicBool>,
    pub video_offset_ms: Arc<AtomicU64>,
}

/// Synthetic fallback pixels can be kept for diagnostics, but never unlock agent Stop.
pub fn push_candidate(
    frames: &Mutex<CandidateBuffer>,
    real_frame_seen: &AtomicBool,
    frame: RgbaFrame,
    urgent: bool,
    real: bool,
) {
    frames
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(frame, urgent);
    if real {
        real_frame_seen.store(true, Ordering::Release);
    }
}

/// Attempt one native screenshot without bypassing a terminal setup failure.
pub fn sample_real_candidate(
    frames: &Mutex<CandidateBuffer>,
    real_frame_seen: &AtomicBool,
    startup_failed: &AtomicBool,
    urgent: bool,
    sample: impl FnOnce() -> std::result::Result<RgbaFrame, String>,
) {
    if startup_failed.load(Ordering::Acquire) {
        return;
    }
    // Individual acquisition/decode errors are not classified terminal source errors.
    // Skip this sample and let the sampler try again, including before its first frame.
    if let Ok(frame) = sample() {
        push_candidate(frames, real_frame_seen, frame, urgent, true);
    }
}

/// Active screen + optional movie recorder driven by the caller's native sampler.
pub struct ScreenCaptureSession {
    started_at: Instant,
    stopping: Arc<AtomicBool>,
    urgent_snapshot: Arc<AtomicBool>,
    frames: Arc<Mutex<CandidateBuffer>>,
    real_frame_seen: Arc<AtomicBool>,
    startup_failed: Arc<AtomicBool>,
    video_offset_ms: Arc<AtomicU64>,
    sampler_thread: Option<JoinHandle<Option<PathBuf>>>,
}

impl ScreenCaptureSession {
    pub fn start(
        target: TargetInfo,
        video_output_path: Option<PathBuf>,
        excluded_window_ids: Arc<Mutex<Vec<u32>>>,
        run_sampler: impl FnOnce(Sampler) -> Option<PathBuf> + Send + 'static,
    ) -> Self {
        let started_at = Instant::now();
        let stopping = Arc::new(AtomicBool::new(false));
        let urgent_snapshot = Arc::new(AtomicBool::new(true));
        let frames = Arc::new(Mutex::new(CandidateBuffer::new(CANDIDATE_BUDGET_BYTES)));
        let real_frame_seen = Arc::new(AtomicBool::new(false));
        let startup_failed = Arc::new(AtomicBool::new(false));
        let video_offset_ms = Arc::new(AtomicU64::new(0));

        let thread_stopping = Arc::clone(&stopping);
        let thread_urgent = Arc::clone(&urgent_snapshot);
        let thread_frames = Arc::clone(&frames);
        let thread_real_frame_seen = Arc::clone(&real_frame_seen);
        let thread_startup_failed = Arc::clone(&startup_failed);
        let thread_offset = Arc::clone(&video_offset_ms);
        let sampler_target = target;
        let sampler_thread = thread::Builder::new()
            .name("apparent-frame-sampler".into())
            .spawn(move || {
                run_sampler(Sampler {
                    target: sampler_target,
                    video_output_path,
                    excluded_window_ids,
                    started_at,
                    stopping: thread_stopping,
                    urgent: thread_urgent,
                    frames: thread_frames,
                    real_frame_seen: thread_real_frame_seen,
                    startup_failed: thread_startup_failed,
                    video_offset_ms: thread_offset,
                })
            })
            .ok();

        Self {
            started_at,
            stopping,
            urgent_snapshot,
            frames,
            real_frame_seen,
            startup_failed,
            video_offset_ms,
            sampler_thread,
        }
    }

    pub fn request_immediate_frame(&self) {
        self.urgent_snapshot.store(true, Ordering::Release);
        if let Some(ref handle) = self.sampler_thread {
            handle.thread().unpark();
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// A portal or ScreenCaptureKit session is still pending until a real frame arrives.
    pub fn has_frames(&self) -> bool {
        self.real_frame_seen.load(Ordering::Acquire)
    }

    /// A known terminal error is failure even while the worker is alive or cleaning up.
    /// A pending portal reply alone never sets this producer flag.
    pub fn failed_to_start(&self) -> bool {
        !self.has_frames()
            && (self.startup_failed.load(Ordering::Acquire)
                || self
                    .sampler_thread
                    .as_ref()
                    .is_none_or(JoinHandle::is_finished))
    }

    pub fn stop(self) -> ScreenRecording {
        self.stop_with_readiness().0
    }

    /// Readiness is distinct from candidates, which can be synthetic fallbacks.
    pub fn stop_with_readiness(mut self) -> (ScreenRecording, bool) {
        let duration_ms = self.elapsed_ms();
        self.stopping.store(true, Ordering::Release);
        let video_path = self
            .sampler_thread
            .take()
            .and_then(|handle| {
                handle.thread().unpark();
                handle.join().ok()
            })
            .flatten();
        // The first real sample may have been in flight when Stop was requested.
        // Both OS samplers update this flag before exiting; only join seals it.
        let frame_seen = self.has_frames();
        let buffer = std::mem::replace(
            &mut *self
                .frames
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            CandidateBuffer::new(0),
        );
        let collected = buffer.into_frames();
        let valid_video =
            video_path.filter(|path| path.exists() && path.metadata().is_ok_and(|m| m.len() > 0));
        (
            ScreenRecording {
                frames: collected,
                video: valid_video,
                duration_ms: duration_ms.max(100),
                video_offset_ms: self.video_offset_ms.load(Ordering::Acquire),
            },
            frame_seen,
        )
    }
}

impl Drop for ScreenCaptureSession {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(handle) = self.sampler_thread.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
#[path = "tests_session.rs"]
mod tests;
