<script setup lang="ts">
import type { AppConfig } from "../types";
import { isSilent, SILENT_VALUE } from "../silent";

defineProps<{
  config: AppConfig;
  dirty: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  (e: "pick", index: number): void;
  (e: "pickDefault"): void;
  (e: "play", path: string): void;
  (e: "save"): void;
  (e: "add"): void;
  (e: "remove", index: number): void;
  (e: "clearDefault"): void;
  (e: "toggleMute", index: number): void;
}>();

function base(p: string): string {
  if (!p) return "";
  const parts = p.split("/");
  return parts[parts.length - 1] || p;
}

function mute(index: number) {
  emit("toggleMute", index);
}
</script>

<template>
  <div class="card">
    <div class="row" style="margin-bottom: 4px">
      <div class="card-title" style="margin: 0">应用 → 提示音 映射</div>
      <span class="spacer" />
      <span v-if="dirty" class="tag bad">有未保存的修改</span>
      <button class="btn primary" :disabled="saving || !dirty" @click="emit('save')">
        {{ saving ? "保存中…" : "保存配置" }}
      </button>
    </div>

    <div style="font-size: 12px; color: var(--text-faint); margin-bottom: 14px">
      匹配规则：通知来源的应用名<strong>包含</strong>关键词即命中（不区分大小写），自上而下匹配，命中第一条即生效。
      若某应用<strong>自带提示音</strong>，把它设为 <span class="mono">🔇 静音</span> 即可跳过系统提示音。
    </div>

    <table class="map-table">
      <thead>
        <tr>
          <th style="width: 28%">应用关键词</th>
          <th>提示音</th>
          <th class="col-act">操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(m, i) in config.mappings" :key="i">
          <td>
            <input v-model="m.keyword" class="input" placeholder="如 wechat / qq / firefox" />
          </td>
          <td>
            <div v-if="isSilent(m.sound)" class="row" style="gap: 8px">
              <span class="tag" style="background: rgba(246,116,0,.15); color: var(--warn); border-color: rgba(246,116,0,.4)">
                🔇 静音（不播放）
              </span>
            </div>
            <div v-else class="row" style="gap: 8px; min-width: 0">
              <div style="min-width: 0; flex: 1">
                <span class="basename">{{ base(m.sound) || "（未选择）" }}</span>
                <span class="path-display faint" :title="m.sound">{{ m.sound }}</span>
              </div>
            </div>
          </td>
          <td class="col-act">
            <div class="row" style="gap: 6px; justify-content: flex-end">
              <button
                class="btn ghost sm"
                title="试听"
                :disabled="isSilent(m.sound) || !m.sound"
                @click="emit('play', m.sound)"
              >
                ▶
              </button>
              <button
                class="btn sm"
                :class="{ danger: !isSilent(m.sound) }"
                :title="isSilent(m.sound) ? '取消静音（重新选择声音）' : '静音：该应用自带提示音，跳过系统提示音'"
                @click="mute(i)"
              >
                {{ isSilent(m.sound) ? "🔊 取消静音" : "🔇 静音" }}
              </button>
              <button class="btn sm" @click="emit('pick', i)">选择</button>
              <button class="btn danger sm" title="删除" @click="emit('remove', i)">✕</button>
            </div>
          </td>
        </tr>

        <!-- 兜底音 -->
        <tr>
          <td>
            <span class="tag" style="background: var(--elev)">default</span>
            <span class="faint" style="font-size: 11px; margin-left: 6px">兜底</span>
          </td>
          <td>
            <div v-if="isSilent(config.default_sound)" class="row">
              <span class="tag" style="background: rgba(246,116,0,.15); color: var(--warn); border-color: rgba(246,116,0,.4)">
                🔇 静音（所有未匹配通知都不播放）
              </span>
            </div>
            <div v-else style="min-width: 0">
              <span class="basename">
                {{ config.default_sound ? base(config.default_sound) : "（未设置 → 未匹配通知静音）" }}
              </span>
              <span
                v-if="config.default_sound"
                class="path-display faint"
                :title="config.default_sound"
                >{{ config.default_sound }}</span
              >
            </div>
          </td>
          <td class="col-act">
            <div class="row" style="gap: 6px; justify-content: flex-end">
              <button
                class="btn ghost sm"
                title="试听"
                :disabled="isSilent(config.default_sound) || !config.default_sound"
                @click="config.default_sound && emit('play', config.default_sound)"
              >
                ▶
              </button>
              <button
                class="btn sm"
                :class="{ danger: !isSilent(config.default_sound) }"
                @click="config.default_sound = isSilent(config.default_sound) ? null : SILENT_VALUE"
              >
                {{ isSilent(config.default_sound) ? "🔊 恢复" : "🔇 静音" }}
              </button>
              <button class="btn sm" @click="emit('pickDefault')">选择</button>
              <button
                v-if="config.default_sound"
                class="btn danger sm"
                title="清除兜底音"
                @click="emit('clearDefault')"
              >
                ✕
              </button>
            </div>
          </td>
        </tr>
      </tbody>
    </table>

    <div v-if="config.mappings.length === 0" class="empty" style="padding: 24px 0">
      还没有任何映射，点击下方按钮开始添加<br />
      未配置时所有通知都会播放「兜底音」
    </div>

    <button class="btn ghost" style="margin-top: 14px" @click="emit('add')">
      ＋ 添加一条映射
    </button>
  </div>
</template>
