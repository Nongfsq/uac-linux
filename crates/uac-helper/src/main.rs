//! uac-linux root helper: grant, revoke and clean up a time-limited sudo grant.
//!
//! Runs as root, started by pkexec (`grant`, `revoke`) through the fixed
//! entry points in `/usr/local/libexec/uac-linux/`, or by systemd
//! (`cleanup`). It is the only part of uac-linux that touches
//! `/etc/sudoers.d`. Paths are compiled in, never taken from the environment.
//!
//! Output: the new state as JSON on stdout, or `{"error": {...}}` on stdout
//! plus an English message on stderr.

use std::fs::{self, File};
use std::process::ExitCode;

use uac_core::UacError;
use uac_core::client::ErrorReply;
use uac_core::engine::{Engine, RealSystem};
use uac_core::paths::Paths;
use uac_core::state::{EndReason, State};

/// Effective uid, from `/proc/self/status` ("Uid: real effective saved fs").
fn effective_uid() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("Uid:"))?;
    line.split_whitespace().nth(2)?.parse().ok()
}

fn run(args: &[&str]) -> Result<State, UacError> {
    if !matches!(args, ["grant", _] | ["revoke"] | ["cleanup"]) {
        return Err(UacError::Usage);
    }
    if effective_uid() != Some(0) {
        return Err(UacError::NotRoot);
    }
    let engine = Engine::new(Paths::default(), RealSystem);
    if let Some(dir) = engine.paths.lock.parent() {
        fs::create_dir_all(dir).map_err(|e| UacError::io(&e, "lock directory"))?;
    }
    // One helper at a time: the timer's cleanup must not race a grant.
    let lock = File::create(&engine.paths.lock).map_err(|e| UacError::io(&e, "lock file"))?;
    lock.lock().map_err(|e| UacError::io(&e, "lock file"))?;
    match args {
        ["grant", hours] => engine.grant(hours, |key| std::env::var(key).ok()),
        ["revoke"] => engine.revoke(EndReason::User),
        _ => engine.cleanup(),
    }
}

fn main() -> ExitCode {
    let owned: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(state) => {
            println!("{}", serde_json::to_string(&state).unwrap_or_default());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("uac-linux: {error}");
            let usage = error == UacError::Usage;
            println!("{}", serde_json::to_string(&ErrorReply { error }).unwrap_or_default());
            ExitCode::from(if usage { 2 } else { 1 })
        }
    }
}
