/// iOS lifecycle management.
///
/// iOS lifecycle differs from desktop and Android:
/// - Apps are suspended when backgrounded (not killed)
/// - Apps can be terminated without warning
/// - Scene-based lifecycle (iPad multi-window support)
/// - `applicationDidEnterBackground` and `applicationWillEnterForeground` events

use std::time::{Duration, Instant};

/// iOS app lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IosLifecycleState {
    /// App is active and receiving events
    Active,
    /// App is in the foreground but not receiving events (e.g., incoming call)
    Inactive,
    /// App is in the background (suspended)
    Background,
    /// App is about to terminate
    Terminated,
}

/// Time in background before auto-locking the signer
const AUTO_LOCK_BACKGROUND_SECS: u64 = 120;

/// Handle iOS lifecycle transitions
pub fn handle_transition(from: IosLifecycleState, to: IosLifecycleState) {
    match (from, to) {
        (IosLifecycleState::Active, IosLifecycleState::Background) => {
            log::info!("iOS app entering background");
        }
        (IosLifecycleState::Background, IosLifecycleState::Active) => {
            log::info!("iOS app returning to foreground");
        }
        (_, IosLifecycleState::Terminated) => {
            log::info!("iOS app terminating");
        }
        _ => {}
    }
}

/// Check whether the signer should auto-lock based on background duration
pub fn should_auto_lock(background_since: Option<Instant>) -> bool {
    match background_since {
        Some(start) => start.elapsed() >= Duration::from_secs(AUTO_LOCK_BACKGROUND_SECS),
        None => false,
    }
}

/// Scene session persistence — stores which scene has the active signer session
#[derive(Debug, Clone)]
pub struct SceneSession {
    pub scene_id: String,
    pub unlocked_at: Instant,
}

impl SceneSession {
    pub fn new(scene_id: &str) -> Self {
        Self {
            scene_id: scene_id.to_string(),
            unlocked_at: Instant::now(),
        }
    }
}

/// URL scheme for deep-link activation from Safari extension
pub const DEEP_LINK_SCHEME: &str = "nostrsigner";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_lock_timeout() {
        assert!(should_auto_lock(Some(
            Instant::now() - Duration::from_secs(AUTO_LOCK_BACKGROUND_SECS + 1)
        )));
    }

    #[test]
    fn test_no_lock_when_active() {
        assert!(!should_auto_lock(None));
    }

    #[test]
    fn test_scene_session() {
        let session = SceneSession::new("scene-1");
        assert_eq!(session.scene_id, "scene-1");
    }
}
