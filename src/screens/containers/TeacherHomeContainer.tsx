// Wires the Teacher Home to real data (repair T1 — it used to render the static
// mock fixture). Loads dashboard_teacher (today's classes), the teacher's requests,
// circulars, refused changes and the sync state, then renders TeacherHomeScreen with
// the view built by lib/teacherHome.ts. Re-reads every 30 s (and when the app comes
// back to the foreground) so the sync footer and "due" rows stay honest. A command
// this platform lacks is skipped and its tile hidden (isAvailableOnWeb on the PWA).

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CircularDto, RequestDto, SyncStatusDto, TeacherDashboard } from '@/lib/api'
import { t, useLang } from '@/lib/i18n'
import { useStore } from '@/lib/store'
import { isWeb } from '@/lib/platform'
import { isAvailableOnWeb } from '@/lib/web/backend'
import { buildTeacherHome } from '@/lib/teacherHome'
import TeacherHomeScreen from '@/screens/phone/TeacherHomeScreen'

const REFRESH_MS = 30_000

/** Native: every command exists. PWA: only what the web backend serves. */
export const commandAvailable = (cmd: string): boolean => !isWeb || isAvailableOnWeb(cmd)

function attempt<T>(cmd: string, run: () => Promise<T>): Promise<T | null> {
  if (!commandAvailable(cmd)) return Promise.resolve(null)
  return run().catch(() => null)
}

interface Loaded {
  dashboard: TeacherDashboard | null
  requests: RequestDto[] | null
  circulars: CircularDto[] | null
  problems: number
  sync: SyncStatusDto | null
}

export default function TeacherHomeContainer() {
  const lang = useLang()
  const store = useStore()
  const staff = store.app?.state.kind === 'unlocked' ? store.app.state.staff : null
  const [data, setData] = useState<Loaded>({ dashboard: null, requests: null, circulars: null, problems: 0, sync: null })
  const [now, setNow] = useState(() => new Date())

  const load = useCallback(() => {
    void Promise.all([
      attempt('dashboard_teacher', api.dashboard_teacher),
      attempt('list_requests', () => api.list_requests()),
      attempt('list_circulars', api.list_circulars),
      attempt('list_sync_problems', api.list_sync_problems),
      attempt('sync_status', api.sync_status),
    ]).then(([dashboard, requests, circulars, problems, sync]) => {
      setData({ dashboard, requests, circulars, problems: problems?.length ?? 0, sync })
      setNow(new Date())
    })
  }, [])

  useEffect(() => {
    load()
    const id = window.setInterval(load, REFRESH_MS)
    const onVisible = () => {
      if (document.visibilityState === 'visible') load()
    }
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      window.clearInterval(id)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [load])

  const view = buildTeacherHome({
    staffName: staff?.name ?? '',
    staffId: staff?.id ?? '',
    lang,
    now,
    ...data,
    available: commandAvailable,
    t,
  })
  return <TeacherHomeScreen view={view} />
}
