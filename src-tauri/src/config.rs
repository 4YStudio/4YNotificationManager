// 共享模块: 配置读写 / 路径约定 / systemd 单元生成
// 同时被 GUI (main.rs) 与 守护进程 (bin/4y-notify-daemon) 使用。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// 平台相关常量统一在 platform 模块定义, 这里重导出以保持调用点简洁
pub use crate::platform::{AUDIO_EXT, DAEMON_BIN, LEGACY_SERVICE, SERVICE};

// ---------- 数据结构 ----------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Mapping {
    pub keyword: String,
    pub sound: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub mappings: Vec<Mapping>,
    pub default_sound: Option<String>,
    /// 文件中保留的注释行 (方便用户写说明)
    pub comments: Vec<String>,
}

// ---------- 路径 ----------

pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".config"));
    base.join("notification-sound.conf")
}

/// 守护进程日志: ~/.local/state/4y-notify-daemon/daemon.log
pub fn log_path() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".local/state"));
    base.join("4y-notify-daemon").join("daemon.log")
}

/// 旧版守护日志(who发现应用的兼容来源)
pub fn legacy_log_path() -> PathBuf {
    PathBuf::from("/tmp/notification-sound.log")
}

pub fn unit_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".config"));
    base.join("systemd/user")
}

pub fn unit_path() -> PathBuf {
    unit_dir().join(SERVICE)
}

// ---------- 配置解析 / 渲染 ----------

pub fn default_header() -> Vec<String> {
    vec![
        "# ===== 通知提示音 按应用配置 =====".into(),
        "# 由 4Y 通知提示音管理器 生成 / 修改".into(),
        "# 语法: 应用关键词=声音文件绝对路径  (关键词不区分大小写, 子串匹配)".into(),
        "# default= 为兜底音, 未匹配的通知播放它".into(),
        "# 修改后即时生效, 无需重启后台服务".into(),
    ]
}

/// 解析配置文本
pub fn parse_config_str(content: &str) -> AppConfig {
    let mut mappings = Vec::new();
    let mut default_sound = None;
    let mut comments: Vec<String> = Vec::new();

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            comments.push(line.to_string());
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if k.eq_ignore_ascii_case("default") {
                if !v.is_empty() {
                    default_sound = Some(v.to_string());
                }
            } else if !k.is_empty() && !v.is_empty() {
                mappings.push(Mapping {
                    keyword: k.to_string(),
                    sound: v.to_string(),
                });
            }
        }
    }

    if comments.is_empty() {
        comments = default_header();
    }
    AppConfig {
        mappings,
        default_sound,
        comments,
    }
}

pub fn read_config_from(path: &Path) -> AppConfig {
    match fs::read_to_string(path) {
        Ok(c) => parse_config_str(&c),
        Err(_) => AppConfig {
            mappings: Vec::new(),
            default_sound: None,
            comments: default_header(),
        },
    }
}

pub fn read_config() -> AppConfig {
    read_config_from(&config_path())
}

/// 渲染为配置文件文本
pub fn render_config(cfg: &AppConfig) -> String {
    let mut out = String::new();
    for c in &cfg.comments {
        out.push_str(c.trim_end());
        out.push('\n');
    }
    out.push('\n');
    for m in &cfg.mappings {
        let (kw, sf) = (m.keyword.trim(), m.sound.trim());
        if kw.is_empty() || sf.is_empty() {
            continue;
        }
        out.push_str(&format!("{}={}\n", kw, sf));
    }
    if let Some(d) = &cfg.default_sound {
        if !d.trim().is_empty() {
            out.push_str(&format!("\ndefault={}\n", d.trim()));
        }
    }
    out
}

pub fn write_config(cfg: &AppConfig) -> std::io::Result<String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, render_config(cfg))?;
    Ok(path.to_string_lossy().to_string())
}

// ---------- 匹配 ----------

/// 根据应用名匹配应播放的声音; 返回 (声音路径, 命中的关键词)
/// 关键词不区分大小写, 子串匹配; 未命中返回兜底音。
pub fn match_sound(cfg: &AppConfig, app: &str) -> Option<(String, String)> {
    let app_lc = app.to_lowercase();
    for m in &cfg.mappings {
        let kw = m.keyword.trim().to_lowercase();
        if kw.is_empty() {
            continue;
        }
        if app_lc.contains(&kw) {
            return Some((m.sound.clone(), m.keyword.clone()));
        }
    }
    cfg.default_sound
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| (s.clone(), "default".to_string()))
}

// ---------- 守护进程定位 & 单元内容 ----------

/// 找到守护进程可执行文件: 优先与当前程序同目录, 否则 /usr/bin
pub fn daemon_bin_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(DAEMON_BIN);
            if cand.exists() {
                return cand;
            }
        }
    }
    PathBuf::from("/usr/bin").join(DAEMON_BIN)
}

/// 生成 systemd 用户单元文本
pub fn unit_content(exec: &Path) -> String {
    format!(
        "[Unit]\n\
Description=4Y 通知提示音守护 (4Y Notification Sound Daemon)\n\
Documentation={}\n\
After=pipewire.service\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart=\"{}\"\n\
Restart=always\n\
RestartSec=2\n\
\n\
[Install]\n\
WantedBy=default.target\n",
        crate::platform::REPO,
        exec.display()
    )
}

/// 包内随附的系统级用户单元(dep 安装到 /usr/lib/systemd/user)
pub const PACKAGED_UNIT: &str = include_str!("../linux/4y-notify-daemon.service");

// ---------- 迁移: 旧 bash 服务 ----------

#[derive(Serialize, Clone, Debug)]
pub struct LegacyInfo {
    pub active: bool,
    pub enabled: bool,
    pub script_exists: bool,
    pub script_path: String,
}

pub fn legacy_script_path() -> PathBuf {
    home_dir().join(".local/bin/notification-sound.sh")
}
