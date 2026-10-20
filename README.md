# 拾刻 Shike Clock

一枚轻巧的 Windows 桌面时钟。它只有时间和日期两行，显示在线北京时间，背景逐像素透明，可以拖到桌面的任意位置。

**[打开在线介绍页](https://fundo123.github.io/shike-clock/)** · **[下载 Shike.exe](Shike.exe)**

```text
12:34:56
10月19日 周1
```

## 特点

- 透明悬浮窗，支持置顶、拖动和记忆位置；拖动结束会重新置顶，放到任务栏区域也能保持可见。
- 通过 HTTPS 从 TimeAPI.io 获取北京时间和日期，每 15 分钟自动校准；断网时沿用最近一次在线校准并继续走秒。
- 自动读取背后画面的明暗并切换文字颜色，也可固定浅色、深色或自选颜色。
- 可选择多档字号，适配屏幕 DPI。
- 可设置 Windows 当前用户登录后自动启动，也可从右键菜单关闭。
- 带系统托盘图标；Rust 原生实现，不依赖第三方 Rust crate。

## 使用

下载仓库中的 [Shike.exe](Shike.exe)，双击运行。适用于 64 位 Windows 10/11，无需安装或管理员权限。按住时钟任意位置即可拖动；右键打开设置菜单。

设置保存在 `%LOCALAPPDATA%\ShikeClock\settings.ini`。右键菜单可调整配色、字号、置顶和开机启动。

## 从源码构建

需要 Rust stable（MSVC 工具链）、Visual Studio C++ Build Tools 和 Windows SDK。在 Windows PowerShell 中运行：

```powershell
cd source
powershell -ExecutionPolicy Bypass -File .\build.ps1
cargo test --locked
```

完整说明见 [使用说明.txt](使用说明.txt)。项目按 MIT License 发布，见 [source/LICENSE](source/LICENSE)。
