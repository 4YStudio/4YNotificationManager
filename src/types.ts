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
  supported: boolean;
  daemon_path: string;
}
