//! uac-tray: top-bar indicator for uac-linux.
//!
//! A StatusNotifierItem over D-Bus, which Ubuntu's AppIndicator extension
//! shows in the GNOME top bar. The raccoon is asleep while sudo asks for a
//! password and awake with a green dot while a grant is active. The top bar
//! shows no time (a bare "5:42" beside the clock reads like a time of day);
//! the expiry is the first line of the menu.

use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};
use notify_rust::{Notification, Urgency};
use uac_core::client::{self, Action, Outcome};
use uac_core::config::Hints;
use uac_core::paths::{CONF, STATE, SYMBOLIC_ICONS};
use uac_core::state::{EndReason, State};
use uac_i18n::tray as t;

const CHOICES: [u32; 6] = [1, 2, 4, 6, 12, 24];
const REFRESH: Duration = Duration::from_secs(15);
const WARN_BEFORE: SignedDuration = SignedDuration::from_mins(10);
const APP_ICON: &str = "uac-linux";

/// Our icon if it is installed, else a stock one, so the tray never shows blank.
fn icon(name: &str, fallback: &str) -> String {
    let installed = Path::new(SYMBOLIC_ICONS).join(format!("{name}.svg")).exists();
    if installed { name } else { fallback }.to_string()
}

enum Cmd {
    Grant(u32),
    Revoke,
    Logs,
    Tick,
}

struct UacTray {
    state: State,
    now: Timestamp,
    hints: Hints,
    busy: bool,
    icon_on: String,
    icon_off: String,
    tx: Sender<Cmd>,
}

impl UacTray {
    fn left(&self) -> Option<SignedDuration> {
        self.state.remaining(self.now)
    }

    fn header(&self) -> String {
        match (self.left(), self.state.expires_at) {
            (Some(left), Some(at)) => t::header_on(
                &uac_i18n::until(at, self.now, &TimeZone::system()),
                &uac_i18n::left(left),
            ),
            _ => t::header_off(),
        }
    }

    fn item(&self, label: String, enabled: bool, cmd: fn() -> Cmd) -> MenuItem<Self> {
        StandardItem {
            label,
            enabled,
            activate: Box::new(move |tray: &mut Self| {
                let _ = tray.tx.send(cmd());
            }),
            ..Default::default()
        }
        .into()
    }
}

impl ksni::Tray for UacTray {
    fn id(&self) -> String {
        "uac-linux".into()
    }

    fn title(&self) -> String {
        "uac-linux".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::SystemServices
    }

    fn icon_name(&self) -> String {
        if self.left().is_some() {
            &self.icon_on
        } else {
            &self.icon_off
        }
        .clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: t::tooltip(self.left().is_some()),
            description: self.header(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let active = self.left().is_some();
        let mut menu = vec![
            StandardItem {
                label: self.header(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];
        for hours in CHOICES.into_iter().filter(|h| *h <= self.hints.max_hours) {
            menu.push(
                StandardItem {
                    label: t::grant(hours, hours == self.hints.default_hours, active),
                    enabled: !self.busy,
                    activate: Box::new(move |tray: &mut Self| {
                        let _ = tray.tx.send(Cmd::Grant(hours));
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }
        menu.push(MenuItem::Separator);
        menu.push(self.item(t::revoke(), active && !self.busy, || Cmd::Revoke));
        menu.push(MenuItem::Separator);
        menu.push(self.item(t::logs(), true, || Cmd::Logs));
        menu.push(
            StandardItem {
                label: t::hide(),
                activate: Box::new(|_: &mut Self| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        );
        menu
    }
}

// ------------------------------------------------------------------ notifications

/// What the tray remembers between refreshes to decide on notifications.
#[derive(Debug, Default, Clone, PartialEq)]
struct Watch {
    /// Grant that was in force at the last refresh.
    seen: Option<String>,
    /// Grant we already warned about.
    warned: Option<String>,
}

#[derive(Debug, PartialEq)]
enum Note {
    Expiring,
    Expired,
}

fn observe(watch: &Watch, state: &State, now: Timestamp) -> (Watch, Option<Note>) {
    match state.remaining(now) {
        Some(left) => {
            let gid = state.grant_id.clone();
            let warn = left <= WARN_BEFORE && watch.warned != gid;
            let warned = if warn { gid.clone() } else { watch.warned.clone() };
            (Watch { seen: gid, warned }, warn.then_some(Note::Expiring))
        }
        None => {
            let ended = watch.seen.is_some() && state.end_reason != Some(EndReason::User);
            (
                Watch {
                    seen: None,
                    warned: watch.warned.clone(),
                },
                ended.then_some(Note::Expired),
            )
        }
    }
}

fn notify(summary: &str, body: &str, urgent: bool) {
    let mut note = Notification::new();
    note.appname("uac-linux").icon(APP_ICON).summary(summary).body(body);
    if urgent {
        note.urgency(Urgency::Critical);
    }
    let _ = note.show();
}

// ------------------------------------------------------------------ actions

/// Terminals to try for the audit log, newest GNOME default first.
const TERMINALS: [(&str, &[&str]); 4] = [
    ("ptyxis", &["--new-window", "--"]),
    ("kgx", &["--"]),
    ("gnome-terminal", &["--"]),
    ("x-terminal-emulator", &["-e"]),
];

fn open_logs() {
    let script = "uac logs 7d; echo; printf '%s' \"$1\"; read -r _";
    for (terminal, prefix) in TERMINALS {
        let started = Command::new(terminal)
            .args(prefix)
            .args(["sh", "-c", script, "sh", &t::press_enter()])
            .spawn()
            .is_ok();
        if started {
            return;
        }
    }
    notify(
        &t::notify_failed(),
        "ptyxis, kgx, gnome-terminal, x-terminal-emulator",
        true,
    );
}

fn perform(handle: &ksni::blocking::Handle<UacTray>, action: Action) {
    handle.update(|tray| tray.busy = true);
    let outcome = client::run(action);
    handle.update(|tray| tray.busy = false);
    match outcome {
        Outcome::Done(state) => {
            let header = handle.update(|tray| {
                tray.state = state;
                tray.now = Timestamp::now();
                tray.header()
            });
            let summary = match action {
                Action::Grant(hours) => t::notify_on(hours),
                Action::Revoke => t::notify_off(),
            };
            notify(&summary, &header.unwrap_or_default(), false);
        }
        Outcome::Cancelled => {}
        Outcome::Refused(err) => notify(&t::notify_failed(), &uac_i18n::error(&err), true),
        Outcome::Failed(detail) => notify(&t::notify_failed(), &detail, true),
    }
}

fn main() {
    // One icon per session: a second copy exits quietly.
    let lock_dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, Into::into);
    let Ok(lock) = std::fs::File::create(Path::new(&lock_dir).join("uac-tray.lock")) else {
        eprintln!("uac-tray: cannot create its lock file in {}", Path::new(&lock_dir).display());
        std::process::exit(1);
    };
    if lock.try_lock().is_err() {
        return;
    }
    uac_i18n::init();
    let (tx, rx) = mpsc::channel();
    let now = Timestamp::now();
    let state = State::read(Path::new(STATE));
    let mut watch = Watch {
        seen: state.remaining(now).and(state.grant_id.clone()),
        warned: None,
    };
    let tray = UacTray {
        state,
        now,
        hints: Hints::read(Path::new(CONF)),
        busy: false,
        icon_on: icon("uac-linux-on-symbolic", "changes-allow-symbolic"),
        icon_off: icon("uac-linux-off-symbolic", "changes-prevent-symbolic"),
        tx: tx.clone(),
    };
    // At login the tray may start before the shell's indicator host.
    let handle = match tray.assume_sni_available(true).spawn() {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("uac-tray: {err}");
            std::process::exit(1);
        }
    };
    thread::spawn(move || {
        while tx.send(Cmd::Tick).is_ok() {
            thread::sleep(REFRESH);
        }
    });
    for cmd in rx {
        match cmd {
            Cmd::Grant(hours) => perform(&handle, Action::Grant(hours)),
            Cmd::Revoke => perform(&handle, Action::Revoke),
            Cmd::Logs => open_logs(),
            Cmd::Tick => {
                let now = Timestamp::now();
                let state = State::read(Path::new(STATE));
                let (next, note) = observe(&watch, &state, now);
                watch = next;
                let until = state.expires_at.map(|at| uac_i18n::until(at, now, &TimeZone::system()));
                handle.update(|tray| {
                    tray.state = state;
                    tray.now = now;
                    tray.hints = Hints::read(Path::new(CONF));
                });
                match note {
                    Some(Note::Expiring) => notify(
                        &t::notify_expiring(),
                        &t::notify_expiring_body(&until.unwrap_or_default()),
                        false,
                    ),
                    Some(Note::Expired) => notify(&t::notify_expired(), &t::notify_expired_body(), false),
                    None => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(gid: &str, expires: &str) -> State {
        State {
            active: true,
            grant_id: Some(gid.into()),
            expires_at: Some(expires.parse().unwrap()),
            ..State::default()
        }
    }

    #[test]
    fn warns_once_before_expiry_then_reports_the_end() {
        let state = active("g1", "2026-11-04T14:00:00Z");
        let early: Timestamp = "2026-11-04T13:00:00Z".parse().unwrap();
        let late: Timestamp = "2026-11-04T13:55:00Z".parse().unwrap();
        let (w, note) = observe(&Watch::default(), &state, early);
        assert_eq!(note, None);
        let (w, note) = observe(&w, &state, late);
        assert_eq!(note, Some(Note::Expiring));
        let (w, note) = observe(&w, &state, late);
        assert_eq!(note, None);
        let ended = State {
            end_reason: Some(EndReason::Expired),
            ..State::default()
        };
        assert_eq!(observe(&w, &ended, late).1, Some(Note::Expired));
    }

    #[test]
    fn no_expiry_notice_after_turning_off() {
        let watch = Watch {
            seen: Some("g1".into()),
            warned: None,
        };
        let off = State {
            end_reason: Some(EndReason::User),
            ..State::default()
        };
        assert_eq!(observe(&watch, &off, Timestamp::now()).1, None);
    }
}
