<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import spec from '../../pet-spec.json';
import avatarUrl from '../../src/assets/pet/core-ip/core-ip.png';
import type { InteractionSpec, PetSpec, PetStats, Settings } from '../../src/shared/contracts';
import { petApi } from '../shared/api';

const petSpec = spec as PetSpec;
const settings = ref<Settings>();
const stats = ref<PetStats>({ affection: 0, mood: 0, todayInteractions: 0, companionMinutes: 0, lastInteractionDate: '' });
const interactions = ref<InteractionSpec[]>([]);
let stopStats: (() => void) | undefined;

async function updateSetting(patch: Partial<Settings>): Promise<void> {
  settings.value = await petApi.settings.update(patch);
}

async function triggerInteraction(id: string): Promise<void> {
  const result = await petApi.interactions.trigger(id);
  stats.value = result.stats;
}

onMounted(async () => {
  [settings.value, stats.value, interactions.value] = await Promise.all([
    petApi.settings.get(),
    petApi.interactions.stats(),
    petApi.interactions.list(),
  ]);
  stopStats = petApi.events.onStats((next) => { stats.value = next; });
});

onBeforeUnmount(() => stopStats?.());
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
          <div><div class="setting-label">鼠标穿透</div><div class="setting-desc">忽略桌宠上的鼠标操作</div></div>
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
    </div>
  </main>
</template>
