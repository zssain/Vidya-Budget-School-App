#!/usr/bin/env node
// Checks that the set of Tauri commands is consistent across three sources:
//   1. src-tauri/src/lib.rs   (the `generate_handler![...]` list)
//   2. src/api/commands.js    (camelCase JS wrappers)
//   3. docs/API.md            (the command tables)
//
// A command present in Rust or JS but missing from another required place is an
// error. Commands that appear only in docs/API.md are "planned" and allowed.
//
// The parsing functions are exported for unit tests (check-api-drift.test.js).

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** Command names from the `generate_handler![...]` macro, module paths stripped. */
export function parseRustCommands(source) {
  const m = source.match(/generate_handler!\s*\[([\s\S]*?)\]/);
  if (!m) return [];
  return (
    m[1]
      .split(',')
      .map((s) => s.trim())
      // drop line comments that may sit inside the macro
      .filter((s) => s && !s.startsWith('//'))
      .map((s) => s.split('::').pop().trim())
      .filter((s) => /^[a-z][a-z0-9_]*$/.test(s))
  );
}

/** camelCase -> snake_case (collectFee -> collect_fee). */
export function camelToSnake(name) {
  return name.replace(/([A-Z])/g, '_$1').toLowerCase();
}

/** Exported function names from src/api/commands.js, converted to snake_case. */
export function parseJsCommands(source) {
  const names = new Set();
  const fnRe = /export\s+(?:async\s+)?function\s+([A-Za-z0-9_]+)/g;
  const constRe = /export\s+const\s+([A-Za-z0-9_]+)\s*=\s*(?:async\s*)?\(/g;
  let m;
  while ((m = fnRe.exec(source))) names.add(m[1]);
  while ((m = constRe.exec(source))) names.add(m[1]);
  return [...names].map(camelToSnake);
}

/**
 * Command names from the first column of markdown tables in docs/API.md.
 * Only cells shaped like `snake_case` (optionally followed by **D** or **M**)
 * count, which excludes the events table (hyphenated) and HTTP routes table
 * (METHOD /path).
 */
export function parseApiCommands(source) {
  const names = new Set();
  for (const line of source.split('\n')) {
    if (!line.trimStart().startsWith('|')) continue;
    const cells = line.split('|').map((c) => c.trim());
    const first = cells[1] ?? '';
    const m = first.match(/^`([a-z_]+)`(?:\s*\*\*[DM]\*\*)?$/);
    if (m) names.add(m[1]);
  }
  return [...names];
}

function main() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const read = (rel) => readFileSync(join(root, rel), 'utf8');

  const libPath = join(root, 'src-tauri/src/lib.rs');
  const rust = new Set(existsSync(libPath) ? parseRustCommands(read('src-tauri/src/lib.rs')) : []);

  const api = new Set(parseApiCommands(read('docs/API.md')));

  let js = null;
  const jsPath = join(root, 'src/api/commands.js');
  if (existsSync(jsPath)) {
    js = new Set(parseJsCommands(read('src/api/commands.js')));
  } else {
    console.log('notice: src/api/commands.js does not exist yet — skipping the JS side.');
  }

  const errors = [];
  for (const cmd of rust) {
    if (!api.has(cmd)) errors.push(`Rust command '${cmd}' is missing from docs/API.md`);
    if (js && !js.has(cmd)) errors.push(`Rust command '${cmd}' is missing from src/api/commands.js`);
  }
  if (js) {
    for (const cmd of js) {
      if (!rust.has(cmd)) errors.push(`JS command '${cmd}' is missing from the generate_handler! list`);
      if (!api.has(cmd)) errors.push(`JS command '${cmd}' is missing from docs/API.md`);
    }
  }

  const planned = [...api].filter((c) => !rust.has(c));
  console.log(`Commands — API.md: ${api.size}, Rust: ${rust.size}, JS: ${js ? js.size : 'skipped'}`);
  console.log(`Planned (in API.md but not yet in Rust): ${planned.length}`);

  if (errors.length) {
    console.error('\nAPI drift detected:');
    for (const e of errors) console.error(`  - ${e}`);
    process.exit(1);
  }
  console.log('check-api-drift: OK');
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
