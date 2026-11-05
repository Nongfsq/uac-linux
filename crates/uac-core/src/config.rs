//! `/etc/uac-linux.conf`: `key=value` lines, `#` starts a comment.

use std::collections::HashMap;
use std::path::Path;

use crate::{MAX_HOURS, MIN_HOURS, UacError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The only account that may receive a grant.
    pub user: String,
    pub default_hours: u32,
    pub max_hours: u32,
}

fn parse_pairs(text: &str) -> HashMap<&str, &str> {
    text.lines()
        .filter_map(|line| line.split('#').next()?.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect()
}

/// A login name as Debian's `adduser` accepts it by default.
pub fn valid_user_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else { return false };
    name.len() <= 32
        && (first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn whole_number(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

impl Config {
    /// Strict load for the root helper: every problem is a refusal.
    pub fn load(path: &Path) -> Result<Self, UacError> {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|_| UacError::ConfigMissing { path: shown.clone() })?;
        let pairs = parse_pairs(&text);
        let user = pairs.get("user").copied().unwrap_or_default();
        if !valid_user_name(user) {
            return Err(UacError::ConfigBadUser { path: shown });
        }
        let mut hours = [6, MAX_HOURS];
        for (slot, key) in hours.iter_mut().zip(["default_hours", "max_hours"]) {
            if let Some(value) = pairs.get(key) {
                *slot = whole_number(value).ok_or_else(|| UacError::ConfigBadNumber {
                    path: shown.clone(),
                    key: key.to_string(),
                })?;
            }
        }
        let [default_hours, max_hours] = hours;
        if !(MIN_HOURS <= default_hours && default_hours <= max_hours && max_hours <= MAX_HOURS) {
            return Err(UacError::ConfigBadRange { path: shown });
        }
        Ok(Self {
            user: user.to_string(),
            default_hours,
            max_hours,
        })
    }
}

/// What the CLI and the tray need from the config, with safe fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hints {
    pub default_hours: u32,
    pub max_hours: u32,
}

impl Hints {
    pub fn read(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let pairs = parse_pairs(&text);
        let get = |key: &str| pairs.get(key).and_then(|v| whole_number(v));
        let max_hours = get("max_hours")
            .filter(|h| (MIN_HOURS..=MAX_HOURS).contains(h))
            .unwrap_or(MAX_HOURS);
        let default_hours = get("default_hours")
            .filter(|h| (MIN_HOURS..=max_hours).contains(h))
            .unwrap_or(6.min(max_hours));
        Self {
            default_hours,
            max_hours,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_names() {
        for good in ["alice", "_svc", "a-b_9"] {
            assert!(valid_user_name(good), "{good}");
        }
        for bad in ["", "Root!", "9abc", "Alice", &"a".repeat(33)] {
            assert!(!valid_user_name(bad), "{bad}");
        }
    }

    #[test]
    fn hints_fall_back() {
        let dir = std::env::temp_dir().join(format!("uac-hints-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("conf");
        std::fs::write(&conf, "default_hours=30\nmax_hours=4 # short\n").unwrap();
        assert_eq!(
            Hints::read(&conf),
            Hints {
                default_hours: 4,
                max_hours: 4
            }
        );
        assert_eq!(
            Hints::read(&dir.join("missing")),
            Hints {
                default_hours: 6,
                max_hours: 24
            }
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
