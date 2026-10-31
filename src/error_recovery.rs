//! Worker-safe operation errors, presented by the shared GPUI loop.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

use gpui::{
    App, Bounds, Context, FocusHandle, FontWeight, IntoElement, KeyDownEvent, Render, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};

struct Request {
    title: &'static str,
    capture: Option<PathBuf>,
}

// Latest error wins: a burst needs one decision, not an unbounded queue of dialogs.
#[derive(Default)]
struct Pending(Option<Request>);

impl Pending {
    fn push(&mut self, request: Request) {
        self.0 = Some(request);
    }

    fn take(&mut self) -> Option<Request> {
        self.0.take()
    }
}

static PENDING: Mutex<Pending> = Mutex::new(Pending(None));

/// Logs the original diagnostic and queues private, actionable error feedback.
/// Safe to call from workers. Neither diagnostic text nor capture paths enter the UI.
/// `title` identifies the operation; unknown titles get a generic heading.
pub fn report(title: impl Into<String>, detail: impl Into<String>, capture: Option<PathBuf>) {
    let title = title.into();
    crate::capture::apparent_log(format!("{title}: {}", detail.into()));
    let request = Request {
        title: operation_title(&title),
        capture: recovery_capture(capture.as_deref(), || {
            crate::handoff::resolve_bundle_dir(Path::new("latest")).ok()
        }),
    };
    PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(request);
}

// Only fixed labels reach the window, even if a caller puts a diagnostic in its title.
fn operation_title(title: &str) -> &'static str {
    let words: Vec<_> = title.split(|c: char| !c.is_alphabetic()).collect();
    for (word, label) in [
        ("paste", "Paste error"),
        ("copy", "Copy error"),
        ("capture", "Capture error"),
        ("recording", "Recording error"),
        ("export", "Export error"),
    ] {
        if words
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(word))
        {
            return label;
        }
    }
    "Operation error"
}

fn completed_capture(path: &Path) -> Option<PathBuf> {
    // An alias is not a pinned target after admission has failed.
    if path == Path::new("latest") {
        return None;
    }
    crate::handoff::load_bundle(path)
        .ok()
        .map(|(directory, _)| directory)
}

/// Resolve once at admission, retaining an unavailable explicit request on failure.
pub fn resolve_target(title: &str, requested: &Path) -> Option<PathBuf> {
    match crate::handoff::resolve_bundle_dir(requested) {
        Ok(directory) => Some(directory),
        Err(error) => {
            report(title, format!("{error:#}"), Some(requested.to_owned()));
            None
        }
    }
}

fn recovery_capture(
    supplied: Option<&Path>,
    latest: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    match supplied {
        // A failed handoff's target is not permission to substitute another capture.
        Some(path) => completed_capture(path),
        None => latest().as_deref().and_then(completed_capture),
    }
}

type OpenCapture = Rc<dyn Fn(&Path, &mut App)>;

pub(crate) struct RecoveryWindow {
    request: Request,
    focus: FocusHandle,
    open_capture: OpenCapture,
}

/// Presents pending errors, updating the existing recovery window instead of opening another.
/// The callback opens the selected completed capture in Review; it must not paste or upload it.
pub fn poll(cx: &mut App, open_capture: impl Fn(&Path, &mut App) + Clone + 'static) {
    let Some(request) = PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    else {
        return;
    };
    let open_capture: OpenCapture = Rc::new(open_capture);
    for handle in cx.windows() {
        if let Some(handle) = handle.downcast::<RecoveryWindow>() {
            let _ = handle.update(cx, |view, window, cx| {
                view.request = request;
                view.open_capture = open_capture;
                window.activate_window();
                window.focus(&view.focus);
                cx.notify();
            });
            crate::ui::take_recovery_keys(cx);
            return;
        }
    }
    #[cfg(target_os = "linux")]
    {
        crate::linux::appearance::initialize();
        crate::linux::appearance::watch();
    }
    let bounds = Bounds::centered(None, size(px(460.0), px(290.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Apparent — Operation error".into()),
                ..Default::default()
            }),
            focus: true,
            show: true,
            is_resizable: false,
            app_id: Some("dev.kitlangton.apparent".into()),
            ..Default::default()
        },
        move |window, cx| {
            #[cfg(not(target_os = "macos"))]
            crate::ui::set_app_icon(window);
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus);
                RecoveryWindow {
                    request,
                    focus,
                    open_capture,
                }
            })
        },
    );
    if let Err(error) = opened {
        crate::capture::apparent_log(format!("error recovery: window failed to open: {error:#}"));
    } else {
        crate::ui::take_recovery_keys(cx);
    }
}

impl RecoveryWindow {
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Revalidate only this pinned target. Deletion must not open a newer capture instead.
        let capture = self.request.capture.as_deref().and_then(completed_capture);
        if let Some(capture) = capture {
            window.remove_window();
            (self.open_capture)(&capture, cx);
        } else {
            self.request.capture = None;
            cx.notify();
        }
    }
}

impl Render for RecoveryWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_os = "linux")]
        let dark = crate::linux::appearance::is_dark();
        #[cfg(not(target_os = "linux"))]
        let dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let paper = rgb(if dark { 0x201b24 } else { 0xf5f0e8 });
        let ink = rgb(if dark { 0xf5f0e8 } else { 0x020102 });
        let available = self.request.capture.is_some();
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .h(px(40.0))
                .px(px(14.0))
                .border_1()
                .border_color(ink)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(if dark { 0x302442 } else { 0xc4a9e6 })))
                .child(label)
        };
        div().size_full().p(px(24.0)).bg(paper).text_color(ink).text_size(px(14.0))
            .flex().flex_col().gap(px(16.0)).track_focus(&self.focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => window.remove_window(),
                    "enter" if view.request.capture.is_some() => view.open(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(div().text_size(px(22.0)).font_weight(FontWeight::SEMIBOLD).child(self.request.title))
            .child("Restart Apparent, then open the last completed capture to retry. Finish any ongoing recording before restarting. Restarting may not resolve the problem.")
            .when(!available, |d| d.child("No completed capture is available to open."))
            .child(div().flex_1())
            .child(div().flex().justify_end().gap(px(8.0))
                .child(button("recovery-dismiss", "Dismiss")
                    .on_click(|_, window, _| window.remove_window()))
                .when(available, |d| d.child(button("recovery-open", "Open Last Capture")
                    .bg(rgb(0x7a58c7)).text_color(rgb(0xf5f0e8))
                    .on_click(cx.listener(|view, _, window, cx| view.open(window, cx))))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "apparent-recovery-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn bundle(&self, name: &str, completed: bool) -> PathBuf {
            let dir = self.0.join(format!("{name}.apparent"));
            fs::create_dir_all(&dir).unwrap();
            if completed {
                let manifest = serde_json::json!({
                    "version": 1, "id": name, "createdAtMs": 100, "durationMs": 1000,
                    "target": {"mode": "display", "displayId": 1,
                        "frame": {"x": 0.0, "y": 0.0, "width": 100.0, "height": 100.0, "scale": 1.0}},
                    "transcript": {"source": "none", "text": "", "segments": []},
                    "strokes": [], "marks": [], "events": [], "keyframes": [], "contactSheet": "contact-sheet.png"
                });
                fs::write(
                    dir.join("apparent.json"),
                    serde_json::to_vec(&manifest).unwrap(),
                )
                .unwrap();
            }
            dir
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recovery_keeps_exact_completed_capture_instead_of_newer_latest() {
        let fixture = Fixture::new();
        let supplied = fixture.bundle("supplied", true);
        assert_eq!(
            recovery_capture(Some(&supplied.join("apparent.json")), || panic!(
                "must not consult latest"
            )),
            Some(supplied.canonicalize().unwrap())
        );
    }

    #[test]
    fn recovery_uses_latest_only_without_an_explicit_target() {
        let fixture = Fixture::new();
        let unfinished = fixture.bundle("unfinished", false);
        let completed = fixture.bundle("completed", true);
        assert_eq!(
            recovery_capture(None, || Some(completed.clone())),
            Some(completed.canonicalize().unwrap())
        );
        assert_eq!(
            recovery_capture(Some(&unfinished), || Some(completed.clone())),
            None
        );
        assert_eq!(recovery_capture(None, || None), None);
        assert_eq!(
            recovery_capture(Some(&unfinished), || Some(unfinished.clone())),
            None
        );
        fs::write(unfinished.join("apparent.json"), b"{}").unwrap();
        assert_eq!(completed_capture(&unfinished), None);
        fs::remove_file(completed.join("apparent.json")).unwrap();
        assert_eq!(completed_capture(&completed), None);
    }

    #[test]
    fn operation_recovery_survives_a_newer_latest_capture() {
        let fixture = Fixture::new();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "error_recovery::tests::operation_recovery_child",
                "--nocapture",
            ])
            .env("APPARENT_CAPTURES_DIR", &fixture.0)
            .env("APPARENT_RECOVERY_TEST_ROOT", &fixture.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn operation_recovery_child() {
        let Some(root) = std::env::var_os("APPARENT_RECOVERY_TEST_ROOT") else {
            return;
        };
        let fixture = Fixture(PathBuf::from(root));
        let original = fixture.bundle("original", true).canonicalize().unwrap();
        // The same real admission boundaries used by Paste and Export. A fresh
        // process keeps latest/config and the global report slot out of other tests.
        let paste = resolve_target("Paste failed", Path::new("latest")).unwrap();
        let (export, _) = crate::handoff::load_bundle(Path::new("latest")).unwrap();
        let newer = fixture.bundle("newer", true);
        let manifest_path = newer.join("apparent.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["createdAtMs"] = 200.into();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            crate::handoff::resolve_bundle_dir(Path::new("latest")).unwrap(),
            newer.canonicalize().unwrap()
        );
        for (title, pinned) in [("Paste failed", paste), ("Video export failed", export)] {
            report(
                title,
                "synthetic failure after latest changed",
                Some(pinned),
            );
            let request = PENDING.lock().unwrap().take().unwrap();
            assert_eq!(request.capture, Some(original.clone()));
        }
        fs::remove_file(original.join("apparent.json")).unwrap();
        report("Paste failed", "original disappeared", Some(original));
        assert!(PENDING.lock().unwrap().take().unwrap().capture.is_none());
        // Production admission must not erase an explicit failed request to None.
        for title in ["Paste failed", "Could not open capture"] {
            assert!(resolve_target(title, &fixture.0.join("missing.apparent")).is_none());
            assert!(PENDING.lock().unwrap().take().unwrap().capture.is_none());
        }
        // An unresolved alias from a failed admission is not permission to retry latest.
        report(
            "Video export failed",
            "admission failed",
            Some(PathBuf::from("latest")),
        );
        assert!(PENDING.lock().unwrap().take().unwrap().capture.is_none());
    }

    #[test]
    fn pending_errors_coalesce_to_latest_and_are_consumed_once() {
        let mut pending = Pending::default();
        for index in 0..1000 {
            pending.push(Request {
                title: "Copy error",
                capture: Some(PathBuf::from(index.to_string())),
            });
        }
        assert_eq!(pending.take().unwrap().capture, Some(PathBuf::from("999")));
        assert!(pending.take().is_none());
    }

    #[test]
    fn popup_titles_never_echo_diagnostics_or_credentials() {
        assert_eq!(operation_title("Copy failed: token=private"), "Copy error");
        assert_eq!(
            operation_title("https://user:password@example.test"),
            "Operation error"
        );
        assert_eq!(operation_title("Paste failed"), "Paste error");
    }
}
