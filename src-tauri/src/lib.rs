// 4Y 通知提示音管理器 — Rust 后端
// 仅适配 Debian 13 + KDE 6 + Wayland 环境
//
// 职责:
//   - 读写 ~/.config/notification-sound.conf (守护脚本的配置)
//   - 探测/控制系统服务 notification-sound.service
//   - 试听声音 (pw-play)
//   - 解析守护日志里"出现过的应用"
//   - 用 KDE 原生 kdialog 选择声音文件
//   - 收集运行环境信息 (Debian / KDE / Wayland)

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "notification-sound.service";
const LOG_FILE: &str = "/tmp/notification-sound.log";
const AUDIO_EXT: [&str; 6] = ["ogg", "oga", "mp3", "wav", "flac", "m4a"];

// ---------- 数据结构 ----------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mapping {
    pub keyword: String,
    pub sound: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub mappings: Vec<Mapping>,
    pub default_sound: Option<String>,
    /// 原文件里的注释行(保留用户手写的说明)
    pub comments: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ServiceStatus {
    pub active: bool,
    pub enabled: bool,
    pub state: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct DiscoveredApp {
    pub name: String,
    pub count: u32,
    pub last_seen: String,
    /// 是否已经在映射里配置过
    pub configured: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct SoundFile {
    pub path: String,
    pub name: String,
    pub group: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SessionInfo {
    pub os: String,
    pub desktop: String,
    pub session_type: String,
    pub kde_version: String,
    /// 是否满足 Debian13+KDE6+Wayland
    pub supported: bool,
}

// ---------- 工具函数 ----------

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".config"));
    base.join("notification-sound.conf")
}

fn default_header() -> Vec<String> {
    vec![
        "# ===== 通知提示音 按应用配置 =====".into(),
        "# 由 4Y 通知提示音管理器 生成 / 修改".into(),
        "# 语法: 应用关键词=声音文件绝对路径  (关键词不区分大小写, 子串匹配)".into(),
        "# default= 为兜底音, 未匹配的通知播放它".into(),
    ]
}

fn is_audio(p: &PathBuf) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXT.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

// ---------- 配置读 / 写 ----------

#[tauri::command]
fn get_config() -> AppConfig {
    let path = config_path();
    let mut mappings = Vec::new();
    let mut default_sound = None;
    let mut comments: Vec<String> = Vec::new();

    if let Ok(content) = fs::read_to_string(&path) {
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

#[tauri::command]
fn save_config(config: AppConfig) -> Result<String, String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut out = String::new();
    for c in &config.comments {
        out.push_str(c.trim_end());
        out.push('\n');
    }
    out.push('\n');

    for m in &config.mappings {
        let kw = m.keyword.trim();
        let sf = m.sound.trim();
        if kw.is_empty() || sf.is_empty() {
            continue;
        }
        out.push_str(&format!("{}={}\n", kw, sf));
    }

    if let Some(d) = &config.default_sound {
        if !d.trim().is_empty() {
            out.push_str(&format!("\ndefault={}\n", d.trim()));
        }
    }

    fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// ---------- 服务控制 ----------

fn run_systemctl(args: &[&str]) -> String {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn service_status() -> ServiceStatus {
    let active_s = run_systemctl(&["is-active", SERVICE]);
    let enabled_s = run_systemctl(&["is-enabled", SERVICE]);
    ServiceStatus {
        active: active_s == "active",
        enabled: matches!(enabled_s.as_str(), "enabled" | "static" | "alias"),
        state: active_s,
    }
}

#[tauri::command]
fn service_action(action: String) -> Result<ServiceStatus, String> {
    let allowed = ["start", "stop", "restart", "enable", "disable"];
    if !allowed.contains(&action.as_str()) {
        return Err(format!("不允许的操作: {}", action));
    }
    let out = Command::new("systemctl")
        .arg("--user")
        .arg(&action)
        .arg(SERVICE)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(service_status())
}

// ---------- 试听 ----------

#[tauri::command]
fn play_sound(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    // 优先 pw-play (PipeWire, Wayland 下 KDE6 默认), 回退 paplay
    let played = Command::new("pw-play").arg(&path).spawn();
    if played.is_err() {
        Command::new("paplay")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法播放: {}", e))?;
    }
    Ok(())
}

// ---------- 发现的应用(解析守护日志) ----------

/// 纯函数: 从日志内容解析出通知来源统计(便于单元测试)
fn parse_log_apps(content: &str, configured: &[String]) -> Vec<DiscoveredApp> {
    let mut map: std::collections::HashMap<String, (u32, String)> =
        std::collections::HashMap::new();

    for line in content.lines() {
        // 形如: [2026-10-27 08:35:16] 应用[wechat] -> ...
        let Some(pos) = line.find("应用[") else { continue };
        let rest = &line[pos + "应用[".len()..];
        let Some(end) = rest.find(']') else { continue };
        let name = rest[..end].trim().to_string();
        if name.is_empty() {
            continue;
        }
        let ts = if line.starts_with('[') {
            line[1..].find(']').map(|e| line[1..1 + e].to_string()).unwrap_or_default()
        } else {
            String::new()
        };
        let e = map.entry(name).or_insert((0, String::new()));
        e.0 += 1;
        if ts > e.1 {
            e.1 = ts;
        }
    }

    let mut v: Vec<DiscoveredApp> = map
        .into_iter()
        .map(|(name, (count, last_seen))| {
            let lc = name.to_lowercase();
            let configured = configured
                .iter()
                .any(|k| !k.is_empty() && (lc.contains(k.as_str()) || k.contains(&lc)));
            DiscoveredApp {
                name,
                count,
                last_seen,
                configured,
            }
        })
        .collect();
    v.sort_by(|a, b| b.count.cmp(&a.count));
    v
}

#[tauri::command]
fn discovered_apps() -> Vec<DiscoveredApp> {
    let content = fs::read_to_string(LOG_FILE).unwrap_or_default();
    let configured: Vec<String> = get_config()
        .mappings
        .iter()
        .map(|m| m.keyword.to_lowercase())
        .collect();
    parse_log_apps(&content, &configured)
}

// ---------- 声音文件库 ----------

#[tauri::command]
fn list_sounds() -> Vec<SoundFile> {
    let home = home_dir();
    let mut dirs: Vec<(PathBuf, String)> = vec![
        (home.join("文档/提示音"), "我的提示音".into()),
        (home.join("音乐/铃声"), "我的铃声".into()),
        (home.join("音乐"), "音乐".into()),
        (PathBuf::from("/usr/share/sounds/ocean/stereo"), "系统 Ocean".into()),
        (
            PathBuf::from("/usr/share/sounds/freedesktop/stereo"),
            "系统 Freedesktop".into(),
        ),
    ];
    dirs.retain(|(d, _)| d.exists());

    let mut out: Vec<SoundFile> = Vec::new();
    for (dir, group) in dirs {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() && is_audio(&p) {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                out.push(SoundFile {
                    path: p.to_string_lossy().to_string(),
                    name,
                    group: group.clone(),
                });
            }
        }
    }
    out
}

// ---------- KDE 原生文件选择器 ----------

#[tauri::command]
fn pick_sound_file() -> Option<String> {
    let home = home_dir();
    let filter = "*.ogg *.oga *.mp3 *.wav *.flac *.m4a|音频文件";
    let out = Command::new("kdialog")
        .arg("--getopenfilename")
        .arg(&home)
        .arg(filter)
        .arg("--title")
        .arg("选择提示音文件")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[tauri::command]
fn open_config_dir() -> Result<(), String> {
    let dir = config_path().parent().map(|p| p.to_path_buf()).unwrap_or_else(home_dir);
    Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- 环境信息 ----------

#[tauri::command]
fn session_info() -> SessionInfo {
    let os = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "未知".into());

    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

    let kde_version = Command::new("plasmashell")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let supported = os.contains("Debian GNU/Linux 13")
        && desktop.to_uppercase().contains("KDE")
        && session_type.eq_ignore_ascii_case("wayland");

    SessionInfo {
        os,
        desktop,
        session_type,
        kde_version,
        supported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("4y-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let cfg = AppConfig {
            mappings: vec![
                Mapping {
                    keyword: "wechat".into(),
                    sound: "/a/呱 一声.mp3".into(),
                },
                Mapping {
                    keyword: "qq".into(),
                    sound: "/b/q.ogg".into(),
                },
            ],
            default_sound: Some("/c/def.ogg".into()),
            comments: default_header(),
        };

        let written = save_config(cfg).unwrap();
        assert!(written.ends_with("notification-sound.conf"));

        let back = get_config();
        assert_eq!(back.mappings.len(), 2);
        assert_eq!(back.mappings[0].keyword, "wechat");
        assert_eq!(back.mappings[0].sound, "/a/呱 一声.mp3"); // 含空格/中文的路径
        assert_eq!(back.default_sound.as_deref(), Some("/c/def.ogg"));
        assert!(back.comments.iter().any(|c| c.starts_with('#')));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn parse_log_apps_works() {
        let log = "[2026-10-27 08:35:16] 应用[wechat] -> 匹配[wechat] 播放: /x.ogg\n\
[2026-10-27 08:35:17] 应用[qq] -> 匹配[qq] 播放: /y.ogg\n\
[2026-10-27 08:35:20] 应用[wechat] -> 默认音: /x.ogg\n\
[2026-10-27 08:35:21] 应用[unknownapp] 未匹配且无默认音，跳过\n";
        let apps = parse_log_apps(log, &["wechat".to_string()]);

        let wechat = apps.iter().find(|a| a.name == "wechat").unwrap();
        assert_eq!(wechat.count, 2);
        assert!(wechat.configured);
        assert_eq!(wechat.last_seen, "2026-10-27 08:35:20");

        let unk = apps.iter().find(|a| a.name == "unknownapp").unwrap();
        assert_eq!(unk.count, 1);
        assert!(!unk.configured);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            service_status,
            service_action,
            play_sound,
            discovered_apps,
            list_sounds,
            pick_sound_file,
            open_config_dir,
            session_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 4Y Notification Manager");
}
