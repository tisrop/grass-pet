import { access, readFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const failures = [];

for (const relative of [
  'node_modules/@tauri-apps/cli/package.json',
  'node_modules/@tauri-apps/api/package.json',
  'node_modules/vue/package.json',
  'node_modules/vite/package.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
  'src-tauri/tauri.conf.json',
]) {
  try { await access(path.join(root, relative)); }
  catch { failures.push(`${relative} is missing`); }
}

const nodeMajor = Number(process.versions.node.split('.')[0]);
if (!Number.isInteger(nodeMajor) || nodeMajor < 22) failures.push(`Node 22+ is required; current ${process.versions.node}`);

function command(name, args) {
  const result = spawnSync(name, args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) {
    failures.push(`${name} ${args.join(' ')} failed: ${(result.stderr || result.stdout || '').trim()}`);
    return undefined;
  }
  return result.stdout.trim();
}

const rustc = command('rustc', ['--version']);
const cargo = command('cargo', ['--version']);
const tauri = command(process.execPath, [path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'), '--version']);
const contract = command(process.execPath, [path.join(root, 'tools', 'validate-tauri-contract.mjs')]);

try {
  JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
  const lock = await readFile(path.join(root, 'pnpm-lock.yaml'), 'utf8');
  if (!/^lockfileVersion:\s*['"]?\d/m.test(lock)) {
    failures.push('pnpm-lock.yaml has no valid lockfile version');
  }
  if (!/^importers:\s*\n  \.:\s*$/m.test(lock)) {
    failures.push('pnpm-lock.yaml is missing the root importer');
  }
} catch (error) {
  failures.push(`package metadata or pnpm lockfile could not be read: ${error instanceof Error ? error.message : String(error)}`);
}

if (failures.length) {
  console.error(`Tauri preflight: FAIL (${failures.length} issue(s))`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`Tauri preflight: PASS (${rustc}; ${cargo}; ${tauri}).`);
if (contract) console.log(contract);
