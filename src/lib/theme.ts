// Accent theme helper (docs/01-MOCK-SPEC.md §2.3). Writes --accent and the
// derived --accent-6/-10/-12 onto :root. Default #2F7479. Persisted to
// localStorage for now; Phase 3 moves it into settings.

const ACCENT_KEY = 'vidya.accent'
export const DEFAULT_ACCENT = '#2F7479'
export const ACCENT_OPTIONS = ['#2F7479', '#1F4E8C', '#5B4B8A'] as const

export function setAccent(hex: string): void {
  const n = parseInt(hex.slice(1), 16)
  const rgba = (a: number) => `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`
  const root = document.documentElement
  root.style.setProperty('--accent', hex)
  root.style.setProperty('--accent-6', rgba(0.06))
  root.style.setProperty('--accent-10', rgba(0.1))
  root.style.setProperty('--accent-12', rgba(0.12))
  try {
    localStorage.setItem(ACCENT_KEY, hex)
  } catch {
    /* storage unavailable — keep the in-memory value */
  }
}

/** Apply the stored accent (or the default) on startup. Returns the hex used. */
export function loadAccent(): string {
  let hex = DEFAULT_ACCENT
  try {
    hex = localStorage.getItem(ACCENT_KEY) || DEFAULT_ACCENT
  } catch {
    hex = DEFAULT_ACCENT
  }
  setAccent(hex)
  return hex
}
