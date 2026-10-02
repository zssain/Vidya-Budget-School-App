import { describe, expect, it } from 'vitest'
import { isAvailableOnWeb, webAppState } from './backend'

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

describe('isAvailableOnWeb', () => {
  it('reports the ported commands and hides the rest', () => {
    expect(isAvailableOnWeb('app_state')).toBe(true)
    expect(isAvailableOnWeb('create_pin')).toBe(true)
    expect(isAvailableOnWeb('lock')).toBe(true)
    expect(isAvailableOnWeb('collect_fee')).toBe(false)
  })
})
