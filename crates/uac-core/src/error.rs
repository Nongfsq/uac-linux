//! Refusals and failures, as data.
//!
//! The root helper prints an error as JSON (`{"error": {"code": ...}}`), so the
//! CLI and the tray can show it in the user's language. [`std::fmt::Display`]
//! gives the English text used in the journal and on the helper's stderr.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum UacError {
    ConfigMissing { path: String },
    ConfigBadUser { path: String },
    ConfigBadNumber { path: String, key: String },
    ConfigBadRange { path: String },
    UnknownCaller,
    NoAccount { uid: u32 },
    WrongUser { allowed: String, caller: String },
    HoursNotNumber,
    HoursOutOfRange { min: u32, max: u32 },
    SudoRsActive,
    VisudoRejected { detail: String },
    AuditUnavailable { detail: String },
    Io { detail: String },
    NotRoot,
    Usage,
}

impl UacError {
    pub fn io(err: &std::io::Error, what: &str) -> Self {
        Self::Io {
            detail: format!("{what}: {err}"),
        }
    }
}

impl fmt::Display for UacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigMissing { path } => write!(f, "{path} is missing; run scripts/install.sh"),
            Self::ConfigBadUser { path } => {
                write!(f, "{path}: 'user' is missing or not a valid user name")
            }
            Self::ConfigBadNumber { path, key } => write!(f, "{path}: {key} must be a whole number"),
            Self::ConfigBadRange { path } => {
                write!(f, "{path}: need 1 <= default_hours <= max_hours <= 24")
            }
            Self::UnknownCaller => {
                write!(f, "run this through 'uac on' (pkexec); the calling user is unknown")
            }
            Self::NoAccount { uid } => write!(f, "uid {uid} has no account"),
            Self::WrongUser { allowed, caller } => {
                write!(f, "only '{allowed}' may be granted; caller is '{caller}'")
            }
            Self::HoursNotNumber => write!(f, "hours must be a whole number"),
            Self::HoursOutOfRange { min, max } => write!(f, "hours must be between {min} and {max}"),
            Self::SudoRsActive => write!(
                f,
                "the active sudo is not classic sudo (sudo.ws); sudo-rs does not support NOTAFTER. \
                 Run: sudo update-alternatives --set sudo /usr/bin/sudo.ws"
            ),
            Self::VisudoRejected { detail } => write!(f, "visudo rejected the generated rule: {detail}"),
            Self::AuditUnavailable { detail } => {
                write!(f, "audit log unavailable, grant refused: {detail}")
            }
            Self::Io { detail } => write!(f, "{detail}"),
            Self::NotRoot => write!(f, "the helper must run as root (use 'uac on' / 'uac off')"),
            Self::Usage => write!(f, "usage: uac-helper grant HOURS | revoke | cleanup"),
        }
    }
}

impl std::error::Error for UacError {}
