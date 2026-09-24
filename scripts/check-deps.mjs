#!/usr/bin/env node
// Dependency gate (docs/00-SYSTEM-CONTEXT.md §13, prompts/P09 §7). Plain Node.
//
// Reads the DIRECT dependencies of the shipped app — package.json (npm) and
// `cargo metadata --no-deps` (the Cargo workspace members) — and fails if any
// is not on the §13 allowlist. Transitive dependencies are not checked (they
// are implied by the allowed direct deps); this gate is about what WE choose to
// pull in. The cloud/* services are separate Cargo workspaces (excluded from the
// root workspace) and are not part of the shipped app, so they are out of scope.
//
// Two direct deps are on the allowlist as EXPLICIT, documented exceptions that
// are pending owner sign-off (they warn, they do not fail):
//   * futures-util — the Sink/Stream companion tokio-tungstenite needs for the
//     relay tunnel; flagged in docs/phase-notes/phase-5.md ("trivially swappable").
//   * @types/node   — types-only dev dependency (zero shipped bytes) needed by
//     vite.config.ts / the Node build scripts; sibling of the §13-listed
//     @types/react / @types/react-dom.
// Both are recorded in docs/phase-notes/phase-9.md for the owner to fold into §13.
import { readFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))

// ---- §13 allowlists (verbatim from docs/00-SYSTEM-CONTEXT.md §13) -----------
const RUST_ALLOWED = new Set([
  'tauri', 'tauri-build', 'serde', 'serde_json', 'tokio', 'axum', 'hyper', 'hyper-util',
  'tokio-rustls', 'rustls', 'rcgen', 'reqwest', 'tokio-tungstenite', 'rusqlite', 'uuid',
  'argon2', 'rand', 'sha2', 'hmac', 'base64', 'chacha20poly1305', 'ed25519-dalek',
  'thiserror', 'time', 'mdns-sd', 'keyring', 'tracing', 'tracing-subscriber', 'qrcode',
  'tauri-plugin-dialog', 'tauri-plugin-opener', 'tauri-plugin-deep-link',
  'tauri-plugin-barcode-scanner',
  'vidya-core', // in-repo pure-rules crate
])

const JS_ALLOWED = new Set([
  // JS runtime
  'react', 'react-dom', '@tauri-apps/api',
  '@tauri-apps/plugin-dialog', '@tauri-apps/plugin-opener', '@tauri-apps/plugin-deep-link',
  '@tauri-apps/plugin-barcode-scanner',
  'class-variance-authority', 'clsx', 'tailwind-merge',
  '@radix-ui/react-slot', '@radix-ui/react-dialog', '@radix-ui/react-dropdown-menu',
  '@radix-ui/react-select', '@radix-ui/react-tabs', '@radix-ui/react-radio-group',
  '@radix-ui/react-checkbox',
  // Fonts (bundled, subset to Latin + Devanagari)
  '@fontsource/geist', '@fontsource-variable/newsreader', '@fontsource/noto-sans-devanagari',
  // JS dev
  'vite', '@vitejs/plugin-react', 'typescript', 'tailwindcss', '@tailwindcss/vite',
  'vitest', '@playwright/test', '@tauri-apps/cli', '@types/react', '@types/react-dom',
])

// Explicit, documented exceptions (warn, don't fail) — see the header note.
const PENDING_OWNER = new Set(['futures-util', '@types/node'])

// Named §13 "NOT allowed" list — anything here gets an especially loud error.
const BANNED = new Set([
  'motion', 'framer-motion', 'lucide-react', 'lucide', 'react-router', 'react-router-dom',
  'zustand', 'redux', '@reduxjs/toolkit', 'react-query', '@tanstack/react-query',
  'date-fns', 'dayjs', 'moment', 'chart.js', 'recharts', 'sonner', 'react-hook-form',
  'zod', 'i18next', 'react-i18next', 'electron', 'axum-server', 'aws-lc-rs',
])

const warnings = []
const errors = []

function check(name, allowed, ecosystem) {
  if (allowed.has(name)) return
  if (PENDING_OWNER.has(name)) {
    warnings.push(`${ecosystem}: "${name}" is not enumerated in §13 — allowed as a documented, owner-pending exception (see phase-9 notes).`)
    return
  }
  if (BANNED.has(name)) {
    errors.push(`${ecosystem}: "${name}" is on §13's explicit NOT-allowed list.`)
    return
  }
  errors.push(`${ecosystem}: "${name}" is not on the §13 allowlist.`)
}

// ---- npm direct deps --------------------------------------------------------
const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'))
const npmDeps = [...Object.keys(pkg.dependencies ?? {}), ...Object.keys(pkg.devDependencies ?? {})]
for (const d of npmDeps) check(d, JS_ALLOWED, 'npm')

// ---- Cargo direct deps (workspace members only) -----------------------------
let meta
try {
  const raw = execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  meta = JSON.parse(raw)
} catch (e) {
  console.error('check-deps: could not run `cargo metadata` —', e.message)
  process.exit(2)
}
const rustDeps = new Set()
for (const p of meta.packages ?? []) {
  for (const dep of p.dependencies ?? []) rustDeps.add(dep.name)
}
for (const d of [...rustDeps].sort()) check(d, RUST_ALLOWED, 'cargo')

// ---- report -----------------------------------------------------------------
for (const w of warnings) console.warn('  warn  ' + w)
if (errors.length) {
  console.error('\ncheck-deps: FAIL — dependencies outside §13:\n')
  for (const e of errors) console.error('  • ' + e)
  console.error('')
  process.exit(1)
}
console.log(
  `check-deps: OK — ${npmDeps.length} npm + ${rustDeps.size} cargo direct deps all within §13` +
    (warnings.length ? ` (${warnings.length} documented owner-pending exception${warnings.length > 1 ? 's' : ''})` : '') +
    '.',
)
