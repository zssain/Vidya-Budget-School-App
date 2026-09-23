import { useEffect, useState } from 'react'
import { useRoute, matchRoute } from '@/lib/router'
import * as api from '@/lib/api'
import type { CmdError, StaffDto } from '@/lib/api'
import { useStore, refreshAppState, routeForState } from '@/lib/store'
import { t } from '@/lib/i18n'
import WelcomeScreen from '@/screens/shared/WelcomeScreen'
import SetupWizard from '@/screens/shared/SetupWizard'
import PinUnlockScreen from '@/screens/shared/PinUnlockScreen'
import ApprovalsScreen from '@/screens/desktop/ApprovalsScreen'
import PrincipalHomeContainer from '@/screens/containers/PrincipalHomeContainer'
import CollectFeeScreen from '@/screens/desktop/CollectFeeScreen'
import TeacherHomeScreen from '@/screens/phone/TeacherHomeScreen'
import AttendanceScreen from '@/screens/phone/AttendanceScreen'
import Placeholder from '@/components/Placeholder'
import Gallery from '@/dev/Gallery'
import Mocks from '@/dev/Mocks'
import { welcomeFixture } from '@/dev/fixtures/welcome'
import { collectFeeFixture } from '@/dev/fixtures/collectFee'
import { teacherHomeFixture } from '@/dev/fixtures/teacherHome'
import { attendanceFixture } from '@/dev/fixtures/attendance'

// A simple full-screen status message (db_key_missing / needs_rejoin / moved).
function StatusScreen({ messageKey }: { messageKey: string }) {
  return (
    <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', padding: 24 }}>
      <p style={{ maxWidth: 420, textAlign: 'center', color: 'var(--ink)', fontSize: 15, lineHeight: 1.5 }}>{t(messageKey)}</p>
    </div>
  )
}

// Fetches the active staff list, then shows the PIN unlock screen.
function PinGate() {
  const [staff, setStaff] = useState<StaffDto[] | null>(null)
  useEffect(() => {
    api.list_staff().then(setStaff).catch(() => setStaff([]))
  }, [])
  if (!staff) return <StatusScreen messageKey="pin.enter" />
  return <PinUnlockScreen staff={staff} schoolName={t('app.name')} />
}

export default function App() {
  const path = useRoute()
  const base = path.split('?')[0]
  const store = useStore()
  const [activateErr, setActivateErr] = useState<CmdError | null>(null)

  const isDevRoute = base === '/__gallery' || base === '/__mocks'

  // Load app_state once (except on the DEV fixture routes used by fidelity).
  useEffect(() => {
    if (!isDevRoute) void refreshAppState()
  }, [isDevRoute])

  // DEV-only fixture routes (compiled out of release; used by the fidelity test).
  if (import.meta.env.DEV) {
    if (base === '/__gallery') return <Gallery />
    if (base === '/__mocks') return <Mocks />
  }

  // Before app_state resolves (or in a plain browser without Tauri).
  if (store.app === null) {
    if (store.error) {
      return <StatusScreen messageKey={store.error.message_key} />
    }
    return <StatusScreen messageKey="app.name" />
  }

  const route = routeForState(store.app.state)

  const handleActivate = (code: string) => {
    setActivateErr(null)
    api
      .activate_licence(code, '')
      .then(() => refreshAppState())
      .catch((e) => setActivateErr(e as CmdError))
  }

  switch (route.screen) {
    case 'welcome':
      return (
        <>
          {activateErr && (
            <div style={{ position: 'fixed', top: 0, left: 0, right: 0, zIndex: 10, background: 'var(--pill-unpaid-bg)', color: 'var(--pill-unpaid-fg)', textAlign: 'center', padding: '10px 16px', fontSize: 13 }}>
              {t(activateErr.message_key, activateErr.vars as Record<string, string | number>)}
            </div>
          )}
          <WelcomeScreen data={welcomeFixture} onActivate={handleActivate} />
        </>
      )
    case 'setup':
      return <SetupWizard startStep={route.step} />
    case 'pin':
      return <PinGate />
    case 'db_key_missing':
      return <StatusScreen messageKey="error.DB_KEY_MISSING" />
    case 'needs_rejoin':
      return <StatusScreen messageKey="error.EPOCH_OLD" />
    case 'moved':
      return <StatusScreen messageKey="licence.banner_moved" />
    case 'home':
      // Unlocked: route within the app by hash path. Wired containers use real
      // data; screens not yet wired fall back to fixtures (flagged in handoff).
      if (matchRoute('/principal/approvals', base)) return <ApprovalsScreen />
      if (base === '/accountant/collect') return <CollectFeeScreen data={collectFeeFixture} />
      if (base === '/teacher/home') return <TeacherHomeScreen data={teacherHomeFixture} />
      if (matchRoute('/teacher/attendance/:classId', base)) return <AttendanceScreen data={attendanceFixture} />
      if (base === '/principal/home') return <PrincipalHomeContainer />
      // Default landing by role.
      if (route.role === 'principal') return <PrincipalHomeContainer />
      if (route.role === 'accountant') return <CollectFeeScreen data={collectFeeFixture} />
      if (route.role === 'teacher') return <TeacherHomeScreen data={teacherHomeFixture} />
      return <Placeholder />
  }
}
