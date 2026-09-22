// DEV-only gallery (#/__gallery). With no query it lists all five mock screens
// at mock size; with `?screen=<key>&state=<state>` it renders exactly one screen
// full-bleed for the Playwright fidelity test (docs/01-MOCK-SPEC.md §9).
import WelcomeScreen from '@/screens/shared/WelcomeScreen'
import PrincipalHomeScreen from '@/screens/desktop/PrincipalHomeScreen'
import CollectFeeScreen from '@/screens/desktop/CollectFeeScreen'
import TeacherHomeScreen from '@/screens/phone/TeacherHomeScreen'
import AttendanceScreen from '@/screens/phone/AttendanceScreen'
import { welcomeFixture } from '@/dev/fixtures/welcome'
import { principalHomeFixture } from '@/dev/fixtures/principalHome'
import { collectFeeFixture } from '@/dev/fixtures/collectFee'
import { teacherHomeFixture } from '@/dev/fixtures/teacherHome'
import { attendanceFixture } from '@/dev/fixtures/attendance'
import { useRoute } from '@/lib/router'

type WelcomeMode = 'setup' | 'join' | 'recover'
type AttMode = 'default' | 'markall' | 'submitted'

function renderScreen(screen: string, state: string | null) {
  switch (screen) {
    case 'welcome': {
      const m: WelcomeMode = state === 'join' || state === 'recover' ? state : 'setup'
      return <WelcomeScreen data={welcomeFixture} initial={m} />
    }
    case 'principalHome':
      return <PrincipalHomeScreen data={principalHomeFixture} />
    case 'collectFee': {
      const initial =
        state === 'over'
          ? { amount: '5000' }
          : state === 'cash'
            ? { mode: 'cash' as const }
            : state === 'success'
              ? { done: true }
              : undefined
      return <CollectFeeScreen data={collectFeeFixture} initial={initial} />
    }
    case 'teacherHome':
      return <TeacherHomeScreen data={teacherHomeFixture} />
    case 'attendance': {
      const m: AttMode = state === 'markall' || state === 'submitted' ? state : 'default'
      return <AttendanceScreen data={attendanceFixture} initial={m} />
    }
    default:
      return null
  }
}

// Every screen/state used by the gallery index and the fidelity test.
const ENTRIES: { key: string; label: string; state?: string; w: number; h: number }[] = [
  { key: 'welcome', label: 'Welcome · setup', state: 'setup', w: 1440, h: 960 },
  { key: 'welcome', label: 'Welcome · join', state: 'join', w: 1440, h: 960 },
  { key: 'welcome', label: 'Welcome · recover', state: 'recover', w: 1440, h: 960 },
  { key: 'principalHome', label: 'Principal Home', w: 1440, h: 1080 },
  { key: 'collectFee', label: 'Collect fee · default', state: 'default', w: 1440, h: 1080 },
  { key: 'collectFee', label: 'Collect fee · amount 5000', state: 'over', w: 1440, h: 1080 },
  { key: 'collectFee', label: 'Collect fee · cash', state: 'cash', w: 1440, h: 1080 },
  { key: 'collectFee', label: 'Collect fee · success', state: 'success', w: 1440, h: 1080 },
  { key: 'teacherHome', label: 'Teacher Home', w: 390, h: 844 },
  { key: 'attendance', label: 'Attendance · default', state: 'default', w: 390, h: 844 },
  { key: 'attendance', label: 'Attendance · mark all', state: 'markall', w: 390, h: 844 },
  { key: 'attendance', label: 'Attendance · submitted', state: 'submitted', w: 390, h: 844 },
]

export default function Gallery() {
  const path = useRoute()
  const query = path.includes('?') ? path.slice(path.indexOf('?') + 1) : ''
  const params = new URLSearchParams(query)
  const screen = params.get('screen')
  const state = params.get('state')

  // Single-screen mode (fidelity test): render just the screen, no chrome.
  if (screen) {
    return <>{renderScreen(screen, state)}</>
  }

  // Index: all five screens (every state) at mock size.
  return (
    <div
      style={{
        background: '#2a2f34',
        minHeight: '100vh',
        padding: 24,
        display: 'flex',
        flexDirection: 'column',
        gap: 24,
        fontFamily: "'Geist', system-ui, sans-serif",
      }}
    >
      {ENTRIES.map((e) => (
        <div key={e.label} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <a
            href={`#/__gallery?screen=${e.key}${e.state ? `&state=${e.state}` : ''}`}
            style={{ color: '#fff', fontSize: 13, textDecoration: 'none' }}
          >
            {e.label} — {e.w}×{e.h}
          </a>
          <div style={{ width: e.w, height: e.h, overflow: 'hidden', boxShadow: '0 8px 30px rgba(0,0,0,0.4)' }}>
            {renderScreen(e.key, e.state ?? null)}
          </div>
        </div>
      ))}
    </div>
  )
}
