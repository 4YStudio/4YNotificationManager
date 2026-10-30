// 4Y 通知提示音管理器 — Rust 后端 (GUI)
// 仅适配 Debian 13 + KDE 6 + Wayland
//
// 守护进程本身已内置于本工程 (bin/daemon.rs -> fourey-notify-daemon),
// GUI 负责: 安装/启用 systemd 用户单元、读写配置、试听、日志分析。

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde::Serialize;

pub mod config;
pub mod daemon;
pub mod platform;

use config::AppConfig;

// ---------- 数据结构 ----------

#[derive(Serialize, Clone, Debug)]
pub struct ServiceStatus {
    pub active: bool,
    pub enabled: bool,
    /// systemd 用户单元是否已安装
    pub installed: bool,
    pub state: String,
    pub unit_path: String,
    pub exec_path: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct DiscoveredApp {
    pub name: String,
    pub count: u32,
    pub last_seen: String,
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
    pub supported: bool,
    /// 是否在作者测试过的环境
    pub tested: bool,
    /// 检测到的可用播放器
    pub player: Option<String>,
    /// 守护进程可执行文件路径
    pub daemon_path: String,
}

// ---------- 工具 ----------

fn run_systemctl(args: &[&str]) -> String {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn is_audio(p: &PathBuf) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| config::AUDIO_EXT.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

// ---------- 配置读 / 写 ----------

#[tauri::command]
fn get_config() -> AppConfig {
    config::read_config()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<String, String> {
    config::write_config(&config).map_err(|e| e.to_string())
}

// ---------- 服务状态 / 控制 ----------

fn build_status() -> ServiceStatus {
    let active = run_systemctl(&["is-active", config::SERVICE]);
    let enabled = run_systemctl(&["is-enabled", config::SERVICE]);
    ServiceStatus {
        active: active == "active",
        enabled: matches!(enabled.as_str(), "enabled" | "static" | "alias"),
        installed: config::unit_path().exists(),
        state: active,
        unit_path: config::unit_path().to_string_lossy().to_string(),
        exec_path: config::daemon_bin_path().to_string_lossy().to_string(),
    }
}

#[tauri::command]
fn service_status() -> ServiceStatus {
    build_status()
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
        .arg(config::SERVICE)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(build_status())
}

/// 一键安装: 写单元文件 -> daemon-reload -> 开机自启 -> 启动
/// (CLI `--install-service` 与 GUI 按钮共用)
pub fn do_install_service() -> Result<ServiceStatus, String> {
    let exec = config::daemon_bin_path();
    if !exec.exists() {
        return Err(format!(
            "找不到守护进程: {}。请安装 .deb 包, 或先执行 npm run tauri build",
            exec.display()
        ));
    }

    // 首次安装时若没有配置, 生成一份带默认兜底音的配置
    if !config::config_path().exists() {
        let cfg = AppConfig {
            mappings: Vec::new(),
            default_sound: pick_default_sound(),
            comments: config::default_header(),
        };
        let _ = config::write_config(&cfg);
    }

    fs::create_dir_all(config::unit_dir()).map_err(|e| e.to_string())?;
    fs::write(config::unit_path(), config::unit_content(&exec)).map_err(|e| e.to_string())?;

    // 迁移: 旧版 bash 服务若在自启, 先停用, 避免双重播放
    if matches!(
        run_systemctl(&["is-enabled", config::LEGACY_SERVICE]).as_str(),
        "enabled" | "static" | "alias"
    ) {
        run_systemctl(&["disable", "--now", config::LEGACY_SERVICE]);
    }

    run_systemctl(&["daemon-reload"]);
    let en = Command::new("systemctl")
        .args(["--user", "enable", "--now", config::SERVICE])
        .output()
        .map_err(|e| e.to_string())?;
    if !en.status.success() {
        return Err(String::from_utf8_lossy(&en.stderr).trim().to_string());
    }
    Ok(build_status())
}

#[tauri::command]
fn install_service() -> Result<ServiceStatus, String> {
    do_install_service()
}

/// 卸载服务 (停止 + 取消自启 + 删除用户单元)
pub fn do_uninstall_service() -> Result<ServiceStatus, String> {
    run_systemctl(&["disable", "--now", config::SERVICE]);
    if config::unit_path().exists() {
        let _ = fs::remove_file(config::unit_path());
    }
    run_systemctl(&["daemon-reload"]);
    Ok(build_status())
}

#[tauri::command]
fn uninstall_service() -> Result<ServiceStatus, String> {
    do_uninstall_service()
}

// ---------- 旧版服务迁移 ----------

#[tauri::command]
fn legacy_service_info() -> config::LegacyInfo {
    let script = config::legacy_script_path();
    config::LegacyInfo {
        active: run_systemctl(&["is-active", config::LEGACY_SERVICE]) == "active",
        enabled: matches!(
            run_systemctl(&["is-enabled", config::LEGACY_SERVICE]).as_str(),
            "enabled" | "static" | "alias"
        ),
        script_exists: script.exists(),
        script_path: script.to_string_lossy().to_string(),
    }
}

/// 停用旧版 bash 守护 (停止 + 取消自启), 不删除文件
#[tauri::command]
fn disable_legacy_service() -> Result<(), String> {
    run_systemctl(&["disable", "--now", config::LEGACY_SERVICE]);
    Ok(())
}

// ---------- 试听 ----------

#[tauri::command]
fn play_sound(path: String) -> Result<(), String> {
    if !PathBuf::from(&path).exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if platform::play_file(&path) {
        Ok(())
    } else {
        Err("未找到可用播放器(pw-play/paplay/mpv/ffplay/aplay)，\n可设置环境变量 FOURY_PLAYER 指定".into())
    }
}

// ---------- 日志 / 发现的应用 ----------

fn read_log() -> String {
    let p = config::log_path();
    if let Ok(c) = fs::read_to_string(&p) {
        return c;
    }
    // 兼容旧版日志
    fs::read_to_string(config::legacy_log_path()).unwrap_or_default()
}

/// 纯函数: 从日志内容解析通知来源统计
fn parse_log_apps(content: &str, configured: &[String]) -> Vec<DiscoveredApp> {
    let mut map: std::collections::HashMap<String, (u32, String)> =
        std::collections::HashMap::new();

    for line in content.lines() {
        let Some(pos) = line.find("应用[") else { continue };
        let rest = &line[pos + "应用[".len()..];
        let Some(end) = rest.find(']') else { continue };
        let name = rest[..end].trim().to_string();
        if name.is_empty() {
            continue;
        }
        let ts = if line.starts_with('[') {
            line[1..]
                .find(']')
                .map(|e| line[1..1 + e].to_string())
                .unwrap_or_default()
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
    let configured: Vec<String> = config::read_config()
        .mappings
        .iter()
        .map(|m| m.keyword.to_lowercase())
        .collect();
    parse_log_apps(&read_log(), &configured)
}

#[tauri::command]
fn log_tail(lines: usize) -> String {
    let content = read_log();
    let all: Vec<&str> = content.lines().collect();
    let n = all.len().saturating_sub(lines.max(1));
    all[n..].join("\n")
}

#[tauri::command]
fn open_log_dir() -> Result<(), String> {
    let p = config::log_path();
    let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_else(config::home_dir);
    let _ = fs::create_dir_all(&dir);
    Command::new("xdg-open").arg(dir).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- 声音文件库 ----------

#[tauri::command]
fn list_sounds() -> Vec<SoundFile> {
    let mut out: Vec<SoundFile> = Vec::new();
    for (dir, group) in platform::sound_dirs() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() && is_audio(&p) {
                out.push(SoundFile {
                    name: p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    path: p.to_string_lossy().to_string(),
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
    platform::pick_file("选择提示音文件")
}

#[tauri::command]
fn open_config_dir() -> Result<(), String> {
    let dir = config::config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(config::home_dir);
    Command::new("xdg-open").arg(dir).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- 环境信息 ----------

#[tauri::command]
fn session_info() -> SessionInfo {
    let env = platform::detect_env();
    SessionInfo {
        os: env.os,
        desktop: env.desktop,
        session_type: env.session_type,
        kde_version: env.kde_version,
        supported: env.supported,
        tested: env.tested,
        player: env.player,
        daemon_path: config::daemon_bin_path().to_string_lossy().to_string(),
    }
}

/// 首次安装时挑选一个合理的默认兜底音
fn pick_default_sound() -> Option<String> {
    for (dir, _) in platform::sound_dirs() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_audio(p))
            .collect();
        files.sort();
        if let Some(f) = files.first() {
            return Some(f.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mapping;

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
            comments: config::default_header(),
        };

        config::write_config(&cfg).unwrap();
        let back = config::read_config();
        assert_eq!(back.mappings, cfg.mappings);
        assert_eq!(back.default_sound, cfg.default_sound);
        assert!(back.comments.iter().any(|c| c.starts_with('#')));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_match_sound() {
        let cfg = AppConfig {
            mappings: vec![Mapping {
                keyword: "wechat".into(),
                sound: "/w.ogg".into(),
            }],
            default_sound: Some("/d.ogg".into()),
            comments: vec![],
        };
        // 大小写不敏感 + 子串匹配
        assert_eq!(
            config::match_sound(&cfg, "WeChat"),
            crate::config::SoundMatch::Play {
                sound: "/w.ogg".into(),
                keyword: "wechat".into()
            }
        );
        assert_eq!(
            config::match_sound(&cfg, "com.tencent.wechat"),
            crate::config::SoundMatch::Play {
                sound: "/w.ogg".into(),
                keyword: "wechat".into()
            }
        );
        // 未命中走兜底
        assert_eq!(
            config::match_sound(&cfg, "firefox"),
            crate::config::SoundMatch::Play {
                sound: "/d.ogg".into(),
                keyword: "default".into()
            }
        );
    }

    #[test]
    fn test_silent_mapping() {
        let cfg = AppConfig {
            mappings: vec![
                Mapping {
                    keyword: "wechat".into(),
                    sound: "silent".into(), // 该应用自带提示音 -> 静音
                },
                Mapping {
                    keyword: "qq".into(),
                    sound: "/q.ogg".into(),
                },
            ],
            default_sound: Some("/d.ogg".into()),
            comments: vec![],
        };
        // 静音命中
        assert_eq!(
            config::match_sound(&cfg, "WeChat"),
            config::SoundMatch::Silent {
                keyword: "wechat".into()
            }
        );
        // 静音取值容错: mute / off / - 都算静音
        assert!(config::is_silent("MUTE"));
        assert!(config::is_silent(" off "));
        assert!(config::is_silent("-"));
        assert!(!config::is_silent("/a.ogg"));
        // 其它应用不受影响
        assert_eq!(
            config::match_sound(&cfg, "qq"),
            config::SoundMatch::Play {
                sound: "/q.ogg".into(),
                keyword: "qq".into()
            }
        );

        // 兜底音也可设为静音 -> 全部未匹配应用静音
        let cfg2 = AppConfig {
            mappings: vec![],
            default_sound: Some("silent".into()),
            comments: vec![],
        };
        assert_eq!(
            config::match_sound(&cfg2, "anything"),
            config::SoundMatch::Silent {
                keyword: "default".into()
            }
        );
    }

    #[test]
    fn parse_log_apps_works() {
        let log = "[2026-10-27 08:35:16] 应用[wechat] -> 匹配[wechat] 播放: /x.ogg\n\
[2026-10-27 08:35:17] 应用[qq] -> 匹配[qq] 播放: /y.ogg\n\
[2026-10-27 08:35:20] 应用[wechat] -> 默认音: /x.ogg\n\
[2026-10-27 08:35:21] 应用[unknownapp] 未匹配且无兜底音，跳过\n";
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
            install_service,
            uninstall_service,
            legacy_service_info,
            disable_legacy_service,
            play_sound,
            discovered_apps,
            log_tail,
            open_log_dir,
            list_sounds,
            pick_sound_file,
            open_config_dir,
            session_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 4Y Notification Manager");
}
