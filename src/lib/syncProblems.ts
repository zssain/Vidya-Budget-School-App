// How a refused change (list_sync_problems → SyncProblemDto) is described to the
// user on the phone (repair B1). Pure: the translator is injected so it is unit-
// tested without the i18n runtime. Copy lives in strings/syncProblems.ts.
import type { SyncProblemDto } from './api'

type Tr = (key: string, vars?: Record<string, string | number>) => string

/** The kinds of change the phone can make (op table → label key suffix). */
export const SYNC_PROBLEM_KINDS = [
  'attendance_sheet', 'marks_sheet', 'report_remark', 'homework_note', 'staff_attendance',
  'request', 'message', 'circular_read',
] as const

/** Translate `key`, or use `fallback` when the key is not defined (t returns the key). */
function tOr(tr: Tr, key: string, fallback: string): string {
  const s = tr(key)
  return s === key ? fallback : s
}

/** `{what, why}` for one refused change: the kind of change, and the school PC's
 *  reason in words (its error copy when known, else the raw code, else "no reason"). */
export function describeSyncProblem(p: SyncProblemDto, tr: Tr): { what: string; why: string } {
  const kind = p.table && (SYNC_PROBLEM_KINDS as readonly string[]).includes(p.table) ? p.table : 'other'
  const what = tr(`syncp.kind.${kind}`)
  const why = p.reason ? tOr(tr, `error.${p.reason}`, p.reason) : tr('syncp.noReason')
  return { what, why }
}
