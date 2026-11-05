use std::collections::BTreeSet;
use std::sync::Mutex;

use super::*;

/// The loader is global, so tests that switch language run one at a time.
static LANGUAGE: Mutex<()> = Mutex::new(());

fn message_ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.split_once(" ="))
        .map(|(id, _)| id)
        .filter(|id| id.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
        .filter(|id| {
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .map(String::from)
        .collect()
}

#[test]
fn every_language_has_exactly_the_fallback_messages() {
    let fallback = Localizations::get("en-US/uac-i18n.ftl").expect("fallback file");
    let expected = message_ids(std::str::from_utf8(&fallback.data).unwrap());
    assert!(expected.len() > 50, "parsed {} ids", expected.len());
    let languages = available();
    assert!(languages.len() >= 2, "{languages:?}");
    for tag in languages {
        let file = Localizations::get(&format!("{tag}/uac-i18n.ftl")).expect("translation file");
        let found = message_ids(std::str::from_utf8(&file.data).unwrap());
        let missing: Vec<_> = expected.difference(&found).collect();
        let extra: Vec<_> = found.difference(&expected).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{tag}: missing {missing:?}, extra {extra:?}"
        );
    }
}

#[test]
fn renders_english_and_chinese() {
    let _guard = LANGUAGE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    init_with(&["en-US"]);
    assert_eq!(tray::grant(1, false, false), "Turn on for 1 hour");
    assert_eq!(tray::grant(6, true, false), "Turn on for 6 hours (default)");
    assert_eq!(left(SignedDuration::from_mins(342)), "5 h 42 min");
    assert_eq!(
        error(&UacError::HoursOutOfRange { min: 1, max: 4 }),
        "Hours must be between 1 and 4."
    );
    init_with(&["zh-CN"]);
    assert_eq!(tray::grant(6, true, true), "改为从现在起 6 小时（默认）");
    assert_eq!(left(SignedDuration::from_mins(342)), "5 小时 42 分");
    init_with(&["fr-FR"]);
    assert_eq!(tray::revoke(), "Turn off now (sudo asks for a password again)");
}

#[test]
fn until_says_tomorrow_across_midnight() {
    let _guard = LANGUAGE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    init_with(&["en-US"]);
    let tz = TimeZone::fixed(jiff::tz::offset(-8));
    let now: Timestamp = "2026-11-04T20:00:00Z".parse().unwrap(); // 12:00 local
    let today: Timestamp = "2026-11-05T02:30:00Z".parse().unwrap(); // 18:30 local
    let tomorrow: Timestamp = "2026-11-05T09:30:00Z".parse().unwrap(); // 01:30 local, next day
    assert_eq!(until(today, now, &tz), "18:30");
    assert_eq!(until(tomorrow, now, &tz), "tomorrow 01:30");
}
