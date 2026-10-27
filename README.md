# 4Y 通知提示音管理器 (4Y Notification Manager)

> 4YStudio · 专为 **Debian 13 + KDE 6 + Wayland** 打造的桌面管理系统通知提示音的客户端。

基于 **Tauri 2 + Vue 3 (TypeScript)**，后端 Rust，前端 Vite。

---

## ✨ 功能

| 功能 | 说明 |
|---|---|
| 应用 → 提示音映射 | 为微信 / QQ / 浏览器等不同软件配置不同提示音，可视化增删改 |
| 兜底音 (default) | 未匹配到的通知统一播放的声音；不设置则未匹配通知静音 |
| 发现的应用 | 自动从守护日志中解析"曾经发过通知的软件"，一键添加映射 |
| 声音库 | 浏览系统 Ocean / Freedesktop 音效与个人提示音，支持试听 |
| 服务控制 | 启动 / 停止 / 重启守护服务，开关开机自启 |
| KDE 原生集成 | 使用 `kdialog` 做文件选择器，`pw-play` 试听（PipeWire） |
| 环境自检 | 启动时检测 Debian 13 / KDE 6 / Wayland 是否满足适配条件 |

它管理的是上一个项目里搭好的通知提示音守护脚本：

- 守护脚本：`~/.local/bin/notification-sound.sh`
- 配置文件：`~/.config/notification-sound.conf`
- 系统服务：`notification-sound.service`（systemd 用户服务）

---

## 🧱 架构

```
4YNotificationManager/
├── src/                         # 前端 (Vue 3 + TS)
│   ├── App.vue                  # 主界面 / 状态编排 / 导航
│   ├── api.ts                   # invoke 封装
│   ├── types.ts                 # 与后端对齐的类型
│   ├── style.css                # Breeze Dark 风格全局样式
│   └── components/
│       ├── MappingsPanel.vue    # 映射管理
│       ├── DiscoveredPanel.vue  # 发现的应用
│       ├── SoundLibraryPanel.vue# 声音库
│       ├── ServicePanel.vue     # 服务 & 环境
│       └── SoundPicker.vue      # 声音选择弹窗
└── src-tauri/                   # 后端 (Rust)
    └── src/lib.rs               # 全部 Tauri 命令 + 单元测试
```

### 后端命令 (Rust)

`get_config` `save_config` `service_status` `service_action` `play_sound`
`discovered_apps` `list_sounds` `pick_sound_file` `open_config_dir` `session_info`

---

## 🚀 开发 / 构建

```bash
# 开发（热更新）
npm install
npm run tauri dev

# 打包 .deb
npm run tauri build -- --bundles deb

# 单元测试（配置往返、日志解析）
cd src-tauri && cargo test
```

产物：`src-tauri/target/release/bundle/deb/4Y Notification Manager_0.1.0_amd64.deb`
直接二进制：`src-tauri/target/release/fourey-notification-manager`

### 安装

```bash
sudo dpkg -i "src-tauri/target/release/bundle/deb/4Y Notification Manager_0.1.0_amd64.deb"
```

---

## 📝 配置文件格式

`~/.config/notification-sound.conf`：

```ini
# 关键词不区分大小写，子串匹配，自上而下命中第一条
wechat=/home/you/文档/提示音/呱 一声.mp3
qq=/home/you/音乐/铃声/简短 提示音.ogg
firefox=/usr/share/sounds/ocean/stereo/message-attention.oga

# 兜底音（未匹配通知播放它；删除本行则未匹配通知静音）
default=/home/you/音乐/铃声/简短 提示音.ogg
```

---

## ⚠️ 平台说明

本项目**只针对 Debian 13 + KDE 6 + Wayland** 适配，依赖：

- `pipewire-bin`（`pw-play`）
- `kdialog`（KDE 文件选择器）
- `systemctl --user`（管理守护服务）
- `libwebkit2gtk-4.1` / `libgtk-3`
