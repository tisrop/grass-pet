import { access, copyFile, lstat, symlink, unlink } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const profile = process.argv[2] || 'release';
if (!['debug', 'release'].includes(profile)) {
  throw new Error('Usage: create-named-launcher.mjs [debug|release]');
}

const extension = process.platform === 'win32' ? '.exe' : '';
const directory = path.join(root, 'src-tauri', 'target', profile);
const internalName = `apiao-daozhang-desktop-pet${extension}`;
const displayName = `阿飘道长桌宠${extension}`;
const internalPath = path.join(directory, internalName);
const displayPath = path.join(directory, displayName);

await access(internalPath);
try {
  await lstat(displayPath);
  await unlink(displayPath);
} catch (error) {
  if (error?.code !== 'ENOENT') throw error;
}

if (process.platform === 'win32') {
  await copyFile(internalPath, displayPath);
  console.log(`Named application executable: ${displayPath}`);
} else {
  await symlink(internalName, displayPath);
  console.log(`Named application launcher: ${displayPath} -> ${internalName}`);
}
