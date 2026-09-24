#!/usr/bin/env node
// Release-artifact cleanliness gate (prompts/P09 §2). Plain Node, no deps.
//
// Builds the production frontend (unless dist/ already exists and --no-build is
// passed) and searches it for dev-only artifacts that must never ship:
// the __gallery / __mocks routes, the seed_demo_school command, the design/runtime
// mock server, dev fixtures, and the dev licence public key. Fails if any is
// found. (The Rust side is gated by #[cfg(debug_assertions)]; a release binary
// strings-scan belongs in the release job — see docs/phase-notes/phase-9.md.)
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const DIST = join(ROOT, 'dist')

if (!process.argv.includes('--no-build') || !existsSync(DIST)) {
  console.log('check-release-clean: building production frontend…')
  execSync('npm run build', { cwd: ROOT, stdio: 'inherit' })
}

// Forbidden substrings and the dev licence public key (from build-config/dev.json).
const FORBIDDEN = [
  '__gallery',
  '__mocks',
  'seed_demo_school',
  'design/runtime',
  'yLQ8lt26cM/ZdKnfaYGS/VgV6DT6CrAyLHS1br28XJs=', // dev licence public key
]

function files(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    const st = statSync(full)
    if (st.isDirectory()) files(full, out)
    else if (/\.(js|css|html|map)$/.test(name)) out.push(full)
  }
  return out
}

const hits = []
for (const f of files(DIST)) {
  const text = readFileSync(f, 'utf8')
  for (const s of FORBIDDEN) {
    if (text.includes(s)) hits.push({ file: f.slice(ROOT.length + 1), needle: s })
  }
}

if (hits.length) {
  console.error('\ncheck-release-clean: FAIL — dev-only artifacts leaked into the build:\n')
  for (const h of hits) console.error(`  • "${h.needle}"  in  ${h.file}`)
  console.error('')
  process.exit(1)
}
console.log('check-release-clean: OK — no dev routes, seed, fixtures or dev keys in the built frontend.')
