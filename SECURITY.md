# Security policy

uac-linux grants root through sudo, so security reports are welcome.

## Reporting

Please report vulnerabilities privately through GitHub:
**Security → Report a vulnerability** on this repository. Do not open a public
issue for a vulnerability.

Include the version (`uac version`), your Ubuntu release, and the steps to
reproduce. You can expect a first reply within a week.

## Scope

In scope: the root helper (`uac-helper` and its pkexec entry points), the
generated sudoers rule, the polkit policy, the cleanup service, and anything
that lets a grant outlive its expiry or reach another account.

Known and documented limits, not vulnerabilities: while a grant is active,
every process running as the granted user can use root; membership in the
`docker` group is root-equivalent; other NOPASSWD rules in sudoers are outside
uac-linux's control.
