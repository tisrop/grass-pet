import { access, mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const checks = [];
const record = (name, ok, details) => checks.push({ name, ok, details });

const nodeMajor = Number(process.versions.node.split('.')[0]);
record('node-version', Number.isInteger(nodeMajor) && nodeMajor >= 22, { current: process.versions.node, required: '>=22' });

function probe(name, command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8' });
  const output = (result.stdout || result.stderr || '').trim();
  record(name, result.status === 0, { command: [command, ...args], output });
}

probe('rustc', 'rustc', ['--version']);
probe('cargo', 'cargo', ['--version']);
probe('tauri-cli', process.execPath, [path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'), '--version']);
probe('tauri-contract', process.execPath, [path.join(root, 'tools', 'validate-tauri-contract.mjs')]);

for (const relative of [
  'node_modules/@tauri-apps/cli/package.json',
  'node_modules/@tauri-apps/api/package.json',
  'node_modules/@tauri-apps/plugin-notification/package.json',
  'node_modules/vue/package.json',
  'node_modules/vite/package.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
  'src-tauri/tauri.conf.json',
]) {
  try {
    await access(path.join(root, relative));
    record(`file:${relative}`, true, {});
  } catch (error) {
    record(`file:${relative}`, false, { error: error instanceof Error ? error.message : String(error) });
  }
}

for (const [name, relative] of [
  ['asset-qa', 'qa/assets-report.json'],
  ['ui-qa', 'qa/ui-report.json'],
  ['experience-qa', 'qa/experience-report.json'],
]) {
  try {
    const report = JSON.parse(await readFile(path.join(root, relative), 'utf8'));
    record(name, report.passed === true, { generatedAt: report.generatedAt, passed: report.passed });
  } catch (error) {
    record(name, false, { error: error instanceof Error ? error.message : String(error), repair: 'Run pnpm run check' });
  }
}

const report = {
  generatedAt: new Date().toISOString(),
  runtime: 'tauri',
  platform: process.platform,
  arch: process.arch,
  passed: checks.every((check) => check.ok),
  checks,
};
await mkdir(path.join(root, '.build'), { recursive: true });
await writeFile(path.join(root, '.build', 'doctor-tauri-report.json'), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Tauri doctor: ${report.passed ? 'PASS' : 'ATTENTION'} (${checks.filter((check) => !check.ok).length} issue(s))`);
for (const check of checks.filter((item) => !item.ok)) console.error(`- ${check.name}: ${JSON.stringify(check.details)}`);
if (!report.passed) process.exit(1);
