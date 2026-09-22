// Hand-written hash router (no react-router). State via useSyncExternalStore.
import { useSyncExternalStore } from 'react'

const DEFAULT = '/welcome'

function currentPath(): string {
  if (typeof location === 'undefined') return DEFAULT
  const h = location.hash.replace(/^#/, '')
  return h.length > 0 ? h : DEFAULT
}

const listeners = new Set<() => void>()
function emit() {
  for (const l of listeners) l()
}
if (typeof window !== 'undefined') {
  window.addEventListener('hashchange', emit)
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb)
  return () => {
    listeners.delete(cb)
  }
}

/** Navigate to a hash path, e.g. `navigate('/teacher/attendance/v-a')`. */
export function navigate(path: string): void {
  const next = path.startsWith('/') ? path : `/${path}`
  if (currentPath() !== next) {
    location.hash = next
  } else {
    emit()
  }
}

/** Reactive current path (leading slash, no `#`). */
export function useRoute(): string {
  return useSyncExternalStore(subscribe, currentPath, () => DEFAULT)
}

/**
 * Match a `/a/:id/b` pattern against a concrete path. Returns the params map,
 * or null if it does not match. `:name` captures one segment.
 */
export function matchRoute(pattern: string, path: string): Record<string, string> | null {
  const pp = pattern.split('/').filter(Boolean)
  const cp = path.split('/').filter(Boolean)
  if (pp.length !== cp.length) return null
  const params: Record<string, string> = {}
  for (let i = 0; i < pp.length; i++) {
    const seg = pp[i]
    if (seg.startsWith(':')) params[seg.slice(1)] = decodeURIComponent(cp[i])
    else if (seg !== cp[i]) return null
  }
  return params
}
