//! The sudoers rule: rendering it and reading its expiry back.

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;

use crate::VERSION;

/// `NOTAFTER` wants `YYYYMMDDHHMMSSZ` in UTC.
pub fn notafter_stamp(at: Timestamp) -> String {
    at.strftime("%Y%m%d%H%M%SZ").to_string()
}

pub fn render(user: &str, grant_id: &str, start: Timestamp, expires: Timestamp, hours: u32) -> String {
    format!(
        "# Managed by uac-linux {VERSION}. Do not edit; use 'uac off' to remove.\n\
         # grant {grant_id}: {start} -> {expires} ({hours} h)\n\
         {user} ALL=(ALL:ALL) NOTAFTER={stamp} NOPASSWD: ALL\n",
        stamp = notafter_stamp(expires),
    )
}

/// `NOTAFTER` of a rule file's text, or `None` when there is none.
pub fn expiry(text: &str) -> Option<Timestamp> {
    let (_, rest) = text.split_once("NOTAFTER=")?;
    let digits = rest.get(..14)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) || rest.as_bytes().get(14) != Some(&b'Z') {
        return None;
    }
    let civil = DateTime::strptime("%Y%m%d%H%M%S", digits).ok()?;
    civil.to_zoned(TimeZone::UTC).ok().map(|z| z.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let start: Timestamp = "2026-11-04T08:00:00Z".parse().unwrap();
        let end: Timestamp = "2026-11-04T14:00:00Z".parse().unwrap();
        let text = render("alice", "abc", start, end, 6);
        assert!(text.contains("alice ALL=(ALL:ALL) NOTAFTER=20261104140000Z NOPASSWD: ALL\n"));
        assert_eq!(expiry(&text), Some(end));
    }

    #[test]
    fn unreadable() {
        for text in [
            "alice ALL=(ALL) NOPASSWD: ALL",
            "NOTAFTER=2026110414000Z",
            "NOTAFTER=20261304140000Z",
        ] {
            assert_eq!(expiry(text), None, "{text}");
        }
    }
}
