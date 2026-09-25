#!/usr/bin/env node
// Generate site/releases.json from a folder of built release artifacts + their
// SHA256SUMS.txt (prompts/P12 Step 9). The Downloads page (site/downloads.html)
// reads this file. Honest by construction: a `-UNSIGNED` / `-UNSIGNED-TEST` suffix
// in the filename → `signed_by: null` (the page shows "Unsigned (verify the
// SHA-256)"). Installed size is not known in the checksums job → `installed_bytes:
// null` (the page then shows only the download size).
//
// Usage:
//   node scripts/write-site-releases.mjs <release-dir> <version> <released-on> <repo> <out-file>
//     release-dir : folder holding the .exe/.dmg/.apk artifacts and SHA256SUMS.txt
//     version     : e.g. 1.2.3 (no leading v)
//     released-on : YYYY-MM-DD
//     repo        : owner/name (for the download URL)
//     out-file    : path to write (e.g. site/releases.json)

import { readFileSync, writeFileSync, statSync, readdirSync, existsSync } from 'node:fs'
import { join } from 'node:path'

const [dir, version, releasedOn, repo, outFile] = process.argv.slice(2)
if (!dir || !version || !releasedOn || !repo || !outFile) {
  console.error('usage: write-site-releases.mjs <release-dir> <version> <released-on> <repo> <out-file>')
  process.exit(2)
}

// Parse SHA256SUMS.txt ("<hex>␠␠<filename>") if present.
const sums = {}
const sumsPath = join(dir, 'SHA256SUMS.txt')
if (existsSync(sumsPath)) {
  for (const line of readFileSync(sumsPath, 'utf8').split('\n')) {
    const m = line.trim().match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/)
    if (m) sums[m[2]] = m[1].toLowerCase()
  }
}

function classify(file) {
  const unsigned = /UNSIGNED/i.test(file)
  const signed_by = unsigned ? null : 'signed'
  if (file.endsWith('.exe')) {
    return { os: 'windows', label: 'Windows 10/11 (64-bit)', requirements: 'Windows 10/11 x64', signed_by }
  }
  if (file.endsWith('.dmg')) {
    return { os: 'macos', label: 'macOS 12+ (Intel + Apple Silicon)', requirements: 'macOS 12 or newer', signed_by }
  }
  if (/arm64-v8a.*\.apk$/.test(file)) {
    return { os: 'android', label: 'Android 7+ (arm64-v8a)', requirements: 'Android 7+ (64-bit)', signed_by }
  }
  if (/armeabi-v7a.*\.apk$/.test(file)) {
    return { os: 'android', label: 'Android 7+ (armeabi-v7a)', requirements: 'Android 7+ (32-bit)', signed_by }
  }
  return null
}

const order = { windows: 0, macos: 1, android: 2 }
const platforms = []
for (const file of readdirSync(dir)) {
  if (file === 'SHA256SUMS.txt') continue
  const info = classify(file)
  if (!info) continue
  platforms.push({
    os: info.os,
    label: info.label,
    file,
    url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(file)}`,
    download_bytes: statSync(join(dir, file)).size,
    installed_bytes: null,
    sha256: sums[file] ?? null,
    requirements: info.requirements,
    signed_by: info.signed_by,
  })
}
platforms.sort((a, b) => (order[a.os] - order[b.os]) || a.label.localeCompare(b.label))

const manifest = {
  generated_at: new Date().toISOString(),
  releases: [
    {
      version,
      released_on: releasedOn,
      draft: false,
      notes_url: `https://github.com/${repo}/releases/tag/v${version}`,
      platforms,
    },
  ],
}
writeFileSync(outFile, JSON.stringify(manifest, null, 2) + '\n')
console.log(`wrote ${outFile} — ${platforms.length} platform artifact(s) for v${version}`)
