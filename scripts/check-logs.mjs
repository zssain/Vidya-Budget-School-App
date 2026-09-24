#!/usr/bin/env node
// Log-scan gate (prompts/P09 §2). Plain Node, no dependencies.
//
// Logs must never contain tokens, keys, PINs, OAuth tokens, recovery keys or a
// full mobile number (last 4 digits only). This scans every Rust source file for
// logging macros (tracing::{trace,debug,info,warn,error}!, log::*!, println!,
// eprintln!, dbg!, print!) and fails if a secret-bearing identifier is
// INTERPOLATED into the message — i.e. appears inside a `{...}` placeholder or as
// a trailing format argument. Static strings that merely mention "token" (e.g.
// "device token revoked") are fine.
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const SCAN_DIRS = ['src-tauri/src', 'cloud/relay/src', 'cloud/licence/src', 'crates/vidya-core/src']

// Identifiers that must never be logged as a value. Word-boundary matched.
const SECRETS = [
  'device_token', 'session_key', 'audience_key', 'backup_key', 'relay_shared_key',
  'recovery_key', 'recovery', 'pin_hash', 'pin', 'password', 'passwd', 'secret',
  'access_token', 'refresh_token', 'oauth', 'id_token', 'private_key', 'key_hex',
  'token', 'mobile', 'phone',
]
// Allow a few obviously-safe identifiers that contain a secret word.
const ALLOW = new Set(['token_hash', 'mobile_last4', 'phone_last4', 'has_token', 'token_kind', 'key_id', 'pin_len'])

const LOG_MACRO = /\b(?:tracing::)?(?:trace|debug|info|warn|error)!\s*\(|\b(?:e?println|print|dbg)!\s*\(/
const secretWord = new RegExp(`\\b(${SECRETS.join('|')})\\b`, 'i')

function rustFiles(dir, out = []) {
  let entries
  try {
    entries = readdirSync(join(ROOT, dir))
  } catch {
    return out
  }
  for (const name of entries) {
    const rel = join(dir, name)
    const st = statSync(join(ROOT, rel))
    if (st.isDirectory()) rustFiles(rel, out)
    else if (name.endsWith('.rs')) out.push(rel)
  }
  return out
}

const findings = []
for (const dir of SCAN_DIRS) {
  for (const rel of rustFiles(dir)) {
    const lines = readFileSync(join(ROOT, rel), 'utf8').split('\n')
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]
      if (!LOG_MACRO.test(line)) continue
      // Gather the (possibly multi-line) macro call.
      let call = line
      for (let j = i + 1; j < lines.length && !call.includes(');') && j < i + 6; j++) call += '\n' + lines[j]
      // Look only at interpolations {name} and format args (after the format string).
      const interps = [...call.matchAll(/\{([a-zA-Z_][a-zA-Z0-9_.]*)[:?}]/g)].map((m) => m[1].split('.').pop())
      const afterFmt = call.replace(/^[^,]*"/s, '').split('"').slice(1).join('"') // args after the format literal
      const argIdents = [...afterFmt.matchAll(/\b([a-z_][a-z0-9_]*)\b/gi)].map((m) => m[1])
      for (const id of [...interps, ...argIdents]) {
        if (!id || ALLOW.has(id)) continue
        if (secretWord.test(id)) {
          findings.push({ file: rel, line: i + 1, ident: id, text: line.trim().slice(0, 100) })
          break
        }
      }
    }
  }
}

if (findings.length) {
  console.error('\ncheck-logs: FAIL — a secret-bearing value may be logged:\n')
  for (const f of findings) console.error(`  • ${f.file}:${f.line}  [${f.ident}]  ${f.text}`)
  console.error('\nRedact it (hash, last-4, or omit) before logging.\n')
  process.exit(1)
}
console.log('check-logs: OK — no tokens/keys/PINs/recovery keys/full mobiles interpolated into logs.')
