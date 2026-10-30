import { describe, expect, it } from 'vitest'
import { routeForState, homePathForRole } from './store'

describe('routeForState', () => {
  it('routes each state-machine state to the right screen', () => {
    expect(routeForState({ kind: 'no_school' })).toEqual({ screen: 'welcome' })
    expect(routeForState({ kind: 'activated' })).toEqual({ screen: 'setup', step: 1 })
    expect(routeForState({ kind: 'setup_in_progress', step: 4 })).toEqual({ screen: 'setup', step: 4 })
    expect(routeForState({ kind: 'locked' })).toEqual({ screen: 'pin' })
    const meena = { id: 't', name: 'Meena', role: 'teacher' }
    expect(routeForState({ kind: 'pin_setup', staff: meena })).toEqual({ screen: 'pin_setup', staff: meena })
    expect(routeForState({ kind: 'bootstrap_pending', staff: meena })).toEqual({ screen: 'bootstrap_pending' })
    expect(routeForState({ kind: 'unlocked', staff: { id: 'p', name: 'Priya', role: 'principal' } })).toEqual({
      screen: 'home',
      role: 'principal',
    })
    expect(routeForState({ kind: 'db_key_missing' })).toEqual({ screen: 'db_key_missing' })
    expect(routeForState({ kind: 'needs_rejoin' })).toEqual({ screen: 'needs_rejoin' })
    expect(routeForState({ kind: 'moved' })).toEqual({ screen: 'moved' })
  })

  it('maps roles to home paths', () => {
    expect(homePathForRole('principal')).toBe('/principal/home')
    expect(homePathForRole('accountant')).toBe('/accountant/collect')
    expect(homePathForRole('teacher')).toBe('/teacher/home')
  })
})
