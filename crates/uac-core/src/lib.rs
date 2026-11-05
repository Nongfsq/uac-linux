//! Shared logic of uac-linux.
//!
//! The hard expiry is sudoers' own `NOTAFTER` option: sudo stops honouring
//! the rule at the expiry time even if nothing else runs. `NOTAFTER` needs
//! classic sudo (`sudo.ws`); sudo-rs rejects it, so grants are refused while
//! sudo-rs is the active sudo.
//!
//! - [`engine`] grants, revokes and cleans up. Only the root helper calls it.
//! - [`client`] is what the CLI and the tray use: read the state file, read
//!   the config, and ask the root helper (through pkexec) to act.

pub mod client;
pub mod config;
pub mod engine;
pub mod error;
pub mod paths;
pub mod rule;
pub mod state;

pub use error::UacError;

/// Version of the whole suite; written into the rule file and the state file.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Journal tag (syslog identifier) of every audit record.
pub const TAG: &str = "uac-linux";

/// Smallest grant, in hours. The largest comes from the config (at most 24).
pub const MIN_HOURS: u32 = 1;

/// Largest `max_hours` the config may set.
pub const MAX_HOURS: u32 = 24;
