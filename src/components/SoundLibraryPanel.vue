<script setup lang="ts">
import { computed, ref } from "vue";
import type { SoundFile } from "../types";

const props = defineProps<{
  sounds: SoundFile[];
}>();

const emit = defineEmits<{
  (e: "play", path: string): void;
  (e: "setDefault", path: string): void;
}>();

const query = ref("");

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.sounds;
  return props.sounds.filter(
    (s) =>
      s.name.toLowerCase().includes(q) || s.group.toLowerCase().includes(q)
  );
});

const groups = computed(() => {
  const m = new Map<string, SoundFile[]>();
  for (const s of filtered.value) {
    if (!m.has(s.group)) m.set(s.group, []);
    m.get(s.group)!.push(s);
  }
  return [...m.entries()];
});
</script>

<template>
  <div class="card">
    <div class="card-title">可用声音库</div>

    <input v-model="query" class="input" placeholder="搜索声音名称…" />

    <div v-if="filtered.length === 0" class="empty">没有匹配的声音文件</div>

    <template v-for="[group, items] in groups" :key="group">
      <div
        style="
          font-size: 11px;
          color: var(--text-faint);
          text-transform: uppercase;
          letter-spacing: 0.6px;
          padding: 16px 2px 8px;
        "
      >
        {{ group }} · {{ items.length }}
      </div>
      <div
        v-for="s in items"
        :key="s.path"
        class="row"
        style="
          padding: 10px 12px;
          border-radius: 8px;
          border-bottom: 1px solid var(--border-soft);
        "
      >
        <div style="min-width: 0; flex: 1">
          <div style="font-size: 13.5px">{{ s.name }}</div>
          <div class="path-display faint">{{ s.path }}</div>
        </div>
        <button class="btn ghost sm" @click="emit('play', s.path)">▶ 试听</button>
        <button class="btn ghost sm" @click="emit('setDefault', s.path)">
          设为兜底
        </button>
      </div>
    </template>
  </div>
</template>
