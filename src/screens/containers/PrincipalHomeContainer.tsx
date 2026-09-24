// Wires Principal Home to real data (prompts/P03 Step 8). Fetches
// dashboard_principal and maps it to the pixel-exact PrincipalHomeScreen's props
// (the same component the fidelity gallery renders with fixtures, so no pixels
// change). The narrative fields (greeting, date, subtitle, needs, backup) are
// REAL here — the greeting uses the logged-in staff (from app_state), the date is
// today's, the subtitle/needs/backup come from the dashboard — never fixture copy.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, PrincipalDashboard } from '@/lib/api'
import PrincipalHomeScreen from '@/screens/desktop/PrincipalHomeScreen'
import type { HomeApproval, HomeNeed, PrincipalHomeData } from '@/dev/fixtures/principalHome'
import { formatDateLong, formatMoney, formatRelative, greeting } from '@/lib/format'
import { t } from '@/lib/i18n'
import { useStore } from '@/lib/store'
import { navigate } from '@/lib/router'

const NAV_DEST: Record<string, string> = {
  admission: '/principal/students/new',
  approvals: '/principal/approvals',
  attendance: '/principal/attendance',
  fees: '/principal/fees',
  students: '/principal/students',
}

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

function firstName(full: string): string {
  const n = full.trim().split(/\s+/)[0]
  return n || full
}

/** Subtitle from the real pending-approval count (honest, not "Four…"). */
function subtitle(total: number): string {
  if (total <= 0) return t('home.subtitle.none')
  if (total === 1) return t('home.subtitle.one')
  return t('home.subtitle.many', { n: total })
}

/** "Needs attention" items from real data: unsubmitted classes + open conflicts. */
function buildNeeds(d: PrincipalDashboard): HomeNeed[] {
  const needs: HomeNeed[] = []
  for (const cls of d.attendance_pending) {
    needs.push({
      tone: 'warn',
      title: t('home.needs.unsubmitted', { class: cls.class }),
      sub: cls.teacher
        ? t('home.needs.unsubmittedSubTeacher', { name: cls.teacher })
        : t('home.needs.unsubmittedSub'),
    })
  }
  if (d.open_conflicts > 0) {
    needs.push({
      tone: 'danger',
      title:
        d.open_conflicts === 1
          ? t('home.needs.conflictOne')
          : t('home.needs.conflictMany', { n: d.open_conflicts }),
      sub: t('home.needs.conflictSub'),
    })
  }
  return needs.slice(0, 4)
}

/** "Last backup" line from the real backup run, or null → "No backup yet." */
function backupLine(d: PrincipalDashboard): string | null {
  if (!d.last_backup) return null
  const line = t('home.backupLine', { when: formatRelative(new Date(d.last_backup.at)) })
  return d.last_backup.destination ? `${line} · ${d.last_backup.destination}` : line
}

function mapDashboard(d: PrincipalDashboard, staffName: string): PrincipalHomeData {
  const now = new Date()
  const approvals: HomeApproval[] = d.approvals.map((a) => ({
    type: TYPE_LABEL[a.kind] ?? a.kind,
    badge: BADGE[a.kind] ?? { bg: 'var(--panel)', fg: 'var(--muted)' },
    what: a.what,
    who: a.who,
    age: formatRelative(new Date(a.created_at)),
  }))
  const pct = `${(d.attendance_pct_tenths / 10).toFixed(1)}%`
  const pending = d.attendance_pending.map((p) => p.class).join(', ')
  return {
    eyebrow: d.term_label ? `${formatDateLong(now)} · ${d.term_label}` : formatDateLong(now),
    greeting: t(`home.greeting.${greeting(now)}`, { name: firstName(staffName) }),
    subtitle: subtitle(d.approvals_total),
    stats: [
      {
        value: pct,
        label: 'attendance today',
        note:
          d.attendance_total === 0
            ? 'no students yet'
            : `${d.attendance_marked} of ${d.attendance_total} marked${pending ? ` · ${pending} pending` : ''}`,
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
    needs: buildNeeds(d),
    lastBackup: backupLine(d),
    classes: d.classes.map((c) => ({ name: c.name, pct: c.pct })),
    days: d.fee_days.map((x) => ({ day: x.day, value: Math.round(x.value_paise / 100) })),
    feeTotal: formatMoney(d.fee_total_paise),
  }
}

export default function PrincipalHomeContainer() {
  const store = useStore()
  const staffName = store.app?.state.kind === 'unlocked' ? store.app.state.staff.name : ''
  const [data, setData] = useState<PrincipalHomeData | null>(null)
  const [error, setError] = useState<CmdError | null>(null)

  useEffect(() => {
    api
      .dashboard_principal()
      .then((d) => setData(mapDashboard(d, staffName)))
      .catch((e) => setError(e as CmdError))
  }, [staffName])

  if (error) {
    return (
      <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--danger)', fontSize: 14 }}>
        {t(error.message_key, error.vars as Record<string, string | number>)}
      </div>
    )
  }
  if (!data) {
    return (
      <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--muted)', fontSize: 14 }}>
        …
      </div>
    )
  }
  return <PrincipalHomeScreen data={data} chrome={false} onNav={(dest) => navigate(NAV_DEST[dest])} />
}
