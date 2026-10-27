// 与 Rust 后端通信的封装层
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  DiscoveredApp,
  LegacyInfo,
  ServiceStatus,
  SessionInfo,
  SoundFile,
} from "./types";

export const api = {
  // 配置
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<string>("save_config", { config }),

  // 后台服务 (systemd 用户单元)
  serviceStatus: () => invoke<ServiceStatus>("service_status"),
  serviceAction: (action: string) =>
    invoke<ServiceStatus>("service_action", { action }),
  installService: () => invoke<ServiceStatus>("install_service"),
  uninstallService: () => invoke<ServiceStatus>("uninstall_service"),

  // 旧版 bash 服务迁移
  legacyServiceInfo: () => invoke<LegacyInfo>("legacy_service_info"),
  disableLegacyService: () => invoke<void>("disable_legacy_service"),

  // 声音
  playSound: (path: string) => invoke<void>("play_sound", { path }),
  listSounds: () => invoke<SoundFile[]>("list_sounds"),
  pickSoundFile: () => invoke<string | null>("pick_sound_file"),

  // 日志 / 发现
  discoveredApps: () => invoke<DiscoveredApp[]>("discovered_apps"),
  logTail: (lines: number) => invoke<string>("log_tail", { lines }),
  openLogDir: () => invoke<void>("open_log_dir"),

  // 杂项
  openConfigDir: () => invoke<void>("open_config_dir"),
  sessionInfo: () => invoke<SessionInfo>("session_info"),
};
