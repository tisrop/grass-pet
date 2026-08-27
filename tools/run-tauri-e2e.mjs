import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const temporary = await mkdtemp(path.join(os.tmpdir(), 'grass-pet-tauri-e2e-'));
const binary = path.join(
  root,
  'src-tauri',
  'target',
  'release',
  process.platform === 'win32' ? 'grass-pet-tauri-e2e.exe' : 'grass-pet-tauri-e2e',
);
const cli = path.join(root, 'node_modules', '@wdio', 'cli', 'bin', 'wdio.js');

const child = spawn(process.execPath, [cli, 'run', path.join(root, 'wdio.conf.ts')], {
  cwd: root,
  env: {
    ...process.env,
    GRASS_PET_E2E_DATA_DIR: temporary,
    TAURI_E2E_BINARY: binary,
  },
  stdio: 'inherit',
});

const exitCode = await new Promise((resolve, reject) => {
  child.once('error', reject);
  child.once('exit', (code, signal) => resolve(code ?? (signal ? 1 : 0)));
});

await rm(temporary, { recursive: true, force: true });
process.exitCode = exitCode;
