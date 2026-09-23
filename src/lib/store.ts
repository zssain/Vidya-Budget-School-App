// App store (prompts/P03 Step 8). Hand-built with useSyncExternalStore (no
// zustand/redux, per docs §13). Holds the routing app_state; screens fetch their
// own data through `api` and keep local state. `run()` executes a command and
// refreshes app_state so the UI re-routes (activate → setup → PIN → home …).

import { useSyncExternalStore } from 'react'
import * as api from './api'
import type { AppState, AppStateResponse, CmdError } from './api'

export interface StoreState {
  app: AppStateResponse | null
  loading: boolean
  error: CmdError | null
}

let state: StoreState = { app: null, loading: false, error: null }
const listeners = new Set<() => void>()

function emit(): void {
  for (const l of listeners) l()
}
function set(patch: Partial<StoreState>): void {
  state = { ...state, ...patch }
  emit()
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb)
  return () => {
    listeners.delete(cb)
  }
}
function getSnapshot(): StoreState {
  return state
}

/** Fetch app_state from the backend and update the store. */
export async function refreshAppState(): Promise<void> {
  set({ loading: true })
  try {
    const app = await api.app_state()
    set({ app, loading: false, error: null })
  } catch (e) {
    set({ error: e as CmdError, loading: false })
  }
}

/** Run a command, then refresh app_state (so routing follows the state machine). */
export async function run<T>(fn: () => Promise<T>): Promise<T> {
  const result = await fn()
  await refreshAppState()
  return result
}

/** React hook for the whole store snapshot. */
export function useStore(): StoreState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot)
}

// ---- Pure routing from the state machine (unit-tested) -------------------

export type Route =
  | { screen: 'welcome' }
  | { screen: 'setup'; step: number }
  | { screen: 'pin' }
  | { screen: 'home'; role: string }
  | { screen: 'db_key_missing' }
  | { screen: 'needs_rejoin' }
  | { screen: 'moved' }

/** Map an app state to the screen the UI should show (docs §6.2, Step 4). */
export function routeForState(app: AppState): Route {
  switch (app.kind) {
    case 'no_school':
      return { screen: 'welcome' }
    case 'activated':
      return { screen: 'setup', step: 1 }
    case 'setup_in_progress':
      return { screen: 'setup', step: app.step }
    case 'locked':
      return { screen: 'pin' }
    case 'unlocked':
      return { screen: 'home', role: app.staff.role }
    case 'db_key_missing':
      return { screen: 'db_key_missing' }
    case 'needs_rejoin':
      return { screen: 'needs_rejoin' }
    case 'moved':
      return { screen: 'moved' }
  }
}

/** The home route path for a role (Principal/Accountant/Teacher). */
export function homePathForRole(role: string): string {
  switch (role) {
    case 'principal':
      return '/principal/home'
    case 'accountant':
      return '/accountant/collect'
    default:
      return '/teacher/home'
  }
}
