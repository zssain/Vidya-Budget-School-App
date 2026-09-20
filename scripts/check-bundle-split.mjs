#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const mobileDist = fileURLToPath(new URL('../dist-mobile/', import.meta.url));
if (!existsSync(mobileDist)) {
  console.error('Mobile bundle not found. Run npm run build:mobile first.');
  process.exit(1);
}

function filesBelow(path) {
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const child = join(path, entry.name);
    return entry.isDirectory() ? filesBelow(child) : [child];
  });
}

for (const file of filesBelow(mobileDist).filter((path) => statSync(path).isFile())) {
  if (readFileSync(file, 'utf8').includes('VIDYA_DESKTOP_ONLY')) {
    console.error(`Desktop-only code leaked into the mobile bundle: ${file}`);
    process.exit(1);
  }
}

console.log('check-bundle-split: OK');
