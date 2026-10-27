// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // ---------- CLI 子命令 (无界面, 便于脚本化部署) ----------
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--install-service") {
        match fourey_notification_manager_lib::do_install_service() {
            Ok(s) => {
                println!("✓ 后台服务已安装并启用");
                println!("  单元: {}", s.unit_path);
                println!("  程序: {}", s.exec_path);
                println!("  状态: active={} enabled={}", s.active, s.enabled);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("✗ 安装失败: {}", e);
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--uninstall-service") {
        match fourey_notification_manager_lib::do_uninstall_service() {
            Ok(_) => {
                println!("✓ 后台服务已卸载 (配置文件保留)");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("✗ 卸载失败: {}", e);
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--version") {
        println!("4Y Notification Manager {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    fourey_notification_manager_lib::run()
}
