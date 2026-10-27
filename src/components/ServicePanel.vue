<script setup lang="ts">
import type { ServiceStatus, SessionInfo } from "../types";

defineProps<{
  status: ServiceStatus;
  session: SessionInfo;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: "action", action: string): void;
  (e: "openConfig"): void;
  (e: "refresh"): void;
}>();
</script>

<template>
  <div class="card">
    <div class="card-title">守护服务状态</div>

    <div class="row" style="gap: 16px">
      <span class="pill" :class="status.active ? 'on' : 'off'">
        <span class="dot" />
        {{ status.active ? "运行中" : "已停止" }}
      </span>
      <span class="tag" :class="status.enabled ? 'good' : 'bad'">
        {{ status.enabled ? "开机自启已开启" : "开机自启未开启" }}
      </span>
      <span class="spacer" />
      <button class="btn ghost sm" :disabled="busy" @click="emit('refresh')">
        刷新
      </button>
    </div>

    <div class="divider" />

    <div class="row" style="flex-wrap: wrap; gap: 10px">
      <button
        class="btn ok"
        :disabled="busy || status.active"
        @click="emit('action', 'start')"
      >
        ▶ 启动
      </button>
      <button
        class="btn"
        :disabled="busy || !status.active"
        @click="emit('action', 'stop')"
      >
        ■ 停止
      </button>
      <button class="btn" :disabled="busy" @click="emit('action', 'restart')">
        ↻ 重启
      </button>
      <button
        v-if="!status.enabled"
        class="btn"
        :disabled="busy"
        @click="emit('action', 'enable')"
      >
        ⏻ 开机自启
      </button>
      <button
        v-else
        class="btn danger"
        :disabled="busy"
        @click="emit('action', 'disable')"
      >
        ⏻ 取消自启
      </button>
    </div>

    <div class="divider" />

    <div class="row">
      <div style="min-width: 0">
        <div class="faint" style="font-size: 11px; margin-bottom: 3px">
          配置文件
        </div>
        <div class="mono faint" style="font-size: 12px">
          ~/.config/notification-sound.conf
        </div>
      </div>
      <span class="spacer" />
      <button class="btn sm" @click="emit('openConfig')">打开目录</button>
    </div>
  </div>

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
        <div style="font-size: 13px">
          {{ session.kde_version || "未知" }}
        </div>
      </div>
      <div>
        <div class="faint" style="font-size: 11px; margin-bottom: 4px">适配检查</div>
        <span class="tag" :class="session.supported ? 'good' : 'bad'">
          {{ session.supported ? "✓ Debian13 + KDE6 + Wayland" : "当前环境非目标平台" }}
        </span>
      </div>
    </div>
  </div>
</template>
