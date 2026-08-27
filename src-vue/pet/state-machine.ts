import type { PetState } from '../../src/shared/contracts';

export interface StateFrame {
  stateId: string;
  frameIndex: number;
  frame: string;
  stateChanged: boolean;
}

interface ActiveState {
  state: PetState;
  frameIndex: number;
  startedAt: number;
  durationMs: number;
}

export class PetStateMachine {
  private readonly states: Map<string, PetState>;
  private readonly idleState: PetState;
  private active: ActiveState;
  private readonly completedAt = new Map<string, number>();
  private pendingStateChange = false;

  constructor(states: PetState[], now = 0, idleStateId = 'idle') {
    this.states = new Map(states.map((state) => [state.id, state]));
    const idle = this.states.get(idleStateId);
    if (!idle) throw new Error(`Missing idle state: ${idleStateId}`);
    this.idleState = idle;
    this.active = this.makeActive(idle, now);
  }

  private durationFor(state: PetState, requested?: number): number {
    if (requested !== undefined && Number.isFinite(requested) && requested > 0) return requested;
    if (state.id === this.idleState.id && state.loop) return 0;
    return Math.max(1, state.frames.length * state.frameDurationMs);
  }

  private makeActive(state: PetState, now: number, durationMs?: number): ActiveState {
    return {
      state,
      frameIndex: 0,
      startedAt: now,
      durationMs: this.durationFor(state, durationMs),
    };
  }

  start(stateId: string, now: number, durationMs?: number, force = false): boolean {
    const next = this.states.get(stateId);
    if (!next) return false;
    // 行走是"持续移动背景状态"：主进程下发 walk 代表桌宠正在移动，须能覆盖
    // edge-snap/peek 等低优先级瞬时反馈（否则拖拽后会被更高优先级状态挡住、退不出 idle）。
    // 但强制进入不能打断高优先级互动（chant 85 / notify 95）：否则拖拽恢复时重发的
    // walk 会把正在念咒的金光状态覆盖掉（文字还在念、金光却消失）。
    const protectedFromForce = this.active.state.priority >= 80;
    if (!force && this.active.state.priority > next.priority) return false;
    if (force && protectedFromForce && this.active.state.priority > next.priority) return false;
    if (!force && this.active.state.id === next.id && next.interrupt === 'resume') return false;
    const lastCompleted = this.completedAt.get(next.id);
    if (lastCompleted !== undefined && now - lastCompleted < next.cooldownMs) return false;
    this.active = this.makeActive(next, now, durationMs);
    this.pendingStateChange = true;
    return true;
  }

  tick(now: number): StateFrame {
    let stateChanged = this.pendingStateChange;
    this.pendingStateChange = false;
    let elapsed = Math.max(0, now - this.active.startedAt);
    if (this.active.durationMs > 0 && elapsed >= this.active.durationMs) {
      this.completedAt.set(this.active.state.id, now);
      this.active = this.makeActive(this.idleState, now);
      elapsed = 0;
      stateChanged = true;
    }

    const { state } = this.active;
    const frameCount = Math.max(1, state.frames.length);
    const rawIndex = Math.floor(elapsed / Math.max(1, state.frameDurationMs));
    const frameIndex = state.loop
      ? rawIndex % frameCount
      : Math.min(frameCount - 1, rawIndex);
    const frameChanged = frameIndex !== this.active.frameIndex;
    this.active.frameIndex = frameIndex;

    return {
      stateId: state.id,
      frameIndex,
      frame: state.frames[frameIndex] ?? state.frames[0] ?? '',
      stateChanged: stateChanged || frameChanged,
    };
  }

  currentStateId(): string {
    return this.active.state.id;
  }

  /** Immediately leave a long-running activity and resume the idle state. */
  resetToIdle(now = performance.now()): void {
    this.active = this.makeActive(this.idleState, now);
    this.pendingStateChange = true;
  }
}
