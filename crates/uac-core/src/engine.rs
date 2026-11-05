//! Grant, revoke and clean up. Runs as root inside `uac-helper`.
//!
//! Order matters and fails closed: the rule is written to a temporary file,
//! checked with `visudo`, audited, and only then renamed into place. A grant
//! is refused if any step fails. Revocation always reaches OFF, even when
//! auditing fails.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::process::Command;

use jiff::{SignedDuration, Timestamp};

use crate::config::Config;
use crate::paths::Paths;
use crate::state::{EndReason, State};
use crate::{MIN_HOURS, UacError, VERSION, rule};

/// Everything the engine needs from the outside world, so tests can fake it.
pub trait System {
    fn now(&self) -> Timestamp;
    /// Write one audit record. An error refuses a grant.
    fn log(&self, message: &str) -> Result<(), String>;
    fn user_name(&self, uid: u32) -> Option<String>;
    /// A fresh 16-hex-digit grant ID.
    fn grant_id(&self) -> Result<String, UacError>;
    /// Make `path` owned by root (a no-op in tests, which run unprivileged).
    fn chown_root(&self, path: &Path) -> std::io::Result<()>;
}

/// The real system: the journal, NSS and `/dev/urandom`.
pub struct RealSystem;

impl System for RealSystem {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    fn log(&self, message: &str) -> Result<(), String> {
        // authpriv, next to sudo's own records.
        let status = Command::new("/usr/bin/logger")
            .args(["-t", crate::TAG, "-p", "authpriv.notice", "--", message])
            .status()
            .map_err(|e| e.to_string())?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| format!("logger exited with {status}"))
    }

    fn user_name(&self, uid: u32) -> Option<String> {
        // getent goes through NSS, so LDAP and SSSD accounts resolve too.
        let out = Command::new("/usr/bin/getent")
            .args(["passwd", &uid.to_string()])
            .output()
            .ok()?;
        let line = String::from_utf8(out.stdout).ok()?;
        let name = line.split(':').next()?.trim();
        (out.status.success() && !name.is_empty()).then(|| name.to_string())
    }

    fn grant_id(&self) -> Result<String, UacError> {
        let mut bytes = [0u8; 8];
        let mut file = File::open("/dev/urandom").map_err(|e| UacError::io(&e, "/dev/urandom"))?;
        std::io::Read::read_exact(&mut file, &mut bytes).map_err(|e| UacError::io(&e, "/dev/urandom"))?;
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }

    fn chown_root(&self, path: &Path) -> std::io::Result<()> {
        std::os::unix::fs::chown(path, Some(0), Some(0))
    }
}

pub struct Engine<S: System> {
    pub paths: Paths,
    pub system: S,
}

fn io(what: &Path) -> impl Fn(std::io::Error) -> UacError + '_ {
    move |e| UacError::io(&e, &what.display().to_string())
}

impl<S: System> Engine<S> {
    pub fn new(paths: Paths, system: S) -> Self {
        Self { paths, system }
    }

    /// The account that asked: pkexec sets `PKEXEC_UID`, plain sudo `SUDO_UID`.
    pub fn caller(&self, env: impl Fn(&str) -> Option<String>) -> Result<String, UacError> {
        let uid = env("PKEXEC_UID")
            .or_else(|| env("SUDO_UID"))
            .ok_or(UacError::UnknownCaller)?;
        let uid: u32 = uid.parse().map_err(|_| UacError::UnknownCaller)?;
        self.system.user_name(uid).ok_or(UacError::NoAccount { uid })
    }

    pub fn classic_sudo_active(&self) -> bool {
        match (
            fs::canonicalize(&self.paths.sudo_link),
            fs::canonicalize(&self.paths.sudo_ws),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }

    pub fn read_state(&self) -> State {
        State::read(&self.paths.state)
    }

    pub fn grant(&self, hours_arg: &str, env: impl Fn(&str) -> Option<String>) -> Result<State, UacError> {
        let config = Config::load(&self.paths.conf)?;
        let caller = self.caller(env)?;
        if caller != config.user {
            return Err(UacError::WrongUser {
                allowed: config.user,
                caller,
            });
        }
        if hours_arg.is_empty() || !hours_arg.bytes().all(|b| b.is_ascii_digit()) {
            return Err(UacError::HoursNotNumber);
        }
        let range = UacError::HoursOutOfRange {
            min: MIN_HOURS,
            max: config.max_hours,
        };
        let hours: u32 = hours_arg.parse().map_err(|_| range.clone())?;
        if !(MIN_HOURS..=config.max_hours).contains(&hours) {
            return Err(range);
        }
        if !self.classic_sudo_active() {
            return Err(UacError::SudoRsActive);
        }

        let now = self.system.now();
        let start = Timestamp::from_second(now.as_second()).unwrap_or(now);
        let expires = start
            .checked_add(SignedDuration::from_hours(i64::from(hours)))
            .map_err(|e| UacError::Io { detail: e.to_string() })?;
        let grant_id = self.system.grant_id()?;
        let previous = self.read_state();
        let replaces = previous.grant_id.filter(|_| previous.active);

        let tmp = self.paths.rule_tmp();
        self.write_tmp_rule(&rule::render(&config.user, &grant_id, start, expires, hours))?;
        if let Err(err) = self.visudo_check(&tmp) {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
        // Audit before the rule takes effect: no record, no grant.
        let begin = format!(
            "event=grant_begin grant={grant_id} user={user} hours={hours} expires={expires} replaces={r}",
            user = config.user,
            r = replaces.as_deref().unwrap_or("-"),
        );
        if let Err(detail) = self.system.log(&begin) {
            let _ = fs::remove_file(&tmp);
            return Err(UacError::AuditUnavailable { detail });
        }
        fs::rename(&tmp, self.paths.rule()).map_err(io(&tmp))?;
        sync_dir(&self.paths.sudoers_dir);

        let state = State {
            active: true,
            grant_id: Some(grant_id.clone()),
            user: Some(config.user.clone()),
            hours: Some(hours),
            granted_at: Some(start),
            expires_at: Some(expires),
            replaces,
            version: Some(VERSION.into()),
            ..State::default()
        };
        self.write_state(&state)?;
        self.log_quiet(&format!(
            "event=grant_active grant={grant_id} user={} expires={expires}",
            config.user
        ));
        Ok(state)
    }

    pub fn revoke(&self, reason: EndReason) -> Result<State, UacError> {
        let previous = self.read_state();
        let removed = self.remove_rule()?;
        let state = State {
            active: false,
            last_grant_id: previous.grant_id.clone().or(previous.last_grant_id.clone()),
            ended_at: Some(Timestamp::from_second(self.system.now().as_second()).unwrap_or_default()),
            end_reason: Some(reason),
            version: Some(VERSION.into()),
            ..State::default()
        };
        if removed || previous.active {
            self.write_state(&state)?;
            let reason_text = serde_json::to_value(reason)
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            self.log_quiet(&format!(
                "event=revoke grant={} reason={} removed={removed}",
                state.last_grant_id.as_deref().unwrap_or("-"),
                reason_text.as_deref().unwrap_or("-"),
            ));
        }
        Ok(state)
    }

    /// Remove an expired, unreadable or orphaned grant. Safe to run any time.
    pub fn cleanup(&self) -> Result<State, UacError> {
        let state = self.read_state();
        let rule_path = self.paths.rule();
        let text = match fs::read_to_string(&rule_path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return if state.active {
                    self.revoke(EndReason::RuleMissing)
                } else {
                    Ok(state)
                };
            }
            Err(_) => return self.revoke(EndReason::RuleUnreadable),
        };
        let Some(expiry) = rule::expiry(&text) else {
            return self.revoke(EndReason::RuleUnreadable);
        };
        if self.system.now() >= expiry {
            return self.revoke(EndReason::Expired);
        }
        if !self.classic_sudo_active() {
            // sudo-rs cannot parse NOTAFTER; leaving the file would break sudo.
            return self.revoke(EndReason::SudoRsActive);
        }
        if !state.active || state.expires_at != Some(expiry) {
            return self.revoke(EndReason::StateMismatch);
        }
        Ok(state)
    }

    // ------------------------------------------------------------ internals

    fn write_tmp_rule(&self, text: &str) -> Result<(), UacError> {
        let tmp = self.paths.rule_tmp();
        match fs::remove_file(&tmp) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(io(&tmp)(e)),
            _ => {}
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o440)
            .open(&tmp)
            .map_err(io(&tmp))?;
        file.write_all(text.as_bytes()).map_err(io(&tmp))?;
        file.sync_all().map_err(io(&tmp))?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o440)).map_err(io(&tmp))?;
        self.system.chown_root(&tmp).map_err(io(&tmp))
    }

    fn visudo_check(&self, file: &Path) -> Result<(), UacError> {
        let out = Command::new(&self.paths.visudo)
            .args(["-c", "-q", "-f"])
            .arg(file)
            .output()
            .map_err(|e| UacError::VisudoRejected { detail: e.to_string() })?;
        if out.status.success() {
            return Ok(());
        }
        let text = if out.stderr.is_empty() { out.stdout } else { out.stderr };
        Err(UacError::VisudoRejected {
            detail: String::from_utf8_lossy(&text).trim().to_string(),
        })
    }

    fn remove_rule(&self) -> Result<bool, UacError> {
        let mut removed = false;
        for path in [self.paths.rule(), self.paths.rule_tmp()] {
            match fs::remove_file(&path) {
                Ok(()) => removed |= path == self.paths.rule(),
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(io(&path)(e)),
            }
        }
        sync_dir(&self.paths.sudoers_dir);
        Ok(removed)
    }

    fn write_state(&self, state: &State) -> Result<(), UacError> {
        let path = &self.paths.state;
        if let Some(dir) = path.parent() {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o755)
                .create(dir)
                .map_err(io(dir))?;
            // The mode above is masked by the umask; readers need 0755.
            fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).map_err(io(dir))?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut text = serde_json::to_string_pretty(state).map_err(|e| UacError::Io { detail: e.to_string() })?;
        text.push('\n');
        let mut file = File::create(&tmp).map_err(io(&tmp))?;
        file.write_all(text.as_bytes()).map_err(io(&tmp))?;
        file.sync_all().map_err(io(&tmp))?;
        // World-readable so the tray and `uac status` need no root.
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644)).map_err(io(&tmp))?;
        fs::rename(&tmp, path).map_err(io(&tmp))
    }

    /// Logging must never undo a state change.
    fn log_quiet(&self, message: &str) {
        if let Err(err) = self.system.log(message) {
            eprintln!("warning: audit log failed: {err}");
        }
    }
}

fn sync_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}

#[cfg(test)]
mod tests;
