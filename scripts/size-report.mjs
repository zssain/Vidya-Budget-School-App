#!/usr/bin/env node
// Size gate (docs/00-SYSTEM-CONTEXT.md §2, prompts/P09 §5). Plain Node, no deps.
// Scans the Tauri bundle output folders and enforces BOTH hard limits:
//   * download  ≤ 40,000,000 bytes (40 MB decimal) — the file a school downloads
//   * installed ≤ 50,000,000 bytes (50 MB decimal) — what lands on disk
// Writes size-report.json (CI uploads it) and exits 1 if any limit is exceeded.
//
// Installed size by platform:
//   * macOS  — the summed bytes of the .app bundle.
//   * Android— the APK itself (native libs are stored uncompressed & NOT
//     extracted at install: extractNativeLibs=false / useLegacyPackaging=false,
//     so installed ≈ APK size — prompts/P09 §5).
//   * Windows— the NSIS installer's silent-install footprint. That can only be
//     measured by actually running the installer, which CI does; it writes the
//     number to scripts/.windows-installed-bytes (one integer) and this script
//     picks it up. Absent that file, the Windows installed size is reported as
//     "unmeasured" (never silently passed).
import { readdirSync, statSync, existsSync, writeFileSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const MAX_DOWNLOAD = 40_000_000 // 40 MB decimal (§2)
const MAX_INSTALLED = 50_000_000 // 50 MB decimal (§2)

const BUNDLE_DIRS = [
  'target/release/bundle',
  'target/universal-apple-darwin/release/bundle',
  'target/aarch64-apple-darwin/release/bundle',
  'target/x86_64-apple-darwin/release/bundle',
  'src-tauri/gen/android/app/build/outputs/apk',
  'src-tauri/gen/android/app/build/outputs/bundle',
]

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
      // .app bundles are a single installed artifact; don't recurse into them.
      if (lower.endsWith('.app')) {
        out.push({ path: full, bytes: dirSize(full), kind: 'installed', limit: MAX_INSTALLED })
      } else {
        walk(full, out)
      }
    } else if (DOWNLOAD_EXT.some((e) => lower.endsWith(e))) {
      out.push({ path: full, bytes: st.size, kind: 'download', limit: MAX_DOWNLOAD })
      // An APK's installed footprint ≈ its own size (non-extracted native libs).
      if (lower.endsWith('.apk')) {
        out.push({ path: full + ' (installed ≈ apk)', bytes: st.size, kind: 'installed', limit: MAX_INSTALLED })
      }
    }
  }
}

const artifacts = []
for (const d of BUNDLE_DIRS) walk(join(ROOT, d), artifacts)

// Windows installed footprint, if CI measured it (one integer, bytes).
const winFile = join(ROOT, 'scripts', '.windows-installed-bytes')
if (existsSync(winFile)) {
  const n = parseInt(readFileSync(winFile, 'utf8').trim(), 10)
  if (Number.isFinite(n)) {
    artifacts.push({ path: 'Windows NSIS silent-install footprint', bytes: n, kind: 'installed', limit: MAX_INSTALLED })
  }
}

const fmt = (n) => (n / 1_000_000).toFixed(2).padStart(8) + ' MB'
const rel = (p) => (p.startsWith(ROOT) ? p.slice(ROOT.length).replace(/^\/+/, '') : p)

// Always write the machine-readable report (even when empty) for CI upload.
writeFileSync(
  join(ROOT, 'size-report.json'),
  JSON.stringify(
    {
      limits: { download: MAX_DOWNLOAD, installed: MAX_INSTALLED },
      artifacts: artifacts.map((a) => ({ artifact: rel(a.path), bytes: a.bytes, kind: a.kind, limit: a.limit, over: a.bytes > a.limit })),
    },
    null,
    2,
  ) + '\n',
)

if (artifacts.length === 0) {
  console.log('size-report: no bundle artifacts found yet. Run `npm run tauri build` first.')
  console.log('Searched:')
  for (const d of BUNDLE_DIRS) console.log('  ' + d)
  process.exit(0)
}

artifacts.sort((a, b) => b.bytes - a.bytes)
console.log('\nVidya size gate (download ≤ 40 MB, installed ≤ 50 MB; decimal)\n')
console.log('  ' + 'bytes'.padStart(12) + '   ' + 'size'.padStart(11) + '   kind        artifact')
console.log('  ' + '-'.repeat(78))
let over = false
for (const a of artifacts) {
  const flag = a.bytes > a.limit ? `  <-- OVER ${a.limit / 1_000_000} MB` : ''
  if (flag) over = true
  console.log(
    '  ' + String(a.bytes).padStart(12) + '   ' + fmt(a.bytes) + '   ' + a.kind.padEnd(10) + '  ' + rel(a.path) + flag,
  )
}
console.log('')
if (over) {
  console.error('size-report: FAIL — an artifact exceeds its size budget. See flags above.\n')
  process.exit(1)
}
console.log('size-report: OK — all artifacts within budget (download ≤ 40 MB, installed ≤ 50 MB).\n')
