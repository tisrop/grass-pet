import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

function argument(name) {
  const index = process.argv.indexOf(name);
  const value = process.argv[index + 1];
  if (index === -1 || !value || value.startsWith('--')) {
    throw new Error(`${name} requires a value`);
  }
  return value;
}

const assetsPath = argument('--assets');
const releasePath = argument('--release');
const signaturesDir = argument('--signatures-dir');
const outputPath = argument('--output');
const repository = argument('--repository');
const tag = argument('--tag');

const assets = JSON.parse(await readFile(assetsPath, 'utf8'));
const release = JSON.parse(await readFile(releasePath, 'utf8'));
if (!Array.isArray(assets)) throw new Error('Release assets response must be an array');
if (release.tag_name !== tag) throw new Error(`Release tag mismatch: ${release.tag_name} !== ${tag}`);

const assetNames = assets.map((asset) => asset?.name).filter((name) => typeof name === 'string');
const expectedUrlPrefix = `https://github.com/${repository}/releases/download/${tag}/`;

const targets = [
  ['darwin-aarch64', /(?:aarch64|arm64)/i],
  ['darwin-x86_64', /(?:x86_64|x64|amd64)/i],
  ['windows-x86_64', /(?:x86_64|x64|amd64)/i],
];

async function findArtifact(platform, architecture) {
  const suffix = platform.startsWith('darwin-') ? '.app.tar.gz' : '.exe';
  const candidates = assets.filter((asset) => {
    const name = asset?.name;
    return typeof name === 'string'
      && name.startsWith('grass-pet_')
      && architecture.test(name)
      && name.endsWith(suffix);
  });
  if (candidates.length !== 1) {
    throw new Error(`${platform} updater artifact must match exactly once; found ${candidates.length}`);
  }

  const artifact = candidates[0];
  const signatureName = `${artifact.name}.sig`;
  const signaturePath = path.join(signaturesDir, signatureName);
  if (!assetNames.includes(signatureName)) {
    throw new Error(`${platform} updater signature is missing: ${signatureName}`);
  }
  const signature = (await readFile(signaturePath, 'utf8')).trim();
  if (!signature) throw new Error(`${platform} updater signature is empty`);

  const url = artifact.browser_download_url;
  if (typeof url !== 'string' || !url.startsWith(expectedUrlPrefix)) {
    throw new Error(`${platform} updater URL is not an official tagged Release URL`);
  }

  return { signature, url };
}

const platforms = {};
for (const [platform, architecture] of targets) {
  platforms[platform] = await findArtifact(platform, architecture);
}

const metadata = {
  version: tag.slice(1),
  notes: typeof release.body === 'string' ? release.body : '',
  pub_date: release.published_at ?? release.created_at ?? new Date().toISOString(),
  platforms,
};
await writeFile(outputPath, `${JSON.stringify(metadata, null, 2)}\n`, 'utf8');
console.log(`Updater metadata generated for ${Object.keys(platforms).length} platforms`);
