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
  const packageJson = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
  const lock = JSON.parse(await readFile(path.join(root, 'package-lock.json'), 'utf8'));
  if (packageJson.name !== lock.name || packageJson.version !== lock.version) {
    failures.push('package-lock identity does not match package.json');
  }
  if (lock.packages?.['']?.engines?.node !== packageJson.engines?.node) {
    failures.push('package-lock root engine does not match package.json');
  }
} catch (error) {
  failures.push(`package metadata could not be read: ${error instanceof Error ? error.message : String(error)}`);
}

if (failures.length) {
  console.error(`Tauri preflight: FAIL (${failures.length} issue(s))`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`Tauri preflight: PASS (${rustc}; ${cargo}; ${tauri}).`);
if (contract) console.log(contract);
