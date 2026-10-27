# 移植 / 适配指南

本项目默认目标是 **Debian 13 + KDE 6 + Wayland**，但通知监听的原理是**通用的**：
任何实现了 `org.freedesktop.Notifications` 的 Linux 桌面（KDE / GNOME / XFCE / Cinnamon /
dunst / mako / swaync / xfce4-notifyd / notify-osd …）都能用，只是需要把少数几处
"平台假设"改掉。

好消息：**所有平台相关内容都集中在 `src-tauri/src/platform.rs` 一个文件里。**

---

## 一分钟清单

| 你想改的东西 | 改哪个文件 | 具体位置 |
|---|---|---|
| 播放器（PipeWire/Pulse/ALSA/mpv…） | `src-tauri/src/platform.rs` | `PLAYERS` 常量 |
| 扫描哪些目录找声音 | `src-tauri/src/platform.rs` | `sound_dirs()` |
| 文件选择框（kdialog/zenity/yad） | `src-tauri/src/platform.rs` | `pick_file()` |
| 服务名 / 守护进程名 | `src-tauri/src/platform.rs` | `SERVICE` / `DAEMON_BIN` |
| 通知接口（极少需要改） | `src-tauri/src/platform.rs` | `NOTIFY_INTERFACE` |
| 应用名 / 包名 / 图标 | `src-tauri/tauri.conf.json` | `productName` / `identifier` |
| deb 依赖包名 | `src-tauri/tauri.conf.json` | `bundle.linux.deb.depends` |
| 仓库地址（fork 后） | `src-tauri/Cargo.toml` | `repository` |
| 一键安装脚本的下载地址 | `install.sh` | `REPO=` 那一行 |
| 品牌名（4Y → 你的） | `src/App.vue`（界面）+ `tauri.conf.json`（包名） | 搜索 `4Y` |

改完执行：

```bash
npm install
npm run tauri dev                      # 先跑起来看看
cd src-tauri && cargo test             # 单元测试
npm run tauri build -- --bundles deb   # 出包
```

---

## 一、平台适配层 `src-tauri/src/platform.rs`

### 1. 播放器候选

```rust
const PLAYERS: &[&str] = &["pw-play", "paplay", "mpv", "ffplay", "aplay"];
```

程序按顺序找第一个**存在于 PATH** 的播放器并用它播放；播放器参数在
`player_args()` 里按名字匹配。

常见组合：

| 你的系统 | 建议 `PLAYERS` |
|---|---|
| PipeWire（KDE6 / Fedora / Arch 新版） | `["pw-play", "paplay", "mpv"]` |
| PulseAudio（Ubuntu / Debian 12） | `["paplay", "mpv"]` |
| 纯 ALSA 无声音服务器 | `["mpv", "ffplay", "aplay"]` |
| 想统一用 mpv | `["mpv"]` |

> 也可以不改代码，直接设环境变量：`FOURY_PLAYER=paplay`（冒号分隔可写多个，如 `paplay:mpv`）。

### 2. 声音目录

`sound_dirs()` 会按这个顺序收集：

1. 环境变量 `FOURY_SOUND_DIRS`（冒号分隔，支持 `$HOME`）
2. XDG `XDG_MUSIC_DIR` / `XDG_DOCUMENTS_DIR`（自动读 `~/.config/user-dirs.dirs`，**已自动支持中文目录**）
3. 常见回退：`~/Music`、`~/音乐/铃声`、`~/文档/提示音` …
4. 系统音效主题：KDE `ocean`、`freedesktop`、GNOME `default`、Ubuntu `Yaru/ubuntu`

要加自己的目录，改第 4 段数组即可，例如：

```rust
for p in [
    "/usr/share/sounds/mytheme",
    "/opt/sounds",
] { add_dir(&mut out, PathBuf::from(p)); }
```

### 3. 文件选择器

`pick_file()` 依次尝试 `kdialog` → `zenity` → `yad`，哪个存在用哪个：

- KDE → 有 `kdialog`
- GNOME / 通用 → 装 `zenity` 即可
- 都没有 → 返回 `None`，界面上「声音库」列表仍然可用（只能选已扫描到的声音）

### 4. 服务名 / 守护进程名

```rust
pub const SERVICE: &str = "4y-notify-daemon.service";
pub const DAEMON_BIN: &str = "fourey-notify-daemon";
```

改名时要同步 `src-tauri/linux/4y-notify-daemon.service` 的**文件名**和
`tauri.conf.json → bundle.linux.deb.files` 里的目标路径。

---

## 二、不用改代码的运行时开关

| 环境变量 | 作用 | 示例 |
|---|---|---|
| `FOURY_PLAYER` | 指定播放器（优先级高于内置候选） | `FOURY_PLAYER=paplay` |
| `FOURY_SOUND_DIRS` | 覆盖声音扫描目录（冒号分隔） | `FOURY_SOUND_DIRS=$HOME/snd:/opt/audio` |
| `XDG_CONFIG_HOME` | 配置目录（默认 `~/.config`） | — |
| `XDG_STATE_HOME` | 日志目录（默认 `~/.local/state`） | — |

调试时可这样验证守护进程而不装服务：

```bash
FOURY_PLAYER=mpv cargo run --bin fourey-notify-daemon
notify-send -a test "标题" "正文"      # 另开终端发一条
tail -f ~/.local/state/4y-notify-daemon/daemon.log
```

---

## 三、不同桌面的注意事项

| 桌面 | 说明 |
|---|---|
| KDE Plasma 6 | 默认目标，已测试。通知由 `plasmashell` 提供。 |
| GNOME | 需要 `libnotify`，播放器建议 `paplay`/`mpv`，文件框自动用 `zenity`。 |
| XFCE | `xfce4-notifyd` 提供通知接口，同样可用。 |
| Sway / Hyprland | 用 `mako` / `swaync` / `dunst` 作为通知服务，同样可用；注意这些环境没有 `kdialog`，装 `zenity` 或直接用「声音库」。 |
| 纯 Wayland 无 PulseAudio | `PLAYERS` 里保留 `pw-play` 或 `mpv`。 |

**唯一硬性要求**：会话总线上有一个实现了 `org.freedesktop.Notifications` 的服务，
且系统里至少有一个可用的音频播放器。

---

## 四、打包适配

`src-tauri/tauri.conf.json`：

```jsonc
"bundle": {
  "linux": {
    "deb": {
      "depends": [
        "libwebkit2gtk-4.1-0",          // ← 发行版不同名字可能不同
        "libgtk-3-0",
        "pipewire-bin | pulseaudio-utils", // ← 播放器所在包
        "dbus-user-session | dbus",        // ← dbus-monitor 所在包
        "kdialog"                          // ← 若目标不用 KDE, 改成 zenity
      ]
    }
  }
}
```

- 打 **rpm**：`npm run tauri build -- --bundles rpm`（依赖字段不同，需另配）。
- 打 **AppImage**：`--bundles appimage`，但 AppImage 里不含 systemd 单元，
  自动启停服务仍需应用内「一键启用」写入 `~/.config/systemd/user/`。

---

## 五、验证你改对了没

1. `cargo test` 全绿（含 `platform::tests`）。
2. `npm run tauri dev` → 「服务与环境」页显示**检测到的播放器**和"未测试环境，通常可用"。
3. 点「一键启用后台服务」→ 状态变 `运行中`。
4. 用任意软件弹一条通知 → 听到声音。
5. `cat ~/.local/state/4y-notify-daemon/daemon.log` 能看到 `应用[xxx] -> 匹配[...] 播放: ...`。

如果第 5 步没有日志：说明通知接口不匹配 → 检查目标桌面的通知服务是否在跑
（`busctl --user list | grep -i notif`）。
如果只有日志没声音：播放器不对 → 用 `FOURY_PLAYER` 换一个再试。
