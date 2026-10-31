// Teacher Home sample data for the DEV-only fidelity gallery (/__gallery). It
// reproduces design/screens/TeacherHome.dc.html's static content (Meena, V-A,
// "synced 2 min ago") as a TeacherHomeView. Shipped code never uses it: the real
// home is built from live data by TeacherHomeContainer (repair T1). The tile
// layout itself lives in src/lib/teacherHome.ts (TEACHER_TILES).
import { TEACHER_TILES, type HomeTile, type TeacherHomeView } from '@/lib/teacherHome'

const tiles: HomeTile[] = TEACHER_TILES.map((tile) => ({
  ...tile,
  to: tile.id === 'attendance' ? '/teacher/attendance' : '/teacher/home',
  badge: tile.id === 'requests' ? '1' : undefined,
}))

export const teacherHomeFixture: TeacherHomeView = {
  eyebrow: 'Wednesday, 23 September',
  greeting: 'Good morning, Meena.',
  dueLine: 'Two things are due.',
  tasks: [
    { key: 'att', title: 'V-A attendance', sub: 'Not submitted · 34 students', dot: '#C5AB7A', pulse: true, to: '/teacher/attendance', action: { label: 'Start' }, due: true },
    { key: 'req', title: 'Reply to the Principal', sub: 'Rahul Kumar’s attendance correction was returned', dot: '#7DB1B5', pulse: false, to: '/teacher/home', action: null, due: true },
  ],
  tiles,
  initials: 'MI',
  unread: 2,
  footer: { text: 'Up to date · synced 2 min ago', tone: 'ok' },
}

export default teacherHomeFixture
