//! `/var/lib/uac-linux/state.json`: the current or the last grant.
//!
//! Only the root helper writes it; everything else reads it without root.

use std::path::Path;

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// `uac off` or the tray's revoke item.
    User,
    Expired,
    /// The state said active but the rule file was gone.
    RuleMissing,
    /// The rule file had no readable `NOTAFTER`.
    RuleUnreadable,
    /// sudo-rs became the active sudo; it cannot parse the rule.
    SudoRsActive,
    /// A rule file the state file did not know about.
    StateMismatch,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
    /// Grant this one replaced, if a grant was active when it was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_grant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<EndReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl State {
    /// A missing or unreadable file means no grant.
    pub fn read(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Time left at `now`, or `None` when no grant is in force.
    pub fn remaining(&self, now: Timestamp) -> Option<SignedDuration> {
        let expires = self.expires_at.filter(|_| self.active)?;
        let left = expires.duration_since(now);
        left.is_positive().then_some(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_python_era_state() {
        let old = r#"{"active": true, "expires_at": "2026-11-04T14:00:00+00:00", "grant_id": "ab",
                      "granted_at": "2026-11-04T08:00:00+00:00", "hours": 6, "replaces": null,
                      "user": "alice", "version": "0.1.0"}"#;
        let state: State = serde_json::from_str(old).unwrap();
        let now: Timestamp = "2026-11-04T13:00:00Z".parse().unwrap();
        assert_eq!(state.remaining(now), Some(SignedDuration::from_hours(1)));
        let later: Timestamp = "2026-11-04T14:00:00Z".parse().unwrap();
        assert_eq!(state.remaining(later), None);
    }

    #[test]
    fn missing_file_is_off() {
        assert!(!State::read(Path::new("/nonexistent/uac/state.json")).active);
    }
}
