# Project profile: uac-linux

Facts for agents and Skills. Rules live in [AGENTS.md](../../AGENTS.md).

## Stack

- Rust 2024 workspace, minimum Rust 1.89 (`File::lock`). `Cargo.lock` is committed.
- Target: Ubuntu with GNOME and the bundled AppIndicator extension, systemd,
  classic sudo (`sudo.ws`), polkit (`pkexec`).

| Crate | Binary | Role | Key dependencies |
|---|---|---|---|
| `uac-core` | – | Config, sudoers rule, state file, grant engine, pkexec client | `jiff`, `serde_json` |
| `uac-helper` | `uac-helper` | Root helper started by pkexec and systemd | `uac-core` only |
| `uac-i18n` | – | Fluent translations, typed message functions | `i18n-embed`, `i18n-embed-fl` |
| `uac-cli` | `uac` | Command line | `clap` |
| `uac-tray` | `uac-tray` | Top-bar StatusNotifierItem | `ksni`, `notify-rust` (pure D-Bus, no GTK) |

## Commands

| Purpose | Command |
|---|---|
| Validate (the one gate) | `bash scripts/check.sh` |
| Gate self-test | `bash scripts/check.sh --self-test` |
| Dependency licenses and advisories | `cargo deny check` |
| Install or upgrade on this machine | `bash scripts/install.sh` |
| Uninstall | `bash scripts/uninstall.sh` |
| Installed health check | `uac doctor` |

## Locations

| What | Where |
|---|---|
| Translations | `crates/uac-i18n/i18n/<language>/uac-i18n.ftl`; polkit and `.desktop` translations in `data/` |
| Icons and README images | `data/icons/`, `docs/topbar-*.svg` |
| Decisions | `docs/decisions/` |
| Quality standard | this file, section Quality |

## Quality

- Lint floor: `cargo fmt --check`; clippy `all` at deny level (workspace
  lints); `unsafe_code` forbidden; shellcheck in CI.
- Ranked attributes: 1 security of the root path (fail closed), 2 recoverability
  (cleanup always converges to a consistent state), 3 modifiability.
- Comments explain why, not what.
- Script performance: not applicable. Two sequential install scripts and one
  check script; nothing to parallelize beyond what cargo does itself.

## Not applicable

Runtime performance budgets (idle tray, ~6 MB), databases, design system,
motion.
