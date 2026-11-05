# 0001: Rust workspace, StatusNotifierItem tray, Fluent translations

Status: accepted (2026-11-04)

## Context

The first version was Python: a stdlib root helper, a CLI, and a GTK3
AppIndicator tray. It worked, but a public release needed one build with no
Python GTK bindings to install, the tray used about 57 MB at idle, and every
user-facing string was hard-coded Chinese.

## Decision

- Rewrite in Rust as a workspace (`uac-core`, `uac-helper`, `uac-cli`,
  `uac-tray`, `uac-i18n`), keeping every installed path, polkit action ID,
  the sudoers file name and the `state.json` format, so it installs over the
  Python version and rolls back by reinstalling it.
- Tray: `ksni` (StatusNotifierItem over D-Bus via `zbus`) and `notify-rust`.
  No GTK. Ubuntu's AppIndicator extension shows it.
- Translations: Fluent through `i18n-embed` and `i18n-embed-fl`. One `.ftl`
  file per language; `fl!` checks message IDs and arguments at compile time
  against en-US; a test requires every language to have the same messages.
- The helper reports refusals as JSON error codes, so the CLI and the tray
  translate them; the journal keeps stable English `key=value` records.

## Rejected

- Staying on Python: no reason to change the helper, but GTK bindings and
  hard-coded strings were the actual problems.
- `tray-icon`: pulls GTK3 and libayatana back in. `gtk4-rs`: GTK4 has no tray.
- gettext (`gettext-rs`): English sentences as keys, missing translations
  found only at run time, and a C library plus install paths.
- `rust-i18n`: weaker plural handling and no compile-time check of arguments.

## Consequences

- Building needs Rust 1.89 or newer (`apt install cargo` on Ubuntu 26.04).
- The root helper depends only on `uac-core` (`jiff`, `serde`, `serde_json`);
  `cargo deny check` gates licenses and advisories.
- Tray idle memory dropped from about 57 MB to about 6 MB.

## Revisit when

- Distributing through Launchpad translations (then gettext), or
- shipping a `.deb` (then add a packaging crate such as `cargo-deb`).
