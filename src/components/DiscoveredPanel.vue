<script setup lang="ts">
import type { DiscoveredApp } from "../types";

defineProps<{
  apps: DiscoveredApp[];
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: "add", name: string): void;
  (e: "refresh"): void;
}>();
</script>

<template>
  <div class="card">
    <div class="row" style="margin-bottom: 4px">
      <div class="card-title" style="margin: 0">发现的应用</div>
      <span class="spacer" />
      <button class="btn ghost sm" :disabled="loading" @click="emit('refresh')">
        刷新
      </button>
    </div>

    <div style="font-size: 12px; color: var(--text-faint); margin-bottom: 14px">
      这些是系统通知日志里出现过的通知来源。点击「添加」即可为它配置专属提示音。
    </div>

    <div v-if="apps.length === 0" class="empty">
      还没有捕获到任何通知来源<br />
      让任意软件弹一条通知后再来刷新
    </div>

    <div v-else>
      <div
        v-for="a in apps"
        :key="a.name"
        class="row"
        style="
          padding: 10px 12px;
          border-radius: 8px;
          border-bottom: 1px solid var(--border-soft);
        "
      >
        <div style="min-width: 0">
          <div style="font-size: 13.5px; font-weight: 550">{{ a.name }}</div>
          <div class="faint" style="font-size: 11px">
            出现 {{ a.count }} 次
            <template v-if="a.last_seen"> · 最近 {{ a.last_seen }}</template>
          </div>
        </div>
        <span class="spacer" />
        <span v-if="a.configured" class="tag good">已配置</span>
        <button v-else class="btn sm" @click="emit('add', a.name)">＋ 添加</button>
      </div>
    </div>
  </div>
</template>
