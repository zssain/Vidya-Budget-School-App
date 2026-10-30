// Session watcher (§9 auto-lock). The backend owns the idle deadline — the Rust
// RtCtx on desktop/Android, lock.ts on the PWA — and refuses protected commands once
// it passes. This module only (1) reports REAL user input, throttled, so the deadline
// moves, and (2) re-reads app_state when the session may have ended (a LOCKED error,
// returning to the app, or a periodic check), so the UI drops to the PIN screen
// instead of leaving protected data on screen. Background polling never counts as
// activity.

import * as api from './api'
import { LOCKED_EVENT } from './backend'
import { isWeb } from './platform'
import { refreshAppState } from './store'
import { installAutoLock } from './web/lock'

/** Send at most one activity ping per this interval. */
const ACTIVITY_THROTTLE_MS = 30_000
/** How often to re-check the state while the app is open. */
const STATE_CHECK_MS = 20_000

let installed = false

/** Install once at app start. Idempotent. */
export function installSessionWatcher(): void {
  if (installed || typeof window === 'undefined') return
  installed = true

  if (isWeb) {
    // The PWA keeps its own deadline in lock.ts (input listeners live there).
    installAutoLock(() => void refreshAppState())
  } else {
    let lastSent = 0
    const onInput = () => {
      const now = Date.now()
      if (now - lastSent < ACTIVITY_THROTTLE_MS) return
      lastSent = now
      api.session_activity().catch(() => {})
    }
    for (const ev of ['pointerdown', 'keydown', 'touchstart', 'wheel']) {
      document.addEventListener(ev, onInput, { passive: true, capture: true })
    }
  }

  window.addEventListener(LOCKED_EVENT, () => void refreshAppState())
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void refreshAppState()
  })
  setInterval(() => void refreshAppState(), STATE_CHECK_MS)
}
