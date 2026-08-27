const sharp = require('sharp');
const path = require('path');

(async () => {
  const src = path.resolve('src/assets/pet/core-ip/core-ip.png');
  const out = path.resolve('src/assets/tray/tray-icon.png');
  const trimmed = await sharp(src).trim().toBuffer();
  const tmeta = await sharp(trimmed).metadata();
  await sharp(trimmed)
    .resize(32, 32, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png({ compressionLevel: 9 })
    .toFile(out);
  const outMeta = await sharp(out).metadata();
  const { data, info } = await sharp(out).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  let visible = 0;
  for (let index = 3; index < data.length; index += 4) if (data[index] >= 16) visible += 1;
  const visibleRatio = visible / (info.width * info.height);
  console.log('tray generated:', outMeta.width + 'x' + outMeta.height, 'trimmed source', tmeta.width + 'x' + tmeta.height, 'visibleRatio', (visibleRatio * 100).toFixed(1) + '%');
})().catch((e) => { console.error(e); process.exit(1); });
