//! Mirror protocol reader. Session shutdown joins this before consuming final state.
use super::*;

pub(super) fn read_events(
    output: impl BufRead,
    reader_state: Arc<Mutex<MirrorState>>,
    reader_cancelled: Arc<std::sync::atomic::AtomicBool>,
    end_identity: Arc<()>,
    telemetry: Option<Arc<SharedCaptureTelemetry>>,
) {
    for line in output.lines() {
        let Ok(line) = line else { break };
        let Ok(event) = serde_json::from_str::<MirrorEvent>(&line) else {
            continue;
        };
        let mut state = reader_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match event {
            MirrorEvent::Device(device) => state.device = Some(device),
            MirrorEvent::Rect {
                x,
                y,
                width,
                height,
            } => {
                let rect = [x as f32, y as f32, width as f32, height as f32];
                state.rect = Some(rect);
                if let Some(telemetry) = &telemetry {
                    *telemetry
                        .target_frame
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = TargetFrame {
                        x,
                        y,
                        width,
                        height,
                        scale: 2.0,
                    };
                    if telemetry
                        .selecting
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        telemetry.set_picker_rect(Some((rect, 3)));
                    }
                }
            }
            MirrorEvent::Recording { started_at } => {
                state.recording_epoch_ms = Some(started_at);
                state.recording_instant = Some(Instant::now());
                if let Some(telemetry) = &telemetry {
                    telemetry
                        .active
                        .store(true, std::sync::atomic::Ordering::Release);
                }
            }
            MirrorEvent::Marker { at } => state.markers_epoch_ms.push(at),
            MirrorEvent::Finished { path, duration_ms } => {
                state.finished = Some((path, duration_ms));
                state.ended = true;
            }
            MirrorEvent::Cancelled => state.ended = true,
            MirrorEvent::Warning { message } => apparent_log(format!("iPhone mirror: {message}")),
            MirrorEvent::Error { message } => {
                apparent_log(format!("iPhone mirror: {message}"));
                state.error = Some(message);
                state.ended = true;
            }
        }
    }
    reader_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .ended = true;
    if !reader_cancelled.load(std::sync::atomic::Ordering::Acquire)
        && let Some(telemetry) = &telemetry
    {
        let _ = telemetry
            .ui_sender
            .try_send(LiveOverlayEvent::PhoneEnded(end_identity));
    }
}
