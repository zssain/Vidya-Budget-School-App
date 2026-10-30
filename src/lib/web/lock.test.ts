import { describe, expect, it } from 'vitest'
import { idleExpired } from './lock'
import { webInvoke, LOCKED_ALLOWED } from './backend'

describe('idleExpired (PWA auto-lock, §9)', () => {
  const t0 = 1_700_000_000_000
  it('stays unlocked within 5 minutes of input', () => {
    expect(idleExpired(t0, t0)).toBe(false)
    expect(idleExpired(t0, t0 + 5 * 60 * 1000)).toBe(false)
  })
  it('locks after 5 minutes, including time the phone was asleep', () => {
    expect(idleExpired(t0, t0 + 5 * 60 * 1000 + 1)).toBe(true)
    expect(idleExpired(t0, t0 + 60 * 60 * 1000)).toBe(true)
  })
  it('fails closed if the clock jumps backwards', () => {
    expect(idleExpired(t0, t0 - 10 * 60 * 1000)).toBe(true)
  })
})

describe('webInvoke lock gate', () => {
  it('refuses protected commands while locked', async () => {
    await expect(webInvoke('my_staff_day', { today: '2026-10-31' })).rejects.toMatchObject({ code: 'LOCKED' })
    await expect(webInvoke('list_homework_notes', { class_id: 'c1' })).rejects.toMatchObject({ code: 'LOCKED' })
  })
  it('only lets the PIN gate and routing through', () => {
    expect([...LOCKED_ALLOWED].sort()).toEqual(['app_state', 'create_pin', 'list_staff', 'lock', 'session_activity', 'unlock'])
  })
  it('still reports unported commands as NOT_AVAILABLE_ON_WEB', async () => {
    await expect(webInvoke('backup_now')).rejects.toMatchObject({ code: 'NOT_AVAILABLE_ON_WEB' })
  })
})
