<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import spec from '../../pet-spec.json';
import type { PetSpec, StateActivity } from '../../src/shared/contracts';
import { exceedsDragThreshold } from '../../src/shared/drag';
import { PetStateMachine } from './state-machine';
import { petApi } from '../shared/api';

const petSpec = spec as PetSpec;
const sprite = ref<HTMLImageElement>();
const stateId = ref('idle');
const frameUrl = ref('');
const bubbleText = ref('');
const bubbleVisible = ref(false);
const chantVisible = ref(false);
const invisible = ref(true);

const modules = import.meta.glob('../../src/assets/pet/**/*.png', {
  eager: true,
  import: 'default',
  query: '?url',
}) as Record<string, string>;
const assets = new Map<string, string>();
for (const [path, url] of Object.entries(modules)) {
  assets.set(path.split('/assets/pet/')[1] ?? path, url);
}

const initialFrame = petSpec.states.find((state) => state.id === 'idle')?.frames[0]
  ?? petSpec.character.coreAsset;
frameUrl.value = assets.get(initialFrame) ?? '';

const expectedAssets = [...new Set([
  petSpec.character.coreAsset,
  ...petSpec.states.flatMap((state) => state.frames),
])];
const machine = new PetStateMachine(petSpec.states, performance.now());
const cleanups: Array<() => void> = [];
let animationFrame = 0;
let idleTimer: ReturnType<typeof setTimeout> | undefined;
let blinkTimer: ReturnType<typeof setTimeout> | undefined;
let bubbleTimer: ReturnType<typeof setTimeout> | undefined;
let chantTimer: ReturnType<typeof setInterval> | undefined;
let walkPauseQueue: Promise<void> = Promise.resolve();
let lastWalkState: 'walk-left' | 'walk-right' = 'walk-left';
let activePointerId: number | undefined;
let pointerStart = { x: 0, y: 0 };
let dragging = false;
let dragBegin: Promise<void> | undefined;
let suppressNextClick = false;

const mantra = [
  '天地玄宗，万气本根。', '广修亿劫，证吾神通。', '三界内外，惟道独尊。',
  '体有金光，覆映吾身。', '视之不见，听之不闻。', '包罗天地，养育群生。',
  '受持万遍，身有光明。', '三界侍卫，五帝司迎。', '万神朝礼，役使雷霆。',
  '鬼妖丧胆，精怪亡形。', '内有霹雳，雷神隐名。', '洞慧交彻，五气腾腾。',
  '金光速现，覆护真人。',
];
const MANTRA_STEP_MS = 500;
const CHANT_DURATION_MS = mantra.length * MANTRA_STEP_MS;

function setWalkPaused(reason: 'drag' | 'chant', paused: boolean): Promise<void> {
  const update = walkPauseQueue
    .catch(() => {})
    .then(() => petApi.walk.setPaused(reason, paused));
  walkPauseQueue = update;
  return update;
}

function updateFrame(timestamp: number): void {
  const snapshot = machine.tick(timestamp);
  stateId.value = snapshot.stateId;
  if (snapshot.stateChanged || !frameUrl.value) frameUrl.value = assets.get(snapshot.frame) ?? '';
}

function setState(nextState: string, durationMs?: number): void {
  const walking = nextState === 'walk-left' || nextState === 'walk-right';
  if (!machine.start(nextState, performance.now(), durationMs, walking)) return;
  if (walking) lastWalkState = nextState as typeof lastWalkState;
  updateFrame(performance.now());
  if (nextState === 'chant') showChant();
}

function animate(timestamp: number): void {
  updateFrame(timestamp);
  animationFrame = requestAnimationFrame(animate);
}

function scheduleIdleEvents(): void {
  clearTimeout(blinkTimer);
  clearTimeout(idleTimer);
  blinkTimer = setTimeout(() => {
    if (machine.currentStateId() === 'idle') setState('blink');
    scheduleIdleEvents();
  }, 2000 + Math.random() * 4000);
  const { min, max } = petSpec.motion.idleIntervalMs;
  idleTimer = setTimeout(scheduleIdleEvents, min + Math.random() * (max - min));
}

function showFeedback(text: string): void {
  clearTimeout(bubbleTimer);
  bubbleText.value = text;
  bubbleVisible.value = true;
  chantVisible.value = false;
  bubbleTimer = setTimeout(() => { bubbleVisible.value = false; }, 2000);
}

function showChant(): void {
  clearInterval(chantTimer);
  invisible.value = false;
  // 点击事件可能在透明窗口切换时丢失 pointerup；开始念咒时先清掉残留拖拽暂停，
  // 避免 chant 结束后 Rust 仍被 drag 原因锁住，无法恢复启动时的行走跳动。
  void setWalkPaused('drag', false)
    .then(() => setWalkPaused('chant', true))
    .catch(() => {});
  let index = 0;
  bubbleText.value = mantra[index] ?? '';
  bubbleVisible.value = true;
  chantVisible.value = true;
  chantTimer = setInterval(() => {
    index += 1;
    if (index < mantra.length) {
      bubbleText.value = mantra[index] ?? '';
      return;
    }
    clearInterval(chantTimer);
    chantTimer = undefined;
    bubbleVisible.value = false;
    chantVisible.value = false;
    machine.resetToIdle(performance.now());
    invisible.value = true;
    setState(lastWalkState, 60_000);
    void setWalkPaused('chant', false).catch(() => {});
  }, MANTRA_STEP_MS);
}

function playSquash(): void {
  if (!sprite.value || !petSpec.motion.squashStretch.enabled) return;
  sprite.value.classList.remove('squash');
  void sprite.value.offsetWidth;
  sprite.value.classList.add('squash');
}

function handleClick(): void {
  if (suppressNextClick) {
    suppressNextClick = false;
    return;
  }
  playSquash();
  setState('chant', CHANT_DURATION_MS);
  scheduleIdleEvents();
}

function showContextMenu(event: MouseEvent): void {
  event.preventDefault();
  event.stopPropagation();
  void petApi.window.showContextMenu().catch(() => undefined);
}

function handlePointerDown(event: PointerEvent): void {
  if (event.button !== 0 || activePointerId !== undefined) return;
  activePointerId = event.pointerId;
  pointerStart = { x: event.screenX, y: event.screenY };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  dragBegin = petApi.window.beginDrag();
}

function handlePointerMove(event: PointerEvent): void {
  if (event.pointerId !== activePointerId) return;
  if (dragging) return;
  if (!exceedsDragThreshold(pointerStart, { x: event.screenX, y: event.screenY })) return;
  dragging = true;
  suppressNextClick = true;
}

function handlePointerEnd(event: PointerEvent): void {
  if (event.pointerId !== activePointerId) return;
  const target = event.currentTarget as HTMLElement;
  if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  activePointerId = undefined;
  const wasDragging = dragging;
  dragging = false;
  void (dragBegin ?? Promise.resolve())
    .then(() => petApi.window.endDrag())
    .catch(() => undefined)
    .then(() => setWalkPaused('drag', false))
    .catch(() => undefined);
  if (!wasDragging) suppressNextClick = false;
  dragBegin = undefined;
}

async function loadAssets(): Promise<HTMLImageElement[]> {
  return Promise.all(expectedAssets.map((name) => new Promise<HTMLImageElement>((resolve, reject) => {
    const url = assets.get(name);
    if (!url) return reject(new Error(`Missing runtime asset: ${name}`));
    const image = new Image();
    image.onload = () => image.naturalWidth > 0 ? resolve(image) : reject(new Error(`Invalid asset: ${name}`));
    image.onerror = () => reject(new Error(`Failed to decode asset: ${name}`));
    image.src = url;
  })));
}

onMounted(async () => {
  const breathing = petSpec.motion.breathing;
  document.documentElement.style.setProperty('--breath-period', `${breathing.periodMs}ms`);
  document.documentElement.style.setProperty('--breath-scale-x', `${1 + breathing.scaleX}`);
  document.documentElement.style.setProperty('--breath-scale-y', `${1 + breathing.scaleY}`);
  document.documentElement.style.setProperty('--squash-duration', `${petSpec.motion.squashStretch.durationMs}ms`);
  document.documentElement.style.setProperty('--squash-intensity', `${petSpec.motion.squashStretch.intensity}`);

  // 首帧、待机动画和原生事件监听不能等待全部动作素材预加载完成。
  // Tauri 原生窗口会在 runtime.ready 后才显示，因此用户看到的第一帧已经在持续运动。
  updateFrame(performance.now());
  scheduleIdleEvents();
  animationFrame = requestAnimationFrame(animate);
  cleanups.push(petApi.events.onStateActivity((activity: StateActivity) => {
    if (activity.stateId) setState(activity.stateId, activity.durationMs);
    if (activity.feedback && activity.stateId !== 'chant') showFeedback(activity.feedback);
  }));

  try {
    const loaded = await loadAssets();
    const reference = loaded[0];
    if (!reference || loaded.some((image) => image.naturalWidth !== reference.naturalWidth || image.naturalHeight !== reference.naturalHeight)) {
      throw new Error('Runtime assets do not share one decoded frame size');
    }
    await petApi.runtime.ready({
      status: 'ready', stateId: 'idle', frame: petSpec.states[0]?.frames[0] ?? '',
      assetCount: loaded.length, expectedAssetCount: expectedAssets.length,
      naturalWidth: reference.naturalWidth, naturalHeight: reference.naturalHeight,
      petVisible: true, ipcReady: true,
    });
  } catch (error) {
    await petApi.runtime.fail({ message: error instanceof Error ? error.message : String(error) });
  }
});

onBeforeUnmount(() => {
  cancelAnimationFrame(animationFrame);
  clearTimeout(idleTimer);
  clearTimeout(blinkTimer);
  clearTimeout(bubbleTimer);
  clearInterval(chantTimer);
  for (const cleanup of cleanups) cleanup();
});
</script>

<template>
  <main
    id="pet-container"
    class="tauri-pet-layout"
    :data-state="stateId"
  >
    <div
      class="pet-hit-area"
      @click="handleClick"
      @contextmenu="showContextMenu"
      @dragstart.prevent
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerEnd"
      @pointercancel="handlePointerEnd"
    >
      <div class="pet-breath-layer breathing">
        <img
          id="pet-sprite"
          ref="sprite"
          :src="frameUrl"
          alt=""
          draggable="false"
          :class="{ 'is-invisible': invisible }"
        />
      </div>
    </div>
    <div id="feedback-bubble" :class="{ show: bubbleVisible, 'chant-long': chantVisible }">{{ bubbleText }}</div>
  </main>
</template>
