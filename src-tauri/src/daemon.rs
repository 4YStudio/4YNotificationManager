// 4Y 通知提示音守护进程
//
// 工作方式:
//   1. 启动 `dbus-monitor` 监听会话总线上 org.freedesktop.Notifications.Notify 调用
//   2. 从调用参数中取出第一个 string 参数(即发送通知的应用名)
//   3. 按 ~/.config/notification-sound.conf 匹配应播放的声音
//   4. 用 pw-play (PipeWire) 播放, 失败回退 paplay
//
// 配置每次通知时重新读取, 因此改完配置立即生效, 无需重启。

use crate::config;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LOG_MAX_BYTES: u64 = 2 * 1024 * 1024; // 2MB 后轮转
const DEBOUNCE: Duration = Duration::from_millis(1000);

fn timestamp() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn log(msg: &str) {
    let path = config::log_path();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    // 超过上限则清空, 避免无限增长
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > LOG_MAX_BYTES {
            let _ = fs::write(&path, "");
        }
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{}] {}", timestamp(), msg);
    }
}

fn play_file(path: &str) {
    if path.trim().is_empty() {
        return;
    }
    if !PathBuf::from(path).exists() {
        log(&format!("声音文件不存在: {}", path));
        return;
    }
    let ok = Command::new("pw-play")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if !ok {
        let _ = Command::new("paplay")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// 处理一条通知: 应用名 -> 匹配 -> 播放
fn handle(app: &str) {
    let cfg = config::read_config();
    match config::match_sound(&cfg, app) {
        Some((sound, matched)) => {
            log(&format!("应用[{}] -> 匹配[{}] 播放: {}", app, matched, sound));
            play_file(&sound);
        }
        None => {
            log(&format!("应用[{}] 未匹配且无兜底音，跳过", app));
        }
    }
}

/// 启动一次 dbus-monitor 并处理其输出; 返回时表示监听已断开
fn monitor_once() -> std::io::Result<()> {
    let mut child = Command::new("dbus-monitor")
        .arg("--session")
        .arg("interface='org.freedesktop.Notifications',member='Notify'")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().expect("piped stdout");
    let reader = BufReader::new(stdout);

    let mut expect_app = false;
    let mut last_play = Instant::now() - DEBOUNCE;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        // 进入 Notify 调用块
        if line.contains("member=Notify") {
            expect_app = true;
            continue;
        }

        if !expect_app {
            continue;
        }

        // Notify 的第一个参数就是应用名, 形如:    string "wechat"
        if let Some(rest) = line.trim_start().strip_prefix("string \"") {
            let app = rest.strip_suffix('"').unwrap_or(rest).to_string();
            expect_app = false;

            let now = Instant::now();
            if now.duration_since(last_play) >= DEBOUNCE {
                if !app.is_empty() {
                    handle(&app);
                }
                last_play = now;
            } else {
                log(&format!("通知过于频繁已合并({})", app));
            }
            continue;
        }

        // 遇到下一条消息却还没找到 string 参数 -> 放弃本块
        if line.starts_with("method ")
            || line.starts_with("signal ")
            || line.starts_with("error ")
        {
            expect_app = false;
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

/// 守护进程主循环: 永不返回
pub fn run() -> ! {
    log("4Y 通知提示音守护进程启动");
    loop {
        if let Err(e) = monitor_once() {
            log(&format!("dbus-monitor 启动失败: {}，1 秒后重试", e));
        } else {
            log("dbus-monitor 已断开，1 秒后重连");
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
