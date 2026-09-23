// Wires Principal Home to real data (prompts/P03 Step 8). Fetches
// dashboard_principal and maps it to the pixel-exact PrincipalHomeScreen's props
// (the same component the fidelity test renders with fixtures via the gallery, so
// no pixels change). The demo seed is proven (Rust test) to produce the same
// numbers the fixtures encode, so the real screen renders identically.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, PrincipalDashboard } from '@/lib/api'
import PrincipalHomeScreen from '@/screens/desktop/PrincipalHomeScreen'
import type { HomeApproval, PrincipalHomeData } from '@/dev/fixtures/principalHome'
import { formatMoney, formatRelative } from '@/lib/format'
import { t } from '@/lib/i18n'

const BADGE: Record<string, { bg: string; fg: string; accent?: boolean }> = {
  marks_correction: { bg: '#E7E3F1', fg: '#4A3B78' },
  payment_reversal: { bg: '#F6E4E2', fg: '#8E2F2A' },
  attendance_correction: { bg: 'var(--accent-12)', fg: 'var(--accent)', accent: true },
  student_details: { bg: '#F4ECDC', fg: '#6B5220' },
}
const TYPE_LABEL: Record<string, string> = {
  marks_correction: 'Marks correction',
  payment_reversal: 'Payment reversal',
  attendance_correction: 'Attendance correction',
  student_details: 'Student details',
  access_change: 'Access',
}

function mapDashboard(d: PrincipalDashboard): PrincipalHomeData {
  const pct = `${(d.attendance_pct_tenths / 10).toFixed(1)}%`
  const pending = d.attendance_pending.join(', ')
  const approvals: HomeApproval[] = d.approvals.map((a) => ({
    type: TYPE_LABEL[a.kind] ?? a.kind,
    badge: BADGE[a.kind] ?? { bg: 'var(--panel)', fg: 'var(--muted)' },
    what: a.what,
    who: a.who,
    age: formatRelative(new Date(a.created_at)),
  }))
  return {
    stats: [
      {
        value: pct,
        label: 'attendance today',
        note: `${d.attendance_marked} of ${d.attendance_total} marked · ${pending} pending`,
      },
      {
        value: formatMoney(d.collected_today_paise),
        label: 'collected today',
        note: `${d.receipts_today} receipts · ${formatMoney(d.waiting_paise)} more waiting for server`,
      },
      {
        value: formatMoney(d.outstanding_paise),
        label: 'outstanding this term',
        note: `${d.students_with_dues} students with dues`,
      },
      {
        value: String(d.active_students),
        label: 'active students',
        note: `${d.admissions_this_week} new admissions this week`,
      },
    ],
    approvals,
    classes: d.classes.map((c) => ({ name: c.name, pct: c.pct })),
    days: d.fee_days.map((x) => ({ day: x.day, value: Math.round(x.value_paise / 100) })),
    feeTotal: formatMoney(d.fee_total_paise),
  }
}

export default function PrincipalHomeContainer() {
  const [data, setData] = useState<PrincipalHomeData | null>(null)
  const [error, setError] = useState<CmdError | null>(null)

  useEffect(() => {
    api
      .dashboard_principal()
      .then((d) => setData(mapDashboard(d)))
      .catch((e) => setError(e as CmdError))
  }, [])

  if (error) {
    return (
      <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--danger)', fontSize: 14 }}>
        {t(error.message_key, error.vars as Record<string, string | number>)}
      </div>
    )
  }
  if (!data) {
    // Loading skeleton (existing surfaces).
    return (
      <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--muted)', fontSize: 14 }}>
        …
      </div>
    )
  }
  return <PrincipalHomeScreen data={data} />
}
