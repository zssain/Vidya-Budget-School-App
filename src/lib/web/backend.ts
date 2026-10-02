// The Web backend for the iPhone PWA (Phase 19, Step 2). It implements EXACTLY the
// phone commands the staff client needs (registered in Step 3+ as the IndexedDB
// store, WASM and Drive sync land). Every other command — the school server, setup,
// restore, backups, licences — returns `NOT_AVAILABLE_ON_WEB` so the UI can hide
// what the PWA never provides (00-context §18).
//
// Import-safe: no browser API (IndexedDB, WebCrypto, fetch) runs at module load, so
// this file imports cleanly under vitest/Node and in the Tauri bundle (where it is
// dead code — `isWeb` is false there).

import type {
  AppState,
  AppStateResponse,
  ClassDto,
  CmdError,
  SessionStaff,
  StaffDayDto,
  StaffDto,
  TeacherTimetableDto,
} from '../api'
import { hasJoined } from './join'
import { createPin, hasPin, isUnlocked, lock, type UnlockResult, unlock as pinUnlock } from './lock'
import { kvGet, listRecords } from './store'
import {
  buildStaffDay,
  type LeaveRecordRec,
  type LeaveTypeRec,
  type SchoolRec,
  type SessionRec,
  type StaffAttendanceRec,
  truthy,
} from './staffday'
import {
  buildTeacherTimetable,
  type ClassRec,
  type ClassSubjectRec,
  type PeriodRec,
  type SubjectRec,
  type TimetableSlotRec,
} from './timetable'

/** A Web command handler: `(args) => result`. */
export type WebHandler = (args?: Record<string, unknown>) => Promise<unknown>

/**
 * Map the PWA's local onboarding/session flags to the shared `AppState` the router
 * (`routeForState`) already understands, so the app boots past the status screen
 * (Phase 19 "what remains" #1). Pure (no I/O) → unit-tested directly.
 *
 * A device that has not finished joining — no identity, or an identity without a PIN
 * (the join flow sets the PIN: `/join` → request → poll → set PIN → home) — routes to
 * `no_school`, where the join/onboarding UI (C2) picks it up; it is never an impassable
 * PIN gate. Once joined with a PIN it is `locked` until the PIN is entered, then
 * `unlocked` for the staff member who joined (teachers get the phone screens).
 */
export function webAppState(f: {
  joined: boolean
  hasPin: boolean
  unlocked: boolean
  staff: SessionStaff | undefined
}): AppState {
  if (!f.joined || !f.hasPin) return { kind: 'no_school' }
  if (f.unlocked && f.staff) return { kind: 'unlocked', staff: f.staff }
  return { kind: 'locked' }
}

/** Map a failed PIN `unlock` to the SAME CmdError the desktop throws, so the shared
 *  PinUnlockScreen shows identical copy: PIN_LOCKED `{until}` (an RFC3339 instant) when
 *  locked out, else PIN_WRONG `{remaining}`. Pure → unit-tested. `nowMs` is injected so
 *  the `until` instant is deterministic in tests. */
export function unlockError(res: UnlockResult, nowMs: number): CmdError {
  if (res.locked_seconds != null) {
    const until = new Date(nowMs + res.locked_seconds * 1000).toISOString()
    return { code: 'PIN_LOCKED', message_key: 'error.PIN_LOCKED', vars: { until } }
  }
  if (res.remaining != null) {
    return { code: 'PIN_WRONG', message_key: 'error.PIN_WRONG', vars: { remaining: res.remaining } }
  }
  return { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} }
}

/**
 * The commands the Web backend serves. Step 3 wires the local-store foundation
 * (PIN create/lock via WASM Argon2id + the encrypted IndexedDB store); the rest of
 * the phone-screen commands (unlock→app_state, attendance, notes, staff check-in,
 * leave, requests, inbox, the accountant collect-fee, sync status…) are registered
 * as the join flow (Step 5) and each screen are ported. A command absent from this
 * map is intentionally unavailable in the PWA and returns NOT_AVAILABLE_ON_WEB.
 */
export const WEB_COMMANDS: Record<string, WebHandler> = {
  // Boot/route state — the first ported command, so the PWA leaves the status screen
  // and lands on join (not yet onboarded), the PIN gate, or the teacher home.
  app_state: async (): Promise<AppStateResponse> => {
    const [joined, pinSet] = await Promise.all([hasJoined(), hasPin()])
    const staff = await kvGet<SessionStaff>('staff')
    return {
      state: webAppState({ joined, hasPin: pinSet, unlocked: isUnlocked(), staff }),
      licence_status: null, // the PWA is a joined client; licensing lives on the school PC
    }
  },
  create_pin: async (args) => {
    await createPin(String(args?.pin ?? ''))
  },
  // The one staff member who joined on this device (the PIN screen's user picker).
  list_staff: async (): Promise<StaffDto[]> => {
    const staff = await kvGet<SessionStaff>('staff')
    return staff ? [staff] : []
  },
  // The teacher-home dashboard: today's check-in state, this month's counts, the
  // recent days and leave balances — aggregated from the records sync populated,
  // identical to the Rust my_staff_day. Empty/zero until data has synced (honest).
  my_staff_day: async (args): Promise<StaffDayDto> => {
    const staff = await kvGet<SessionStaff>('staff')
    const today = String(args?.today ?? '')
    const [attendance, schools, sessions, leaveTypes, leaveRecords] = await Promise.all([
      listRecords<StaffAttendanceRec>('staff_attendance'),
      listRecords<SchoolRec & { id: string }>('school'),
      listRecords<SessionRec & { id: string }>('academic_session'),
      listRecords<LeaveTypeRec>('leave_type'),
      listRecords<LeaveRecordRec>('leave_record'),
    ])
    return buildStaffDay({
      staffId: staff?.id ?? '',
      today,
      attendance,
      school: schools[0],
      session: sessions.find((s) => truthy(s.is_current)) ?? null,
      leaveTypes,
      leaveRecords,
    })
  },
  // All classes (the attendance class-picker), ordered by sort_order — list_classes.
  list_classes: async (): Promise<ClassDto[]> => {
    type ClassRow = { id: string; display: string; name: string; section: string | null; sort_order?: number | null }
    const classes = await listRecords<ClassRow>('class')
    return classes
      .sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0))
      .map((c) => ({ id: c.id, display: c.display, name: c.name, section: c.section ?? null }))
  },
  // The teacher's own weekly timetable (periods + their slots), joined like SLOT_SELECT.
  my_timetable: async (): Promise<TeacherTimetableDto> => {
    const staff = await kvGet<SessionStaff>('staff')
    const [slots, classes, classSubjects, subjects, periods, sessions] = await Promise.all([
      listRecords<TimetableSlotRec>('timetable_slot'),
      listRecords<ClassRec>('class'),
      listRecords<ClassSubjectRec>('class_subject'),
      listRecords<SubjectRec>('subject'),
      listRecords<PeriodRec>('period'),
      listRecords<SessionRec & { id: string }>('academic_session'),
    ])
    const session = sessions.find((s) => truthy(s.is_current))
    return buildTeacherTimetable({
      staffId: staff?.id ?? '',
      staffName: staff?.name ?? '',
      sessionId: session?.id ?? null,
      slots,
      classes,
      classSubjects,
      subjects,
      periods,
    })
  },
  // Verify the PIN (same lockout curve as the app, via WASM) → the unlocked state. On
  // failure, reject with the desktop's PIN_WRONG / PIN_LOCKED CmdError. staffId is
  // ignored: a PWA holds exactly one joined identity.
  unlock: async (args): Promise<AppStateResponse> => {
    const res = await pinUnlock(String(args?.pin ?? ''))
    if (!res.ok) throw unlockError(res, Date.now())
    const staff = await kvGet<SessionStaff>('staff')
    return {
      state: webAppState({ joined: true, hasPin: true, unlocked: true, staff }),
      licence_status: null,
    }
  },
  lock: async () => {
    lock()
  },
}

/** True if the PWA implements `cmd`. The UI hides any feature whose command isn't. */
export function isAvailableOnWeb(cmd: string): boolean {
  return Object.prototype.hasOwnProperty.call(WEB_COMMANDS, cmd)
}

/** Dispatch a command to its Web handler, or reject with NOT_AVAILABLE_ON_WEB. */
export async function webInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = WEB_COMMANDS[cmd]
  if (!handler) {
    const err: CmdError = {
      code: 'NOT_AVAILABLE_ON_WEB',
      message_key: 'error.NOT_AVAILABLE_ON_WEB',
      vars: { cmd: cmd ?? '' },
    }
    // api.ts wrappers reject with the CmdError shape (as Tauri's invoke does on Err).
    return Promise.reject(err)
  }
  return (await handler(args)) as T
}
