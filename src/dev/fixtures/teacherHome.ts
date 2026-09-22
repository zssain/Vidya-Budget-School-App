// Teacher Home sample data — the screen is STATIC in the mock (renderVals()
// returns {}), so this fixture just enumerates the two task rows and the nine
// tiles in the exact order/animation-delays of design/screens/TeacherHome.dc.html
// (docs/01-MOCK-SPEC.md §8). Visible text is kept as i18n keys (see
// src/lib/i18n/strings/teacherHome.ts); the English behind each key matches the
// mock 1:1.

/** One entry in the 3×3 tile grid. */
export interface TeacherTile {
  /** i18n key for the tile label. */
  labelKey: string
  /** How this tile is drawn: an Icon name, or an inline two-tone SVG (see below). */
  icon: TeacherTileIcon
  /** Optional count badge (mock shows "1" on My requests). */
  badge?: string
  /** vRise14 480ms {delay} — the tile's exact stagger delay from the mock. */
  delay: string
  /** Navigation target (Attendance tile → the V-A attendance screen). */
  to: string
}

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

/** One row of the task card (10px dot + title/sub + trailing element). */
export interface TeacherTask {
  /** i18n key for the row title. */
  titleKey: string
  /** i18n key for the row sub-line. */
  subKey: string
  /** Colour of the 10px leading dot. */
  dot: string
  /** First row pulses (vPulse); second does not. */
  pulse: boolean
}

export interface TeacherHomeData {
  tasks: TeacherTask[]
  tiles: TeacherTile[]
}

export const teacherHomeFixture: TeacherHomeData = {
  tasks: [
    // V-A attendance — gold pulsing dot + gold "Start" pill → Attendance.
    { titleKey: 'teacher.task.attendance', subKey: 'teacher.task.attendanceSub', dot: '#C5AB7A', pulse: true },
    // Reply to the Principal — teal dot + chevron.
    { titleKey: 'teacher.task.reply', subKey: 'teacher.task.replySub', dot: '#7DB1B5', pulse: false },
  ],
  tiles: [
    { labelKey: 'teacher.tile.attendance', icon: 'inline:attendance', delay: '140ms', to: '/teacher/attendance/v-a' },
    { labelKey: 'teacher.tile.marks', icon: 'inline:marks', delay: '180ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.reportCard', icon: 'icon:reportCard', delay: '220ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.myClasses', icon: 'icon:myClasses', delay: '260ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.students', icon: 'inline:students', delay: '300ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.requests', icon: 'inline:requests', badge: '1', delay: '340ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.inbox', icon: 'icon:inbox', delay: '380ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.cloudSync', icon: 'icon:cloudSync', delay: '420ms', to: '/placeholder' },
    { labelKey: 'teacher.tile.profile', icon: 'icon:profile', delay: '460ms', to: '/placeholder' },
  ],
}

export default teacherHomeFixture
