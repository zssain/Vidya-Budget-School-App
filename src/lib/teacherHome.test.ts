import { describe, expect, it } from 'vitest'
import type { RequestDto, TeacherDashboard } from './api'
import { agoText, attendanceTarget, buildFooter, buildTasks, buildTeacherHome, buildTiles, dueLine, initials, TEACHER_TILES } from './teacherHome'
import { formatDateLongIn } from './format'
import { enDict, hiDict } from './i18n'

// A t() over the real English dictionary (so the assertions read like the screen).
const t = (key: string, vars?: Record<string, string | number>) => {
  let s = enDict[key] ?? key
  for (const [k, v] of Object.entries(vars ?? {})) s = s.split(`{${k}}`).join(String(v))
  return s
}

const now = new Date(2026, 8, 23, 8, 30) // Wed 23 Sep 2026, 08:30 local

const dash: TeacherDashboard = {
  classes: [
    { class_id: 'cls-5a', display: 'V-A', submitted_today: false, status: 'not_started', student_count: 34, duty: false },
    { class_id: 'cls-7b', display: 'VII-B', submitted_today: true, status: 'submitted', student_count: 50, duty: false },
    { class_id: 'cls-1a', display: 'I-A', submitted_today: false, status: 'draft', student_count: 50, duty: true },
  ],
  class_teacher_of: ['cls-5a', 'cls-7b'],
  class_subject_ids: ['cs-5a-eng'],
}

const req = (over: Partial<RequestDto>): RequestDto => ({
  id: 'r', kind: 'attendance_correction', target_table: 'attendance_sheet', target_id: 'x', reason: 'Fix', status: 'pending',
  apply_state: 'not_applied', before_json: null, after_json: null, requested_by: 'stf-meena', requester_name: 'Meena Iyer',
  created_at: '2026-09-22T10:00:00Z', note: null, ...over,
})

describe('Teacher Home tasks (real data, not the mock)', () => {
  it('one row per class still to submit today, routed to the real class id', () => {
    const tasks = buildTasks(dash, [], 'stf-meena', now, t)
    expect(tasks.map((x) => [x.title, x.sub, x.to, x.action?.label])).toEqual([
      ['V-A attendance', 'Not submitted · 34 students', '/teacher/attendance/cls-5a', 'Start'],
      ['I-A attendance', 'Covering today · Draft saved · 50 students', '/teacher/attendance/cls-1a', 'Continue'],
    ])
    expect(tasks.every((x) => !x.to.includes('v-a'))).toBe(true)
  })

  it('a request row only for my returned / recently decided requests', () => {
    const tasks = buildTasks(null, [
      req({ id: 'a', status: 'returned', note: 'Which day was it?' }),
      req({ id: 'b', status: 'returned', kind: 'leave' }),
      req({ id: 'c', status: 'approved', kind: 'leave', created_at: '2026-09-21T10:00:00Z' }),
      req({ id: 'd', status: 'rejected', created_at: '2026-08-01T10:00:00Z' }), // too old
      req({ id: 'e', status: 'pending' }),
      req({ id: 'f', status: 'returned', requested_by: 'someone-else' }),
    ], 'stf-meena', now, t)
    expect(tasks.map((x) => [x.key, x.title, x.sub])).toEqual([
      ['req-a', 'Reply to the Principal', 'Which day was it?'],
      ['req-b', 'Reply to the Principal', 'Leave request was returned'],
      ['req-c', 'Request approved', 'Leave'],
    ])
    expect(tasks.every((x) => x.to === '/teacher/requests')).toBe(true)
  })

  it('the headline counts real things due', () => {
    expect(dueLine([], t)).toBe('Nothing is due.')
    expect(dueLine(buildTasks(dash, [], 'stf-meena', now, t), t)).toBe('2 things are due.')
    const one = buildTasks(null, [req({ status: 'approved', created_at: '2026-09-22T00:00:00Z' }), req({ id: 'x', status: 'returned' })], 'stf-meena', now, t)
    expect(dueLine(one, t)).toBe('One thing is due.')
  })
})

describe('Teacher Home tiles', () => {
  const all = () => true
  it('keeps the mock order and layout, with real targets (no /placeholder)', () => {
    const tiles = buildTiles({ available: all, dashboard: dash, returned: 0, unread: 0, problems: 0 })
    expect(tiles.map((x) => x.id)).toEqual(TEACHER_TILES.map((x) => x.id))
    expect(tiles.map((x) => x.to)).toEqual([
      '/teacher/attendance', '/teacher/marks', '/teacher/remarks', '/teacher/timetable', '/teacher/students',
      '/teacher/requests', '/teacher/inbox', '/teacher/sync', '/teacher/profile',
    ])
    expect(tiles.every((x) => x.badge === undefined)).toBe(true)
  })
  it('badges are real counts or nothing', () => {
    const tiles = buildTiles({ available: all, dashboard: dash, returned: 2, unread: 1, problems: 3 })
    expect(tiles.find((x) => x.id === 'requests')?.badge).toBe('2')
    expect(tiles.find((x) => x.id === 'inbox')?.badge).toBe('4')
  })
  it('hides a tile whose command the platform lacks', () => {
    const tiles = buildTiles({ available: (c) => c !== 'list_students', dashboard: dash, returned: 0, unread: 0, problems: 0 })
    expect(tiles.some((x) => x.id === 'students')).toBe(false)
    expect(tiles).toHaveLength(8)
  })
  it('the Attendance tile goes straight to the only class due', () => {
    expect(attendanceTarget({ ...dash, classes: [dash.classes[0], dash.classes[1]] })).toBe('/teacher/attendance/cls-5a')
    expect(attendanceTarget({ ...dash, classes: [dash.classes[1]] })).toBe('/teacher/attendance/cls-7b')
    expect(attendanceTarget(dash)).toBe('/teacher/attendance')
    expect(attendanceTarget(null)).toBe('/teacher/attendance')
  })
})

describe('Teacher Home footer + header', () => {
  it('footer from the real sync state, hidden when there is nothing true to say', () => {
    expect(buildFooter(null, now, t)).toBeNull()
    expect(buildFooter({ pending_count: 0, pending_paise: 0, last_confirmed_at: null }, now, t)).toBeNull()
    expect(buildFooter({ pending_count: 2, pending_paise: 0, last_confirmed_at: null }, now, t)).toEqual({ text: '2 change(s) waiting to sync', tone: 'pending' })
    const twoMinAgo = new Date(now.getTime() - 2 * 60_000).toISOString()
    expect(buildFooter({ pending_count: 0, pending_paise: 0, last_confirmed_at: twoMinAgo }, now, t)).toEqual({ text: 'Up to date · synced 2 min ago', tone: 'ok' })
  })
  it('ago text', () => {
    expect(agoText(new Date(now.getTime() - 5_000), now, t)).toBe('just now')
    expect(agoText(new Date(now.getTime() - 3 * 3_600_000), now, t)).toBe('3 h ago')
    expect(agoText(new Date(now.getTime() - 2 * 86_400_000), now, t)).toBe('2 d ago')
  })
  it('initials from the signed-in name', () => {
    expect(initials('Meena Iyer')).toBe('MI')
    expect(initials('Anita')).toBe('A')
    expect(initials('  ')).toBe('·')
  })
  it('greeting + date + bell from real data', () => {
    const v = buildTeacherHome({
      staffName: 'Anita Rao', staffId: 'stf-anita', lang: 'en', now, dashboard: dash, requests: [],
      circulars: [
        { id: 'c1', read_by_me: false } as never,
        { id: 'c2', read_by_me: true } as never,
      ],
      problems: 0, sync: null, available: () => true, t,
    })
    expect(v.greeting).toBe('Good morning, Anita.')
    expect(v.eyebrow).toBe('Wednesday, 23 September')
    expect(v.initials).toBe('AR')
    expect(v.unread).toBe(1)
    expect(v.tiles.find((x) => x.id === 'inbox')?.badge).toBe('1')
    expect(buildTeacherHome({ staffName: 'Anita Rao', staffId: 'x', lang: 'en', now: new Date(2026, 8, 23, 18, 0), dashboard: null, requests: null, circulars: null, problems: 0, sync: null, available: () => true, t }).greeting).toBe('Good evening, Anita.')
  })
  it('the date follows the app language', () => {
    expect(formatDateLongIn(now, 'hi')).toBe('बुधवार, 23 सितंबर')
    expect(formatDateLongIn(now, 'te')).toContain('బుధవారం')
    expect(formatDateLongIn(now, 'en')).toBe('Wednesday, 23 September')
  })
  it('no mock copy is left in the strings', () => {
    for (const d of [enDict, hiDict]) {
      for (const v of Object.values(d)) {
        expect(v).not.toMatch(/synced 2 min ago|Rahul Kumar|Meena\./)
      }
    }
  })
})
