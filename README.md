# MQTT关机

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

订阅 MQTT 主题，按指令关闭或重启这台 Windows 电脑。关闭窗口后会留在托盘继续运行。

仓库：<https://github.com/renxing/mqtt-shutdown>

## 功能

- 连接任意 MQTT 服务器（巴法云把私钥当作 Client ID，一般不用填用户名和密码）
- 收到 `off` / `reboot` 后倒计时关机或重启，`on` 取消
- 关闭窗口最小化到托盘，远程关机仍然有效
- 开机自启动（登录后只进托盘）
- WinUI 3 + Mica，跟随系统浅色 / 深色

## 指令

向订阅主题推送文本：

| 载荷 | 动作 |
| --- | --- |
| `off` / `关机` | 按当前倒计时关机 |
| `off#10` | 10 秒后关机 |
| `on` / `取消` | 取消正在进行的关机或重启 |
| `reboot` / `重启` | 按当前倒计时重启 |
| `reboot#5` | 5 秒后重启 |

配置保存在 `%APPDATA%\MqttShutdown\settings.json`。

## 运行

1. 需要已安装的 [Windows App SDK](https://learn.microsoft.com/windows/apps/windows-app-sdk/) 运行时（本程序按官方 framework-dependent 方式启动）。
2. 打开 `mqtt-shutdown.exe`。第一次使用先到「连接」页填写服务器、Client ID 和主题。
3. 加 `--hidden` 时只进托盘，不弹出主窗口（开机自启会这样启动）。

## 从源码构建

按 Microsoft Learn：[在 Windows 上配置 Rust 开发环境](https://learn.microsoft.com/windows/dev-environment/rust/setup)

1. Visual Studio 或 [Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 安装 **「使用 C++ 的桌面开发」**（MSVC + Windows SDK）。
2. 用官方 rustup 安装稳定版 Rust：

```powershell
winget install Rustlang.Rustup
```

`windows-reactor` 0.100 还没发到 crates.io，需要官方仓库作为本地依赖：

```powershell
git clone --depth 1 https://github.com/microsoft/windows-rs.git ..\vendor\windows-rs
cargo build --release
```

可执行文件：`target\release\mqtt-shutdown.exe`。

目录约定：

```
vendor/windows-rs/     # 官方 windows-rs（不包含在本仓库）
mqtt-shutdown/         # 本项目
```

## 图标

托盘和窗口图标来自 Windows **Segoe Fluent Icons** 的 Power 字形（`U+E7E8`），画在 Windows 强调色圆角方块上。重新生成：

```powershell
powershell -ExecutionPolicy Bypass -File tools\make_icon.ps1
```

## 许可

MIT。`windows-reactor` 来自 [microsoft/windows-rs](https://github.com/microsoft/windows-rs)（MIT OR Apache-2.0）。
