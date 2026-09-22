#!/usr/bin/env node
// Size report (docs/00-SYSTEM-CONTEXT.md §2). Plain Node, no dependencies.
// Scans the Tauri bundle output folders, prints a table of artifact bytes, and
// exits 1 if any DOWNLOAD artifact exceeds 40,000,000 bytes (40 MB decimal).
// Installed-size checks are added in Phase 9.
import { readdirSync, statSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const MAX = 40_000_000 // 40 MB, decimal (§2)

// Where Tauri writes bundles (desktop + android split-per-abi).
const BUNDLE_DIRS = [
  'src-tauri/target/release/bundle',
  'src-tauri/target/universal-apple-darwin/release/bundle',
  'src-tauri/target/aarch64-apple-darwin/release/bundle',
  'src-tauri/target/x86_64-apple-darwin/release/bundle',
  'src-tauri/gen/android/app/build/outputs/apk',
  'src-tauri/gen/android/app/build/outputs/bundle',
]

// Extensions that are DOWNLOAD artifacts (subject to the 40 MB gate).
const DOWNLOAD_EXT = ['.dmg', '.exe', '.msi', '.apk', '.aab', '.appimage', '.deb', '.rpm']

function dirSize(p) {
  let total = 0
  for (const name of readdirSync(p)) {
    const full = join(p, name)
    const st = statSync(full)
    total += st.isDirectory() ? dirSize(full) : st.size
  }
  return total
}

function walk(dir, out) {
  if (!existsSync(dir)) return
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    const st = statSync(full)
    const lower = name.toLowerCase()
    if (st.isDirectory()) {
      // .app / .framework bundles are single installed artifacts; don't recurse into them.
      if (lower.endsWith('.app')) {
        out.push({ path: full, bytes: dirSize(full), kind: 'installed' })
      } else {
        walk(full, out)
      }
    } else if (DOWNLOAD_EXT.some((e) => lower.endsWith(e))) {
      out.push({ path: full, bytes: st.size, kind: 'download' })
    }
  }
}

const artifacts = []
for (const d of BUNDLE_DIRS) walk(join(ROOT, d), artifacts)

if (artifacts.length === 0) {
  console.log('size-report: no bundle artifacts found yet. Run `npm run tauri build` first.')
  console.log('Searched:')
  for (const d of BUNDLE_DIRS) console.log('  ' + d)
  process.exit(0)
}

const fmt = (n) => (n / 1_000_000).toFixed(2).padStart(8) + ' MB'
const rel = (p) => p.slice(ROOT.length).replace(/^\/+/, '')

artifacts.sort((a, b) => b.bytes - a.bytes)
console.log('\nVidya size report (limit: download ≤ 40,000,000 bytes)\n')
console.log('  ' + 'bytes'.padStart(12) + '   ' + 'size'.padStart(11) + '   kind        artifact')
console.log('  ' + '-'.repeat(78))
let over = false
for (const a of artifacts) {
  const flag = a.kind === 'download' && a.bytes > MAX ? '  <-- OVER 40 MB' : ''
  if (flag) over = true
  console.log(
    '  ' +
      String(a.bytes).padStart(12) +
      '   ' +
      fmt(a.bytes) +
      '   ' +
      a.kind.padEnd(10) +
      '  ' +
      rel(a.path) +
      flag,
  )
}
console.log('')
if (over) {
  console.error('size-report: FAIL — a download artifact exceeds 40 MB.\n')
  process.exit(1)
}
console.log('size-report: OK — all download artifacts within budget.\n')
