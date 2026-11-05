#!/usr/bin/env bash
# Install uac-linux for the current user (or the account given as $1).
# Builds the Rust binaries as you, then uses sudo for the system files.
# Safe to re-run; it upgrades in place, including from the Python version.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_USER="${1:-$(id -un)}"
LIBEXEC=/usr/local/libexec/uac-linux
ICONS=/usr/share/icons/hicolor
BIN="$REPO/target/release"

say()  { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[[ "$EUID" -ne 0 ]] || die "run as your normal user; sudo is used where needed"
id "$TARGET_USER" >/dev/null 2>&1 || die "no such user: $TARGET_USER"
id -nG "$TARGET_USER" | tr ' ' '\n' | grep -qx sudo || die "$TARGET_USER is not in the sudo group"
command -v systemctl >/dev/null || die "systemd is required"
command -v cargo >/dev/null || die "Rust is required to build: sudo apt install cargo (or https://rustup.rs)"
[[ -x /usr/bin/sudo.ws && -x /usr/sbin/visudo.ws ]] || die "classic sudo (package 'sudo') is required: sudo apt install sudo"

say "Building (cargo build --release)"
cargo build --release --locked --manifest-path "$REPO/Cargo.toml"

sudo grep -Eq '^[@#]includedir[[:space:]]+/etc/sudoers.d' /etc/sudoers || die "/etc/sudoers does not include /etc/sudoers.d"
say "Packages (Ubuntu repositories)"
sudo apt-get install -y -qq pkexec >/dev/null

if [[ "$(readlink -f /usr/bin/sudo)" != /usr/bin/sudo.ws ]]; then
  say "Switching sudo to classic sudo (sudo.ws); sudo-rs does not support NOTAFTER"
  sudo /usr/sbin/visudo.ws -c >/dev/null || die "current sudoers does not parse under classic sudo; not switching"
  sudo update-alternatives --set sudo /usr/bin/sudo.ws
  hash -r
fi

say "Installing files"
sudo install -d -o root -g root -m 0755 "$LIBEXEC" /var/lib/uac-linux
sudo install -o root -g root -m 0755 "$BIN/uac-helper" "$LIBEXEC/uac-helper"
for a in grant revoke cleanup; do
  sudo install -o root -g root -m 0755 "$REPO/libexec/$a" "$LIBEXEC/$a"
done
sudo rm -f "$LIBEXEC/uac_helper.py"   # left over from the Python version
sudo install -o root -g root -m 0755 "$BIN/uac" /usr/local/bin/uac
sudo install -o root -g root -m 0755 "$BIN/uac-tray" /usr/local/bin/uac-tray
sudo install -o root -g root -m 0644 "$REPO/data/io.github.nongfsq.uac-linux.policy" \
  /usr/share/polkit-1/actions/io.github.nongfsq.uac-linux.policy
sudo install -o root -g root -m 0644 "$REPO/data/uac-linux-cleanup.service" /etc/systemd/system/uac-linux-cleanup.service
sudo install -o root -g root -m 0644 "$REPO/data/uac-linux-cleanup.timer" /etc/systemd/system/uac-linux-cleanup.timer
sudo install -o root -g root -m 0644 "$REPO/data/uac-tray.desktop" /usr/share/applications/uac-tray.desktop
sudo install -D -o root -g root -m 0644 "$REPO/data/icons/uac-linux.svg" "$ICONS/scalable/apps/uac-linux.svg"
for s in on off; do
  sudo install -D -o root -g root -m 0644 "$REPO/data/icons/uac-linux-$s-symbolic.svg" \
    "$ICONS/symbolic/apps/uac-linux-$s-symbolic.svg"
done
sudo gtk-update-icon-cache -q -t -f "$ICONS" 2>/dev/null || true
if sudo test -f /etc/uac-linux.conf; then
  echo "keeping existing /etc/uac-linux.conf"
else
  tmpconf="$(mktemp)"
  sed "s/@USER@/$TARGET_USER/" "$REPO/data/uac-linux.conf" > "$tmpconf"
  sudo install -o root -g root -m 0644 "$tmpconf" /etc/uac-linux.conf
  rm -f "$tmpconf"
fi

say "Enabling cleanup at boot and every minute"
sudo systemctl daemon-reload
sudo systemctl enable --now uac-linux-cleanup.timer >/dev/null
sudo systemctl enable uac-linux-cleanup.service >/dev/null
sudo systemctl start uac-linux-cleanup.service

say "Tray autostart for $TARGET_USER"
home="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
sudo -u "$TARGET_USER" install -D -m 0644 "$REPO/data/uac-tray.desktop" "$home/.config/autostart/uac-tray.desktop"
if [[ "$TARGET_USER" == "$(id -un)" && -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]]; then
  pkill -u "$TARGET_USER" -x uac-tray 2>/dev/null || true
  pkill -u "$TARGET_USER" -f '/usr/local/bin/uac-tray' 2>/dev/null || true
  setsid -f /usr/local/bin/uac-tray >/dev/null 2>&1 < /dev/null
fi

say "Checks"
uac doctor || true
cat <<EOF

Installed. Use the raccoon in the top bar, or:
  uac on 6      # passwordless sudo for 6 hours (asks for your password once)
  uac off       # back to normal now
  uac status
EOF
