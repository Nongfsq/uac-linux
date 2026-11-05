<div align="center">
  <img src="data/icons/uac-linux.svg" width="120" alt="uac-linux raccoon logo">
  <h1>uac-linux</h1>
  <p><strong>Time-limited passwordless sudo, from the GNOME top bar.</strong></p>
  <p>English · <a href="README.zh-CN.md">中文说明</a></p>
</div>

---

Type your password once. For the next few hours, `sudo` needs no password. Then it asks again.

## Use

| Top bar | State |
|:---:|---|
| <img src="docs/topbar-off.svg" height="28" alt="raccoon asleep"> | sudo asks for a password |
| <img src="docs/topbar-on.svg" height="28" alt="raccoon awake with a green dot"> | passwordless sudo is on |

Click the raccoon to grant 1–24 hours (6 by default) or revoke. The menu shows when the grant ends.

```console
uac on 6       # grant for 6 hours
uac off        # revoke now
uac status     # current state
uac logs 7d    # audit records
uac doctor     # check the installation
```

## Install

```bash
git clone https://github.com/Nongfsq/uac-linux.git
bash uac-linux/scripts/install.sh
```

Needs Ubuntu with GNOME, systemd, Rust 1.89+ (`sudo apt install cargo`), and an account in the `sudo` group. The installer switches `sudo` to classic sudo, because sudo-rs has no `NOTAFTER`.

Uninstall: `bash scripts/uninstall.sh`

## How it works

- The grant is one sudoers rule with `NOTAFTER`, so sudo itself stops honoring it on time.
- Every grant and revoke is written to the journal (tag `uac-linux`).
- While a grant is active, every process running as you can use root.

## Languages

English and 简体中文, following your system language. To add one, copy `crates/uac-i18n/i18n/en-US/uac-i18n.ftl` to a new language folder and translate it.

## Develop

`bash scripts/check.sh` runs every check. Security reports: [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
