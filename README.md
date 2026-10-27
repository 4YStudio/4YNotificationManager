# 4Y 通知提示音管理器

> **4YStudio** 出品 · 专为 **Debian 13 + KDE 6 + Wayland** 打造
> 给系统通知加上提示音，并且**按应用区分**：微信一声、QQ 一声、浏览器又是另一声。

[![Release](https://img.shields.io/github/v/release/4YStudio/4YNotificationManager?color=3daee9)](https://github.com/4YStudio/4YNotificationManager/releases)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Debian%2013%20%7C%20KDE%206%20%7C%20Wayland-3daee9)]()

---

## 为什么需要它

KDE Plasma 6 的通知气泡**默认不发声**：通知守护进程本身不含音频播放代码，KDE 自家应用靠自己播放，而 Chrome / 微信 / QQ 这类走 D-Bus 的第三方应用弹的通知**天生静音**。

本项目用一个常驻后台服务监听 D-Bus 通知总线，在任意气泡弹出瞬间播放你指定的声音，并支持按「通知来源应用」匹配不同提示音。

## ✨ 功能

- **按应用配置提示音**：`wechat` → 呱.mp3，`qq` → 叮.ogg，其余 → 兜底音
- **后台服务内置**：守护进程已编译进安装包，**点一下按钮即可启用**，无需手动装脚本
- **发现的应用**：自动从日志里列出「谁发过通知」，一键添加映射
- **声音库**：汇总系统 Ocean 音效与个人提示音，可搜索、试听
- **KDE 原生集成**：`kdialog` 文件选择器、PipeWire `pw-play` 播放、systemd 用户单元
- **配置即时生效**：改完保存就生效，不需要重启服务
- **旧版自动迁移**：检测到旧 bash 守护会自动停用，避免双重播放

## 📦 安装（开箱即用）

### 方式一：一键脚本

```bash
curl -fsSL https://raw.githubusercontent.com/4YStudio/4YNotificationManager/main/install.sh | bash
```

### 方式二：下载 .deb

从 [Releases](https://github.com/4YStudio/4YNotificationManager/releases) 下载后：

```bash
sudo dpkg -i 4Y.Notification.Manager_*.deb
```

然后从**应用菜单**打开「4Y 通知提示音」，点击 **⚡ 一键启用后台服务**。

也可以用命令行直接启用（无界面）：

```bash
fourey-notification-manager --install-service
```

### 方式三：从源码构建

```bash
git clone https://github.com/4YStudio/4YNotificationManager.git
cd 4YNotificationManager
npm install
npm run tauri build -- --bundles deb
sudo dpkg -i "src-tauri/target/release/bundle/deb/"*.deb
```

## 🧩 组成

安装包内含**两个二进制**，协同工作：

| 程序 | 作用 |
|---|---|
| `fourey-notification-manager` | GUI 管理端（Tauri + Vue3） |
| `fourey-notify-daemon` | 后台守护进程（Rust，监听 D-Bus 并播放声音） |

以及一个 systemd **用户单元** `4y-notify-daemon.service`（随包安装到 `/usr/lib/systemd/user/`，应用内启用时会写入 `~/.config/systemd/user/` 以便自定义路径生效）。

```
D-Bus (org.freedesktop.Notifications.Notify)
        │
        ▼
fourey-notify-daemon ──读取──> ~/.config/notification-sound.conf
        │                          (应用关键词 = 声音路径)
        ▼
   pw-play  播放对应提示音
```

## ⚙️ 配置文件

`~/.config/notification-sound.conf`

```ini
# 关键词不区分大小写，子串匹配，自上而下命中第一条
wechat=/home/you/文档/提示音/呱 一声.mp3
qq=/home/you/音乐/铃声/简短 提示音.ogg
firefox=/usr/share/sounds/ocean/stereo/message-attention.oga

# 兜底音（未匹配的通知播放它；删除本行则未匹配通知静音）
default=/usr/share/sounds/ocean/stereo/message-attention.oga
```

> 通常不需要手动改：GUI 会帮你写。手工修改后**立即生效**。

## 🛠 开发

```bash
npm install
npm run tauri dev                  # 开发热更新

cd src-tauri && cargo test         # 后端单元测试
npm run tauri build -- --bundles deb
```

### 目录结构

```
├── src/                       # 前端 Vue3 + TS
│   ├── App.vue                # 主界面 / 状态编排
│   ├── api.ts  types.ts
│   └── components/            # 各功能面板 + 声音选择弹窗
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs             # Tauri 命令 (GUI 后端)
│   │   ├── config.rs          # 配置读写 / 路径 / 单元模板 (共享)
│   │   ├── daemon.rs          # 守护进程主循环 (共享)
│   │   ├── main.rs            # GUI 入口 (+ CLI)
│   │   └── bin/daemon.rs      # 守护进程入口
│   └── linux/                 # 随包分发的 systemd 单元 / 示例配置
├── install.sh                 # 一键安装脚本
└── .github/workflows/         # 打 tag 自动构建并发布 .deb
```

## 🔧 常见问题

**Q：通知还是没声音？**
依次检查：①「服务与环境」页服务是否 `运行中`；② 是否配置了兜底音；③ 终端执行
`cat ~/.local/state/4y-notify-daemon/daemon.log` 看是否捕获到通知。

**Q：某个软件不响？**
让 TA 弹一条通知，然后到「发现的应用」页刷新，把显示的真实应用名加入映射。

**Q：旧版 bash 版怎么办？**
应用会自动停用它并提示。旧的脚本 `~/.local/bin/notification-sound.sh` 可自行删除。

**Q：卸载**

```bash
fourey-notification-manager --uninstall-service   # 或 GUI 里点「卸载服务」
sudo apt remove 4y-notification-manager
```

## 📄 许可

[MIT](LICENSE) © 2026 4YStudio
