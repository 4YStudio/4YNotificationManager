<script setup lang="ts">
import { computed } from "vue";
import type { SoundFile } from "../types";
import { SILENT_VALUE } from "../silent";

const props = defineProps<{
  open: boolean;
  sounds: SoundFile[];
  title: string;
}>();

const emit = defineEmits<{
  (e: "choose", path: string): void;
  (e: "play", path: string): void;
  (e: "browse"): void;
  (e: "close"): void;
}>();

// 按分组归类展示
const groups = computed(() => {
  const m = new Map<string, SoundFile[]>();
  for (const s of props.sounds) {
    if (!m.has(s.group)) m.set(s.group, []);
    m.get(s.group)!.push(s);
  }
  return [...m.entries()];
});
</script>

<template>
  <div v-if="open" class="modal-mask" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-head">
        <span>{{ title }}</span>
        <span class="spacer" />
        <button class="btn ghost sm" @click="emit('close')">✕</button>
      </div>

      <div class="modal-body">
        <!-- 静音选项 -->
        <div
          class="sound-item"
          style="border-bottom: 1px solid var(--border-soft); margin-bottom: 4px"
          @click="emit('choose', SILENT_VALUE)"
        >
          <span class="grp">特殊</span>
          <span class="nm" style="color: var(--warn)">🔇 静音（不播放）</span>
        </div>

        <div v-if="sounds.length === 0" class="empty">
          没有找到可用的声音文件<br />
          点击下方「浏览文件…」手动选择
        </div>

        <template v-for="[group, items] in groups" :key="group">
          <div
            style="
              font-size: 11px;
              color: var(--text-faint);
              padding: 10px 12px 4px;
              text-transform: uppercase;
              letter-spacing: 0.6px;
            "
          >
            {{ group }}
          </div>
          <div
            v-for="s in items"
            :key="s.path"
            class="sound-item"
            @click="emit('choose', s.path)"
          >
            <span class="nm">{{ s.name }}</span>
            <button
              class="btn ghost sm"
              title="试听"
              @click.stop="emit('play', s.path)"
            >
              ▶
            </button>
          </div>
        </template>
      </div>

      <div class="modal-foot">
        <button class="btn" @click="emit('browse')">浏览文件…</button>
        <button class="btn ghost" @click="emit('close')">取消</button>
      </div>
    </div>
  </div>
</template>
