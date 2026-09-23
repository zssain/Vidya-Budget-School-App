// Wires the Attendance screen to real data (prompts/P03 Step 8). Loads the sheet
// for a class + date, renders the pixel-exact AttendanceScreen with the real
// roster + existing marks, and submits / saves drafts through the real commands.
// Fixture/gallery path is unchanged (no marks/onSubmit props) → no pixels move.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError } from '@/lib/api'
import AttendanceScreen from '@/screens/phone/AttendanceScreen'
import type { AttendanceData, Mark } from '@/dev/fixtures/attendance'
import { t } from '@/lib/i18n'

function today(): string {
  return new Date().toISOString().slice(0, 10)
}

export default function AttendanceContainer({ classId }: { classId: string }) {
  const [data, setData] = useState<AttendanceData | null>(null)
  const [marks, setMarks] = useState<Mark[]>([])
  const [ids, setIds] = useState<string[]>([])
  const [error, setError] = useState<CmdError | null>(null)
  const date = today()

  useEffect(() => {
    api
      .get_attendance_sheet(classId, date)
      .then((sheet) => {
        setData({ className: sheet.class_display, date, names: sheet.rows.map((r) => r.name) })
        setMarks(sheet.rows.map((r) => (r.mark ?? '') as Mark))
        setIds(sheet.rows.map((r) => r.student_id))
      })
      .catch((e) => setError(e as CmdError))
  }, [classId, date])

  if (error) {
    return <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--danger)', fontSize: 14 }}>{t(error.message_key, error.vars as Record<string, string | number>)}</div>
  }
  if (!data) {
    return <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--muted)', fontSize: 14 }}>…</div>
  }

  const toInput = (ms: Mark[]) =>
    ms.map((m, i) => ({ student_id: ids[i], mark: m })).filter((x) => x.mark !== '')

  return (
    <AttendanceScreen
      data={data}
      marks={marks}
      onSaveDraft={(ms) => {
        void api.save_attendance_draft(classId, date, toInput(ms)).catch((e) => setError(e as CmdError))
      }}
      onSubmit={(ms) => {
        void api.submit_attendance(classId, date, toInput(ms)).catch((e) => setError(e as CmdError))
      }}
    />
  )
}
