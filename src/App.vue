<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { api } from "./api";
import type {
  AppConfig,
  DiscoveredApp,
  LegacyInfo,
  ServiceStatus,
  SessionInfo,
  SoundFile,
} from "./types";
import MappingsPanel from "./components/MappingsPanel.vue";
import ServicePanel from "./components/ServicePanel.vue";
import DiscoveredPanel from "./components/DiscoveredPanel.vue";
import SoundLibraryPanel from "./components/SoundLibraryPanel.vue";
import SoundPicker from "./components/SoundPicker.vue";

type Tab = "mappings" | "service" | "discovered" | "library";

const tab = ref<Tab>("mappings");

const config = reactive<AppConfig>({ mappings: [], default_sound: null, comments: [] });
const snapshot = ref("");

const status = reactive<ServiceStatus>({
  active: false,
  enabled: false,
  installed: false,
  state: "",
  unit_path: "",
  exec_path: "",
});
const session = reactive<SessionInfo>({
  os: "",
  desktop: "",
  session_type: "",
  kde_version: "",
  supported: false,
  tested: false,
  player: null,
  daemon_path: "",
});
const legacy = ref<LegacyInfo | null>(null);
const discovered = ref<DiscoveredApp[]>([]);
const sounds = ref<SoundFile[]>([]);
const logText = ref("");

const saving = ref(false);
const busy = ref(false);
const discLoading = ref(false);

const dirty = computed(() => JSON.stringify(config) !== snapshot.value);
const legacyRunning = computed(() => !!legacy.value && (legacy.value.active || legacy.value.enabled));

// ---------- Toast ----------
interface Toast {
  id: number;
  msg: string;
  kind: "ok" | "err" | "";
}
const toasts = ref<Toast[]>([]);
let toastSeq = 0;
function toast(msg: string, kind: "ok" | "err" | "" = "") {
  const id = ++toastSeq;
  toasts.value.push({ id, msg, kind });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 3200);
}

// ---------- 加载 ----------
async function loadConfig() {
  const c = await api.getConfig();
  config.mappings = c.mappings;
  config.default_sound = c.default_sound;
  config.comments = c.comments;
  snapshot.value = JSON.stringify({
    mappings: config.mappings,
    default_sound: config.default_sound,
    comments: config.comments,
  });
}
async function loadStatus() {
  Object.assign(status, await api.serviceStatus());
}
async function loadLegacy() {
  legacy.value = await api.legacyServiceInfo().catch(() => null);
}
async function loadDiscovered() {
  discLoading.value = true;
  try {
    discovered.value = await api.discoveredApps();
  } finally {
    discLoading.value = false;
  }
}
async function loadSounds() {
  sounds.value = await api.listSounds();
}
async function loadLog() {
  logText.value = await api.logTail(200).catch(() => "");
}

onMounted(async () => {
  try {
    Object.assign(session, await api.sessionInfo());
    await Promise.all([loadConfig(), loadStatus(), loadSounds(), loadLegacy(), loadLog()]);
    await loadDiscovered();
  } catch (e) {
    toast("初始化失败：" + String(e), "err");
  }
});

// ---------- 保存 ----------
async function save() {
  const incomplete = config.mappings.findIndex(
    (m) => (m.keyword.trim() && !m.sound.trim()) || (!m.keyword.trim() && m.sound.trim())
  );
  if (incomplete >= 0) {
    toast("第 " + (incomplete + 1) + " 行的关键词或提示音未填写完整", "err");
    return;
  }
  if (config.mappings.some((m) => !m.keyword.trim())) {
    toast("存在空关键词的映射，请填写或删除", "err");
    return;
  }

  saving.value = true;
  try {
    const cleaned: AppConfig = {
      mappings: config.mappings.map((m) => ({ keyword: m.keyword.trim(), sound: m.sound.trim() })),
      default_sound: config.default_sound,
      comments: config.comments,
    };
    await api.saveConfig(cleaned);
    await loadConfig();
    toast(status.installed ? "已保存，配置立即生效" : "已保存（启用后台服务后生效）", "ok");
    await loadDiscovered();
  } catch (e) {
    toast("保存失败：" + String(e), "err");
  } finally {
    saving.value = false;
  }
}

// ---------- 服务 ----------
async function serviceAction(action: string) {
  busy.value = true;
  try {
    Object.assign(status, await api.serviceAction(action));
    const label: Record<string, string> = {
      start: "已启动",
      stop: "已停止",
      restart: "已重启",
      enable: "已开启开机自启",
      disable: "已关闭开机自启",
    };
    toast(label[action] || "操作完成", "ok");
  } catch (e) {
    toast("操作失败：" + String(e), "err");
  } finally {
    busy.value = false;
  }
}

async function installService() {
  busy.value = true;
  try {
    Object.assign(status, await api.installService());
    await Promise.all([loadConfig(), loadLog()]);
    toast("后台服务已启用，通知提示音已生效 🎉", "ok");
  } catch (e) {
    toast("启用失败：" + String(e), "err");
  } finally {
    busy.value = false;
  }
}

async function uninstallService() {
  busy.value = true;
  try {
    Object.assign(status, await api.uninstallService());
    toast("已卸载后台服务", "ok");
  } catch (e) {
    toast("卸载失败：" + String(e), "err");
  } finally {
    busy.value = false;
  }
}

async function disableLegacy() {
  busy.value = true;
  try {
    await api.disableLegacyService();
    await loadLegacy();
    toast("旧版守护服务已停用", "ok");
  } catch (e) {
    toast("操作失败：" + String(e), "err");
  } finally {
    busy.value = false;
  }
}

// ---------- 播放 ----------
async function play(path: string) {
  if (!path) return;
  try {
    await api.playSound(path);
  } catch (e) {
    toast("播放失败：" + String(e), "err");
  }
}

// ---------- 映射编辑 ----------
function addMapping() {
  config.mappings.push({ keyword: "", sound: config.default_sound || "" });
}
function removeMapping(i: number) {
  config.mappings.splice(i, 1);
}
function addFromDiscovered(name: string) {
  config.mappings.push({ keyword: name, sound: config.default_sound || "" });
  tab.value = "mappings";
  toast(`已添加「${name}」，请选择提示音`, "ok");
}

// ---------- 声音选择器 ----------
const picker = reactive<{ open: boolean; mode: "mapping" | "default"; index: number }>({
  open: false,
  mode: "mapping",
  index: -1,
});
function openPickerForIndex(i: number) {
  picker.mode = "mapping";
  picker.index = i;
  picker.open = true;
}
function openPickerForDefault() {
  picker.mode = "default";
  picker.open = true;
}
function applyPick(path: string) {
  if (picker.mode === "mapping" && picker.index >= 0) {
    config.mappings[picker.index].sound = path;
  } else {
    config.default_sound = path;
  }
  picker.open = false;
}
async function browse() {
  const p = await api.pickSoundFile().catch(() => null);
  if (p) applyPick(p);
}
function setDefault(path: string) {
  config.default_sound = path;
  toast("已设为兜底音（记得保存）", "ok");
}

// ---------- 导航 ----------
const navItems = [
  {
    key: "mappings" as const,
    label: "映射管理",
    icon: '<path d="M4 6h16M4 12h16M4 18h16" stroke-linecap="round"/><circle cx="9" cy="6" r="2" fill="currentColor"/><circle cx="15" cy="12" r="2" fill="currentColor"/><circle cx="8" cy="18" r="2" fill="currentColor"/>',
  },
  {
    key: "discovered" as const,
    label: "发现的应用",
    icon: '<circle cx="11" cy="11" r="7"/><path d="M16 16l4 4" stroke-linecap="round"/>',
  },
  {
    key: "library" as const,
    label: "声音库",
    icon: '<path d="M9 18V5l10-2v13" stroke-linecap="round"/><circle cx="6" cy="18" r="3"/><circle cx="16" cy="16" r="3"/>',
  },
  {
    key: "service" as const,
    label: "服务与环境",
    icon: '<path d="M12 3v9" stroke-linecap="round"/><path d="M6.5 6.5a8 8 0 1 0 11 0" stroke-linecap="round"/>',
  },
];

const titleMap: Record<Tab, { title: string; sub: string }> = {
  mappings: { title: "映射管理", sub: "为不同应用配置不同的提示音" },
  discovered: { title: "发现的应用", sub: "从通知日志中捕获的通知来源" },
  library: { title: "声音库", sub: "浏览并试听可用的声音文件" },
  service: { title: "服务与环境", sub: "后台服务控制与运行环境信息" },
};

function switchTab(t: Tab) {
  tab.value = t;
  if (t === "discovered") loadDiscovered();
  if (t === "service") {
    loadStatus();
    loadLog();
    loadLegacy();
  }
}
</script>

<template>
  <div class="app">
    <!-- 侧边栏 -->
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark">4Y</div>
        <div class="brand-text">
          <b>4Y 通知提示音</b>
          <span>NOTIFICATION MANAGER</span>
        </div>
      </div>

      <div
        v-for="item in navItems"
        :key="item.key"
        class="nav-item"
        :class="{ active: tab === item.key }"
        @click="switchTab(item.key)"
      >
        <svg
          class="nav-ico"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.7"
          v-html="item.icon"
        />
        {{ item.label }}
        <span v-if="item.key === 'mappings' && config.mappings.length" class="nav-badge">{{
          config.mappings.length
        }}</span>
        <span v-if="item.key === 'discovered' && discovered.length" class="nav-badge">{{
          discovered.length
        }}</span>
        <span v-if="item.key === 'service' && !status.installed" class="nav-badge" style="color: var(--warn)"
          >!</span
        >
      </div>

      <div class="sidebar-foot">
        <div>4YStudio · v0.1.0</div>
        <div>Debian 13 · KDE 6 · Wayland</div>
      </div>
    </aside>

    <!-- 主区 -->
    <div class="main">
      <header class="topbar">
        <div>
          <h1>{{ titleMap[tab].title }}</h1>
          <div class="sub">{{ titleMap[tab].sub }}</div>
        </div>
        <div class="topbar-right">
          <span class="pill" :class="status.installed && status.active ? 'on' : 'off'">
            <span class="dot" />
            {{ !status.installed ? "服务未安装" : status.active ? "运行中" : "已停止" }}
          </span>
        </div>
      </header>

      <div class="content">
        <!-- 未安装提示条(仅非服务页) -->
        <div
          v-if="!status.installed && tab !== 'service'"
          class="card"
          style="
            border-color: rgba(61, 174, 233, 0.45);
            background: linear-gradient(90deg, rgba(61, 174, 233, 0.12), transparent);
            display: flex;
            align-items: center;
            gap: 14px;
            padding: 14px 18px;
          "
        >
          <div style="flex: 1; font-size: 13px">
            <b>后台服务尚未启用</b>
            <span class="muted"> —— 现在通知不会播放提示音。</span>
          </div>
          <button class="btn primary" :disabled="busy" @click="installService">⚡ 一键启用</button>
        </div>

        <!-- 旧版服务迁移提示 -->
        <div
          v-if="legacyRunning"
          class="card"
          style="
            border-color: rgba(246, 116, 0, 0.45);
            background: linear-gradient(90deg, rgba(246, 116, 0, 0.1), transparent);
            display: flex;
            align-items: center;
            gap: 14px;
            padding: 14px 18px;
          "
        >
          <div style="flex: 1; font-size: 13px">
            <b style="color: var(--warn)">检测到旧版守护服务仍在运行</b>
            <div class="muted" style="font-size: 12px; margin-top: 3px">
              内置服务与旧版 bash 服务同时运行会导致重复播放，建议停用旧版。
            </div>
          </div>
          <button class="btn" :disabled="busy" @click="disableLegacy">停用旧版服务</button>
        </div>

        <MappingsPanel
          v-if="tab === 'mappings'"
          :config="config"
          :dirty="dirty"
          :saving="saving"
          @pick="openPickerForIndex"
          @pickDefault="openPickerForDefault"
          @play="play"
          @save="save"
          @add="addMapping"
          @remove="removeMapping"
          @clearDefault="config.default_sound = null"
        />

        <DiscoveredPanel
          v-else-if="tab === 'discovered'"
          :apps="discovered"
          :loading="discLoading"
          @add="addFromDiscovered"
          @refresh="loadDiscovered"
        />

        <SoundLibraryPanel
          v-else-if="tab === 'library'"
          :sounds="sounds"
          @play="play"
          @setDefault="setDefault"
        />

        <ServicePanel
          v-else
          :status="status"
          :session="session"
          :busy="busy"
          :log-text="logText"
          @action="serviceAction"
          @install="installService"
          @uninstall="uninstallService"
          @openConfig="api.openConfigDir()"
          @openLog="api.openLogDir()"
          @refresh="loadStatus"
          @refreshLog="loadLog"
        />
      </div>
    </div>

    <SoundPicker
      :open="picker.open"
      :sounds="sounds"
      :title="picker.mode === 'default' ? '选择兜底音' : '选择提示音'"
      @choose="applyPick"
      @play="play"
      @browse="browse"
      @close="picker.open = false"
    />

    <div class="toasts">
      <div v-for="t in toasts" :key="t.id" class="toast" :class="t.kind">{{ t.msg }}</div>
    </div>
  </div>
</template>
