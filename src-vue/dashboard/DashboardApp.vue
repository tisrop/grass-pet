<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import spec from '../../pet-spec.json';
import avatarUrl from '../../src/assets/pet/core-ip/core-ip.png';
import type { InteractionSpec, PetSpec, PetStats, Settings, UpdateCheckResult } from '../../src/shared/contracts';
import { petApi } from '../shared/api';

const petSpec = spec as PetSpec;
const settings = ref<Settings>();
const stats = ref<PetStats>({ affection: 0, mood: 0, todayInteractions: 0, companionMinutes: 0, lastInteractionDate: '' });
const interactions = ref<InteractionSpec[]>([]);
const updateResult = ref<UpdateCheckResult>();
const updateError = ref('');
const isCheckingUpdate = ref(false);
const isInstallingUpdate = ref(false);
const isUpdateInstalled = ref(false);
const isRestartingUpdate = ref(false);
const updateDownloaded = ref(0);
const updateTotal = ref<number | null>(null);
const updatePhase = ref<'downloading' | 'installing' | null>(null);
const hasCheckedForUpdates = ref(false);
let stopStats: (() => void) | undefined;
let stopUpdateProgress: (() => void) | undefined;
let stopDashboardShown: (() => void) | undefined;

const updateProgressPercent = computed(() => {
  if (!updateTotal.value || updateTotal.value <= 0) return null;
  return Math.min(100, Math.round((updateDownloaded.value / updateTotal.value) * 100));
});

async function updateSetting(patch: Partial<Settings>): Promise<void> {
  settings.value = await petApi.settings.update(patch);
}

async function triggerInteraction(id: string): Promise<void> {
  const result = await petApi.interactions.trigger(id);
  stats.value = result.stats;
}

async function checkForUpdates(showErrors = true): Promise<void> {
  if (isCheckingUpdate.value || isInstallingUpdate.value || isRestartingUpdate.value) return;
  isCheckingUpdate.value = true;
  if (showErrors) {
    updateError.value = '';
    updateResult.value = undefined;
    hasCheckedForUpdates.value = false;
  }
  try {
    updateResult.value = await petApi.updates.check();
    hasCheckedForUpdates.value = true;
  } catch (error) {
    if (showErrors) updateError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isCheckingUpdate.value = false;
  }
}

function createUpdateRequestId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `update-${Date.now()}`;
}

async function installUpdate(): Promise<void> {
  const version = updateResult.value?.version;
  if (!updateResult.value?.available || !version || isInstallingUpdate.value) return;
  if (updateResult.value.update_mode === 'portable') {
    updateError.value = '当前版本为便携版，请下载新版安装包后手动覆盖。';
    return;
  }

  updateError.value = '';
  isInstallingUpdate.value = true;
  updateDownloaded.value = 0;
  updateTotal.value = null;
  updatePhase.value = 'downloading';
  try {
    await petApi.updates.downloadAndInstall(createUpdateRequestId(), version);
    isUpdateInstalled.value = true;
  } catch (error) {
    updateError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isInstallingUpdate.value = false;
  }
}

async function restartAfterUpdate(): Promise<void> {
  if (!isUpdateInstalled.value || isRestartingUpdate.value) return;
  isRestartingUpdate.value = true;
  updateError.value = '';
  try {
    await petApi.updates.restart();
  } catch (error) {
    isRestartingUpdate.value = false;
    updateError.value = error instanceof Error ? error.message : String(error);
  }
}

onMounted(async () => {
  stopDashboardShown = petApi.events.onDashboardShown(() => {
    void checkForUpdates();
  });
  [settings.value, stats.value, interactions.value] = await Promise.all([
    petApi.settings.get(),
    petApi.interactions.stats(),
    petApi.interactions.list(),
  ]);
  stopStats = petApi.events.onStats((next) => { stats.value = next; });
  stopUpdateProgress = petApi.events.onUpdateProgress((progress) => {
    if (isInstallingUpdate.value === false && progress.phase !== 'installing') return;
    updateDownloaded.value = progress.downloaded;
    updateTotal.value = progress.total;
    updatePhase.value = progress.phase;
  });
  void checkForUpdates(false);
});

onBeforeUnmount(() => {
  stopStats?.();
  stopUpdateProgress?.();
  stopDashboardShown?.();
});
</script>

<template>
  <main id="dashboard">
    <button class="close-btn" type="button" aria-label="关闭道观" @click="petApi.window.hideDashboard()">×</button>
    <header class="window-header">
      <div>
        <p class="eyebrow">COMPANION STATUS</p>
        <h1>道观</h1>
      </div>
    </header>

    <div class="content-scroll">
      <section class="identity-section">
        <img class="pet-avatar" :src="avatarUrl" alt="" />
        <div class="pet-info">
          <h2>{{ petSpec.character.displayName }}</h2>
          <p>{{ petSpec.character.personality.join('、') }}</p>
        </div>
      </section>

      <section class="stats-grid" aria-label="陪伴数据">
        <div class="stat-item"><div class="stat-label"><span class="stat-icon" aria-hidden="true">💕</span>好感度</div><div class="stat-value">{{ stats.affection }}</div></div>
        <div class="stat-item"><div class="stat-label"><span class="stat-icon" aria-hidden="true">😊</span>心情</div><div class="stat-value">{{ stats.mood }}</div></div>
        <div class="stat-item"><div class="stat-label"><span class="stat-icon" aria-hidden="true">🎯</span>今日互动</div><div class="stat-value">{{ stats.todayInteractions }}</div></div>
        <div class="stat-item"><div class="stat-label"><span class="stat-icon" aria-hidden="true">⏰</span>陪伴时长</div><div class="stat-value">{{ stats.companionMinutes }}<small> 分钟</small></div></div>
      </section>

      <section v-if="interactions.length" class="panel-section">
        <h2 class="section-title"><span class="section-icon" aria-hidden="true">✨</span>互动</h2>
        <div class="interactions">
          <button v-for="interaction in interactions" :key="interaction.id" class="interaction-btn" type="button" @click="triggerInteraction(interaction.id)">
            <span aria-hidden="true">{{ interaction.emoji }}</span><span>{{ interaction.label }}</span>
          </button>
        </div>
      </section>

      <section v-if="settings" class="panel-section settings-group">
        <h2 class="section-title"><span class="section-icon" aria-hidden="true">⚙️</span>设置</h2>
        <div class="setting-item">
          <div><div class="setting-label">边缘吸附</div><div class="setting-desc">拖到屏幕边缘时自动贴齐</div></div>
          <button class="toggle" :class="{ active: settings.edgeSnap }" type="button" role="switch" :aria-checked="settings.edgeSnap" aria-label="边缘吸附" @click="updateSetting({ edgeSnap: !settings.edgeSnap })" />
        </div>
        <div class="setting-item">
          <div><div class="setting-label">始终置顶</div><div class="setting-desc">保持在其他窗口上方</div></div>
          <button class="toggle" :class="{ active: settings.alwaysOnTop }" type="button" role="switch" :aria-checked="settings.alwaysOnTop" aria-label="始终置顶" @click="updateSetting({ alwaysOnTop: !settings.alwaysOnTop })" />
        </div>
        <div class="setting-item">
          <div><div class="setting-label">鼠标穿透</div><div class="setting-desc">透明区域可穿透，右键桌宠仍可打开菜单</div></div>
          <button class="toggle" :class="{ active: settings.clickThrough }" type="button" role="switch" :aria-checked="settings.clickThrough" aria-label="鼠标穿透" @click="updateSetting({ clickThrough: !settings.clickThrough })" />
        </div>
        <div class="setting-item size-row">
          <div><div class="setting-label">桌宠大小</div><div class="setting-desc">调整显示尺寸</div></div>
          <div class="size-selector" aria-label="桌宠大小">
            <button v-for="option in [{ label: '小', scale: 0.65 }, { label: '中', scale: 0.8 }, { label: '大', scale: 1 }]" :key="option.scale" class="size-btn" :class="{ active: Math.abs(settings.petScale - option.scale) < 0.01 }" type="button" @click="updateSetting({ petScale: option.scale })">{{ option.label }}</button>
          </div>
        </div>
        <button class="reminder-action" type="button" @click="petApi.window.showReminder()"><span aria-hidden="true">⏰</span>添加提醒</button>
      </section>

      <section class="panel-section update-section" aria-labelledby="update-title">
        <div class="update-heading">
          <h2 id="update-title" class="section-title"><span class="section-icon" aria-hidden="true">⬆️</span>应用更新</h2>
          <span class="version-badge">v{{ updateResult?.current_version ?? petSpec.app.version }}</span>
        </div>
        <p class="setting-desc">检查新版本，下载后重启即可完成更新。</p>
        <button
          class="reminder-action update-check-button"
          type="button"
          data-testid="check-update"
          :disabled="isCheckingUpdate || isInstallingUpdate || isUpdateInstalled || isRestartingUpdate"
          @click="checkForUpdates()"
        >
          {{ isCheckingUpdate ? '检查中…' : '检查更新' }}
        </button>
        <p v-if="updateError" class="update-status update-status--error" role="status" aria-live="polite">{{ updateError }}</p>
        <p v-else-if="isCheckingUpdate" class="update-status" role="status" aria-live="polite">正在连接更新服务器…</p>
        <p v-else-if="hasCheckedForUpdates && !updateResult?.available" class="update-status" role="status">当前已是最新版本。</p>
        <div v-if="updateResult?.available" class="update-result" role="status">
          <strong>发现新版本 v{{ updateResult.version }}</strong>
          <p v-if="updateResult.update_mode === 'portable'" class="update-status">便携版不支持自动安装，请下载新版安装包后手动覆盖。</p>
          <pre v-if="updateResult.notes" class="update-notes">{{ updateResult.notes }}</pre>
          <div v-if="isInstallingUpdate" class="update-progress" aria-live="polite">
            <progress v-if="updateProgressPercent !== null" :value="updateProgressPercent" max="100" />
            <span v-if="updatePhase === 'installing'">正在准备重启…</span>
            <span v-else-if="updateProgressPercent !== null">正在下载 {{ updateProgressPercent }}%</span>
            <span v-else>正在下载更新…</span>
          </div>
          <button
            v-if="isUpdateInstalled"
            class="reminder-action"
            type="button"
            data-testid="restart-update"
            :disabled="isRestartingUpdate"
            @click="restartAfterUpdate"
          >
            {{ isRestartingUpdate ? '正在重启…' : '立即重启完成更新' }}
          </button>
          <button
            v-else-if="updateResult.update_mode !== 'portable'"
            class="reminder-action"
            type="button"
            data-testid="install-update"
            :disabled="isInstallingUpdate"
            @click="installUpdate"
          >
            {{ isInstallingUpdate ? '更新中…' : '下载并安装' }}
          </button>
        </div>
      </section>
    </div>
  </main>
</template>
