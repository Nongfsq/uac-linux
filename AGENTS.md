# AGENTS.md

uac-linux: time-limited passwordless sudo for Ubuntu GNOME, switched from a
raccoon in the top bar. Rust workspace; public repository.

Project profile: [docs/agents/project-profile.md](docs/agents/project-profile.md)

## Rules

- Run `bash scripts/check.sh` before every commit. It is the only gate: rustfmt,
  clippy with warnings as errors, tests, shell syntax, translation coverage and
  a privacy scan. CI runs the same script.
- The root helper (`crates/uac-helper`, `crates/uac-core/src/engine.rs`) fails
  closed. Never weaken: fixed paths, `visudo -c` before the rename, audit
  before the rule takes effect, revocation that always reaches OFF.
- Keep the helper's dependencies minimal. A new crate needs a reason in the
  commit message and must pass `cargo deny check`.
- No user-facing string in code. Add the message to every
  `crates/uac-i18n/i18n/*/uac-i18n.ftl` and a typed function in
  `crates/uac-i18n/src/lib.rs`. Journal records and `--json` output stay
  English and stable.
- Privacy: never commit home paths, local account or host names, personal
  email addresses, or machine-specific examples. Test fixtures use `alice`.
- Keep installed paths, polkit action IDs and the `state.json` format stable;
  an upgrade must install over the previous version.
- English in code, comments, docs and commit messages; README has an English
  and a Chinese version with the same content.
