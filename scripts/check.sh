#!/usr/bin/env bash
# The one validation entry point: run before every commit; CI runs it too.
#
#   bash scripts/check.sh              all checks
#   bash scripts/check.sh --self-test  prove the privacy scan catches a planted leak
#
# Checks: rustfmt, clippy (warnings are errors), tests, cargo deny, shell syntax,
# ShellCheck when installed, translation coverage, and a privacy scan of
# every tracked file.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31mFAIL:\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- privacy
# Public repository, text and binary files alike: no home paths, local account or host names, or personal
# email addresses. The account and host names come from this machine at run
# time, so they are never written into the repository.
privacy_scan() {
  local files=("$@") hits="" name
  hits+="$(grep -naE '(/home/|/Users/)[A-Za-z0-9_.-]+' "${files[@]}" 2>/dev/null || true)"$'\n'
  hits+="$(grep -naoE '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}' "${files[@]}" 2>/dev/null \
    | grep -vE '@(users\.noreply\.github\.com|example\.(com|org)|anthropic\.com)$' || true)"$'\n'
  for name in "${USER:-}" "$(hostname 2>/dev/null || true)"; do
    [[ ${#name} -ge 3 ]] || continue
    hits+="$(grep -nawF -- "$name" "${files[@]}" 2>/dev/null || true)"$'\n'
  done
  hits="$(printf '%s' "$hits" | sed '/^$/d')"
  [[ -z "$hits" ]] || { printf '%s\n' "$hits" >&2; return 1; }
}

if [[ "${1:-}" == "--self-test" ]]; then
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  printf 'clean text\n' > "$tmp/clean"
  # Built at run time so this file does not trip its own scan.
  printf 'path /%s/someone/x\n' home > "$tmp/home"
  printf 'mail someone@%s\n' corp.example.net > "$tmp/mail"
  # Binary files are scanned too (a compiled cache once carried a home path).
  printf '\0\1/%s/someone/x\0' home > "$tmp/binary"
  privacy_scan "$tmp/clean" || fail "privacy scan flagged a clean file"
  for f in home mail binary; do
    if privacy_scan "$tmp/$f" 2>/dev/null; then fail "privacy scan missed a planted leak ($f)"; fi
  done
  echo "self-test passed"
  exit 0
fi

mapfile -t tracked < <(git ls-files)

step "Privacy scan ($((${#tracked[@]})) tracked files)"
privacy_scan "${tracked[@]}" || fail "private data in tracked files (see above)"

step "rustfmt"
cargo fmt --all --check

step "clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings

step "tests"
cargo test --workspace --locked --quiet

step "Dependencies (licenses, advisories)"
if command -v cargo-deny >/dev/null; then
  cargo deny --log-level error check
else
  echo "cargo-deny not installed; skipped (CI runs it): cargo install --locked cargo-deny"
fi

step "Shell syntax"
for f in scripts/*.sh; do bash -n "$f"; done
for f in libexec/*; do sh -n "$f"; done
if command -v shellcheck >/dev/null; then
  shellcheck scripts/*.sh libexec/*
else
  echo "shellcheck not installed; skipped (CI runs it)"
fi

step "Translations outside Fluent (polkit policy, .desktop)"
for dir in crates/uac-i18n/i18n/*/; do
  tag="$(basename "$dir")"
  [[ "$tag" == en-US ]] && continue
  posix="${tag/-/_}"
  n="$(grep -c "xml:lang=\"$posix\"" data/io.github.nongfsq.uac-linux.policy || true)"
  [[ "$n" -eq 4 ]] || fail "polkit policy has $n/4 messages for $posix"
  grep -q "^Name\[$posix\]=" data/uac-tray.desktop || fail "uac-tray.desktop has no Name[$posix]"
  grep -q "^Comment\[$posix\]=" data/uac-tray.desktop || fail "uac-tray.desktop has no Comment[$posix]"
done

printf '\n\033[1;32mAll checks passed.\033[0m\n'
