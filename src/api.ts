// 与 Rust 后端通信的封装层
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  DiscoveredApp,
  ServiceStatus,
  SessionInfo,
  SoundFile,
} from "./types";

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<string>("save_config", { config }),
  serviceStatus: () => invoke<ServiceStatus>("service_status"),
  serviceAction: (action: string) =>
    invoke<ServiceStatus>("service_action", { action }),
  playSound: (path: string) => invoke<void>("play_sound", { path }),
  discoveredApps: () => invoke<DiscoveredApp[]>("discovered_apps"),
  listSounds: () => invoke<SoundFile[]>("list_sounds"),
  pickSoundFile: () => invoke<string | null>("pick_sound_file"),
  openConfigDir: () => invoke<void>("open_config_dir"),
  sessionInfo: () => invoke<SessionInfo>("session_info"),
};
