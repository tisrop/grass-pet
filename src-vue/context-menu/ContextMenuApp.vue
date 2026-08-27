<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import spec from '../../pet-spec.json';
import type { PetSpec } from '../../src/shared/contracts';
import { petApi } from '../shared/api';

const petSpec = spec as PetSpec;
const menuInteractions = petSpec.features.interactions ? petSpec.experience.interactions : [];
const menu = ref<HTMLElement>();
const clickThroughEnabled = ref(false);

async function closeMenu(): Promise<void> {
  await invoke<void>('window_hide_context_menu');
}

async function refreshMenu(): Promise<void> {
  try {
    const settings = await petApi.settings.get();
    clickThroughEnabled.value = settings.clickThrough;
  } catch {
    // Keep the last rendered label if settings cannot be refreshed.
  }
}

function runContextAction(action: () => Promise<unknown>): void {
  void closeMenu()
    .catch(() => undefined)
    .then(action)
    .catch(() => undefined);
}

function triggerInteraction(id: string): void {
  runContextAction(() => petApi.interactions.trigger(id));
}

function toggleClickThrough(): void {
  runContextAction(() => petApi.settings.update({ clickThrough: !clickThroughEnabled.value }));
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault();
    void closeMenu().catch(() => undefined);
    return;
  }
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
  const items = [...(menu.value?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
  if (!items.length) return;
  event.preventDefault();
  const current = items.indexOf(document.activeElement as HTMLButtonElement);
  const next = event.key === 'Home'
    ? 0
    : event.key === 'End'
      ? items.length - 1
      : event.key === 'ArrowDown'
        ? (current + 1 + items.length) % items.length
        : (current - 1 + items.length) % items.length;
  items[next]?.focus();
}

function handleBlur(): void {
  void closeMenu().catch(() => undefined);
}

onMounted(() => {
  window.addEventListener('focus', refreshMenu);
  window.addEventListener('blur', handleBlur);
  void refreshMenu();
});

onBeforeUnmount(() => {
  window.removeEventListener('focus', refreshMenu);
  window.removeEventListener('blur', handleBlur);
});
</script>

<template>
  <main class="context-menu-surface">
    <div
      ref="menu"
      class="pet-context-menu"
      role="menu"
      aria-label="桌宠菜单"
      @contextmenu.prevent
      @keydown="handleKeydown"
    >
      <button
        v-for="interaction in menuInteractions"
        :key="interaction.id"
        class="pet-context-menu__item"
        type="button"
        role="menuitem"
        @click="triggerInteraction(interaction.id)"
      >
        <span class="pet-context-menu__icon" aria-hidden="true">{{ interaction.emoji }}</span>
        <span>{{ interaction.label }}</span>
      </button>
      <div v-if="menuInteractions.length" class="pet-context-menu__separator" role="separator" />
      <button class="pet-context-menu__item" type="button" role="menuitem" @click="runContextAction(() => petApi.window.showReminder())">
        <span class="pet-context-menu__icon" aria-hidden="true">⏰</span>
        <span>添加提醒</span>
      </button>
      <button class="pet-context-menu__item" type="button" role="menuitem" @click="runContextAction(() => petApi.window.showDashboard())">
        <span class="pet-context-menu__icon" aria-hidden="true">🏠</span>
        <span>{{ petSpec.character.displayName }}的道观</span>
      </button>
      <div class="pet-context-menu__separator" role="separator" />
      <button class="pet-context-menu__item" type="button" role="menuitem" @click="toggleClickThrough">
        <span class="pet-context-menu__icon" aria-hidden="true">🖱️</span>
        <span>{{ clickThroughEnabled ? '关闭鼠标穿透' : '开启鼠标穿透' }}</span>
      </button>
      <button class="pet-context-menu__item pet-context-menu__item--muted" type="button" role="menuitem" @click="runContextAction(() => petApi.window.hidePet())">
        <span class="pet-context-menu__icon" aria-hidden="true">🙈</span>
        <span>隐藏桌宠</span>
      </button>
    </div>
  </main>
</template>
