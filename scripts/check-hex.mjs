// check-hex — every hex colour used in shipped `src/` must be a value defined in
// src/styles/tokens.css (docs/00-SYSTEM-CONTEXT.md §15, prompts/P07 Verification).
//
// The mock is the design; new screens may use ONLY the mock's tokens. This gate
// fails the build if a raw colour sneaks into a shipped source file.
//
// Scope: src/**/*.{ts,tsx,css} EXCEPT src/dev/** — the dev-only Gallery / mock
// renderer is compiled out of release (import.meta.env.DEV) and legitimately
// renders the raw .dc.html mock files, so it is not held to the token palette.
//
// 3-digit shorthand (#fff) is normalised to 6-digit (#ffffff) before comparison.

import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const SRC = join(ROOT, 'src')
const TOKENS = join(SRC, 'styles', 'tokens.css')
const EXCLUDE_DIRS = new Set(['dev'])
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g

function normalise(hex) {
  let h = hex.slice(1).toLowerCase()
  if (h.length === 3) h = h.split('').map((c) => c + c).join('')
  if (h.length === 8) h = h.slice(0, 6) // ignore an alpha byte if ever present
  return '#' + h
}

// 1) Collect the allowed palette from tokens.css.
const allowed = new Set()
for (const m of readFileSync(TOKENS, 'utf8').matchAll(HEX_RE)) {
  allowed.add(normalise(m[0]))
}

// 2) Walk shipped src files.
function walk(dir, acc) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    const st = statSync(p)
    if (st.isDirectory()) {
      if (dir === SRC && EXCLUDE_DIRS.has(name)) continue
      walk(p, acc)
    } else if (/\.(ts|tsx|css)$/.test(name) && p !== TOKENS) {
      acc.push(p)
    }
  }
  return acc
}

const violations = []
for (const file of walk(SRC, [])) {
  const lines = readFileSync(file, 'utf8').split('\n')
  lines.forEach((line, i) => {
    for (const m of line.matchAll(HEX_RE)) {
      const hex = normalise(m[0])
      if (!allowed.has(hex)) {
        violations.push(`${relative(ROOT, file).split(sep).join('/')}:${i + 1}  ${m[0]} (not in tokens.css)`)
      }
    }
  })
}

if (violations.length > 0) {
  console.error(`check-hex: ${violations.length} colour(s) not defined in tokens.css:\n`)
  for (const v of violations) console.error('  ' + v)
  console.error('\nAdd the colour to the mock token set, or use an existing --token.')
  process.exit(1)
}

console.log(`check-hex: OK — every hex in shipped src/ is a tokens.css value (${allowed.size} tokens).`)
