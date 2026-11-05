//! Labs › Send after pasting: Return pressed once after a dictation finished
//! with Return has pasted, so a chat box sends the message.

use crate::paste::PasteMode;
use crate::suppression::InputActivity;

/// Why Return is not pressed after this paste, or `None` to press it. It
/// goes only to the app the dictation started in (`send_to`), and only for a
/// paste that happened: not copy-only, not empty, not cancelled.
fn skip_reason(
    send_to: i32,
    frontmost: Option<i32>,
    mode: PasteMode,
    text: &str,
    cancelled: bool,
) -> Option<&'static str> {
    if cancelled {
        Some("the dictation was cancelled")
    } else if matches!(mode, PasteMode::CopyOnly) {
        Some("the dictation was only copied")
    } else if text.trim().is_empty() {
        Some("nothing was pasted")
    } else if frontmost != Some(send_to) {
        Some("another app is in front")
    } else {
        None
    }
}

/// Presses Return once if `send_to` (the app the dictation started in) is
/// still in front after its paste returned. `paste` has already waited out
/// the target's consumption window, so the key follows the pasted text.
/// Logs why it skipped, never the text.
pub(super) fn after_paste(
    send_to: Option<i32>,
    mode: PasteMode,
    text: &str,
    cancelled: bool,
    activity: &InputActivity,
) {
    let Some(send_to) = send_to else {
        return;
    };
    let frontmost = crate::dictation::context::frontmost_pid();
    if let Some(reason) = skip_reason(send_to, frontmost, mode, text, cancelled) {
        tracing::info!(reason, "send after pasting skipped");
        return;
    }
    match crate::keyboard::post_return() {
        // The message is sent: the next dictation starts a new one, as it
        // would after any key the person types.
        Ok(()) => {
            activity.invalidate();
            tracing::info!("send after pasting pressed Return");
        }
        Err(error) => tracing::warn!(%error, "send after pasting could not press Return"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: i32 = 501;

    #[test]
    fn return_follows_a_paste_into_the_app_the_dictation_started_in() {
        let reason =
            |frontmost, mode, text, cancelled| skip_reason(START, frontmost, mode, text, cancelled);
        assert_eq!(
            reason(Some(START), PasteMode::Continue, "hello", false),
            None
        );
        assert_eq!(
            reason(Some(START), PasteMode::Standalone, "hello", false),
            None
        );
    }

    #[test]
    fn return_is_skipped_when_nothing_was_pasted_into_that_app() {
        let reason =
            |frontmost, mode, text, cancelled| skip_reason(START, frontmost, mode, text, cancelled);
        assert!(reason(Some(START), PasteMode::CopyOnly, "hello", false).is_some());
        assert!(reason(Some(START), PasteMode::Continue, " \n", false).is_some());
        assert!(reason(Some(START), PasteMode::Continue, "hello", true).is_some());
        // Another app took the front, or none can be named.
        assert!(reason(Some(START + 1), PasteMode::Continue, "hello", false).is_some());
        assert!(reason(None, PasteMode::Continue, "hello", false).is_some());
    }
}
