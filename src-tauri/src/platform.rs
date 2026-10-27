//! # 平台适配层
//!
//! 把所有"跟具体机器/发行版/桌面环境相关"的东西集中在这里。
//! 想让本项目适配你的机器，**通常只需要改这个文件**（详见仓库根目录 ADAPTING.md）。
//!
//! 可调项：
//!   - 服务名 / 守护进程名            -> SERVICE / DAEMON_BIN
//!   - 声音播放器候选(按优先级)        -> PLAYERS 或环境变量 FOURY_PLAYER
//!   - 声音文件扫描目录                -> DEFAULT_SOUND_DIRS 或环境变量 FOURY_SOUND_DIRS
//!   - 文件选择器候选                  -> pick_file()
//!   - 通知来源 D-Bus 接口             -> NOTIFY_INTERFACE

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// systemd 用户单元名
pub const SERVICE: &str = "4y-notify-daemon.service";
/// 旧版(bash)服务名, 仅用于迁移提示
pub const LEGACY_SERVICE: &str = "notification-sound.service";
/// 守护进程可执行文件名
pub const DAEMON_BIN: &str = "fourey-notify-daemon";
/// 仓库地址(单元文件 Documentation= 用)
pub const REPO: &str = env!("CARGO_PKG_REPOSITORY");
/// 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 通知接口: 绝大多 Linux 桌面(nofityd/dunst/plasmashell/mako/swaync)都实现它
pub const NOTIFY_INTERFACE: &str = "org.freedesktop.Notifications";
pub const NOTIFY_MEMBER: &str = "Notify";

/// 可识别的音频扩展名
pub const AUDIO_EXT: [&str; 6] = ["ogg", "oga", "mp3", "wav", "flac", "m4a"];

/// 声音播放器候选(按顺序尝试第一个存在的)
const PLAYERS: &[&str] = &["pw-play", "paplay", "mpv", "ffplay", "aplay"];

// ---------------------------------------------------------------------------
// 播放
// ---------------------------------------------------------------------------

fn find_in_path(bin: &str) -> Option<PathBuf> {
    if bin.contains('/') {
        let p = PathBuf::from(bin);
        return if p.is_file() { Some(p) } else { None };
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|c| c.is_file())
}

/// 某播放器对应的参数(不含程序名)
fn player_args(player: &str, file: &str) -> Vec<String> {
    match player {
        "mpv" => vec!["--no-video".into(), "--really-quiet".into(), file.into()],
        "ffplay" => vec![
            "-nodisp".into(),
            "-autoexit".into(),
            "-loglevel".into(),
            "quiet".into(),
            file.into(),
        ],
        "aplay" => vec!["-q".into(), file.into()],
        "canberra-gtk-play" => vec!["-f".into(), file.into()],
        // pw-play / paplay / 其它: 直接跟路径
        _ => vec![file.into()],
    }
}

/// 播放一个声音文件。依次尝试候选播放器，成功启动即返回 true。
///
/// 可通过环境变量 `FOURY_PLAYER` 覆盖，例如 `FOURY_PLAYER=paplay`。
pub fn play_file(file: &str) -> bool {
    if file.trim().is_empty() || !Path::new(file).exists() {
        return false;
    }

    let order: Vec<String> = std::env::var("FOURY_PLAYER")
        .ok()
        .map(|s| {
            s.split(':')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| PLAYERS.iter().map(|s| s.to_string()).collect());

    for p in order {
        if find_in_path(&p).is_none() {
            continue;
        }
        let spawned = Command::new(&p)
            .args(player_args(&p, file))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if spawned.is_ok() {
            return true;
        }
    }
    false
}

/// 当前系统可用的播放器(用于界面提示)
pub fn available_player() -> Option<String> {
    PLAYERS
        .iter()
        .find(|p| find_in_path(p).is_some())
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// 声音目录
// ---------------------------------------------------------------------------

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

/// 读取 ~/.config/user-dirs.dirs 中的 XDG 目录(支持 $HOME 展开)
fn xdg_user_dir(key: &str) -> Option<PathBuf> {
    let home = home_dir();
    let content = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).ok()?;
    let prefix = format!("{}=", key);
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&prefix) {
            let v = rest.trim().trim_matches('"');
            let expanded = v.replace("$HOME", &home.to_string_lossy());
            let p = PathBuf::from(expanded);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn label_for(dir: &Path) -> String {
    let s = dir.to_string_lossy();
    if s.starts_with("/usr/share/sounds") {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        format!("系统 · {}", name)
    } else {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| s.to_string());
        format!("我的 · {}", name)
    }
}

/// 返回可扫描的声音目录列表 (路径, 分组标签)
///
/// 优先级: 环境变量 `FOURY_SOUND_DIRS`(冒号分隔) > XDG 音乐/文档目录 > 常见回退目录
fn add_dir(out: &mut Vec<(PathBuf, String)>, p: PathBuf) {
    if p.is_dir() && !out.iter().any(|(q, _)| q == &p) {
        let label = label_for(&p);
        out.push((p, label));
    }
}

pub fn sound_dirs() -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();

    // 1) 环境变量覆盖
    if let Ok(v) = std::env::var("FOURY_SOUND_DIRS") {
        for part in v.split(':').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let p = PathBuf::from(part.replace("$HOME", &home_dir().to_string_lossy()));
            add_dir(&mut out, p);
        }
        if !out.is_empty() {
            return out;
        }
    }

    // 2) XDG 标准目录
    let home = home_dir();
    let music = xdg_user_dir("XDG_MUSIC_DIR").unwrap_or_else(|| home.join("Music"));
    let docs = xdg_user_dir("XDG_DOCUMENTS_DIR").unwrap_or_else(|| home.join("Documents"));
    add_dir(&mut out, music.clone());
    add_dir(&mut out, music.join("铃声"));
    add_dir(&mut out, music.join("Ringtones"));
    add_dir(&mut out, docs.join("提示音"));
    add_dir(&mut out, docs.join("Sounds"));

    // 3) 常见回退(不同发行版/语言的目录名)
    for p in [
        home.join("音乐"),
        home.join("音乐/铃声"),
        home.join("文档/提示音"),
        home.join("Music"),
    ] {
        add_dir(&mut out, p);
    }

    // 4) 系统音效主题(不同发行版各有一套)
    for p in [
        "/usr/share/sounds/ocean/stereo",         // KDE
        "/usr/share/sounds/freedesktop/stereo",   // 通用
        "/usr/share/sounds/gnome/default/alerts", // GNOME
        "/usr/share/sounds/Yaru/stereo",          // Ubuntu
        "/usr/share/sounds/ubuntu/stereo",        // 旧 Ubuntu
    ] {
        add_dir(&mut out, PathBuf::from(p));
    }

    out
}

// ---------------------------------------------------------------------------
// 文件选择器
// ---------------------------------------------------------------------------

/// 音频文件过滤器(按环境变量按需改)
const AUDIO_GLOBS: &str = "*.ogg *.oga *.mp3 *.wav *.flac *.m4a";

/// 弹出一个原生文件选择框, 取消返回 None
///
/// 依次尝试 kdialog(KDE) / zenity(GNOME/通用) / yad。
pub fn pick_file(title: &str) -> Option<String> {
    let start = home_dir();

    // KDE
    if let Some(out) = run_picker(
        "kdialog",
        &[
            "--getopenfilename",
            &start.to_string_lossy(),
            &format!("{}|音频文件", AUDIO_GLOBS),
            "--title",
            title,
        ],
    ) {
        return out;
    }

    // GNOME / 通用
    if let Some(out) = run_picker(
        "zenity",
        &[
            "--file-selection",
            &format!("--title={}", title),
            &format!("--file-filter=音频文件 | {}", AUDIO_GLOBS),
        ],
    ) {
        return out;
    }

    // yad
    if let Some(out) = run_picker(
        "yad",
        &[
            "--file",
            &format!("--title={}", title),
            &format!("--file-filter=音频文件 | {}", AUDIO_GLOBS),
        ],
    ) {
        return out;
    }

    None
}

/// 返回 Some(Ok(路径)) / Some(None 表示用户取消) / None 表示该选择器不可用
fn run_picker(bin: &str, args: &[&str]) -> Option<Option<String>> {
    if find_in_path(bin).is_none() {
        return None;
    }
    let out = Command::new(bin).args(args).output().ok()?;
    if !out.status.success() {
        // 用户取消(非 0 退出) 或失败, 都当作取消
        return Some(None);
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(if s.is_empty() { None } else { Some(s) })
}

// ---------------------------------------------------------------------------
// 环境自检
// ---------------------------------------------------------------------------

pub struct EnvSummary {
    pub os: String,
    pub desktop: String,
    pub session_type: String,
    pub kde_version: String,
    /// 是否在作者测试过的环境(Debian13+KDE6+Wayland)
    pub tested: bool,
    /// 是否理论上可用(任何实现了通知接口的 Linux)
    pub supported: bool,
    pub player: Option<String>,
}

pub fn detect_env() -> EnvSummary {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| {
                    l.trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string()
                })
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

    let tested = os.contains("Debian GNU/Linux 13")
        && desktop.to_uppercase().contains("KDE")
        && session_type.eq_ignore_ascii_case("wayland");

    // 只要在 Linux 且有播放器, 就认为"理论可用" —— 通知监听走 D-Bus, 与桌面无关
    let has_player = available_player().is_some();
    let supported = cfg!(target_os = "linux") && has_player;

    EnvSummary {
        os,
        desktop,
        session_type,
        kde_version,
        tested,
        supported,
        player: available_player(),
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_args_shapes() {
        assert_eq!(player_args("pw-play", "/a.ogg"), vec!["/a.ogg"]);
        assert_eq!(player_args("mpv", "/a.ogg").len(), 3);
        assert!(player_args("ffplay", "/a.ogg").contains(&"-nodisp".to_string()));
    }

    #[test]
    fn find_sh_exists() {
        // 任何 Linux 上 /bin/sh 都在 PATH 中
        assert!(find_in_path("sh").is_some());
    }

    #[test]
    fn sound_dirs_have_system_theme() {
        let dirs = sound_dirs();
        assert!(!dirs.is_empty(), "至少应包含一个系统音效目录");
    }
}
