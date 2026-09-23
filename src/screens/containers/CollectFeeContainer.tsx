// Wires Collect fee to real data (prompts/P03 Step 8). Lists students + their
// dues into the pixel-exact CollectFeeScreen, and records payments through the
// real `record_payment` command (returns the real receipt number). Print / Share
// open a "coming in a later phase" notice (Phase 7). The fixture/gallery path is
// unchanged, so no pixels move.

import { useEffect, useMemo, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, StudentDto } from '@/lib/api'
import CollectFeeScreen from '@/screens/desktop/CollectFeeScreen'
import type { CollectFeeData, CollectFeeStudentRow } from '@/dev/fixtures/collectFee'
import { formatMoney } from '@/lib/format'
import { t } from '@/lib/i18n'

type Mode = 'cash' | 'upi' | 'cheque'

function pillFor(balance: number, amount: number): CollectFeeStudentRow['pill'] {
  if (balance <= 0) return 'paid'
  if (balance < amount) return 'partpaid'
  return 'unpaid'
}
function statusFor(pill: CollectFeeStudentRow['pill']): string {
  return pill === 'paid' ? 'Paid' : pill === 'partpaid' ? 'Part paid' : 'Unpaid'
}

export default function CollectFeeContainer() {
  const [students, setStudents] = useState<StudentDto[] | null>(null)
  const [rows, setRows] = useState<CollectFeeStudentRow[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [data, setData] = useState<CollectFeeData | null>(null)
  const [error, setError] = useState<CmdError | null>(null)
  const [notice, setNotice] = useState(false)

  // Initial list (first 25 active students) + their dues → the search table.
  useEffect(() => {
    api
      .list_students()
      .then((all) => {
        const slice = all.slice(0, 25)
        setStudents(slice)
        return Promise.all(
          slice.map(async (s) => {
            const dues = await api.list_fee_dues(s.id)
            const total = dues.lines.reduce((n, l) => n + Math.max(l.balance_paise, 0), 0)
            const amount = dues.lines.reduce((n, l) => n + l.amount_paise, 0)
            const pill = pillFor(total, amount)
            const row: CollectFeeStudentRow = {
              id: s.id,
              name: s.name,
              adm: s.admission_no ?? s.provisional_no ?? '—',
              cls: s.class_display ?? '—',
              due: formatMoney(total),
              status: statusFor(pill),
              pill,
            }
            return row
          }),
        )
      })
      .then((r) => {
        if (r) {
          setRows(r)
          const firstDue = r.find((x) => x.pill !== 'paid') ?? r[0]
          if (firstDue) setSelectedId(firstDue.id ?? null)
        }
      })
      .catch((e) => setError(e as CmdError))
  }, [])

  // Load the selected student's dues into the sheet.
  useEffect(() => {
    if (!selectedId || !students) return
    const s = students.find((x) => x.id === selectedId)
    if (!s) return
    api
      .list_fee_dues(selectedId)
      .then((dues) => {
        const initials = s.name.split(' ').map((w) => w[0]).slice(0, 2).join('').toUpperCase()
        setData({
          rows,
          student: {
            name: s.name,
            meta: `${s.class_display ?? ''} · Adm. ${s.admission_no ?? s.provisional_no ?? '—'}`,
            initials,
          },
          lines: dues.lines.map((l) => ({
            label: l.label,
            amount: formatMoney(l.amount_paise),
            right: l.balance_paise > 0 ? `${formatMoney(l.balance_paise)} due` : 'Paid',
            paid: l.balance_paise <= 0,
          })),
          totalDue: formatMoney(dues.total_due_paise),
          due: Math.round(dues.total_due_paise / 100),
        })
      })
      .catch((e) => setError(e as CmdError))
  }, [selectedId, students, rows])

  const onRecord = useMemo(
    () => async (amountPaise: number, mode: Mode, reference: string) => {
      if (!selectedId) throw new Error('no student')
      const p = await api.record_payment({ student_id: selectedId, amount_paise: amountPaise, mode, reference })
      return p.receipt_no
    },
    [selectedId],
  )

  if (error) {
    return <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--danger)', fontSize: 14 }}>{t(error.message_key, error.vars as Record<string, string | number>)}</div>
  }
  if (!data) {
    return <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color: 'var(--muted)', fontSize: 14 }}>…</div>
  }
  return (
    <>
      {notice && (
        <div style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 20, background: 'var(--navy)', color: '#fff', borderRadius: 10, padding: '12px 18px', fontSize: 14 }} role="status">
          {t('fee.comms.notice')}
        </div>
      )}
      <CollectFeeScreen
        data={data}
        chrome={false}
        onRecord={onRecord}
        onSelectStudent={(id) => setSelectedId(id)}
        onCommsNotice={() => {
          setNotice(true)
          window.setTimeout(() => setNotice(false), 2200)
        }}
      />
    </>
  )
}
