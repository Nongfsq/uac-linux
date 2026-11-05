//! Every filesystem location uac-linux uses.
//!
//! Paths are fixed constants, never taken from the environment, because
//! pkexec runs the root helper on behalf of an untrusted caller. Tests build
//! a [`Paths`] that points into a temporary tree.

use std::path::PathBuf;

/// Directory of the root helper and its fixed pkexec entry points.
pub const LIBEXEC: &str = "/usr/local/libexec/uac-linux";
/// Configuration: `user`, `default_hours`, `max_hours`.
pub const CONF: &str = "/etc/uac-linux.conf";
/// Current grant, world-readable so the CLI and the tray need no root.
pub const STATE: &str = "/var/lib/uac-linux/state.json";
/// polkit actions for `grant` and `revoke`.
pub const POLICY: &str = "/usr/share/polkit-1/actions/io.github.nongfsq.uac-linux.policy";
/// Where the top-bar icons are installed.
pub const SYMBOLIC_ICONS: &str = "/usr/share/icons/hicolor/symbolic/apps";

#[derive(Debug, Clone)]
pub struct Paths {
    pub conf: PathBuf,
    pub sudoers_dir: PathBuf,
    pub rule_name: String,
    pub state: PathBuf,
    pub lock: PathBuf,
    pub visudo: PathBuf,
    pub sudo_link: PathBuf,
    pub sudo_ws: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            conf: CONF.into(),
            sudoers_dir: "/etc/sudoers.d".into(),
            rule_name: "90-uac-linux".into(),
            state: STATE.into(),
            lock: "/run/lock/uac-linux.lock".into(),
            visudo: "/usr/sbin/visudo.ws".into(),
            sudo_link: "/usr/bin/sudo".into(),
            sudo_ws: "/usr/bin/sudo.ws".into(),
        }
    }
}

impl Paths {
    /// The installed rule file.
    pub fn rule(&self) -> PathBuf {
        self.sudoers_dir.join(&self.rule_name)
    }

    /// Temporary rule file. sudo's `@includedir` skips names containing '.',
    /// so a half-written file in the same directory is never read as a rule.
    pub fn rule_tmp(&self) -> PathBuf {
        self.sudoers_dir.join(format!(".{}.tmp", self.rule_name))
    }
}
