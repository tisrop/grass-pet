import { access, readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const issues = [];

async function read(relative) {
  try {
    return await readFile(path.join(root, relative), 'utf8');
  } catch {
    issues.push(`${relative}: required Tauri file is missing`);
    return '';
  }
}

function requireMatch(value, pattern, message) {
  if (!pattern.test(value)) issues.push(message);
}

function forbidMatch(value, pattern, message) {
  if (pattern.test(value)) issues.push(message);
}

const [packageText, configText, e2eConfigText, cargoText, libText, commandsText, trayText, contextMenuText, apiText, viteText, wdioText] = await Promise.all([
  read('package.json'),
  read('src-tauri/tauri.conf.json'),
  read('src-tauri/tauri.e2e.conf.json'),
  read('src-tauri/Cargo.toml'),
  read('src-tauri/src/lib.rs'),
  read('src-tauri/src/commands.rs'),
  read('src-tauri/src/tray.rs'),
  read('src-vue/context-menu/ContextMenuApp.vue'),
  read('src-vue/shared/api.ts'),
  read('vite.config.mts'),
  read('wdio.conf.ts'),
]);

let packageJson = {};
let tauriConfig = {};
let e2eConfig = {};
try { packageJson = JSON.parse(packageText); }
catch { issues.push('package.json: invalid JSON'); }
try { tauriConfig = JSON.parse(configText); }
catch { issues.push('src-tauri/tauri.conf.json: invalid JSON'); }
try { e2eConfig = JSON.parse(e2eConfigText); }
catch { issues.push('src-tauri/tauri.e2e.conf.json: invalid JSON'); }

const scripts = packageJson.scripts ?? {};
if (packageJson.name !== 'apiao-daozhang-desktop-pet') {
  issues.push('package.json: internal package name must match 阿飘道长桌宠');
}
if (packageJson.productName !== '阿飘道长桌宠') {
  issues.push('package.json: productName must be 阿飘道长桌宠');
}
for (const name of ['dev', 'start']) {
  const value = scripts[name] ?? '';
  requireMatch(value, /tauri:dev|\btauri\s+dev\b/, `package.json: ${name} must start Tauri`);
  forbidMatch(value, /electron|forge/i, `package.json: ${name} must not start Electron`);
}

requireMatch(scripts.build ?? '', /tauri:build|\btauri\s+build\b/, 'package.json: build must use Tauri');
requireMatch(scripts['tauri:build'] ?? '', /tauri\s+build\s+--no-bundle/, 'package.json: tauri:build must compile without creating installers');
requireMatch(scripts['tauri:build'] ?? '', /create-named-launcher\.mjs/, 'package.json: tauri:build must create the Chinese launcher name');
requireMatch(scripts.bundle ?? '', /tauri:bundle|\btauri\s+build\b/, 'package.json: bundle must use Tauri');
requireMatch(scripts.check ?? '', /check:tauri/, 'package.json: check must use check:tauri');
requireMatch(scripts['check:tauri'] ?? '', /validate-tauri-contract\.mjs/, 'package.json: check:tauri must validate the runtime contract');
requireMatch(scripts['check:tauri'] ?? '', /check:frontend/, 'package.json: check:tauri must type-check Vue');
requireMatch(scripts['check:tauri'] ?? '', /check:rust/, 'package.json: check:tauri must check Rust');
requireMatch(scripts['test:e2e'] ?? '', /test:e2e:build.*test:e2e:run/, 'package.json: test:e2e must build and run the Tauri E2E suite');
requireMatch(scripts.preflight ?? '', /preflight-tauri\.mjs/, 'package.json: preflight must use the Tauri preflight');
requireMatch(scripts.doctor ?? '', /doctor-tauri\.mjs/, 'package.json: doctor must use the Tauri doctor');

for (const name of Object.keys(scripts)) {
  if (name.startsWith('electron:') || name.startsWith('preelectron:')) {
    issues.push(`package.json: Electron script must be removed: ${name}`);
  }
}
if ('main' in packageJson) issues.push('package.json: Electron main entry must be removed');
if ('build' in packageJson) issues.push('package.json: Electron Builder configuration must be removed');

const directDependencies = { ...packageJson.dependencies, ...packageJson.devDependencies };
const forbiddenDependency = /^(electron|electron-builder|@electron(?:\/|-)|@electron-forge\/|@playwright\/test$|webpack$|css-loader$|ts-loader$|mini-css-extract-plugin$)/;
for (const name of Object.keys(directDependencies)) {
  if (forbiddenDependency.test(name)) issues.push(`package.json: Electron-only dependency must be removed: ${name}`);
}
for (const name of ['@wdio/cli', '@wdio/globals', '@wdio/tauri-plugin', '@wdio/tauri-service', 'webdriverio']) {
  if (!directDependencies[name]) issues.push(`package.json: missing Tauri E2E dependency ${name}`);
}

for (const relative of [
  'src/main.ts',
  'src/preload.ts',
  'src/forge-env.d.ts',
  'src/renderer',
  'forge.config.js',
  'webpack.main.config.js',
  'webpack.renderer.config.js',
  'tests/e2e',
  'tools/run-dev.mjs',
  'tools/run-build.mjs',
  'tools/preflight.mjs',
  'tools/doctor.mjs',
  'tools/validate-dev-contract.mjs',
]) {
  try {
    await access(path.join(root, relative));
    issues.push(`${relative}: legacy Electron artifact must be removed`);
  } catch {
    // Expected: Electron artifact is absent.
  }
}

if (tauriConfig.build?.beforeDevCommand !== 'pnpm run vue:dev') {
  issues.push('src-tauri/tauri.conf.json: beforeDevCommand must run vue:dev');
}
if (tauriConfig.productName !== '阿飘道长桌宠') {
  issues.push('src-tauri/tauri.conf.json: productName must be 阿飘道长桌宠');
}
if (tauriConfig.mainBinaryName !== 'apiao-daozhang-desktop-pet') {
  issues.push('src-tauri/tauri.conf.json: mainBinaryName must use the stable ASCII executable target');
}
if (tauriConfig.build?.beforeBuildCommand !== 'pnpm run vue:build') {
  issues.push('src-tauri/tauri.conf.json: beforeBuildCommand must run vue:build');
}
if (tauriConfig.build?.frontendDist !== '../dist-vue') {
  issues.push('src-tauri/tauri.conf.json: frontendDist must be ../dist-vue');
}
if (tauriConfig.bundle?.targets !== 'all') {
  issues.push('src-tauri/tauri.conf.json: bundle.targets must use platform defaults via "all"');
}
for (const label of ['pet', 'dashboard', 'reminder', 'context-menu']) {
  if (!tauriConfig.app?.windows?.some((window) => window.label === label)) {
    issues.push(`src-tauri/tauri.conf.json: missing ${label} Tauri window`);
  }
}

if (e2eConfig.build?.beforeBuildCommand !== 'pnpm run vue:build:e2e') {
  issues.push('src-tauri/tauri.e2e.conf.json: E2E build must enable the test frontend');
}
if (e2eConfig.mainBinaryName !== 'grass-pet-tauri-e2e') {
  issues.push('src-tauri/tauri.e2e.conf.json: E2E must use its internal ASCII binary name');
}
if (e2eConfig.app?.withGlobalTauri !== true) {
  issues.push('src-tauri/tauri.e2e.conf.json: withGlobalTauri must be enabled only in the E2E overlay');
}
if (!JSON.stringify(e2eConfig).includes('wdio:default')) {
  issues.push('src-tauri/tauri.e2e.conf.json: missing WDIO test capability');
}

requireMatch(cargoText, /name\s*=\s*"grass-pet-tauri"/, 'src-tauri/Cargo.toml: missing Tauri crate');
requireMatch(cargoText, /tauri\s*=\s*\{/, 'src-tauri/Cargo.toml: missing tauri dependency');
requireMatch(cargoText, /e2e\s*=\s*\[[^\]]*tauri-plugin-wdio[^\]]*tauri-plugin-wdio-webdriver/s, 'src-tauri/Cargo.toml: WDIO plugins must be behind the e2e feature');
requireMatch(libText, /AppState::load_shared\(&data_root\)/, 'src-tauri/src/lib.rs: all pet processes must use the shared state file');
requireMatch(commandsText, /summon_new_pet/, 'src-tauri/src/commands.rs: missing summon command');
requireMatch(commandsText, /std::env::current_exe\(\)/, 'src-tauri/src/commands.rs: summon command must relaunch the current application binary');
requireMatch(trayText, /再召唤一个阿飘/, 'src-tauri/src/tray.rs: tray summon item is missing');
requireMatch(contextMenuText, /再召唤一个阿飘/, 'src-vue/context-menu/ContextMenuApp.vue: context-menu summon item is missing');
requireMatch(libText, /cfg\(feature\s*=\s*"e2e"\)/, 'src-tauri/src/lib.rs: E2E plugin registration must be feature-gated');
requireMatch(libText, /tauri_plugin_wdio_webdriver::init\(\)/, 'src-tauri/src/lib.rs: embedded WebDriver plugin is missing');
requireMatch(libText, /tauri_plugin_wdio::init\(\)/, 'src-tauri/src/lib.rs: WDIO bridge plugin is missing');
requireMatch(apiText, /@tauri-apps\/api\/core/, 'src-vue/shared/api.ts: Vue runtime must use the Tauri API');
forbidMatch(apiText, /electron|ipcRenderer|contextBridge/, 'src-vue/shared/api.ts: Vue API must not contain Electron code');
requireMatch(viteText, /src-vue\/pet\/main\.ts|index\.html/, 'vite.config.mts: Vue/Tauri frontend entries are missing');
requireMatch(wdioText, /driverProvider:\s*['"]embedded['"]/, 'wdio.conf.ts: macOS-capable embedded driver must be selected');

if (issues.length) {
  console.error(`Tauri contract: FAIL (${issues.length} issue(s))`);
  for (const issue of issues) console.error(`- ${issue}`);
  process.exit(1);
}

console.log('Tauri contract: PASS (Tauri-only runtime, embedded E2E driver, no Electron source or tooling).');
