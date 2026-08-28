import { readFile } from 'node:fs/promises';
import process from 'node:process';

const root = new URL('../../', import.meta.url);

async function readJson(relativePath) {
  return JSON.parse(await readFile(new URL(relativePath, root), 'utf8'));
}

function readArgument(name) {
  const index = process.argv.indexOf(name);
  if (index === -1) return undefined;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('--')) throw new Error(`${name} requires a value`);
  return value;
}

const [packageJson, pnpmLock, petSpec, tauriConfig, cargoToml] = await Promise.all([
  readJson('package.json'),
  readFile(new URL('pnpm-lock.yaml', root), 'utf8'),
  readJson('pet-spec.json'),
  readJson('src-tauri/tauri.conf.json'),
  readFile(new URL('src-tauri/Cargo.toml', root), 'utf8'),
]);

const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const expectedVersion = packageJson.version;
const versions = new Map([
  ['package.json', packageJson.version],
  ['pet-spec.json', petSpec.app?.version],
  ['src-tauri/Cargo.toml', cargoVersion],
  ['src-tauri/tauri.conf.json', tauriConfig.version],
]);
const errors = [];

if (!/^lockfileVersion:\s*['"]?\d/m.test(pnpmLock)) {
  errors.push('pnpm-lock.yaml does not declare a valid lockfile version');
}
if (!/^importers:\s*\n  \.:\s*$/m.test(pnpmLock)) {
  errors.push('pnpm-lock.yaml is missing the root importer');
}

for (const [source, version] of versions) {
  if (typeof version !== 'string' || version.length === 0) {
    errors.push(`${source} does not declare a version`);
  } else if (version !== expectedVersion) {
    errors.push(`${source} has ${version}, expected ${expectedVersion}`);
  }
}

const expectedProductName = petSpec.app?.name;
for (const [source, productName] of [
  ['package.json productName', packageJson.productName],
  ['src-tauri/tauri.conf.json productName', tauriConfig.productName],
]) {
  if (productName !== expectedProductName) {
    errors.push(`${source} (${productName}) does not match pet-spec.json app.name (${expectedProductName})`);
  }
}

if (tauriConfig.identifier !== petSpec.app?.appId) {
  errors.push(`src-tauri/tauri.conf.json identifier (${tauriConfig.identifier}) does not match pet-spec.json app.appId (${petSpec.app?.appId})`);
}
if (tauriConfig.mainBinaryName !== packageJson.name) {
  errors.push(`src-tauri/tauri.conf.json mainBinaryName (${tauriConfig.mainBinaryName}) does not match package.json name (${packageJson.name})`);
}

const tag = readArgument('--tag');
if (tag && tag !== `v${expectedVersion}`) {
  errors.push(`release tag ${tag} does not match application version v${expectedVersion}`);
}

if (errors.length > 0) {
  console.error('Version/configuration consistency check failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`Version/configuration consistency: PASS (${expectedVersion})`);
