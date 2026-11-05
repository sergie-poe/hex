//! Labs › Finish with Return, Send after pasting and App tracking, on the
//! listener's side: when the cancel and Return keys are Hex's, which app a
//! capture started in, and where the paste's Return goes.

use super::*;

impl Hotkeys {
    /// A capture starts: notes whether it skips processing and, while Labs ›
    /// Finish with Return, Send after pasting, or App tracking is on, the app
    /// in front. That app's process ID is all anything later needs to know
    /// where the dictation began (Send after pasting returns there; App
    /// tracking compares it with the app in front). Read here, once per
    /// start, so nothing asks macOS with all three labs off.
    pub(super) fn begin_capture(&mut self, raw: bool) {
        self.raw_capture = raw;
        self.start_app = (crate::labs::finish_with_return()
            || crate::labs::send_after_pasting()
            || crate::labs::app_tracking())
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
    /// `action`: the one the capture started in, for a shortcut's own finish
    /// (Hold's release, a locked press) or Return, while the lab is on.
    /// `hex dictation stop` never sends: it finishes outside this loop.
    pub(super) fn send_to(&self, action: HotkeyAction) -> Option<i32> {
        if !sends_after(action) || !crate::labs::send_after_pasting() {
            return None;
        }
        // The finish has ended the capture by now, but `arm_keys` has not yet
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

    /// Arms or disarms the cancel key and Return as the finish key on every
    /// pass of the loop, and forgets the app of a capture that has ended.
    ///
    /// Each key has one decision for both halves: the shortcut machines act
    /// on it only when armed, and the event tap swallows it only when armed
    /// and its recording state holds (a toggled recording for cancel, a
    /// locked one for Return). A condition that disarms a key therefore
    /// stops both the swallow and the action. Labs › App tracking is such a
    /// condition: while the app in front is not the one the capture started
    /// in, both keys are the app's. The app in front is asked only while a
    /// dictation records and the lab is on. Returns whether the keys
    /// are Hex's (the lab is off, or the app in front is the start app), for
    /// the Caps Lock cancel, which follows the same rule.
    pub(super) fn arm_keys(&mut self, input_monitor: &InputMonitor, programmatic: bool) -> bool {
        let recording = self.any_recording();
        if !recording {
            self.start_app = None;
        }
        let in_start_app = !(recording && crate::labs::app_tracking())
            || keys_armed(self.start_app, crate::dictation::context::frontmost_pid());
        for machine in self.machines() {
            machine.set_cancel_armed(in_start_app);
        }
        input_monitor.set_takes_cancel(!programmatic && recording && in_start_app);
        let finish = !programmatic && crate::labs::finish_with_return() && in_start_app;
        for machine in self.machines() {
            machine.set_finish_armed(finish);
        }
        input_monitor.set_takes_finish(finish && self.any_locked());
        in_start_app
    }
}

/// Whether Escape and Return are Hex's for a recording that started in
/// `start_app` while `frontmost` is in front, with Labs › App tracking on.
/// Either app unknown fails open: the keys stay Hex's, as with the lab off.
fn keys_armed(start_app: Option<i32>, frontmost: Option<i32>) -> bool {
    match (start_app, frontmost) {
        (Some(start), Some(front)) => start == front,
        _ => true,
    }
}

/// Whether a dictation finished by `action` may send: the shortcut's own
/// finish and Return do; starts, cancels, and discards never paste.
fn sends_after(action: HotkeyAction) -> bool {
    matches!(action, HotkeyAction::Finish | HotkeyAction::FinishShortcut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shortcut_finish_or_return_may_send_and_nothing_else_does() {
        assert!(sends_after(HotkeyAction::Finish));
        assert!(sends_after(HotkeyAction::FinishShortcut));
        for action in [
            HotkeyAction::Start,
            HotkeyAction::Cancel,
            HotkeyAction::CancelShortcut,
            HotkeyAction::PasteLast,
            HotkeyAction::PasteMeeting,
        ] {
            assert!(!sends_after(action), "{action:?}");
        }
    }

    #[test]
    fn the_keys_are_armed_in_the_start_app_and_unknown_apps_only() {
        assert!(keys_armed(Some(10), Some(10)));
        assert!(!keys_armed(Some(10), Some(11)));
        assert!(keys_armed(None, Some(11)));
        assert!(keys_armed(Some(10), None));
        assert!(keys_armed(None, None));
    }
}
