//! A shortcut capture may attempt one guarded draft handoff, never a chat submission.
use super::hyprland::FocusedWindow;

pub const EXPERIMENT: &str = "autoPasteDraft";

/// Window identity is necessary, but not sufficient: the shared handoff also checks the editor.
pub fn window_matches(original: &FocusedWindow, current: &FocusedWindow) -> bool {
    original == current
}

/// A recording owns this target from the initiating shortcut through finalization.
/// Taking it at finish prevents a second finish/retry from pasting the same bundle.
#[derive(Debug)]
pub struct Attempt(Option<Result<FocusedWindow, String>>);

impl Attempt {
    pub fn new(target: Result<FocusedWindow, String>) -> Self {
        Self(Some(target))
    }

    pub fn take(&mut self) -> Option<Result<FocusedWindow, String>> {
        self.0.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_attempt_keeps_the_original_window_and_is_consumed_once() {
        let original = FocusedWindow {
            address: "0xabc".into(), pid: 42, class: "com.t3tools.T3Code".into(),
        };
        let mut attempt = Attempt::new(Ok(original.clone()));
        assert_eq!(attempt.take().unwrap().unwrap(), original);
        assert!(attempt.take().is_none(), "a repeated finish cannot deliver twice");
        let mut missing = Attempt::new(Err("no editor at start".into()));
        assert_eq!(missing.take().unwrap().unwrap_err(), "no editor at start");
        assert!(missing.take().is_none());
    }

    #[test]
    fn only_the_exact_original_window_can_receive_a_draft() {
        let original = FocusedWindow {
            address: "0xabc".into(), pid: 42, class: "com.t3tools.T3Code".into(),
        };
        let mut other = original.clone();
        assert!(window_matches(&original, &original));
        other.address = "0xdef".into();
        assert!(!window_matches(&original, &other));
        other = original.clone();
        other.pid = 43;
        assert!(!window_matches(&original, &other));
    }
}
