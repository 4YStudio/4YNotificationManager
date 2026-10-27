<script setup lang="ts">
import { computed } from "vue";
import type { ServiceStatus, SessionInfo } from "../types";

const props = defineProps<{
  status: ServiceStatus;
  session: SessionInfo;
  busy: boolean;
  logText: string;
}>();

const emit = defineEmits<{
  (e: "action", action: string): void;
  (e: "install"): void;
  (e: "uninstall"): void;
  (e: "openConfig"): void;
  (e: "openLog"): void;
  (e: "refresh"): void;
  (e: "refreshLog"): void;
}>();

const stateText = computed(() => {
  if (!props.status.installed) return "未安装";
  return props.status.active ? "运行中" : "已停止";
});

function confirmUninstall() {
  if (window.confirm("确定要卸载后台服务吗？\n将停止服务并删除 systemd 用户单元，配置文件会保留。")) {
    emit("uninstall");
  }
}
</script>

<template>
  <!-- 未安装: 一键启用引导 -->
  <div v-if="!status.installed" class="card" style="border-color: rgba(61, 174, 233, 0.45)">
    <div class="card-title">后台服务未安装</div>
    <p style="font-size: 13px; line-height: 1.8; color: var(--text-dim)">
      通知提示音依赖一个常驻后台服务来监听系统通知。<br />
      点击下方按钮将自动完成：写入 systemd 用户单元 → 设置开机自启 → 立即启动。
    </p>
    <div class="row" style="margin-top: 16px; gap: 10px">
      <button class="btn primary" :disabled="busy" @click="emit('install')">
        ⚡ 一键启用后台服务
      </button>
      <button class="btn ghost" :disabled="busy" @click="emit('refresh')">刷新</button>
    </div>
    <div class="faint" style="font-size: 11px; margin-top: 12px">
      守护程序：<span class="mono">{{ status.exec_path || session.daemon_path }}</span>
    </div>
  </div>

  <!-- 已安装: 状态与控制 -->
  <div v-else class="card">
    <div class="card-title">后台服务</div>

    <div class="row" style="gap: 16px">
      <span class="pill" :class="status.active ? 'on' : 'off'">
        <span class="dot" />
        {{ stateText }}
      </span>
      <span class="tag" :class="status.enabled ? 'good' : 'bad'">
        {{ status.enabled ? "开机自启已开启" : "开机自启未开启" }}
      </span>
      <span class="spacer" />
      <button class="btn ghost sm" :disabled="busy" @click="emit('refresh')">刷新</button>
    </div>

    <div class="divider" />

    <div class="row" style="flex-wrap: wrap; gap: 10px">
      <button class="btn ok" :disabled="busy || status.active" @click="emit('action', 'start')">
        ▶ 启动
      </button>
      <button class="btn" :disabled="busy || !status.active" @click="emit('action', 'stop')">
        ■ 停止
      </button>
      <button class="btn" :disabled="busy" @click="emit('action', 'restart')">↻ 重启</button>
      <button v-if="!status.enabled" class="btn" :disabled="busy" @click="emit('action', 'enable')">
        ⏻ 开机自启
      </button>
      <button v-else class="btn" :disabled="busy" @click="emit('action', 'disable')">
        ⏻ 取消自启
      </button>
    </div>

    <div class="divider" />

    <div class="grid-2" style="gap: 12px">
      <div style="min-width: 0">
        <div class="faint" style="font-size: 11px; margin-bottom: 3px">配置文件</div>
        <div class="mono faint" style="font-size: 12px">~/.config/notification-sound.conf</div>
      </div>
      <div style="min-width: 0">
        <div class="faint" style="font-size: 11px; margin-bottom: 3px">守护程序</div>
        <div class="mono faint" style="font-size: 12px; overflow: hidden; text-overflow: ellipsis">
          {{ status.exec_path }}
        </div>
      </div>
    </div>

    <div class="row" style="margin-top: 14px">
      <button class="btn sm" @click="emit('openConfig')">打开配置目录</button>
      <button class="btn sm" @click="emit('openLog')">打开日志目录</button>
      <span class="spacer" />
      <button class="btn danger sm" :disabled="busy" @click="confirmUninstall">卸载服务</button>
    </div>
  </div>

  <!-- 运行环境 -->
  <div class="card">
    <div class="card-title">运行环境</div>
    <div class="grid-2">
      <div>
        <div class="faint" style="font-size: 11px; margin-bottom: 4px">操作系统</div>
        <div style="font-size: 13px">{{ session.os }}</div>
      </div>
      <div>
        <div class="faint" style="font-size: 11px; margin-bottom: 4px">桌面 / 会话</div>
        <div style="font-size: 13px">
          {{ session.desktop || "未知" }} · {{ session.session_type || "未知" }}
        </div>
      </div>
      <div>
        <div class="faint" style="font-size: 11px; margin-bottom: 4px">KDE Plasma</div>
        <div style="font-size: 13px">{{ session.kde_version || "未知" }}</div>
      </div>
      <div>
        <div class="faint" style="font-size: 11px; margin-bottom: 4px">适配检查</div>
        <span class="tag" :class="session.supported ? 'good' : 'bad'">
          {{ session.supported ? "✓ Debian13 + KDE6 + Wayland" : "当前环境非目标平台" }}
        </span>
      </div>
    </div>
  </div>

  <!-- 日志 -->
  <div class="card">
    <div class="row" style="margin-bottom: 12px">
      <div class="card-title" style="margin: 0">后台服务日志</div>
      <span class="spacer" />
      <button class="btn ghost sm" @click="emit('refreshLog')">刷新</button>
    </div>
    <pre
      class="mono"
      style="
        font-size: 11.5px;
        line-height: 1.6;
        color: var(--text-dim);
        background: var(--bg-2);
        border: 1px solid var(--border-soft);
        border-radius: 8px;
        padding: 12px;
        max-height: 240px;
        overflow: auto;
        white-space: pre-wrap;
        word-break: break-all;
      "
      >{{ logText || "（暂无日志）" }}</pre>
  </div>
</template>
