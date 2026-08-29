import { readFile } from 'node:fs/promises';
import process from 'node:process';

const [assetsPath, tag, latestPath] = process.argv.slice(2);
if (!assetsPath || !/^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(tag ?? '')) {
  throw new Error('Usage: node release-assets.mjs <release-assets.json> <vX.Y.Z tag>');
}

const assets = JSON.parse(await readFile(assetsPath, 'utf8'));
if (!Array.isArray(assets)) throw new Error('Release assets response must be an array');

const names = assets.map((asset) => asset?.name).filter((name) => typeof name === 'string');
const version = tag.slice(1);
const expectedPrefix = 'grass-pet_';
const errors = [];
const required = [
  ['macOS ARM64 DMG', (name) => name.endsWith('.dmg') && /(?:aarch64|arm64)/i.test(name)],
  ['macOS x64 DMG', (name) => name.endsWith('.dmg') && /(?:x64|x86_64)/i.test(name)],
  ['Windows installer', (name) => /\.(?:msi|exe)$/i.test(name)],
];

for (const [label, predicate] of required) {
  if (!names.some(predicate)) errors.push(`missing ${label}`);
}
const updaterMetadata = names.filter((name) => name === 'latest.json');
if (updaterMetadata.length !== 1) {
  errors.push(`expected exactly one latest.json updater metadata asset, found ${updaterMetadata.length}`);
}
for (const name of names) {
  if (name === 'latest.json') continue;
  if (!name.startsWith(expectedPrefix)) {
    errors.push(`asset name must use the ASCII prefix ${expectedPrefix}: ${name}`);
  }
  if (!name.includes(version)) errors.push(`asset name does not include version ${version}: ${name}`);
}
for (const asset of assets) {
  if (asset?.name === 'latest.json' || asset?.name?.endsWith('.sig')) continue;
  if (!Number.isInteger(asset?.size) || asset.size < 1024 * 1024) {
    errors.push(`asset is unexpectedly small or has no size: ${asset?.name ?? '<unknown>'}`);
  }
}

if (latestPath) {
  let metadata;
  try {
    metadata = JSON.parse(await readFile(latestPath, 'utf8'));
  } catch (error) {
    errors.push(`latest.json is not valid JSON: ${error.message}`);
  }
  const expectedPlatforms = ['darwin-aarch64', 'darwin-x86_64', 'windows-x86_64'];
  const actualPlatforms = Object.keys(metadata?.platforms ?? {}).sort();
  if (metadata?.version !== version) {
    errors.push(`latest.json version ${metadata?.version ?? '<missing>'} does not match ${version}`);
  }
  if (actualPlatforms.join(',') !== expectedPlatforms.slice().sort().join(',')) {
    errors.push(`latest.json must contain exactly: ${expectedPlatforms.join(', ')}`);
  }
  for (const platform of expectedPlatforms) {
    const entry = metadata?.platforms?.[platform];
    if (!entry || typeof entry.url !== 'string' || typeof entry.signature !== 'string' || !entry.signature.trim()) {
      errors.push(`latest.json has an incomplete ${platform} entry`);
      continue;
    }
    const assetName = entry.url.split('/').pop();
    if (!assetName || !names.includes(assetName)) {
      errors.push(`latest.json ${platform} URL does not reference a Release asset`);
    }
    if (!names.includes(`${assetName}.sig`)) {
      errors.push(`latest.json ${platform} URL has no matching .sig asset`);
    }
  }
}

if (errors.length > 0) {
  console.error('Release asset validation failed:');
  for (const error of errors) console.error(`- ${error}`);
  console.error(`Assets: ${names.join(', ') || '<none>'}`);
  process.exit(1);
}

console.log(`Release assets: PASS (${names.length} files for ${tag})`);
