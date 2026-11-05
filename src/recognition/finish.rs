//! Labs › Finish with Return and Send after pasting, on the listener's side:
//! when the Return key is Hex's, which app a capture started in, and where
//! the paste's Return goes.

use super::*;

impl Hotkeys {
    /// A capture starts: notes whether it skips processing and, while Labs ›
    /// Finish with Return is on, the app in front. That app's process ID is
    /// all anything later needs to know where the dictation began (Send after
    /// pasting returns there; per-app rules compare it with the app in
    /// front). Read here, once per start, so nothing asks macOS with the lab
    /// off.
    pub(super) fn begin_capture(&mut self, raw: bool) {
        self.raw_capture = raw;
        self.start_app = crate::labs::finish_with_return()
            .then(crate::dictation::context::frontmost_pid)
            .flatten();
    }

    /// Whether a machine records hands-free (a double-tap lock, a Toggle
    /// release, or `hex dictation start`).
    fn any_locked(&self) -> bool {
        Shortcut::ALL
            .into_iter()
            .filter_map(|shortcut| self.machine(shortcut))
            .any(DictationHotkey::is_locked)
    }

    /// The app Send after pasting returns to for a dictation finished by
    /// `action`: the one the capture started in, only for Return finishing
    /// it while the lab is on.
    pub(super) fn send_to(&self, action: HotkeyAction) -> Option<i32> {
        if action != HotkeyAction::FinishShortcut || !crate::labs::send_after_pasting() {
            return None;
        }
        // Return has ended the capture by now, so `arm_finish` has not yet
        // cleared it.
        let start_app = self.start_app;
        if start_app.is_none() {
            tracing::info!(
                reason = "the app it started in is unknown",
                "send after pasting skipped"
            );
        }
        start_app
    }

    /// Arms or disarms Return as the finish key on every pass of the loop,
    /// and forgets the app of a capture that has ended.
    ///
    /// `armed` is the one decision for both halves: the shortcut machines
    /// act on Return only when armed, and the event tap swallows it only
    /// when armed and a capture is locked. A condition that disarms it (a
    /// per-app rule) therefore stops both the swallow and the finish.
    /// `start_app` is what such a rule compares with the app in front.
    pub(super) fn arm_finish(&mut self, input_monitor: &InputMonitor, programmatic: bool) {
        if !self.any_recording() {
            self.start_app = None;
        }
        let armed = !programmatic && crate::labs::finish_with_return();
        for machine in self.machines() {
            machine.set_finish_armed(armed);
        }
        input_monitor.set_takes_finish(armed && self.any_locked());
    }
}
