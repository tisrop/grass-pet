import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const defaultBinary = path.join(
  root,
  'src-tauri',
  'target',
  'release',
  process.platform === 'win32' ? 'grass-pet-tauri-e2e.exe' : 'grass-pet-tauri-e2e',
);
const appBinaryPath = process.env.TAURI_E2E_BINARY || defaultBinary;

export const config = {
  runner: 'local',
  specs: ['./tests/tauri-e2e/**/*.e2e.ts'],
  maxInstances: 1,
  services: [[
    '@wdio/tauri-service',
    {
      appBinaryPath,
      driverProvider: 'embedded',
      windowLabel: 'pet',
      captureBackendLogs: true,
      captureFrontendLogs: true,
      startTimeout: 90_000,
      commandTimeout: 30_000,
    },
  ]],
  capabilities: [{
    browserName: 'tauri',
    'tauri:options': {
      application: appBinaryPath,
    },
  }],
  logLevel: 'silent',
  bail: 0,
  waitforTimeout: 10_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 120_000,
  },
};
