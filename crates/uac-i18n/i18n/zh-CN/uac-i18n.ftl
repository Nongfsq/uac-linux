# uac-linux：简体中文。

## Shared

duration-left = { $hours } 小时 { $minutes } 分
day-today = { $time }
day-tomorrow = 明天 { $time }

## Errors from the root helper

error-config-missing = 缺少 { $path }，请运行 scripts/install.sh
error-config-bad-user = { $path }：'user' 缺失或不是有效的用户名
error-config-bad-number = { $path }：{ $key } 必须是整数
error-config-bad-range = { $path }：需要 1 ≤ default_hours ≤ max_hours ≤ 24
error-unknown-caller = 不知道是哪个用户发起的，请用 'uac on'（pkexec）。
error-no-account = uid { $uid } 没有对应的账户
error-wrong-user = 只有 '{ $allowed }' 可以开启，当前是 '{ $caller }'。
error-hours-not-number = 小时数必须是整数。
error-hours-out-of-range = 小时数必须在 { $min } 到 { $max } 之间。
error-sudo-rs-active = 当前 sudo 是 sudo-rs，不支持 NOTAFTER。请运行：sudo update-alternatives --set sudo /usr/bin/sudo.ws
error-visudo-rejected = visudo 拒绝了生成的规则：{ $detail }
error-audit-unavailable = 审计日志不可用，所以没有开启：{ $detail }
error-io = { $detail }
error-not-root = 辅助程序必须以 root 运行，请用 'uac on' 或 'uac off'。
error-usage = 用法：uac-helper grant 小时数 | revoke | cleanup

## Why a grant ended

end-reason-user = 手动关闭
end-reason-expired = 到期
end-reason-rule-missing = 规则文件已不存在
end-reason-rule-unreadable = 规则文件无法读取
end-reason-sudo-rs-active = sudo-rs 被启用
end-reason-state-mismatch = 规则与状态文件不一致

## Command line

cli-about = 本机的限时免密 sudo。
cli-on = 开启指定小时数（需要输入密码）
cli-on-hours = 小时数，{ $min } 到 { $max }（默认 { $default }）
cli-off = 立即关闭
cli-status = 查看是否已开启、何时到期
cli-status-json = 输出 JSON，供脚本使用
cli-logs = 查看审计记录
cli-logs-since = 时间范围：30m、24h、7d……
cli-doctor = 只读检查安装状态
cli-version = 显示版本
cli-status-on = 开启  到 { $until }（剩 { $left }），授权 { $grant }
cli-status-off = 关闭  sudo 照常需要密码
cli-status-last = { "      " }上次授权于 { $at } 结束（{ $reason }）
cli-status-other-nopasswd = 提示：'sudo -n true' 仍然免密，说明 sudoers 里还有别的 NOPASSWD 规则（见 'uac doctor'）
cli-cancelled = uac：已取消（关闭了密码窗口或密码错误）
cli-failed = uac：{ $detail }
cli-logs-bad-since = uac：时间范围应写成 30m、24h 或 7d

## uac doctor

doctor-classic-sudo = 当前是经典 sudo（sudo → { $target }）；sudo-rs 不支持 NOTAFTER
doctor-root-owned = { $path } 属于 root，且组和其他用户不可写
doctor-exists = { $path } 存在
doctor-polkit = 已安装 polkit 策略（{ $path }）
doctor-config = 配置文件存在（{ $path }）
doctor-icons = 已安装顶栏图标（否则顶栏会用系统自带图标）
doctor-unit = { $unit } 应为 { $want }（当前：{ $now }）
doctor-state-agrees = 状态文件与时间一致（清理每分钟运行一次）
doctor-other-nopasswd = 没有 uac 授权，'sudo -n true' 却能免密：sudoers 里有别的 NOPASSWD 规则，uac 管不了它
doctor-docker = 你在 'docker' 组里，这本身就等同 root，不经过 sudo
doctor-summary = { $problems } 个问题，{ $warnings } 个警告

## Top bar

tray-tooltip-on = 免密 sudo 已开启
tray-tooltip-off = 免密 sudo 已关闭
tray-header-on = 免密 sudo：已开启，到 { $until }（剩 { $left }）
tray-header-off = 免密 sudo：已关闭（sudo 需要密码）
tray-grant = 开启 { $hours } 小时
tray-grant-default = 开启 { $hours } 小时（默认）
tray-replace = 改为从现在起 { $hours } 小时
tray-replace-default = 改为从现在起 { $hours } 小时（默认）
tray-revoke = 立即关闭（恢复需要密码）
tray-logs = 查看最近 7 天的记录
tray-hide = 隐藏图标（不影响授权）
tray-press-enter = 按回车关闭
notify-on = 免密 sudo 已开启 { $hours } 小时
notify-off = 免密 sudo 已关闭
notify-expiring = 免密 sudo 快到期了
notify-expiring-body = { $until } 后恢复需要密码。要延长就在顶栏重新选时长。
notify-expired = 免密 sudo 已到期
notify-expired-body = sudo 恢复需要密码。
notify-failed = uac-linux 操作失败
