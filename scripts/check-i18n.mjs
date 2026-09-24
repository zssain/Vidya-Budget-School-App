// check-i18n.mjs (prompts/P08 Part F, P09 §7) — the i18n gate.
//
// Fails (exit 1) on any of:
//   * a per-module `en`/`hi` key-set mismatch (missing or extra keys);
//   * any `hi` value still containing the `TODO-HI` placeholder;
//   * any empty `en` or `hi` value;
//   * a key defined in more than one module (silent override in the merged dict).
//
// Plain Node (docs §13: hand-built tooling, no i18next). Each string module is a
// tiny TS file (`import type { Bundle }` + string maps). We can't import TS
// directly, so we strip the only TS syntax (the type import + `: Record<...>` /
// `: Bundle` annotations) and import the result as a data-URL ES module — which
// runs the real code, so multi-line values, `{placeholder}` tokens and the
// `Object.fromEntries` auto-generate pattern all evaluate exactly as at runtime.

import { readdir, readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const stringsDir = join(here, '..', 'src', 'lib', 'i18n', 'strings')

/** Turn a strings/*.ts module into runnable JS and import its default export. */
async function loadBundle(file) {
  const src = await readFile(file, 'utf8')
  const js = src
    .replace(/^\s*import\s+type\s+\{[^}]*\}\s+from\s+['"][^'"]*['"];?\s*$/gm, '')
    .replace(/:\s*Record<\s*string\s*,\s*string\s*>/g, '')
    .replace(/:\s*Bundle\b/g, '')
  const mod = await import('data:text/javascript;charset=utf-8,' + encodeURIComponent(js))
  return mod.default
}

const errors = []
const seenKeys = new Map() // key -> module that first defined it

const files = (await readdir(stringsDir)).filter((f) => f.endsWith('.ts')).sort()
let totalKeys = 0

for (const f of files) {
  let bundle
  try {
    bundle = await loadBundle(join(stringsDir, f))
  } catch (e) {
    errors.push(`${f}: failed to load (${e.message})`)
    continue
  }
  if (!bundle || typeof bundle.en !== 'object' || typeof bundle.hi !== 'object') {
    errors.push(`${f}: default export is not a { en, hi } bundle`)
    continue
  }
  const en = bundle.en
  const hi = bundle.hi
  const enKeys = new Set(Object.keys(en))
  const hiKeys = new Set(Object.keys(hi))

  for (const k of enKeys) {
    if (!hiKeys.has(k)) errors.push(`${f}: key "${k}" is in en but missing from hi`)
  }
  for (const k of hiKeys) {
    if (!enKeys.has(k)) errors.push(`${f}: key "${k}" is in hi but not in en`)
  }
  for (const [k, v] of Object.entries(en)) {
    if (typeof v !== 'string' || v.trim() === '') errors.push(`${f}: en "${k}" is empty`)
  }
  for (const [k, v] of Object.entries(hi)) {
    if (typeof v !== 'string' || v.trim() === '') errors.push(`${f}: hi "${k}" is empty`)
    else if (v.includes('TODO-HI')) errors.push(`${f}: hi "${k}" still has a TODO-HI placeholder`)
  }
  for (const k of enKeys) {
    if (seenKeys.has(k)) errors.push(`duplicate key "${k}" in ${f} (already in ${seenKeys.get(k)})`)
    else seenKeys.set(k, f)
  }
  totalKeys += enKeys.size
}

if (errors.length > 0) {
  console.error(`check-i18n: FAILED — ${errors.length} problem(s):`)
  for (const e of errors) console.error('  - ' + e)
  process.exit(1)
}
console.log(`check-i18n: OK — ${files.length} modules, ${totalKeys} keys, en/hi in sync, no TODO-HI.`)
