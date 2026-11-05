//! Every user-facing string of uac-linux, in the user's language.
//!
//! Text lives in `i18n/<language>/uac-i18n.ftl` (Fluent), not in code. Each
//! message has a typed function here, and `fl!` checks at compile time that
//! the message and its arguments exist in the fallback language (en-US).
//! A test checks that every other language has exactly the same messages.
//!
//! To add a language: copy `i18n/en-US/uac-i18n.ftl` to `i18n/<tag>/`,
//! translate the values, and run `cargo test -p uac-i18n`.

use std::sync::LazyLock;

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use rust_embed::RustEmbed;
use uac_core::UacError;
use uac_core::state::EndReason;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    // Unicode isolation marks show up as garbage in terminals.
    loader.set_use_isolating(false);
    let fallback = loader.fallback_language().clone();
    let _ = loader.load_languages(&Localizations, &[fallback]);
    loader
});

macro_rules! fl {
    ($id:literal) => {
        i18n_embed_fl::fl!(LOADER, $id)
    };
    ($id:literal, $($args:tt)*) => {
        i18n_embed_fl::fl!(LOADER, $id, $($args)*)
    };
}

/// Pick languages from the desktop settings (`LANGUAGE`, `LC_ALL`,
/// `LC_MESSAGES`, `LANG`). English is the fallback for anything missing.
pub fn init() {
    let requested = DesktopLanguageRequester::requested_languages();
    let _ = i18n_embed::select(&*LOADER, &Localizations, &requested);
    // Selecting loads new bundles, which isolate placeables again.
    LOADER.set_use_isolating(false);
}

/// Use exactly these languages (tests, and `LANGUAGE`-style overrides).
pub fn init_with(tags: &[&str]) {
    let requested: Vec<_> = tags.iter().filter_map(|t| t.parse().ok()).collect();
    let _ = i18n_embed::select(&*LOADER, &Localizations, &requested);
    LOADER.set_use_isolating(false);
}

/// Languages that have a translation file.
pub fn available() -> Vec<String> {
    LOADER
        .available_languages(&Localizations)
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

// ------------------------------------------------------------------ shared

/// Whole hours and minutes left, rounded down.
pub fn left(duration: SignedDuration) -> String {
    let total = duration.as_secs().max(0) / 60;
    let (hours, minutes): (i64, i64) = (total / 60, total % 60);
    fl!("duration-left", hours = hours, minutes = minutes)
}

/// Local end time: "14:30" today, "tomorrow 01:30" otherwise.
pub fn until(at: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let at = at.to_zoned(tz.clone());
    let time = at.strftime("%H:%M").to_string();
    if at.date() == now.to_zoned(tz.clone()).date() {
        fl!("day-today", time = time)
    } else {
        fl!("day-tomorrow", time = time)
    }
}

pub fn end_reason(reason: EndReason) -> String {
    match reason {
        EndReason::User => fl!("end-reason-user"),
        EndReason::Expired => fl!("end-reason-expired"),
        EndReason::RuleMissing => fl!("end-reason-rule-missing"),
        EndReason::RuleUnreadable => fl!("end-reason-rule-unreadable"),
        EndReason::SudoRsActive => fl!("end-reason-sudo-rs-active"),
        EndReason::StateMismatch => fl!("end-reason-state-mismatch"),
    }
}

pub fn error(err: &UacError) -> String {
    match err {
        UacError::ConfigMissing { path } => fl!("error-config-missing", path = path.as_str()),
        UacError::ConfigBadUser { path } => fl!("error-config-bad-user", path = path.as_str()),
        UacError::ConfigBadNumber { path, key } => {
            fl!("error-config-bad-number", path = path.as_str(), key = key.as_str())
        }
        UacError::ConfigBadRange { path } => fl!("error-config-bad-range", path = path.as_str()),
        UacError::UnknownCaller => fl!("error-unknown-caller"),
        UacError::NoAccount { uid } => {
            let uid = i64::from(*uid);
            fl!("error-no-account", uid = uid)
        }
        UacError::WrongUser { allowed, caller } => {
            fl!("error-wrong-user", allowed = allowed.as_str(), caller = caller.as_str())
        }
        UacError::HoursNotNumber => fl!("error-hours-not-number"),
        UacError::HoursOutOfRange { min, max } => {
            let (min, max) = (i64::from(*min), i64::from(*max));
            fl!("error-hours-out-of-range", min = min, max = max)
        }
        UacError::SudoRsActive => fl!("error-sudo-rs-active"),
        UacError::VisudoRejected { detail } => fl!("error-visudo-rejected", detail = detail.as_str()),
        UacError::AuditUnavailable { detail } => fl!("error-audit-unavailable", detail = detail.as_str()),
        UacError::Io { detail } => fl!("error-io", detail = detail.as_str()),
        UacError::NotRoot => fl!("error-not-root"),
        UacError::Usage => fl!("error-usage"),
    }
}

// ------------------------------------------------------------------ command line

pub mod cli {
    use super::LOADER;

    pub fn about() -> String {
        fl!("cli-about")
    }
    pub fn on() -> String {
        fl!("cli-on")
    }
    pub fn on_hours(min: u32, max: u32, default: u32) -> String {
        fl!("cli-on-hours", min = min, max = max, default = default)
    }
    pub fn off() -> String {
        fl!("cli-off")
    }
    pub fn status() -> String {
        fl!("cli-status")
    }
    pub fn status_json() -> String {
        fl!("cli-status-json")
    }
    pub fn logs() -> String {
        fl!("cli-logs")
    }
    pub fn logs_since() -> String {
        fl!("cli-logs-since")
    }
    pub fn doctor() -> String {
        fl!("cli-doctor")
    }
    pub fn version() -> String {
        fl!("cli-version")
    }
    pub fn status_on(until: &str, left: &str, grant: &str) -> String {
        fl!("cli-status-on", until = until, left = left, grant = grant)
    }
    pub fn status_off() -> String {
        fl!("cli-status-off")
    }
    pub fn status_last(at: &str, reason: &str) -> String {
        fl!("cli-status-last", at = at, reason = reason)
    }
    pub fn status_other_nopasswd() -> String {
        fl!("cli-status-other-nopasswd")
    }
    pub fn cancelled() -> String {
        fl!("cli-cancelled")
    }
    pub fn failed(detail: &str) -> String {
        fl!("cli-failed", detail = detail)
    }
    pub fn logs_bad_since() -> String {
        fl!("cli-logs-bad-since")
    }
}

// ------------------------------------------------------------------ uac doctor

pub mod doctor {
    use super::LOADER;

    pub fn classic_sudo(target: &str) -> String {
        fl!("doctor-classic-sudo", target = target)
    }
    pub fn root_owned(path: &str) -> String {
        fl!("doctor-root-owned", path = path)
    }
    pub fn exists(path: &str) -> String {
        fl!("doctor-exists", path = path)
    }
    pub fn polkit(path: &str) -> String {
        fl!("doctor-polkit", path = path)
    }
    pub fn config(path: &str) -> String {
        fl!("doctor-config", path = path)
    }
    pub fn icons() -> String {
        fl!("doctor-icons")
    }
    pub fn unit(unit: &str, want: &str, now: &str) -> String {
        fl!("doctor-unit", unit = unit, want = want, now = now)
    }
    pub fn state_agrees() -> String {
        fl!("doctor-state-agrees")
    }
    pub fn other_nopasswd() -> String {
        fl!("doctor-other-nopasswd")
    }
    pub fn docker() -> String {
        fl!("doctor-docker")
    }
    pub fn summary(problems: u32, warnings: u32) -> String {
        fl!("doctor-summary", problems = problems, warnings = warnings)
    }
}

// ------------------------------------------------------------------ top bar

pub mod tray {
    use super::LOADER;

    pub fn tooltip(active: bool) -> String {
        if active {
            fl!("tray-tooltip-on")
        } else {
            fl!("tray-tooltip-off")
        }
    }
    pub fn header_on(until: &str, left: &str) -> String {
        fl!("tray-header-on", until = until, left = left)
    }
    pub fn header_off() -> String {
        fl!("tray-header-off")
    }
    /// Menu item that grants `hours`; `replace` while a grant is active.
    pub fn grant(hours: u32, is_default: bool, replace: bool) -> String {
        match (replace, is_default) {
            (false, false) => fl!("tray-grant", hours = hours),
            (false, true) => fl!("tray-grant-default", hours = hours),
            (true, false) => fl!("tray-replace", hours = hours),
            (true, true) => fl!("tray-replace-default", hours = hours),
        }
    }
    pub fn revoke() -> String {
        fl!("tray-revoke")
    }
    pub fn logs() -> String {
        fl!("tray-logs")
    }
    pub fn hide() -> String {
        fl!("tray-hide")
    }
    pub fn press_enter() -> String {
        fl!("tray-press-enter")
    }
    pub fn notify_on(hours: u32) -> String {
        fl!("notify-on", hours = hours)
    }
    pub fn notify_off() -> String {
        fl!("notify-off")
    }
    pub fn notify_expiring() -> String {
        fl!("notify-expiring")
    }
    pub fn notify_expiring_body(until: &str) -> String {
        fl!("notify-expiring-body", until = until)
    }
    pub fn notify_expired() -> String {
        fl!("notify-expired")
    }
    pub fn notify_expired_body() -> String {
        fl!("notify-expired-body")
    }
    pub fn notify_failed() -> String {
        fl!("notify-failed")
    }
}

#[cfg(test)]
mod tests;
