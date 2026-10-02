import { describe, expect, it } from 'vitest'
import { isAvailableOnWeb, unlockError, webAppState } from './backend'

// The pure boot/route mapper (Phase C). The I/O shell (reading the store) is covered
// by the WebKit PWA harness; here we lock the state machine itself.
describe('webAppState', () => {
  const staff = { id: 'stf-1', name: 'Meena', role: 'teacher' }

  it('routes a device that has not joined to the join/onboarding flow', () => {
    expect(webAppState({ joined: false, hasPin: false, unlocked: false, staff: undefined })).toEqual({
      kind: 'no_school',
    })
    // joined but without a PIN is incomplete onboarding, not an impassable PIN gate.
    expect(webAppState({ joined: true, hasPin: false, unlocked: false, staff })).toEqual({ kind: 'no_school' })
  })

  it('locks a joined device with a PIN until it is entered', () => {
    expect(webAppState({ joined: true, hasPin: true, unlocked: false, staff })).toEqual({ kind: 'locked' })
  })

  it('unlocks to the joined staff member once the PIN is entered', () => {
    expect(webAppState({ joined: true, hasPin: true, unlocked: true, staff })).toEqual({ kind: 'unlocked', staff })
  })

  it('stays locked if the session flag is set but the staff identity is missing', () => {
    expect(webAppState({ joined: true, hasPin: true, unlocked: true, staff: undefined })).toEqual({ kind: 'locked' })
  })
})

describe('unlockError', () => {
  const now = Date.UTC(2026, 9, 3, 12, 0, 0) // 2026-10-03T12:00:00Z

  it('maps a lockout to PIN_LOCKED with an RFC3339 instant (matching desktop)', () => {
    expect(unlockError({ ok: false, locked_seconds: 60 }, now)).toEqual({
      code: 'PIN_LOCKED',
      message_key: 'error.PIN_LOCKED',
      vars: { until: '2026-10-03T12:01:00.000Z' },
    })
  })

  it('maps a wrong PIN (no lockout yet) to PIN_WRONG with the tries left', () => {
    expect(unlockError({ ok: false, remaining: 3 }, now)).toEqual({
      code: 'PIN_WRONG',
      message_key: 'error.PIN_WRONG',
      vars: { remaining: 3 },
    })
  })

  it('prefers PIN_LOCKED when a wrong PIN also triggers the lockout', () => {
    expect(unlockError({ ok: false, locked_seconds: 30, remaining: 0 }, now).code).toBe('PIN_LOCKED')
  })

  it('falls back to a generic error when neither field is present', () => {
    expect(unlockError({ ok: false }, now).code).toBe('LOCKED')
  })
})

describe('isAvailableOnWeb', () => {
  it('reports the ported commands and hides the rest', () => {
    expect(isAvailableOnWeb('app_state')).toBe(true)
    expect(isAvailableOnWeb('create_pin')).toBe(true)
    expect(isAvailableOnWeb('list_staff')).toBe(true)
    expect(isAvailableOnWeb('unlock')).toBe(true)
    expect(isAvailableOnWeb('lock')).toBe(true)
    expect(isAvailableOnWeb('my_staff_day')).toBe(true)
    expect(isAvailableOnWeb('my_timetable')).toBe(true)
    expect(isAvailableOnWeb('list_classes')).toBe(true)
    expect(isAvailableOnWeb('collect_fee')).toBe(false)
  })
})
