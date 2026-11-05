# uac-linux: English (fallback language).
# Placeables: $until is a local time ("14:30" or "tomorrow 01:30"),
# $left is a duration from duration-left, $hours a whole number.

## Shared

duration-left = { $hours } h { $minutes } min
day-today = { $time }
day-tomorrow = tomorrow { $time }

## Errors from the root helper

error-config-missing = { $path } is missing; run scripts/install.sh
error-config-bad-user = { $path }: 'user' is missing or not a valid user name
error-config-bad-number = { $path }: { $key } must be a whole number
error-config-bad-range = { $path }: need 1 ≤ default_hours ≤ max_hours ≤ 24
error-unknown-caller = The calling user is unknown; use 'uac on' (pkexec).
error-no-account = uid { $uid } has no account
error-wrong-user = Only '{ $allowed }' may be granted; you are '{ $caller }'.
error-hours-not-number = Hours must be a whole number.
error-hours-out-of-range = Hours must be between { $min } and { $max }.
error-sudo-rs-active = The active sudo is sudo-rs, which has no NOTAFTER. Run: sudo update-alternatives --set sudo /usr/bin/sudo.ws
error-visudo-rejected = visudo rejected the generated rule: { $detail }
error-audit-unavailable = The audit log is unavailable, so nothing was granted: { $detail }
error-io = { $detail }
error-not-root = The helper must run as root; use 'uac on' or 'uac off'.
error-usage = usage: uac-helper grant HOURS | revoke | cleanup

## Why a grant ended

end-reason-user = turned off
end-reason-expired = expired
end-reason-rule-missing = rule file was gone
end-reason-rule-unreadable = rule file was unreadable
end-reason-sudo-rs-active = sudo-rs became active
end-reason-state-mismatch = rule did not match the state file

## Command line

cli-about = Time-limited passwordless sudo for this desktop.
cli-on = Grant for HOURS (asks for your password)
cli-on-hours = Hours, { $min } to { $max } (default { $default })
cli-off = Revoke now
cli-status = Show whether a grant is active and when it ends
cli-status-json = Print JSON for scripts
cli-logs = Show audit records
cli-logs-since = How far back: 30m, 24h, 7d …
cli-doctor = Read-only checks of the installation
cli-version = Print the version
cli-status-on = ON   until { $until } ({ $left } left), grant { $grant }
cli-status-off = OFF  sudo asks for a password as usual
cli-status-last = { "     " }last grant ended { $at } ({ $reason })
cli-status-other-nopasswd = note: 'sudo -n true' still works; another sudoers rule grants NOPASSWD (see 'uac doctor')
cli-cancelled = uac: cancelled (the password dialog was dismissed or the password was wrong)
cli-failed = uac: { $detail }
cli-logs-bad-since = uac: SINCE looks like 30m, 24h or 7d

## uac doctor

doctor-classic-sudo = classic sudo is active (sudo → { $target }); sudo-rs has no NOTAFTER
doctor-root-owned = { $path } is root-owned and not group/world-writable
doctor-exists = { $path } exists
doctor-polkit = polkit policy is installed ({ $path })
doctor-config = config is present ({ $path })
doctor-icons = top-bar icons are installed (otherwise the tray uses a stock icon)
doctor-unit = { $unit } is { $want } (now: { $now })
doctor-state-agrees = the state file agrees with the clock (cleanup runs every minute)
doctor-other-nopasswd = no uac grant, yet 'sudo -n true' works: another NOPASSWD rule exists in sudoers; uac cannot limit that one
doctor-docker = you are in the 'docker' group, which is root-equivalent without sudo
doctor-summary = { $problems ->
    [one] 1 problem
   *[other] { $problems } problems
}, { $warnings ->
    [one] 1 warning
   *[other] { $warnings } warnings
}

## Top bar

tray-tooltip-on = Passwordless sudo is on
tray-tooltip-off = Passwordless sudo is off
tray-header-on = Passwordless sudo: on until { $until } ({ $left } left)
tray-header-off = Passwordless sudo: off (sudo asks for a password)
tray-grant = { $hours ->
    [one] Turn on for { $hours } hour
   *[other] Turn on for { $hours } hours
}
tray-grant-default = { $hours ->
    [one] Turn on for { $hours } hour (default)
   *[other] Turn on for { $hours } hours (default)
}
tray-replace = { $hours ->
    [one] Change to { $hours } hour from now
   *[other] Change to { $hours } hours from now
}
tray-replace-default = { $hours ->
    [one] Change to { $hours } hour from now (default)
   *[other] Change to { $hours } hours from now (default)
}
tray-revoke = Turn off now (sudo asks for a password again)
tray-logs = Show the last 7 days of records
tray-hide = Hide the icon (the grant is not affected)
tray-press-enter = Press Enter to close
notify-on = { $hours ->
    [one] Passwordless sudo is on for { $hours } hour
   *[other] Passwordless sudo is on for { $hours } hours
}
notify-off = Passwordless sudo is off
notify-expiring = Passwordless sudo ends soon
notify-expiring-body = sudo asks for a password again after { $until }. To extend, pick a new length in the top bar.
notify-expired = Passwordless sudo has ended
notify-expired-body = sudo asks for a password again.
notify-failed = uac-linux could not do that
