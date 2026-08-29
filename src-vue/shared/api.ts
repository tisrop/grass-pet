import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './e2e';
import type {
  InteractionResult,
  InteractionSpec,
  PetAPI,
  PetStats,
  Reminder,
  RuntimeFailureReport,
  RuntimeReadyReport,
  Settings,
  StateActivity,
  UpdateCheckResult,
  UpdateProgressEvent,
} from '../../src/shared/contracts';

function onEvent<T>(name: string, listener: (payload: T) => void): () => void {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  void listen<T>(name, (event) => listener(event.payload)).then((stop) => {
    if (disposed) stop();
    else unlisten = stop;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

const tauriApi: PetAPI = {
  settings: {
    get: () => invoke<Settings>('settings_get'),
    update: (patch) => invoke<Settings>('settings_update', { patch }),
  },
  reminders: {
    list: () => invoke<Reminder[]>('reminders_list'),
    save: (input) => invoke<Reminder>('reminders_save', { input }),
    remove: (id) => invoke<boolean>('reminders_remove', { id }),
  },
  interactions: {
    list: () => invoke<InteractionSpec[]>('interactions_list'),
    trigger: (id) => invoke<InteractionResult>('interactions_trigger', { id }),
    stats: () => invoke<PetStats>('interactions_stats'),
  },
  window: {
    beginDrag: () => invoke<void>('window_start_dragging'),
    endDrag: () => invoke<void>('window_finish_drag'),
    summonPet: () => invoke<number>('summon_new_pet'),
    showContextMenu: () => invoke<void>('window_show_context_menu'),
    showReminder: () => invoke<void>('window_show_reminder'),
    showDashboard: () => invoke<void>('window_show_dashboard'),
    hideReminder: () => invoke<void>('window_hide', { label: 'reminder' }),
    hideDashboard: () => invoke<void>('window_hide', { label: 'dashboard' }),
    hidePet: () => invoke<void>('window_hide', { label: 'pet' }),
  },
  walk: {
    setPaused: (reason, paused) => invoke<void>('walk_set_paused', { reason, paused }),
  },
  updates: {
    check: () => invoke<UpdateCheckResult>('update_check'),
    downloadAndInstall: (requestId, expectedVersion) =>
      invoke<void>('update_download_and_install', { requestId, expectedVersion }),
    restart: () => invoke<void>('update_restart'),
  },
  runtime: {
    ready: (report: RuntimeReadyReport) => invoke<void>('runtime_ready', { report }),
    fail: (report: RuntimeFailureReport) => invoke<void>('runtime_fail', { report }),
  },
  events: {
    onStateActivity: (listener) => onEvent<StateActivity>('state-activity', listener),
    onReminder: (listener) => onEvent<Reminder>('reminder-due', listener),
    onReminderCompose: (listener) => onEvent<undefined>('reminder-compose', listener),
    onStats: (listener) => onEvent<PetStats>('stats-updated', listener),
    onUpdateProgress: (listener) => onEvent<UpdateProgressEvent>('update-progress', listener),
    onDashboardShown: (listener) => onEvent<undefined>('dashboard-shown', listener),
  },
};

export const petApi: PetAPI = window.petAPI ?? tauriApi;
