// Hand-written i18n (no i18next). t(key, vars) with {name} interpolation.
// Strings live in per-screen modules under ./strings so no user-visible string
// is hard-coded in a component/screen. `en` holds every visible mock string
// word-for-word; `hi` mirrors the keys with "TODO-HI: <english>" (Phase 8
// fills real Hindi).
import { useSyncExternalStore } from 'react'
import common from './strings/common'
import welcome from './strings/welcome'
import principalHome from './strings/principalHome'
import collectFee from './strings/collectFee'
import teacherHome from './strings/teacherHome'
import attendance from './strings/attendance'
import errors from './strings/errors'
import p04 from './strings/p04'
import shell from './strings/shell'
import students from './strings/students'
import fees from './strings/fees'
import receipts from './strings/receipts'
import daybook from './strings/daybook'
import attendanceReg from './strings/attendanceReg'
import shared from './strings/shared'
import academics from './strings/academics'
import reports from './strings/reports'

export type Lang = 'en' | 'hi'
export interface Bundle {
  en: Record<string, string>
  hi: Record<string, string>
}

const bundles: Bundle[] = [common, welcome, principalHome, collectFee, teacherHome, attendance, errors, p04, shell, students, fees, receipts, daybook, attendanceReg, shared, academics, reports]
const en: Record<string, string> = Object.assign({}, ...bundles.map((b) => b.en))
const hi: Record<string, string> = Object.assign({}, ...bundles.map((b) => b.hi))
const dict: Record<Lang, Record<string, string>> = { en, hi }

let lang: Lang = 'en'
const listeners = new Set<() => void>()

export function setLang(next: Lang): void {
  lang = next
  if (typeof document !== 'undefined') document.documentElement.setAttribute('lang', next)
  for (const l of listeners) l()
}
export function getLang(): Lang {
  return lang
}
function subscribe(cb: () => void): () => void {
  listeners.add(cb)
  return () => {
    listeners.delete(cb)
  }
}
export function useLang(): Lang {
  return useSyncExternalStore(
    subscribe,
    () => lang,
    () => 'en' as Lang,
  )
}

export function t(key: string, vars?: Record<string, string | number>): string {
  let s = dict[lang][key] ?? dict.en[key] ?? key
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.split(`{${k}}`).join(String(v))
    }
  }
  return s
}

/** For tests / tooling: the merged English dictionary. */
export const enDict = en
export const hiDict = hi
