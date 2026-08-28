import { readFile } from 'node:fs/promises';
import process from 'node:process';

const [assetsPath, tag] = process.argv.slice(2);
if (!assetsPath || !/^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(tag ?? '')) {
  throw new Error('Usage: node release-assets.mjs <release-assets.json> <vX.Y.Z tag>');
}

const assets = JSON.parse(await readFile(assetsPath, 'utf8'));
if (!Array.isArray(assets)) throw new Error('Release assets response must be an array');

const names = assets.map((asset) => asset?.name).filter((name) => typeof name === 'string');
const version = tag.slice(1);
const errors = [];
const required = [
  ['macOS ARM64 DMG', (name) => name.endsWith('.dmg') && /(?:aarch64|arm64)/i.test(name)],
  ['macOS x64 DMG', (name) => name.endsWith('.dmg') && /(?:x64|x86_64)/i.test(name)],
  ['Windows installer', (name) => /\.(?:msi|exe)$/i.test(name)],
];

for (const [label, predicate] of required) {
  if (!names.some(predicate)) errors.push(`missing ${label}`);
}
if (names.some((name) => name.endsWith('.sig'))) {
  errors.push('unexpected updater signature asset; updater signing is not configured');
}
for (const name of names) {
  if (!name.includes(version)) errors.push(`asset name does not include version ${version}: ${name}`);
}
for (const asset of assets) {
  if (!Number.isInteger(asset?.size) || asset.size < 1024 * 1024) {
    errors.push(`asset is unexpectedly small or has no size: ${asset?.name ?? '<unknown>'}`);
  }
}

if (errors.length > 0) {
  console.error('Release asset validation failed:');
  for (const error of errors) console.error(`- ${error}`);
  console.error(`Assets: ${names.join(', ') || '<none>'}`);
  process.exit(1);
}

console.log(`Release assets: PASS (${names.length} files for ${tag})`);
