import { useRoute, matchRoute } from '@/lib/router'
import WelcomeScreen from '@/screens/shared/WelcomeScreen'
import PrincipalHomeScreen from '@/screens/desktop/PrincipalHomeScreen'
import CollectFeeScreen from '@/screens/desktop/CollectFeeScreen'
import TeacherHomeScreen from '@/screens/phone/TeacherHomeScreen'
import AttendanceScreen from '@/screens/phone/AttendanceScreen'
import Placeholder from '@/components/Placeholder'
import Gallery from '@/dev/Gallery'
import Mocks from '@/dev/Mocks'
import { welcomeFixture } from '@/dev/fixtures/welcome'
import { principalHomeFixture } from '@/dev/fixtures/principalHome'
import { collectFeeFixture } from '@/dev/fixtures/collectFee'
import { teacherHomeFixture } from '@/dev/fixtures/teacherHome'
import { attendanceFixture } from '@/dev/fixtures/attendance'

export default function App() {
  const path = useRoute()
  const base = path.split('?')[0]

  // DEV-only routes (compiled out of release builds).
  if (import.meta.env.DEV) {
    if (base === '/__gallery') return <Gallery />
    if (base === '/__mocks') return <Mocks />
  }

  switch (base) {
    case '/principal/home':
      return <PrincipalHomeScreen data={principalHomeFixture} />
    case '/accountant/collect':
      return <CollectFeeScreen data={collectFeeFixture} />
    case '/teacher/home':
      return <TeacherHomeScreen data={teacherHomeFixture} />
    case '/welcome':
      return <WelcomeScreen data={welcomeFixture} />
  }

  if (matchRoute('/teacher/attendance/:classId', base)) {
    return <AttendanceScreen data={attendanceFixture} />
  }

  // Every other sidebar link / tile → Placeholder (Phase 1).
  return <Placeholder />
}
