/// Android lifecycle management.
///
/// On Android, the app lifecycle is different from desktop:
/// - Apps can be paused/resumed at any time
/// - The process can be killed without warning
/// - Biometric authentication may interrupt the app
///
/// This module provides lifecycle event handling to ensure
/// the signer locks when appropriate.

use std::time::{Duration, Instant};

use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// App is in the foreground
    Active,
    /// App is in the background (paused)
    Background,
    /// App is being destroyed
    Destroyed,
}

/// Time the app has been in background before we auto-lock
const AUTO_LOCK_BACKGROUND_SECS: u64 = 60;

/// Handle an app lifecycle transition
pub fn handle_lifecycle_transition(
    _state: &AppState,
    from: LifecycleState,
    to: LifecycleState,
) {
    match (from, to) {
        (LifecycleState::Active, LifecycleState::Background) => {
            log::info!("Android app moving to background");
        }
        (LifecycleState::Background, LifecycleState::Active) => {
            log::info!("Android app returning to foreground");
        }
        (_, LifecycleState::Destroyed) => {
            log::info!("Android app being destroyed");
        }
        _ => {}
    }
}

/// Check if the signer should auto-lock based on background time
pub fn should_auto_lock(background_start: Option<Instant>) -> bool {
    match background_start {
        Some(start) => start.elapsed() >= Duration::from_secs(AUTO_LOCK_BACKGROUND_SECS),
        None => false,
    }
}

/// Intent actions for lifecycle events from Android
pub const ACTION_LOCK: &str = "to.nostr.action.LOCK";
pub const ACTION_UNLOCK: &str = "to.nostr.action.UNLOCK";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_not_lock_when_not_in_background() {
        assert!(!should_auto_lock(None));
    }

    #[test]
    fn test_should_lock_after_timeout() {
        assert!(should_auto_lock(Some(
            Instant::now() - Duration::from_secs(AUTO_LOCK_BACKGROUND_SECS + 1)
        )));
    }

    #[test]
    fn test_should_not_lock_before_timeout() {
        assert!(!should_auto_lock(Some(
            Instant::now() - Duration::from_secs(1)
        )));
    }
}
