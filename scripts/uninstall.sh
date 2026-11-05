#!/usr/bin/env bash
# Remove uac-linux. Revokes any active grant first. Leaves sudo on sudo.ws;
# pass --restore-sudo-rs to switch back to Ubuntu's default sudo-rs.
set -euo pipefail

restore_rs=0
[[ "${1:-}" == "--restore-sudo-rs" ]] && restore_rs=1
[[ "$EUID" -ne 0 ]] || { echo "run as your normal user" >&2; exit 1; }

echo "==> Revoking any active grant"
sudo rm -f /etc/sudoers.d/90-uac-linux /etc/sudoers.d/.90-uac-linux.tmp
logger -t uac-linux -p authpriv.notice -- "event=uninstall user=$(id -un)" || true

echo "==> Removing files"
sudo systemctl disable --now uac-linux-cleanup.timer uac-linux-cleanup.service >/dev/null 2>&1 || true
sudo rm -f /etc/systemd/system/uac-linux-cleanup.timer /etc/systemd/system/uac-linux-cleanup.service
sudo systemctl daemon-reload
pkill -u "$(id -un)" -x uac-tray 2>/dev/null || true
pkill -u "$(id -un)" -f '/usr/local/bin/uac-tray' 2>/dev/null || true
sudo rm -f /usr/local/bin/uac /usr/local/bin/uac-tray /usr/share/applications/uac-tray.desktop \
  /usr/share/polkit-1/actions/io.github.nongfsq.uac-linux.policy /etc/uac-linux.conf
sudo rm -rf /usr/local/libexec/uac-linux /var/lib/uac-linux  # helper, entry points, state
sudo rm -f /usr/share/icons/hicolor/scalable/apps/uac-linux.svg \
  /usr/share/icons/hicolor/symbolic/apps/uac-linux-on-symbolic.svg \
  /usr/share/icons/hicolor/symbolic/apps/uac-linux-off-symbolic.svg
sudo gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
rm -f "$HOME/.config/autostart/uac-tray.desktop"

if [[ "$restore_rs" -eq 1 ]]; then
  echo "==> Switching sudo back to sudo-rs"
  sudo update-alternatives --set sudo /usr/lib/cargo/bin/sudo
fi
echo "Done."
