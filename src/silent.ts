/** 配置中表示"命中但静音不播放"的特殊取值 */
export const SILENT_VALUE = "silent";

/** 后端接受的静音写法（与 config.rs 的 SILENT_TOKENS 保持一致） */
const SILENT_TOKENS = ["silent", "mute", "off", "none", "-"];

export function isSilent(value: string | null | undefined): boolean {
  if (!value) return false;
  return SILENT_TOKENS.includes(value.trim().toLowerCase());
}
