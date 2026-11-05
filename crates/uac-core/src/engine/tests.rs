//! Engine tests, run unprivileged in a temporary tree.
//! The real `visudo.ws` parses the generated rule when it is installed.

use std::cell::{Cell, RefCell};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

const REAL_VISUDO: &str = "/usr/sbin/visudo.ws";
const USER: &str = "alice";

fn t0() -> Timestamp {
    "2026-11-04T08:00:00Z".parse().unwrap()
}

struct Fake {
    now: Cell<Timestamp>,
    logs: RefCell<Vec<String>>,
    audit_broken: Cell<bool>,
    next_id: Cell<u32>,
}

impl System for &Fake {
    fn now(&self) -> Timestamp {
        self.now.get()
    }
    fn log(&self, message: &str) -> Result<(), String> {
        if self.audit_broken.get() {
            return Err("journal down".into());
        }
        self.logs.borrow_mut().push(message.to_string());
        Ok(())
    }
    fn user_name(&self, uid: u32) -> Option<String> {
        (uid == 1000).then(|| USER.to_string())
    }
    fn grant_id(&self) -> Result<String, UacError> {
        self.next_id.set(self.next_id.get() + 1);
        Ok(format!("{:016x}", self.next_id.get()))
    }
    fn chown_root(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

struct Tree {
    base: PathBuf,
    paths: Paths,
    fake: Fake,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl Tree {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir().join(format!(
            "uac-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(base.join("sudoers.d")).unwrap();
        fs::create_dir_all(base.join("bin")).unwrap();
        File::create(base.join("bin/sudo.ws")).unwrap();
        File::create(base.join("bin/sudo-rs")).unwrap();
        symlink(base.join("bin/sudo.ws"), base.join("bin/sudo")).unwrap();
        fs::write(
            base.join("uac-linux.conf"),
            format!("user={USER}\ndefault_hours=6\nmax_hours=24\n"),
        )
        .unwrap();
        let visudo = if Path::new(REAL_VISUDO).exists() {
            REAL_VISUDO
        } else {
            "/bin/true"
        };
        let paths = Paths {
            conf: base.join("uac-linux.conf"),
            sudoers_dir: base.join("sudoers.d"),
            rule_name: "90-uac-linux".into(),
            state: base.join("state/state.json"),
            lock: base.join("lock"),
            visudo: visudo.into(),
            sudo_link: base.join("bin/sudo"),
            sudo_ws: base.join("bin/sudo.ws"),
        };
        let fake = Fake {
            now: Cell::new(t0()),
            logs: RefCell::default(),
            audit_broken: Cell::new(false),
            next_id: Cell::new(0),
        };
        Self { base, paths, fake }
    }

    fn engine(&self) -> Engine<&Fake> {
        Engine::new(self.paths.clone(), &self.fake)
    }

    fn grant(&self, hours: &str) -> Result<State, UacError> {
        self.engine()
            .grant(hours, |key| (key == "PKEXEC_UID").then(|| "1000".to_string()))
    }

    fn rule_text(&self) -> String {
        fs::read_to_string(self.paths.rule()).unwrap()
    }

    fn use_sudo_rs(&self) {
        fs::remove_file(&self.paths.sudo_link).unwrap();
        symlink(self.base.join("bin/sudo-rs"), &self.paths.sudo_link).unwrap();
    }

    fn logs(&self) -> Vec<String> {
        self.fake.logs.borrow().clone()
    }
}

// ------------------------------------------------------------------ grant

#[test]
fn grant_writes_rule_with_notafter_and_state() {
    let t = Tree::new();
    let state = t.grant("6").unwrap();
    assert!(
        t.rule_text()
            .contains("alice ALL=(ALL:ALL) NOTAFTER=20261104140000Z NOPASSWD: ALL")
    );
    let mode = fs::metadata(t.paths.rule()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o440);
    assert!(state.active);
    assert_eq!(state.expires_at, Some("2026-11-04T14:00:00Z".parse().unwrap()));
    assert_eq!(t.engine().read_state(), state);
    assert!(!t.paths.rule_tmp().exists());
    let logs = t.logs();
    assert!(logs[0].starts_with("event=grant_begin"));
    assert!(logs[1].starts_with("event=grant_active"));
}

#[test]
fn real_visudo_accepts_generated_rule() {
    if !Path::new(REAL_VISUDO).exists() {
        eprintln!("skipped: classic visudo not installed");
        return;
    }
    let t = Tree::new();
    t.grant("24").unwrap();
    let out = Command::new(REAL_VISUDO)
        .args(["-c", "-f"])
        .arg(t.paths.rule())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("parsed OK"), "{text}");
}

#[test]
fn hours_bounds() {
    let t = Tree::new();
    for bad in ["0", "25", "-1", "1.5", "six", "", "99999999999"] {
        assert!(t.grant(bad).is_err(), "{bad:?}");
    }
    assert!(!t.paths.rule().exists());
    assert!(t.logs().is_empty());
}

#[test]
fn max_hours_from_config() {
    let t = Tree::new();
    fs::write(&t.paths.conf, format!("user={USER}\ndefault_hours=2\nmax_hours=4\n")).unwrap();
    assert_eq!(t.grant("5"), Err(UacError::HoursOutOfRange { min: 1, max: 4 }));
    t.grant("4").unwrap();
}

#[test]
fn only_configured_user() {
    let t = Tree::new();
    fs::write(&t.paths.conf, "user=someoneelse\n").unwrap();
    assert!(matches!(t.grant("1"), Err(UacError::WrongUser { .. })));
    assert!(!t.paths.rule().exists());
}

#[test]
fn unknown_caller_refused() {
    let t = Tree::new();
    assert_eq!(t.engine().grant("1", |_| None), Err(UacError::UnknownCaller));
    assert_eq!(
        t.engine().grant("1", |_| Some("x".into())),
        Err(UacError::UnknownCaller)
    );
    assert_eq!(
        t.engine().grant("1", |_| Some("4242".into())),
        Err(UacError::NoAccount { uid: 4242 })
    );
}

#[test]
fn refused_while_sudo_rs_active() {
    let t = Tree::new();
    t.use_sudo_rs();
    assert_eq!(t.grant("1"), Err(UacError::SudoRsActive));
    assert!(!t.paths.rule().exists());
}

#[test]
fn audit_failure_fails_closed() {
    let t = Tree::new();
    t.fake.audit_broken.set(true);
    assert!(matches!(t.grant("1"), Err(UacError::AuditUnavailable { .. })));
    assert!(!t.paths.rule().exists());
    assert!(!t.paths.rule_tmp().exists());
}

#[test]
fn visudo_rejection_leaves_nothing() {
    let mut t = Tree::new();
    t.paths.visudo = "/bin/false".into();
    assert!(matches!(t.grant("1"), Err(UacError::VisudoRejected { .. })));
    assert!(!t.paths.rule().exists());
    assert!(!t.paths.rule_tmp().exists());
}

#[test]
fn regrant_replaces_and_records_previous() {
    let t = Tree::new();
    let first = t.grant("1").unwrap();
    t.fake.now.set(t0() + SignedDuration::from_mins(30));
    let second = t.grant("12").unwrap();
    assert_eq!(second.replaces, first.grant_id);
    assert!(t.rule_text().contains("NOTAFTER=20261104203000Z"));
}

// ------------------------------------------------------------------ revoke and cleanup

#[test]
fn revoke_removes_rule() {
    let t = Tree::new();
    let granted = t.grant("6").unwrap();
    let state = t.engine().revoke(EndReason::User).unwrap();
    assert!(!t.paths.rule().exists());
    assert!(!state.active);
    assert_eq!(state.last_grant_id, granted.grant_id);
    assert_eq!(state.end_reason, Some(EndReason::User));
}

#[test]
fn revoke_when_off_is_quiet() {
    let t = Tree::new();
    t.engine().revoke(EndReason::User).unwrap();
    assert!(t.logs().is_empty());
}

#[test]
fn revoke_survives_audit_failure() {
    let t = Tree::new();
    t.grant("1").unwrap();
    t.fake.audit_broken.set(true);
    t.engine().revoke(EndReason::User).unwrap();
    assert!(!t.paths.rule().exists());
}

#[test]
fn cleanup_keeps_live_grant() {
    let t = Tree::new();
    t.grant("6").unwrap();
    t.fake.now.set(t0() + SignedDuration::from_mins(5 * 60 + 59));
    assert!(t.engine().cleanup().unwrap().active);
    assert!(t.paths.rule().exists());
}

#[test]
fn cleanup_removes_expired() {
    let t = Tree::new();
    t.grant("6").unwrap();
    t.fake.now.set(t0() + SignedDuration::from_hours(6));
    assert_eq!(t.engine().cleanup().unwrap().end_reason, Some(EndReason::Expired));
    assert!(!t.paths.rule().exists());
}

#[test]
fn cleanup_removes_rule_when_sudo_rs_returns() {
    let t = Tree::new();
    t.grant("6").unwrap();
    t.use_sudo_rs();
    assert_eq!(t.engine().cleanup().unwrap().end_reason, Some(EndReason::SudoRsActive));
    assert!(!t.paths.rule().exists());
}

#[test]
fn cleanup_removes_unreadable_rule() {
    let t = Tree::new();
    fs::write(t.paths.rule(), "alice ALL=(ALL) NOPASSWD: ALL\n").unwrap();
    assert_eq!(
        t.engine().cleanup().unwrap().end_reason,
        Some(EndReason::RuleUnreadable)
    );
    assert!(!t.paths.rule().exists());
}

#[test]
fn cleanup_removes_rule_that_state_does_not_know() {
    let t = Tree::new();
    t.grant("6").unwrap();
    fs::write(&t.paths.state, r#"{"active": false}"#).unwrap();
    assert_eq!(t.engine().cleanup().unwrap().end_reason, Some(EndReason::StateMismatch));
    assert!(!t.paths.rule().exists());
}

#[test]
fn cleanup_clears_state_when_rule_gone() {
    let t = Tree::new();
    t.grant("6").unwrap();
    fs::remove_file(t.paths.rule()).unwrap();
    assert_eq!(t.engine().cleanup().unwrap().end_reason, Some(EndReason::RuleMissing));
}

#[test]
fn bad_config() {
    let t = Tree::new();
    for text in [
        "",
        "user=Root!\n",
        "user=alice\nmax_hours=48\n",
        "user=alice\ndefault_hours=x\n",
    ] {
        fs::write(&t.paths.conf, text).unwrap();
        assert!(Config::load(&t.paths.conf).is_err(), "{text:?}");
    }
    fs::remove_file(&t.paths.conf).unwrap();
    assert!(matches!(
        Config::load(&t.paths.conf),
        Err(UacError::ConfigMissing { .. })
    ));
}
