#!/usr/bin/env node
// Single-source the app version (prompts/P09 §8). Plain Node, no dependencies.
//
//   node scripts/set-version.mjs 1.2.3   → set that version everywhere
//   node scripts/set-version.mjs         → propagate package.json's version
//   node scripts/set-version.mjs --check  → verify every file agrees (CI gate)
//
// package.json is the source of truth; the version flows to:
//   * src-tauri/Cargo.toml            [package] version
//   * crates/vidya-core/Cargo.toml    [package] version
//   * src-tauri/tauri.conf.json       "version"
//   * src-tauri/gen/android/app/tauri.properties
//       tauri.android.versionName = <x.y.z>
//       tauri.android.versionCode = major*10000 + minor*100 + patch  (§8)
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const p = (rel) => new URL('../' + rel, import.meta.url)

const arg = process.argv[2]
const CHECK = arg === '--check'

const pkg = JSON.parse(readFileSync(p('package.json'), 'utf8'))
let version = CHECK || !arg ? pkg.version : arg

const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version)
if (!m) {
  console.error(`set-version: "${version}" is not a plain x.y.z semver.`)
  process.exit(2)
}
const [major, minor, patch] = [Number(m[1]), Number(m[2]), Number(m[3])]
const versionCode = major * 10000 + minor * 100 + patch

// Each target: how to read its current version, and how to write a new one.
const targets = [
  {
    file: 'package.json',
    read: (s) => JSON.parse(s).version,
    write: (s, v) => s.replace(/("version":\s*)"[^"]*"/, `$1"${v}"`),
  },
  {
    file: 'src-tauri/Cargo.toml',
    read: (s) => /^version = "([^"]*)"/m.exec(s)?.[1],
    write: (s, v) => s.replace(/^version = "[^"]*"/m, `version = "${v}"`),
  },
  {
    file: 'crates/vidya-core/Cargo.toml',
    read: (s) => /^version = "([^"]*)"/m.exec(s)?.[1],
    write: (s, v) => s.replace(/^version = "[^"]*"/m, `version = "${v}"`),
  },
  {
    file: 'src-tauri/tauri.conf.json',
    read: (s) => JSON.parse(s).version,
    write: (s, v) => s.replace(/("version":\s*)"[^"]*"/, `$1"${v}"`),
  },
  {
    file: 'src-tauri/gen/android/app/tauri.properties',
    optional: true,
    read: (s) => /tauri\.android\.versionName=(.*)/.exec(s)?.[1]?.trim(),
    write: (s, v) =>
      s
        .replace(/tauri\.android\.versionName=.*/, `tauri.android.versionName=${v}`)
        .replace(/tauri\.android\.versionCode=.*/, `tauri.android.versionCode=${versionCode}`),
  },
]

let mismatched = false
for (const t of targets) {
  const url = p(t.file)
  if (!existsSync(url)) {
    if (t.optional) continue
    console.error(`set-version: missing ${t.file}`)
    process.exit(2)
  }
  const src = readFileSync(url, 'utf8')
  const current = t.read(src)
  if (CHECK) {
    if (current !== version) {
      console.error(`  mismatch  ${t.file}: ${current} (expected ${version})`)
      mismatched = true
    }
    continue
  }
  const next = t.write(src, version)
  if (next !== src) writeFileSync(url, next)
  console.log(`  set  ${t.file} → ${version}`)
}

if (CHECK) {
  if (mismatched) {
    console.error('\nset-version: FAIL — versions are not in sync. Run `node scripts/set-version.mjs`.')
    process.exit(1)
  }
  console.log(`set-version: OK — every file reports ${version} (Android versionCode ${versionCode}).`)
} else {
  console.log(`set-version: done — version ${version}, Android versionCode ${versionCode}.`)
}
