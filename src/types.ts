export interface Mapping {
  keyword: string;
  sound: string;
}

export interface AppConfig {
  mappings: Mapping[];
  default_sound: string | null;
  comments: string[];
}

export interface ServiceStatus {
  active: boolean;
  enabled: boolean;
  /** systemd 用户单元是否已安装 */
  installed: boolean;
  state: string;
  unit_path: string;
  exec_path: string;
}

export interface LegacyInfo {
  active: boolean;
  enabled: boolean;
  script_exists: boolean;
  script_path: string;
}

export interface DiscoveredApp {
  name: string;
  count: number;
  last_seen: string;
  configured: boolean;
}

export interface SoundFile {
  path: string;
  name: string;
  group: string;
}

export interface SessionInfo {
  os: string;
  desktop: string;
  session_type: string;
  kde_version: string;
  /** 理论可用(任何带通知接口的 Linux) */
  supported: boolean;
  /** 是否在作者测试过的环境(Debian13+KDE6+Wayland) */
  tested: boolean;
  /** 检测到的可用播放器 */
  player: string | null;
  daemon_path: string;
}
