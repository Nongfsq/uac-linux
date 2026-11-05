//! What the CLI and the tray share: asking the root helper to act.
//!
//! The helper runs through pkexec, which shows the polkit password dialog for
//! `grant`. It prints the new state as JSON on success, or
//! `{"error": {"code": ...}}` on a refusal.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::UacError;
use crate::paths::LIBEXEC;
use crate::state::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Grant(u32),
    Revoke,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done(State),
    /// The password dialog was dismissed or authentication failed.
    Cancelled,
    /// The helper refused, with a reason the caller can translate.
    Refused(UacError),
    /// Anything else: pkexec missing, the helper crashed, unreadable output.
    Failed(String),
}

/// What the helper prints on stdout when it refuses.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorReply {
    pub error: UacError,
}

/// pkexec's exit status when the dialog is dismissed or the password is wrong.
const PKEXEC_DISMISSED: i32 = 126;

pub fn run(action: Action) -> Outcome {
    let (entry, args) = match action {
        Action::Grant(hours) => ("grant", vec![hours.to_string()]),
        Action::Revoke => ("revoke", Vec::new()),
    };
    let output = match Command::new("pkexec")
        .arg(format!("{LIBEXEC}/{entry}"))
        .args(args)
        .output()
    {
        Ok(output) => output,
        Err(err) => return Outcome::Failed(format!("pkexec: {err}")),
    };
    parse(output.status.code(), &output.stdout, &output.stderr)
}

fn parse(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Outcome {
    let stdout = String::from_utf8_lossy(stdout);
    match code {
        Some(0) => serde_json::from_str(stdout.trim())
            .map(Outcome::Done)
            .unwrap_or_else(|e| Outcome::Failed(format!("unreadable helper output: {e}"))),
        Some(PKEXEC_DISMISSED) => Outcome::Cancelled,
        _ => match serde_json::from_str::<ErrorReply>(stdout.trim()) {
            Ok(reply) => Outcome::Refused(reply.error),
            Err(_) => {
                let text = String::from_utf8_lossy(stderr).trim().to_string();
                Outcome::Failed(if text.is_empty() {
                    format!("helper failed ({code:?})")
                } else {
                    text
                })
            }
        },
    }
}

/// Whether `sudo -n true` works right now, for whatever reason.
pub fn sudo_passwordless_now() -> bool {
    Command::new("sudo")
        .args(["-n", "true"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_helper_replies() {
        assert_eq!(
            parse(Some(0), br#"{"active": false}"#, b""),
            Outcome::Done(State::default())
        );
        assert_eq!(parse(Some(126), b"", b""), Outcome::Cancelled);
        let refused = parse(
            Some(1),
            br#"{"error": {"code": "hours_out_of_range", "min": 1, "max": 4}}"#,
            b"x",
        );
        assert_eq!(refused, Outcome::Refused(UacError::HoursOutOfRange { min: 1, max: 4 }));
        assert_eq!(
            parse(Some(127), b"", b"Not authorized"),
            Outcome::Failed("Not authorized".into())
        );
    }
}
