import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { AuditPageDto, ExamDto, ExamResultRowDto, FeeCollectionDto, FeeOverviewRow, MonthCountDto } from '@/lib/api'

// Reports (prompts/P07 §12): role-based, printable. Audit-log viewer (filters +
// pagination + chain status), admissions by month, fee collection by range,
// dues/defaulters, exam results (subject averages + grade bars). Monthly
// attendance links to the register's month view. Printed via the browser (the
// AppShell chrome is hidden by print CSS in app.css).

type Tab = 'audit' | 'admissions' | 'fees' | 'dues' | 'exam' | 'attendance'
const TABS: Tab[] = ['audit', 'admissions', 'fees', 'dues', 'exam', 'attendance']
const PAGE = 25

function tabBtn(sel: boolean): CSSProperties {
  return { textAlign: 'left', padding: '10px 14px', borderRadius: '8px', border: 'none', background: sel ? 'var(--accent-12)' : 'transparent', color: sel ? 'var(--accent)' : 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '38px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontFamily: 'inherit', padding: '0 10px' }
}
const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }
const th: CSSProperties = { textAlign: 'left', padding: '10px 16px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }
const td: CSSProperties = { padding: '10px 16px', fontSize: '13px', borderTop: '1px solid var(--track)' }

export default function ReportsScreen() {
  const [tab, setTab] = useState<Tab>('audit')
  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('reports.eyebrow')}
        title={t('reports.title')}
        sub={t('reports.sub')}
        actions={<button type="button" onClick={() => void printCurrentWindow()} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('reports.print')}</button>}
      />
      <div style={{ display: 'grid', gridTemplateColumns: '200px 1fr', gap: '20px', alignItems: 'start' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }} data-appshell-chrome>
          {TABS.map((x) => (
            <button key={x} type="button" style={tabBtn(tab === x)} onClick={() => setTab(x)}>{t(`reports.tab.${x}`)}</button>
          ))}
        </div>
        <div style={{ minWidth: 0 }}>
          {tab === 'audit' ? <AuditReport /> : null}
          {tab === 'admissions' ? <AdmissionsReport /> : null}
          {tab === 'fees' ? <FeesReport /> : null}
          {tab === 'dues' ? <DuesReport /> : null}
          {tab === 'exam' ? <ExamReport /> : null}
          {tab === 'attendance' ? <AttendanceReport /> : null}
        </div>
      </div>
    </div>
  )
}

function AuditReport() {
  const [action, setAction] = useState('')
  const [date, setDate] = useState('')
  const [offset, setOffset] = useState(0)
  const [page, setPage] = useState<AuditPageDto | null>(null)

  const load = useCallback(() => {
    api.list_audit({ action: action || null, date: date || null, limit: PAGE, offset }).then(setPage).catch(() => setPage(null))
  }, [action, date, offset])
  useEffect(load, [load])
  useEffect(() => setOffset(0), [action, date])

  const total = page?.total ?? 0
  const from = total === 0 ? 0 : offset + 1
  const to = Math.min(offset + PAGE, total)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
      <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }} data-appshell-chrome>
        <input value={action} onChange={(e) => setAction(e.target.value)} placeholder={t('reports.audit.filterAction')} style={field()} />
        <input type="date" value={date} onChange={(e) => setDate(e.target.value)} style={field()} aria-label={t('reports.audit.filterDate')} />
        {page ? (
          <Pill variant={page.chain_ok ? 'paid' : 'unpaid'}>
            {page.chain_ok ? t('reports.audit.chainOk') : t('reports.audit.chainBad', { seq: page.first_bad_seq ?? 0 })}
          </Pill>
        ) : null}
      </div>
      <div style={CARD}>
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr>
              <th style={th}>{t('reports.audit.col.seq')}</th>
              <th style={th}>{t('reports.audit.col.at')}</th>
              <th style={th}>{t('reports.audit.col.who')}</th>
              <th style={th}>{t('reports.audit.col.action')}</th>
              <th style={th}>{t('reports.audit.col.table')}</th>
            </tr>
          </thead>
          <tbody>
            {(page?.rows ?? []).map((r) => (
              <tr key={r.seq}>
                <td style={{ ...td, color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{r.seq}</td>
                <td style={{ ...td, color: 'var(--muted)' }}>{r.at.replace('T', ' ').slice(0, 16)}</td>
                <td style={td}>{r.staff_name ?? '—'}</td>
                <td style={td}>{r.action}{r.reason ? ` · ${r.reason}` : ''}</td>
                <td style={{ ...td, color: 'var(--muted)' }}>{r.table ?? ''}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {(page?.rows.length ?? 0) === 0 ? <div style={{ padding: '32px', textAlign: 'center', color: 'var(--muted)' }}>{t('reports.none')}</div> : null}
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }} data-appshell-chrome>
        <span style={{ fontSize: '13px', color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{t('reports.showing', { from, to, total })}</span>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button type="button" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE))} style={{ ...field(), padding: '0 14px', opacity: offset === 0 ? 0.5 : 1 }}>{t('reports.prev')}</button>
          <button type="button" disabled={to >= total} onClick={() => setOffset(offset + PAGE)} style={{ ...field(), padding: '0 14px', opacity: to >= total ? 0.5 : 1 }}>{t('reports.next')}</button>
        </div>
      </div>
    </div>
  )
}

function BarRow({ label, value, max, right }: { label: string; value: number; max: number; right: string }) {
  const w = max > 0 ? Math.round((value / max) * 100) : 0
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '160px 1fr 90px', gap: '12px', alignItems: 'center', padding: '8px 16px', borderTop: '1px solid var(--track)' }}>
      <span style={{ fontSize: '13px' }}>{label}</span>
      <span style={{ height: '10px', background: 'var(--track)', borderRadius: '4px', overflow: 'hidden' }}>
        <span style={{ display: 'block', height: '100%', width: `${w}%`, background: 'rgba(125,177,181,0.55)', borderRadius: '4px' }} />
      </span>
      <span style={{ fontSize: '13px', textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{right}</span>
    </div>
  )
}

function AdmissionsReport() {
  const [rows, setRows] = useState<MonthCountDto[]>([])
  useEffect(() => { api.admissions_by_month().then(setRows).catch(() => setRows([])) }, [])
  const max = Math.max(1, ...rows.map((r) => r.count))
  return (
    <div style={CARD}>
      <div style={th}>{t('reports.admissions.month')} · {t('reports.admissions.count')}</div>
      {rows.length === 0 ? <div style={{ padding: '32px', textAlign: 'center', color: 'var(--muted)' }}>{t('reports.none')}</div> : rows.map((r) => <BarRow key={r.month} label={r.month} value={r.count} max={max} right={String(r.count)} />)}
    </div>
  )
}

function FeesReport() {
  const [from, setFrom] = useState(new Date(Date.now() - 30 * 864e5).toISOString().slice(0, 10))
  const [to, setTo] = useState(new Date().toISOString().slice(0, 10))
  const [data, setData] = useState<FeeCollectionDto | null>(null)
  useEffect(() => { api.fee_collection_report(from, to).then(setData).catch(() => setData(null)) }, [from, to])
  const max = Math.max(1, ...(data?.by_day.map((d) => d.total_paise) ?? [1]))
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
      <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }} data-appshell-chrome>
        <label style={{ fontSize: '12px', color: 'var(--muted)' }}>{t('reports.fees.from')} <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} style={field()} /></label>
        <label style={{ fontSize: '12px', color: 'var(--muted)' }}>{t('reports.fees.to')} <input type="date" value={to} onChange={(e) => setTo(e.target.value)} style={field()} /></label>
      </div>
      {data ? (
        <>
          <div style={{ display: 'flex', gap: '24px', fontSize: '14px', fontVariantNumeric: 'tabular-nums' }}>
            <span>{t('reports.fees.cash')}: {formatMoney(data.cash_paise)}</span>
            <span>{t('reports.fees.upi')}: {formatMoney(data.upi_paise)}</span>
            <span>{t('reports.fees.cheque')}: {formatMoney(data.cheque_paise)}</span>
            <span style={{ fontWeight: 600 }}>{t('reports.fees.total')}: {formatMoney(data.total_paise)}</span>
          </div>
          <div style={CARD}>
            <div style={th}>{t('reports.fees.total')}</div>
            {data.by_day.length === 0 ? <div style={{ padding: '32px', textAlign: 'center', color: 'var(--muted)' }}>{t('reports.none')}</div> : data.by_day.map((d) => <BarRow key={d.day} label={d.day} value={d.total_paise} max={max} right={formatMoney(d.total_paise)} />)}
          </div>
        </>
      ) : null}
    </div>
  )
}

function DuesReport() {
  const [rows, setRows] = useState<FeeOverviewRow[]>([])
  useEffect(() => { api.fees_overview().then(setRows).catch(() => setRows([])) }, [])
  return (
    <div style={CARD}>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead><tr><th style={th}>{t('reports.dues.class')}</th><th style={{ ...th, textAlign: 'right' }}>{t('reports.dues.students')}</th><th style={{ ...th, textAlign: 'right' }}>{t('reports.dues.outstanding')}</th></tr></thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.class_id}>
              <td style={td}>{r.class_display}</td>
              <td style={{ ...td, textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{r.students_with_dues}</td>
              <td style={{ ...td, textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.outstanding_paise)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function ExamReport() {
  const [exams, setExams] = useState<ExamDto[]>([])
  const [examId, setExamId] = useState('')
  const [rows, setRows] = useState<ExamResultRowDto[]>([])
  useEffect(() => { api.list_exams().then((es) => { setExams(es); if (es[0]) setExamId(es[0].id) }).catch(() => setExams([])) }, [])
  useEffect(() => { if (examId) api.exam_results(examId).then(setRows).catch(() => setRows([])) }, [examId])
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
      <select value={examId} onChange={(e) => setExamId(e.target.value)} style={{ ...field(), alignSelf: 'flex-start' }} data-appshell-chrome aria-label={t('reports.exam.pick')}>
        {exams.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
      </select>
      {rows.map((r, i) => (
        <div key={i} style={{ ...CARD, padding: '16px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
            <span style={{ fontWeight: 500 }}>{r.class_display} · {r.subject_name}</span>
            <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('reports.exam.avg')}: {(r.average_pct_tenths / 10).toFixed(1)}% · {t('reports.exam.graded')}: {r.graded}</span>
          </div>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'flex-end', marginTop: '12px', height: '80px' }}>
            {r.distribution.map((d) => {
              const maxc = Math.max(1, ...r.distribution.map((x) => x.count))
              return (
                <div key={d.grade} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px', flex: 1 }}>
                  <div style={{ width: '100%', maxWidth: '36px', height: `${Math.round((d.count / maxc) * 60)}px`, background: 'rgba(125,177,181,0.55)', borderRadius: '4px 4px 2px 2px' }} />
                  <span style={{ fontSize: '11px', color: 'var(--muted)' }}>{d.grade}</span>
                  <span style={{ fontSize: '11px', fontVariantNumeric: 'tabular-nums' }}>{d.count}</span>
                </div>
              )
            })}
          </div>
        </div>
      ))}
      {rows.length === 0 ? <div style={{ padding: '32px', textAlign: 'center', color: 'var(--muted)' }}>{t('reports.none')}</div> : null}
    </div>
  )
}

function AttendanceReport() {
  return (
    <div style={{ ...CARD, padding: '24px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
      <div style={{ fontSize: '14px', color: 'var(--ink)' }}>{t('reports.att.note')}</div>
      <button type="button" onClick={() => navigate('/principal/attendance')} style={{ alignSelf: 'flex-start', height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('reports.att.open')}</button>
    </div>
  )
}
