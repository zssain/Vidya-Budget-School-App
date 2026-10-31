// Teacher Home (repair T1): the tile/icon layout of the mock and the pure
// derivations that turn real data into what the home shows — the greeting, today's
// tasks, tile targets + badges, the bell's unread count and the sync footer. Every
// line comes from a real command (dashboard_teacher, list_requests, list_circulars,
// list_sync_problems, sync_status); nothing is fixture copy. Pure → unit-tested
// (teacherHome.test.ts); TeacherHomeContainer does the I/O.

import type { CircularDto, RequestDto, SyncStatusDto, TeacherDashboard } from './api'
import { formatDateLongIn, greeting } from './format'

/**
 * Which glyph a tile draws. Tiles whose two-tone geometry already lives in
 * src/lib/icons.ts (reportCard, myClasses, inbox, cloudSync, profile) reference
 * it by name; the tiles whose icons.ts entry holds the MONO sidebar geometry
 * instead of the mock's two-tone tile geometry (attendance, marks, students,
 * requests) are inlined verbatim in the screen and tagged here as 'inline:<name>'.
 */
export type TeacherTileIcon =
  | 'icon:reportCard'
  | 'icon:myClasses'
  | 'icon:inbox'
  | 'icon:cloudSync'
  | 'icon:profile'
  | 'inline:attendance'
  | 'inline:marks'
  | 'inline:students'
  | 'inline:requests'

export type TeacherTileId =
  | 'attendance'
  | 'marks'
  | 'reportCard'
  | 'myClasses'
  | 'students'
  | 'requests'
  | 'inbox'
  | 'cloudSync'
  | 'profile'

/** One tile of the mock's 3×3 grid (order + stagger delay are the mock's). */
export interface TeacherTileDef {
  id: TeacherTileId
  labelKey: string
  icon: TeacherTileIcon
  /** vRise14 480ms {delay} — the tile's exact stagger delay from the mock. */
  delay: string
  /** The commands the tile's screen needs; the tile is hidden if any is missing. */
  commands: string[]
}

export const TEACHER_TILES: TeacherTileDef[] = [
  { id: 'attendance', labelKey: 'teacher.tile.attendance', icon: 'inline:attendance', delay: '140ms', commands: ['dashboard_teacher', 'get_attendance_sheet'] },
  { id: 'marks', labelKey: 'teacher.tile.marks', icon: 'inline:marks', delay: '180ms', commands: ['dashboard_teacher', 'list_exams', 'get_marks_sheet'] },
  { id: 'reportCard', labelKey: 'teacher.tile.reportCard', icon: 'icon:reportCard', delay: '220ms', commands: ['dashboard_teacher', 'list_exams', 'get_report_remarks'] },
  { id: 'myClasses', labelKey: 'teacher.tile.myClasses', icon: 'icon:myClasses', delay: '260ms', commands: ['my_timetable'] },
  { id: 'students', labelKey: 'teacher.tile.students', icon: 'inline:students', delay: '300ms', commands: ['list_students'] },
  { id: 'requests', labelKey: 'teacher.tile.requests', icon: 'inline:requests', delay: '340ms', commands: ['list_requests'] },
  { id: 'inbox', labelKey: 'teacher.tile.inbox', icon: 'icon:inbox', delay: '380ms', commands: ['list_circulars'] },
  { id: 'cloudSync', labelKey: 'teacher.tile.cloudSync', icon: 'icon:cloudSync', delay: '420ms', commands: ['sync_status'] },
  { id: 'profile', labelKey: 'teacher.tile.profile', icon: 'icon:profile', delay: '460ms', commands: ['lock'] },
]

type T = (key: string, vars?: Record<string, string | number>) => string

/** One row of the task card. */
export interface HomeTask {
  key: string
  title: string
  sub: string
  /** Colour of the 10px leading dot: gold = attendance due, teal = a request. */
  dot: string
  pulse: boolean
  to: string
  /** Trailing element: the gold Start/Continue pill, or a chevron. */
  action: { label: string } | null
  /** Counts toward "N things are due" (attendance to take, a returned request). */
  due: boolean
}

export interface HomeTile extends TeacherTileDef {
  to: string
  /** A real count (> 0) or none. */
  badge?: string
}

export interface TeacherHomeView {
  eyebrow: string
  greeting: string
  dueLine: string
  tasks: HomeTask[]
  tiles: HomeTile[]
  initials: string
  /** Unread circulars (the bell's dot); 0 → no dot. */
  unread: number
  footer: { text: string; tone: 'ok' | 'pending' } | null
}

const GOLD = '#C5AB7A'
const TEAL = '#7DB1B5'

export function firstName(full: string): string {
  return full.trim().split(/\s+/)[0] || full
}

/** "MI" for "Meena Iyer"; one letter for a one-word name. */
export function initials(full: string): string {
  const parts = full.trim().split(/\s+/).filter(Boolean)
  if (parts.length === 0) return '·'
  const first = [...parts[0]][0] ?? ''
  const last = parts.length > 1 ? [...parts[parts.length - 1]][0] ?? '' : ''
  return (first + last).toUpperCase()
}

/** Localised "… ago" for the sync footer. */
export function agoText(from: Date, now: Date, t: T): string {
  const s = Math.max(0, Math.floor((now.getTime() - from.getTime()) / 1000))
  if (s < 60) return t('teacher.ago.now')
  const m = Math.floor(s / 60)
  if (m < 60) return t('teacher.ago.min', { n: m })
  const h = Math.floor(m / 60)
  if (h < 24) return t('teacher.ago.hour', { n: h })
  return t('teacher.ago.day', { n: Math.floor(h / 24) })
}

/** Decided requests stay on the home for this long after they were raised. */
export const DECIDED_WINDOW_DAYS = 7

const REQ_KINDS = new Set([
  'leave', 'attendance_correction', 'marks_correction', 'student_details', 'attendance_duty',
  'class_notice', 'payment_reversal', 'access_change', 'device_replacement',
])
export function requestKindLabel(kind: string, t: T): string {
  return t(REQ_KINDS.has(kind) ? `teacher.reqkind.${kind}` : 'teacher.reqkind.other')
}

/** Today's tasks: one row per class still to submit (own, then duty classes), then a
 *  row per request of mine the Principal returned, then recently decided ones. */
export function buildTasks(d: TeacherDashboard | null, requests: RequestDto[], myId: string, now: Date, t: T): HomeTask[] {
  const tasks: HomeTask[] = []
  for (const c of d?.classes ?? []) {
    if (c.status === 'submitted') continue
    const statusLine = t(c.status === 'draft' ? 'teacher.task.draft' : 'teacher.task.notSubmitted', { n: c.student_count })
    tasks.push({
      key: `att-${c.class_id}`,
      title: t('teacher.task.attendance', { class: c.display }),
      sub: c.duty ? t('teacher.task.covering', { status: statusLine }) : statusLine,
      dot: GOLD,
      pulse: true,
      to: `/teacher/attendance/${c.class_id}`,
      action: { label: t(c.status === 'draft' ? 'teacher.continue' : 'teacher.start') },
      due: true,
    })
  }
  const mine = requests.filter((r) => r.requested_by === myId)
  for (const r of mine) {
    if (r.status !== 'returned') continue
    tasks.push({ key: `req-${r.id}`, title: t('teacher.task.reply'), sub: r.note?.trim() || t('teacher.task.replySub', { kind: requestKindLabel(r.kind, t) }), dot: TEAL, pulse: false, to: '/teacher/requests', action: null, due: true })
  }
  const cutoff = now.getTime() - DECIDED_WINDOW_DAYS * 86_400_000
  for (const r of mine) {
    if (r.status !== 'approved' && r.status !== 'rejected') continue
    const at = Date.parse(r.created_at)
    if (!Number.isFinite(at) || at < cutoff) continue
    tasks.push({ key: `req-${r.id}`, title: t(r.status === 'approved' ? 'teacher.task.approved' : 'teacher.task.rejected'), sub: requestKindLabel(r.kind, t), dot: TEAL, pulse: false, to: '/teacher/requests', action: null, due: false })
  }
  return tasks
}

/** The headline under the greeting, from the real number of things due. */
export function dueLine(tasks: HomeTask[], t: T): string {
  const n = tasks.filter((x) => x.due).length
  if (n === 0) return t('teacher.due.none')
  if (n === 1) return t('teacher.due.one')
  return t('teacher.due.many', { n })
}

/** Where the Attendance tile goes: straight to the one class still due (or the only
 *  class), else the "Attendance today" list. */
export function attendanceTarget(d: TeacherDashboard | null): string {
  const classes = d?.classes ?? []
  const due = classes.filter((c) => c.status !== 'submitted')
  if (due.length === 1) return `/teacher/attendance/${due[0].class_id}`
  if (classes.length === 1) return `/teacher/attendance/${classes[0].class_id}`
  return '/teacher/attendance'
}

const TILE_PATH: Record<Exclude<TeacherTileId, 'attendance'>, string> = {
  marks: '/teacher/marks',
  reportCard: '/teacher/remarks',
  myClasses: '/teacher/timetable',
  students: '/teacher/students',
  requests: '/teacher/requests',
  inbox: '/teacher/inbox',
  cloudSync: '/teacher/sync',
  profile: '/teacher/profile',
}

/** Tiles for this platform (hidden when a needed command is missing) with real
 *  targets and badges: My requests = returned requests waiting for me; Inbox =
 *  unread circulars + changes the school computer did not accept. */
export function buildTiles(p: {
  available: (cmd: string) => boolean
  dashboard: TeacherDashboard | null
  returned: number
  unread: number
  problems: number
}): HomeTile[] {
  const badgeOf = (n: number) => (n > 0 ? String(n) : undefined)
  return TEACHER_TILES.filter((tile) => tile.commands.every(p.available)).map((tile) => {
    const to = tile.id === 'attendance' ? attendanceTarget(p.dashboard) : TILE_PATH[tile.id]
    const badge = tile.id === 'requests' ? badgeOf(p.returned) : tile.id === 'inbox' ? badgeOf(p.unread + p.problems) : undefined
    return { ...tile, to, badge }
  })
}

/** The sync footer from the real sync state: pending changes, else "synced … ago",
 *  else nothing (never synced / the school PC itself / unavailable). */
export function buildFooter(s: SyncStatusDto | null, now: Date, t: T): TeacherHomeView['footer'] {
  if (!s) return null
  if (s.pending_count > 0) return { text: t('teacher.footer.pending', { n: s.pending_count }), tone: 'pending' }
  if (s.last_confirmed_at) {
    const at = new Date(s.last_confirmed_at)
    if (!Number.isNaN(at.getTime())) return { text: t('teacher.footer.synced', { ago: agoText(at, now, t) }), tone: 'ok' }
  }
  return null
}

export function unreadCirculars(cs: CircularDto[] | null): number {
  return (cs ?? []).filter((c) => !c.read_by_me).length
}

/** Assemble the whole home view (pure). */
export function buildTeacherHome(p: {
  staffName: string
  staffId: string
  lang: string
  now: Date
  dashboard: TeacherDashboard | null
  requests: RequestDto[] | null
  circulars: CircularDto[] | null
  problems: number
  sync: SyncStatusDto | null
  available: (cmd: string) => boolean
  t: T
}): TeacherHomeView {
  const requests = p.requests ?? []
  const tasks = buildTasks(p.dashboard, requests, p.staffId, p.now, p.t)
  const unread = unreadCirculars(p.circulars)
  return {
    eyebrow: formatDateLongIn(p.now, p.lang),
    greeting: p.t(`teacher.greeting.${greeting(p.now)}`, { name: firstName(p.staffName) }),
    dueLine: dueLine(tasks, p.t),
    tasks,
    tiles: buildTiles({
      available: p.available,
      dashboard: p.dashboard,
      returned: requests.filter((r) => r.requested_by === p.staffId && r.status === 'returned').length,
      unread,
      problems: p.problems,
    }),
    initials: initials(p.staffName),
    unread,
    footer: buildFooter(p.sync, p.now, p.t),
  }
}
