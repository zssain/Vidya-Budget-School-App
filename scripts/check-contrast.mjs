#!/usr/bin/env node
// Accessibility contrast gate (prompts/P09 §6). Plain Node, no dependencies.
//
// Reads the design tokens from src/styles/tokens.css and checks the WCAG 2.1
// contrast ratio of the text/background pairs the mock actually uses. Body text
// must reach 4.5:1; large text (≥ 18.66px bold / 24px) may use 3:1. The mock
// wins: a FAIL here is REPORTED for the owner to decide, it does not silently
// change a colour. Exits 1 only if a *body-text* pair is below 4.5:1.
import { readFileSync } from 'node:fs'

const css = readFileSync(new URL('../src/styles/tokens.css', import.meta.url), 'utf8')
const tokens = Object.fromEntries(
  [...css.matchAll(/(--[\w-]+):\s*(#[0-9a-fA-F]{3,8})\b/g)].map((m) => [m[1], m[2]]),
)

const rgb = (h) => {
  h = h.replace('#', '')
  if (h.length === 3) h = [...h].map((c) => c + c).join('')
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16))
}
const lin = (c) => {
  c /= 255
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
}
const lum = ([r, g, b]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
const ratio = (a, b) => {
  const la = lum(rgb(a)), lb = lum(rgb(b))
  const [hi, lo] = la > lb ? [la, lb] : [lb, la]
  return (hi + 0.05) / (lo + 0.05)
}

// [foreground token, background token, "body" | "large"] — the pairs §6 calls
// out plus the status pills/badges that carry meaning.
const PAIRS = [
  ['--muted', '--bg', 'body'],
  ['--muted', '--surface', 'body'],
  ['--muted', '--white', 'body'],
  ['--gold-text', '--surface', 'body'],
  ['--gold-text', '--bg', 'body'],
  ['--gold-on-navy-text', '--navy', 'body'],
  ['--on-navy-muted', '--navy', 'body'],
  ['--on-navy-muted', '--navy-deep', 'body'],
  ['--on-navy', '--navy', 'body'],
  ['--on-navy-label', '--navy', 'body'],
  ['--ink', '--bg', 'body'],
  ['--pill-partpaid-fg', '#f4ecdc', 'body'],
  ['--pill-unpaid-fg', '#f6e4e2', 'body'],
  ['--pill-marks-fg', '#e7e3f1', 'body'],
  ['--online-text', '--navy', 'body'],
]

const resolve = (t) => (t.startsWith('--') ? tokens[t] : t)

let failed = false
console.log('\nContrast (WCAG 2.1; body ≥ 4.5:1, large ≥ 3:1)\n')
for (const [fgTok, bgTok, size] of PAIRS) {
  const fg = resolve(fgTok), bg = resolve(bgTok)
  if (!fg || !bg) {
    console.error(`  MISSING  ${fgTok} on ${bgTok} — token not found in tokens.css`)
    failed = true
    continue
  }
  const r = ratio(fg, bg)
  const min = size === 'large' ? 3.0 : 4.5
  const ok = r >= min
  if (!ok && size === 'body') failed = true
  console.log(
    `  ${r.toFixed(2).padStart(5)}:1  ${ok ? 'PASS' : 'FAIL'}  ${fgTok} (${fg}) on ${bgTok} (${bg})`,
  )
}
console.log('')
if (failed) {
  console.error('check-contrast: FAIL — a body-text pair is below 4.5:1. REPORT to the owner (the mock wins unless they decide to change it).\n')
  process.exit(1)
}
console.log('check-contrast: OK — every checked pair meets its WCAG minimum.\n')
