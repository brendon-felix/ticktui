use once_cell::sync::OnceCell;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppAction;
use crate::ui::UIAction;

/// Global sender for debug messages
static DEBUG_SENDER: OnceCell<UnboundedSender<AppAction>> = OnceCell::new();

/// Initialize the global debug sender
/// This should be called once during app startup
pub fn init_debug_sender(sender: UnboundedSender<AppAction>) {
    DEBUG_SENDER
        .set(sender)
        .expect("Debug sender already initialized");
}

/// Send a debug message globally without needing to pass the sender around
///
/// # Arguments
/// * `msg` - The debug message to display
/// * `n_ticks` - Number of ticks to show the message (default: 20 if None)
///
/// # Example
/// ```
/// debug_msg("Something happened", Some(30));
/// debug_msg("Quick message", None); // Uses default 20 ticks
/// ```
pub fn debug_msg<S: AsRef<str>>(msg: S, n_ticks: Option<u16>) {
    let ticks = n_ticks.unwrap_or(20);

    if let Some(sender) = DEBUG_SENDER.get() {
        let _ = sender.send(AppAction::UIAction(UIAction::DebugMsg(
            msg.as_ref().to_string(),
            ticks,
        )));
    }
    // If sender is not initialized, silently ignore the message
    // This prevents panics during testing or early initialization
}

/// Convenience function for quick debug messages with default duration
pub fn debug<S: AsRef<str>>(msg: S) {
    debug_msg(msg, None);
}

/// Convenience function for formatted debug messages
pub fn debug_fmt(args: std::fmt::Arguments, n_ticks: Option<u16>) {
    debug_msg(args.to_string(), n_ticks);
}

/// Macro for easy formatted debug messages
///
/// # Examples
/// ```
/// debug_f!("User clicked at position: {}, {}", x, y);
/// debug_f!("Error occurred: {}", error; 30); // With custom duration
/// ```
#[macro_export]
macro_rules! debug_f {
    ($($arg:tt)*) => {
        $crate::debug::debug_fmt(format_args!($($arg)*), None)
    };
    ($fmt:expr, $($arg:expr),*; $ticks:expr) => {
        $crate::debug::debug_fmt(format_args!($fmt, $($arg)*), Some($ticks))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_debug_without_init() {
        // Should not panic when sender is not initialized
        debug_msg("test message", None);
        debug("test message");
    }

    #[tokio::test]
    async fn test_debug_with_init() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        init_debug_sender(tx);

        debug_msg("test message", Some(25));

        if let Ok(action) = rx.try_recv() {
            match action {
                AppAction::UIAction(UIAction::DebugMsg(msg, ticks)) => {
                    assert_eq!(msg, "test message");
                    assert_eq!(ticks, 25);
                }
                _ => panic!("Expected DebugMsg action"),
            }
        } else {
            panic!("No message received");
        }
    }
}
