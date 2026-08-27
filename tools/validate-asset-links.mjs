import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const assetRoot = path.join(root, 'src', 'assets', 'pet');
const spec = JSON.parse(await readFile(path.join(root, 'pet-spec.json'), 'utf8'));
const errors = [];

async function walk(directory, prefix = '') {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) files.push(...await walk(path.join(directory, entry.name), relative));
    else if (entry.isFile() && entry.name.toLowerCase().endsWith('.png')) files.push(relative);
  }
  return files;
}

const expected = new Set([spec.character.coreAsset, ...spec.states.flatMap((state) => state.frames)]);
for (const name of expected) {
  if (name.includes('\\')) errors.push(`asset paths must use forward slashes: ${name}`);
}
const actual = await walk(assetRoot);
const actualSet = new Set(actual);
for (const name of expected) if (!actualSet.has(name)) errors.push(`missing or case-mismatched runtime asset: ${name}`);
for (const name of actual) if (!expected.has(name)) errors.push(`orphan runtime PNG is not referenced by pet-spec.json: ${name}`);

const folded = new Map();
for (const name of actual) {
  const key = name.toLocaleLowerCase('en-US');
  const previous = folded.get(key);
  if (previous && previous !== name) errors.push(`case-insensitive asset collision: ${previous} <> ${name}`);
  folded.set(key, name);
}

const petSource = await readFile(path.join(root, 'src-vue', 'pet', 'PetApp.vue'), 'utf8');
if (!/import\.meta\.glob\(['"]\.\.\/\.\.\/src\/assets\/pet\/\*\*\/\*\.png['"]/u.test(petSource)) {
  errors.push('src-vue/pet/PetApp.vue must recursively import nested runtime assets');
}
const dashboardSource = await readFile(path.join(root, 'src-vue', 'dashboard', 'DashboardApp.vue'), 'utf8');
if (!/src\/assets\/pet\/core-ip\/core-ip\.png/u.test(dashboardSource)) {
  errors.push('src-vue/dashboard/DashboardApp.vue must import the character core asset');
}

if (errors.length) {
  console.error(`Runtime asset links: FAIL\n${errors.map((error) => `- ${error}`).join('\n')}`);
  process.exit(1);
}
console.log(`Runtime asset links: PASS (${expected.size} referenced PNGs, no orphans).`);
