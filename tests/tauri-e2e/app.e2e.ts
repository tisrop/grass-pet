import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import { browser } from '@wdio/globals';

interface Point {
  x: number;
  y: number;
}

interface Size {
  width: number;
  height: number;
}

interface WindowSnapshot {
  label: string;
  visible: boolean;
  focused: boolean;
  position: Point;
  size: Size;
}

interface Settings {
  edgeSnap: boolean;
  alwaysOnTop: boolean;
  clickThrough: boolean;
  petScale: number;
}

interface Reminder {
  id: string;
  text: string;
  dueAt: string;
  createdAt: string;
}

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return browser.tauri.execute(
    ({ core }, payload) => core.invoke(payload.command, payload.args),
    { command, args },
  ) as Promise<T>;
}

async function switchWindow(label: string): Promise<void> {
  await browser.tauri.switchWindow(label);
}

async function windowSnapshot(label: string): Promise<WindowSnapshot> {
  await switchWindow(label);
  return browser.execute(async () => {
    const tauri = (globalThis as typeof globalThis & {
      __TAURI__: {
        window: {
          getCurrentWindow(): {
            label: string;
            isVisible(): Promise<boolean>;
            isFocused(): Promise<boolean>;
            outerPosition(): Promise<Point>;
            outerSize(): Promise<Size>;
          };
        };
      };
    }).__TAURI__;
    const current = tauri.window.getCurrentWindow();
    const [visible, focused, position, size] = await Promise.all([
      current.isVisible(),
      current.isFocused(),
      current.outerPosition(),
      current.outerSize(),
    ]);
    return {
      label: current.label,
      visible,
      focused,
      position: { x: position.x, y: position.y },
      size: { width: size.width, height: size.height },
    };
  }) as Promise<WindowSnapshot>;
}

function overlaps(left: WindowSnapshot, right: WindowSnapshot): boolean {
  return left.position.x < right.position.x + right.size.width
    && left.position.x + left.size.width > right.position.x
    && left.position.y < right.position.y + right.size.height
    && left.position.y + left.size.height > right.position.y;
}

async function waitForWindowVisibility(label: string, visible: boolean): Promise<WindowSnapshot> {
  let latest: WindowSnapshot | undefined;
  await browser.waitUntil(async () => {
    latest = await windowSnapshot(label);
    return latest.visible === visible;
  }, { timeout: 10_000, interval: 100, timeoutMsg: `${label} visibility did not become ${visible}` });
  return latest!;
}

async function stopChild(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) return;
  child.kill('SIGTERM');
  const exited = await Promise.race([
    new Promise<boolean>((resolve) => child.once('exit', () => resolve(true))),
    new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 5_000)),
  ]);
  if (!exited && child.exitCode === null) child.kill('SIGKILL');
}

describe('Tauri desktop pet', () => {
  let originalSettings: Settings;
  const createdReminderIds = new Set<string>();

  before(async () => {
    const labels = (await browser.tauri.listWindows()).sort();
    assert.deepEqual(labels, ['context-menu', 'dashboard', 'pet', 'reminder']);
    await switchWindow('pet');
    await browser.$('#pet-container').waitForExist({ timeout: 20_000 });
    originalSettings = await invoke<Settings>('settings_get');
  });

  after(async () => {
    await switchWindow('pet');
    for (const id of createdReminderIds) {
      await invoke<boolean>('reminders_remove', { id }).catch(() => false);
    }
    if (originalSettings) {
      await invoke<Settings>('settings_update', { patch: originalSettings }).catch(() => originalSettings);
    }
  });

  it('starts with an invisible pet that is walking and bouncing', async () => {
    const pet = await waitForWindowVisibility('pet', true);
    assert.equal(pet.label, 'pet');
    const container = await browser.$('#pet-container');
    const sprite = await browser.$('#pet-sprite');
    await browser.waitUntil(async () => {
      const state = await container.getAttribute('data-state');
      return state === 'walk-left' || state === 'walk-right';
    }, { timeout: 10_000, timeoutMsg: 'pet did not enter an automatic walking state' });
    assert.match(await sprite.getAttribute('class'), /\bis-invisible\b/);
    assert.match(await browser.$('.pet-breath-layer').getAttribute('class'), /\bbreathing\b/);
  });

  it('chants on a real left click, then immediately returns to invisible walking', async () => {
    await switchWindow('pet');
    const container = await browser.$('#pet-container');
    const sprite = await browser.$('#pet-sprite');
    const hitArea = await browser.$('.pet-hit-area');
    await hitArea.click();
    await browser.waitUntil(async () => (await container.getAttribute('data-state')) === 'chant', {
      timeout: 3_000,
      timeoutMsg: 'left click did not start chanting',
    });
    assert.doesNotMatch(await sprite.getAttribute('class'), /\bis-invisible\b/);
    assert.match(await browser.$('#feedback-bubble').getAttribute('class'), /\bshow\b/);

    await browser.waitUntil(async () => {
      const state = await container.getAttribute('data-state');
      const spriteClass = await sprite.getAttribute('class');
      return (state === 'walk-left' || state === 'walk-right') && /\bis-invisible\b/.test(spriteClass);
    }, { timeout: 12_000, interval: 100, timeoutMsg: 'chant did not return to invisible walking immediately' });
  });

  it('tracks a real pointer drag and moves the native pet window', async () => {
    await switchWindow('pet');
    const before = await windowSnapshot('pet');
    const hitArea = await browser.$('.pet-hit-area');
    await browser.action('pointer')
      .move({ origin: hitArea, x: 0, y: 0, duration: 0 })
      .down('left')
      .pause(100)
      .move({ origin: 'pointer', x: -48, y: -32, duration: 400 })
      .up('left')
      .perform();
    await browser.releaseActions();
    await browser.waitUntil(async () => {
      const after = await windowSnapshot('pet');
      return Math.abs(after.position.x - before.position.x) >= 20
        || Math.abs(after.position.y - before.position.y) >= 20;
    }, { timeout: 5_000, interval: 100, timeoutMsg: 'native pet window did not follow the pointer drag' });
  });

  it('opens the custom context menu beside the pet and reaches dashboard', async () => {
    await switchWindow('pet');
    await browser.execute(() => {
      const hitArea = document.querySelector('.pet-hit-area');
      if (!hitArea) throw new Error('pet hit area is missing');
      hitArea.dispatchEvent(new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        button: 2,
      }));
    });
    const menu = await waitForWindowVisibility('context-menu', true);
    const pet = await windowSnapshot('pet');
    assert.equal(overlaps(pet, menu), false, 'context menu must not cover the pet');

    await switchWindow('context-menu');
    await browser.$('.pet-context-menu').waitForDisplayed();
    const labels = await browser.execute(() => [...document.querySelectorAll('.pet-context-menu__item')]
      .map((item) => item.textContent?.trim() ?? ''));
    for (const expected of ['添加提醒', '道观', '鼠标穿透', '隐藏桌宠']) {
      assert.ok(labels.some((label) => label.includes(expected)), `context menu is missing ${expected}`);
    }
    await browser.$('button*=道观').click();
    await waitForWindowVisibility('dashboard', true);
    await switchWindow('dashboard');
    await browser.$('#dashboard').waitForDisplayed();
    assert.equal(await browser.$('h1').getText(), '道观');
  });

  it('persists settings through the dashboard UI', async () => {
    await switchWindow('pet');
    await invoke<void>('window_show_dashboard');
    await waitForWindowVisibility('dashboard', true);
    await switchWindow('dashboard');
    const edgeSnap = await browser.$('button[aria-label="边缘吸附"]');
    const before = await edgeSnap.getAttribute('aria-checked');
    await edgeSnap.click();
    await browser.waitUntil(async () => (await edgeSnap.getAttribute('aria-checked')) !== before);
    const saved = await invoke<Settings>('settings_get');
    assert.equal(String(saved.edgeSnap), before === 'true' ? 'false' : 'true');
    await edgeSnap.click();
    await browser.waitUntil(async () => (await edgeSnap.getAttribute('aria-checked')) === before);
  });

  it('creates and removes a reminder through the reminder window', async () => {
    await switchWindow('pet');
    await invoke<void>('window_show_reminder');
    await waitForWindowVisibility('reminder', true);
    await switchWindow('reminder');
    const text = `Tauri E2E ${Date.now()}`;
    await browser.$('#reminder-text').setValue(text);
    await browser.$('#reminder-time').setValue('2030-01-01T12:00');
    await browser.$('.btn-primary').click();
    await browser.waitUntil(async () => (await browser.$$('article.reminder-item')).length > 0);
    const reminders = await invoke<Reminder[]>('reminders_list');
    const saved = reminders.find((item) => item.text === text);
    assert.ok(saved, 'reminder was not persisted by the Tauri backend');
    createdReminderIds.add(saved.id);
    await browser.$(`button[aria-label="删除 ${text}"]`).click();
    await browser.waitUntil(async () => !(await invoke<Reminder[]>('reminders_list')).some((item) => item.id === saved.id));
    createdReminderIds.delete(saved.id);
  });

  it('allows a second application process without implementing state isolation', async () => {
    const binary = process.env.TAURI_E2E_BINARY;
    assert.ok(binary, 'TAURI_E2E_BINARY is required');
    const second = spawn(binary, [], {
      env: {
        ...process.env,
        WDIO_EMBEDDED_PORT: '45555',
      },
      stdio: 'ignore',
    });
    try {
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(resolve, 2_000);
        second.once('error', (error) => {
          clearTimeout(timer);
          reject(error);
        });
        second.once('exit', (code, signal) => {
          clearTimeout(timer);
          reject(new Error(`second process exited early: code=${code} signal=${signal}`));
        });
      });
      assert.equal(second.exitCode, null);
    } finally {
      await stopChild(second);
    }
  });
});
