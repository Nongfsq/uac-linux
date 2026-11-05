//! uac: time-limited passwordless sudo for this desktop.
//!
//! `uac` grants for the default time, `uac 3` for three hours. A grant lets
//! every process you own run sudo without a password until it expires;
//! sudo re-reads /etc/sudoers.d on every call, so nothing needs a restart.

use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Command, ExitCode};

use clap::{Arg, ArgAction, ArgMatches, value_parser};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use uac_core::client::{self, Action, Outcome};
use uac_core::config::Hints;
use uac_core::paths::{CONF, LIBEXEC, POLICY, STATE, SYMBOLIC_ICONS};
use uac_core::state::State;
use uac_core::{MIN_HOURS, VERSION};
use uac_i18n::{cli as t, doctor as d};

fn command(hints: Hints) -> clap::Command {
    let hours = Arg::new("hours").value_parser(value_parser!(u32)).help(t::on_hours(
        MIN_HOURS,
        hints.max_hours,
        hints.default_hours,
    ));
    clap::Command::new("uac")
        .about(t::about())
        .version(VERSION)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .long("version")
                .short('V')
                .action(ArgAction::Version)
                .help(t::version()),
        )
        .subcommand(
            clap::Command::new("on")
                .visible_alias("grant")
                .about(t::on())
                .arg(hours),
        )
        .subcommand(clap::Command::new("off").visible_alias("revoke").about(t::off()))
        .subcommand(
            clap::Command::new("status").about(t::status()).arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help(t::status_json()),
            ),
        )
        .subcommand(
            clap::Command::new("logs")
                .about(t::logs())
                .arg(Arg::new("since").default_value("24h").help(t::logs_since())),
        )
        .subcommand(clap::Command::new("doctor").about(t::doctor()))
        .subcommand(clap::Command::new("version").about(t::version()))
}

fn main() -> ExitCode {
    uac_i18n::init();
    let hints = Hints::read(Path::new(CONF));
    let mut args: Vec<String> = std::env::args().collect();
    // `uac 3` means `uac on 3`.
    if args
        .get(1)
        .is_some_and(|a| !a.is_empty() && a.bytes().all(|b| b.is_ascii_digit()))
    {
        args.insert(1, "on".into());
    }
    let matches = command(hints).get_matches_from(args);
    let code = match matches.subcommand() {
        None => on(hints.default_hours),
        Some(("on", m)) => on(m.get_one::<u32>("hours").copied().unwrap_or(hints.default_hours)),
        Some(("off", _)) => act(Action::Revoke),
        Some(("status", m)) => status(m.get_flag("json")),
        Some(("logs", m)) => logs(m),
        Some(("doctor", _)) => doctor(),
        _ => {
            println!("uac-linux {VERSION}");
            0
        }
    };
    ExitCode::from(code)
}

fn on(hours: u32) -> u8 {
    act(Action::Grant(hours))
}

fn act(action: Action) -> u8 {
    match client::run(action) {
        Outcome::Done(_) => status(false),
        Outcome::Cancelled => {
            eprintln!("{}", t::cancelled());
            126
        }
        Outcome::Refused(err) => {
            eprintln!("{}", t::failed(&uac_i18n::error(&err)));
            1
        }
        Outcome::Failed(detail) => {
            eprintln!("{}", t::failed(&detail));
            1
        }
    }
}

fn local(at: Timestamp) -> String {
    at.to_zoned(TimeZone::system())
        .strftime("%Y-%m-%d %H:%M %Z")
        .to_string()
}

fn status(as_json: bool) -> u8 {
    let state = State::read(Path::new(STATE));
    let left = state.remaining(Timestamp::now());
    let passwordless = client::sudo_passwordless_now();
    if as_json {
        let mut out = serde_json::to_value(&state).unwrap_or_default();
        if let Some(map) = out.as_object_mut() {
            map.insert("active".into(), left.is_some().into());
            map.insert("remaining_seconds".into(), left.map_or(0, |l| l.as_secs()).into());
            map.insert("sudo_passwordless_now".into(), passwordless.into());
        }
        println!("{out}");
        return 0;
    }
    match (left, state.expires_at) {
        (Some(left), Some(expires)) => println!(
            "{}",
            t::status_on(
                &local(expires),
                &uac_i18n::left(left),
                state.grant_id.as_deref().unwrap_or("-")
            )
        ),
        _ => {
            println!("{}", t::status_off());
            if let (Some(at), Some(reason)) = (state.ended_at, state.end_reason) {
                println!("{}", t::status_last(&local(at), &uac_i18n::end_reason(reason)));
            }
            if passwordless {
                println!("{}", t::status_other_nopasswd());
            }
        }
    }
    0
}

fn logs(m: &ArgMatches) -> u8 {
    let since = m.get_one::<String>("since").map_or("24h", String::as_str);
    let (number, unit) = since.split_at(since.len().saturating_sub(1));
    let unit = match unit {
        "m" => "minutes",
        "h" => "hours",
        "d" => "days",
        _ => "",
    };
    if unit.is_empty() || number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        eprintln!("{}", t::logs_bad_since());
        return 2;
    }
    let status = Command::new("journalctl")
        .args(["-q", "--no-pager", "-t", uac_core::TAG, "-o", "short-iso", "--since"])
        .arg(format!("-{number} {unit}"))
        .status();
    status
        .ok()
        .and_then(|s| s.code())
        .and_then(|c| u8::try_from(c).ok())
        .unwrap_or(1)
}

struct Doctor {
    problems: u32,
    warnings: u32,
}

impl Doctor {
    fn check(&mut self, ok: bool, text: &str) {
        self.report(ok, text, false);
    }
    fn warn(&mut self, ok: bool, text: &str) {
        self.report(ok, text, true);
    }
    fn report(&mut self, ok: bool, text: &str, warn_only: bool) {
        let mark = match (ok, warn_only) {
            (true, _) => "ok  ",
            (false, true) => "warn",
            (false, false) => "FAIL",
        };
        if !ok {
            if warn_only {
                self.warnings += 1
            } else {
                self.problems += 1
            }
        }
        println!("[{mark}] {text}");
    }
}

fn root_owned_not_writable(path: &Path) -> Option<bool> {
    let meta = std::fs::metadata(path).ok()?;
    Some(meta.uid() == 0 && meta.mode() & 0o022 == 0)
}

fn systemctl(verb: &str, unit: &str) -> String {
    Command::new("systemctl")
        .args([verb, unit])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn doctor() -> u8 {
    let mut doc = Doctor {
        problems: 0,
        warnings: 0,
    };
    let sudo = std::fs::canonicalize("/usr/bin/sudo")
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    doc.check(sudo == "/usr/bin/sudo.ws", &d::classic_sudo(&sudo));
    for name in ["uac-helper", "grant", "revoke", "cleanup"] {
        let path = format!("{LIBEXEC}/{name}");
        match root_owned_not_writable(Path::new(&path)) {
            Some(ok) => doc.check(ok, &d::root_owned(&path)),
            None => doc.check(false, &d::exists(&path)),
        }
    }
    doc.check(
        root_owned_not_writable(Path::new(LIBEXEC)).unwrap_or(false),
        &d::root_owned(LIBEXEC),
    );
    doc.check(Path::new(POLICY).exists(), &d::polkit(POLICY));
    doc.check(Path::new(CONF).exists(), &d::config(CONF));
    let icons = ["on", "off"]
        .iter()
        .all(|s| Path::new(&format!("{SYMBOLIC_ICONS}/uac-linux-{s}-symbolic.svg")).exists());
    doc.warn(icons, &d::icons());
    for (unit, verb, want) in [
        ("uac-linux-cleanup.timer", "is-active", "active"),
        ("uac-linux-cleanup.service", "is-enabled", "enabled"),
    ] {
        let now = systemctl(verb, unit);
        doc.check(
            now == want,
            &d::unit(unit, want, if now.is_empty() { "?" } else { &now }),
        );
    }
    let state = State::read(Path::new(STATE));
    let left = state.remaining(Timestamp::now());
    doc.warn(!state.active || left.is_some(), &d::state_agrees());
    if left.is_none() && client::sudo_passwordless_now() {
        doc.warn(false, &d::other_nopasswd());
    }
    let groups = Command::new("id")
        .arg("-nG")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
    if groups.is_ok_and(|g| g.split_whitespace().any(|x| x == "docker")) {
        doc.warn(false, &d::docker());
    }
    println!("\n{}", d::summary(doc.problems, doc.warnings));
    match (doc.problems, doc.warnings) {
        (0, 0) => 0,
        (0, _) => 3,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_parses() {
        uac_i18n::init_with(&["en-US"]);
        let hints = Hints {
            default_hours: 6,
            max_hours: 24,
        };
        let m = command(hints).try_get_matches_from(["uac", "on", "3"]).unwrap();
        assert_eq!(m.subcommand_matches("on").unwrap().get_one::<u32>("hours"), Some(&3));
        let m = command(hints)
            .try_get_matches_from(["uac", "status", "--json"])
            .unwrap();
        assert!(m.subcommand_matches("status").unwrap().get_flag("json"));
        assert!(command(hints).try_get_matches_from(["uac", "on", "six"]).is_err());
        command(hints).debug_assert();
    }
}
