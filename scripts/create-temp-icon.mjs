import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';

const size = 1024;
const radius = 190;
const raw = Buffer.alloc((size * 4 + 1) * size);

for (let y = 0; y < size; y++) {
  const row = y * (size * 4 + 1);
  for (let x = 0; x < size; x++) {
    const dx = Math.max(radius - x, 0, x - (size - radius - 1));
    const dy = Math.max(radius - y, 0, y - (size - radius - 1));
    const inside = dx * dx + dy * dy <= radius * radius;
    const i = row + 1 + x * 4;
    raw[i] = 0x1f;
    raw[i + 1] = 0x5f;
    raw[i + 2] = 0xa9;
    raw[i + 3] = inside ? 0xff : 0;
  }
}

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const name = Buffer.from(type);
  const out = Buffer.alloc(data.length + 12);
  out.writeUInt32BE(data.length, 0);
  name.copy(out, 4);
  data.copy(out, 8);
  out.writeUInt32BE(crc32(Buffer.concat([name, data])), data.length + 8);
  return out;
}

const header = Buffer.alloc(13);
header.writeUInt32BE(size, 0);
header.writeUInt32BE(size, 4);
header[8] = 8;
header[9] = 6;

writeFileSync(
  new URL('../app-icon.png', import.meta.url),
  Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]),
);
