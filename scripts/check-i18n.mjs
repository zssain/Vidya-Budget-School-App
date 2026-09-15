#!/usr/bin/env node
// Checks that the English and Hindi locale files have exactly the same set of
// keys. Until the locales exist (added in P1.2) this is a no-op.
// P1.2 extends this script to also scan views for untranslated text.

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const enPath = join(root, 'src/locales/en.json');
const hiPath = join(root, 'src/locales/hi.json');

/** Flatten nested JSON to dot-notation keys (a.b.c). */
function flattenKeys(obj, prefix = '') {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      keys.push(...flattenKeys(v, key));
    } else {
      keys.push(key);
    }
  }
  return keys;
}

if (!existsSync(enPath) || !existsSync(hiPath)) {
  console.log('check-i18n: no locales yet');
  process.exit(0);
}

const en = new Set(flattenKeys(JSON.parse(readFileSync(enPath, 'utf8'))));
const hi = new Set(flattenKeys(JSON.parse(readFileSync(hiPath, 'utf8'))));

const missingInHi = [...en].filter((k) => !hi.has(k));
const missingInEn = [...hi].filter((k) => !en.has(k));

if (missingInHi.length || missingInEn.length) {
  console.error('i18n key mismatch between en.json and hi.json:');
  for (const k of missingInHi) console.error(`  - missing in hi.json: ${k}`);
  for (const k of missingInEn) console.error(`  - missing in en.json: ${k}`);
  process.exit(1);
}

console.log(`check-i18n: OK (${en.size} keys in both languages)`);
