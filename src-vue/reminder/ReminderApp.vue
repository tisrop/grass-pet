<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import type { Reminder } from '../../src/shared/contracts';
import { petApi } from '../shared/api';

const text = ref('');
const dueAt = ref('');
const reminders = ref<Reminder[]>([]);
const input = ref<HTMLInputElement>();
let stopReminder: (() => void) | undefined;
let stopCompose: (() => void) | undefined;

async function ensureNotificationPermission(): Promise<boolean> {
  if (await isPermissionGranted()) return true;
  return (await requestPermission()) === 'granted';
}

async function showSystemNotification(reminder: Reminder): Promise<void> {
  if (!await ensureNotificationPermission()) return;
  sendNotification({ title: '阿飘道长提醒', body: reminder.text });
}

function resetForm(): void {
  text.value = '';
  const next = new Date(Date.now() + 60 * 60 * 1000);
  const offset = next.getTimezoneOffset() * 60_000;
  dueAt.value = new Date(next.getTime() - offset).toISOString().slice(0, 16);
}

async function save(): Promise<void> {
  const content = text.value.trim();
  if (!content || !dueAt.value) return;
  await petApi.reminders.save({ text: content, dueAt: new Date(dueAt.value).toISOString() });
  void ensureNotificationPermission();
  reminders.value = await petApi.reminders.list();
  resetForm();
}

async function remove(id: string): Promise<void> {
  await petApi.reminders.remove(id);
  reminders.value = reminders.value.filter((reminder) => reminder.id !== id);
}

async function close(): Promise<void> {
  resetForm();
  await petApi.window.hideReminder();
}

onMounted(async () => {
  resetForm();
  reminders.value = await petApi.reminders.list();
  stopReminder = petApi.events.onReminder((reminder) => {
    text.value = reminder.text;
    dueAt.value = '';
    void showSystemNotification(reminder);
  });
  stopCompose = petApi.events.onReminderCompose(() => {
    resetForm();
    input.value?.focus();
  });
});

onBeforeUnmount(() => { stopReminder?.(); stopCompose?.(); });
</script>

<template>
  <main id="reminder-card">
    <header class="reminder-header">
      <div><p>REMINDERS</p><h1>提醒</h1></div>
    </header>
    <section class="form-section">
      <div class="input-group">
        <label for="reminder-text">提醒内容</label>
        <input id="reminder-text" ref="input" v-model="text" type="text" maxlength="200" placeholder="要提醒什么？" @keyup.enter="save" />
      </div>
      <div class="input-group">
        <label for="reminder-time">提醒时间</label>
        <input id="reminder-time" v-model="dueAt" type="datetime-local" />
      </div>
      <div class="btn-row">
        <button class="btn-secondary" type="button" @click="close">取消</button>
        <button class="btn-primary" type="button" :disabled="!text.trim() || !dueAt" @click="save">保存</button>
      </div>
    </section>
    <section v-if="reminders.length" class="reminder-list" aria-label="待办提醒">
      <article v-for="reminder in reminders" :key="reminder.id" class="reminder-item">
        <div><p class="reminder-text">{{ reminder.text }}</p><time class="reminder-time">{{ new Date(reminder.dueAt).toLocaleString() }}</time></div>
        <button class="delete-btn" type="button" :aria-label="`删除 ${reminder.text}`" @click="remove(reminder.id)">删除</button>
      </article>
    </section>
  </main>
</template>
