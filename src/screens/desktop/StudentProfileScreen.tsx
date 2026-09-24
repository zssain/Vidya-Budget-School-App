import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { Icon } from '@/components/Icon'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { ClassDto, FeeDuesDto, PaymentDto, RequestDto, StudentProfileDto } from '@/lib/api'

// Student profile (prompts/P07 §1): details, enrollment history, attendance %
// this term, fees/ledger (Principal + Accountant), requests, audit. Actions:
// Transfer section, Mark as left. Full page derived from the mock (§6.2).

const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px' }
const CARD_HEAD: CSSProperties = { padding: '16px 24px', borderBottom: '1px solid var(--track)', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
      <span style={{ fontSize: '11px', color: 'var(--muted)' }}>{label}</span>
      <span style={{ fontSize: '14px', color: 'var(--ink)' }}>{value}</span>
    </div>
  )
}

function fieldStyle(): CSSProperties {
  return { height: '38px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontFamily: 'inherit', padding: '0 12px', boxSizing: 'border-box' }
}
function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}

function Modal({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '420px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 12px', fontFamily: 'var(--font-serif)', fontSize: '22px', color: 'var(--ink)' }}>{title}</div>
        <div style={{ padding: '0 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>{children}</div>
      </div>
    </div>
  )
}

export default function StudentProfileScreen({ id, role }: { id: string; role: string }) {
  const base = role === 'accountant' ? '/accountant/students' : '/principal/students'
  const finance = role === 'principal' || role === 'accountant'
  const [p, setP] = useState<StudentProfileDto | null>(null)
  const [dues, setDues] = useState<FeeDuesDto | null>(null)
  const [payments, setPayments] = useState<PaymentDto[]>([])
  const [requests, setRequests] = useState<RequestDto[]>([])
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [error, setError] = useState(false)
  const [dialog, setDialog] = useState<'transfer' | 'left' | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(() => {
    api.get_student_profile(id).then(setP).catch(() => setError(true))
    if (finance) {
      api.list_fee_dues(id).then(setDues).catch(() => setDues(null))
      api.list_payments(id).then(setPayments).catch(() => setPayments([]))
    }
    api.list_requests().then((rs) => setRequests(rs.filter((r) => r.target_id === id))).catch(() => setRequests([]))
    api.list_classes().then(setClasses).catch(() => setClasses([]))
  }, [id, finance])

  useEffect(load, [load])

  if (error) return <div style={{ padding: '48px', color: 'var(--muted)' }}>{t('students.notFound')}</div>
  if (!p) return <div style={{ padding: '48px', color: 'var(--muted)' }}>…</div>

  const adm = p.admission_no ?? p.provisional_no ?? t('students.admissionPending')
  const attPct = (p.attendance.pct_tenths / 10).toFixed(1)

  return (
    <div style={{ padding: '28px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <button type="button" onClick={() => navigate(base)} style={{ display: 'flex', alignItems: 'center', gap: '6px', border: 'none', background: 'transparent', color: 'var(--muted)', fontSize: '13px', cursor: 'pointer', alignSelf: 'flex-start' }}>
        <Icon name="back" size={16} strokeWidth={1.75} />
        {t('students.back')}
      </button>

      {/* Title + actions */}
      <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: '24px', flexWrap: 'wrap' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: '40px', letterSpacing: '-0.02em', color: 'var(--ink)' }}>{p.name}</h1>
            <Pill variant={p.status === 'left' ? 'unpaid' : 'paid'}>{p.status === 'left' ? t('students.pill.left') : t('students.pill.active')}</Pill>
          </div>
          <div style={{ fontSize: '13px', color: p.admission_no ? 'var(--muted)' : 'var(--gold-text)', marginTop: '4px' }}>
            {adm}
            {p.status === 'left' && p.left_on ? ` · ${t('students.leftOn', { date: p.left_on })}` : ''}
          </div>
        </div>
        {p.status !== 'left' ? (
          <div style={{ display: 'flex', gap: '10px' }}>
            <button type="button" style={secondaryBtn()} onClick={() => setDialog('transfer')}>{t('students.action.transfer')}</button>
            <button type="button" style={secondaryBtn()} onClick={() => setDialog('left')}>{t('students.action.left')}</button>
          </div>
        ) : null}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: '20px', alignItems: 'start' }}>
        {/* Details */}
        <div style={CARD}>
          <div style={CARD_HEAD}>{t('students.section.details')}</div>
          <div style={{ padding: '20px 24px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px 24px' }}>
            <Field label={t('students.field.admission')} value={adm} />
            <Field label={t('students.field.class')} value={`${p.class_display ?? '—'}${p.roll_no != null ? ` · #${p.roll_no}` : ''}`} />
            <Field label={t('students.field.dob')} value={p.dob ?? '—'} />
            <Field label={t('students.field.gender')} value={p.gender ?? '—'} />
            <Field label={t('students.field.guardian')} value={p.guardian_name ?? '—'} />
            <Field label={t('students.field.mobile')} value={p.guardian_mobile ?? '—'} />
            <Field label={t('students.field.address')} value={p.address ?? '—'} />
            <Field label={t('students.field.transport')} value={p.transport ? t('students.yes') : t('students.no')} />
            <Field label={t('students.field.rte')} value={p.rte ? t('students.yes') : t('students.no')} />
            <Field label={t('students.field.category')} value={p.category ?? '—'} />
            <Field label={t('students.field.aadhaar')} value={t(`students.aadhaar.${p.aadhaar_status}`)} />
          </div>
        </div>

        {/* Attendance this term */}
        <div style={CARD}>
          <div style={CARD_HEAD}>{t('students.section.attendance')}</div>
          <div style={{ padding: '20px 24px' }}>
            {p.attendance.marked > 0 ? (
              <>
                <div style={{ fontFamily: 'var(--font-serif)', fontSize: '40px', color: 'var(--ink)', lineHeight: 1 }}>{attPct}%</div>
                <div style={{ fontSize: '13px', color: 'var(--muted)', marginTop: '8px' }}>
                  {t('students.attendanceRange', { marked: p.attendance.marked, from: p.attendance.from_date ?? '', to: p.attendance.to_date ?? '' })}
                </div>
                {p.attendance.term_label ? <div style={{ fontSize: '12px', color: 'var(--muted)', marginTop: '2px' }}>{p.attendance.term_label}</div> : null}
              </>
            ) : (
              <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('students.attendanceNone')}</div>
            )}
          </div>
        </div>

        {/* Enrollment history */}
        <div style={CARD}>
          <div style={CARD_HEAD}>{t('students.section.enrollment')}</div>
          <div>
            {p.enrollment_history.length === 0 ? (
              <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: '13px' }}>—</div>
            ) : (
              p.enrollment_history.map((e, i) => (
                <div key={i} style={{ display: 'flex', justifyContent: 'space-between', gap: '16px', padding: '12px 24px', fontSize: '13px', ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}) }}>
                  <span style={{ fontWeight: 500 }}>{e.class_display ?? '—'}{e.roll_no != null ? ` · #${e.roll_no}` : ''}</span>
                  <span style={{ color: 'var(--muted)' }}>{e.session_label ?? ''} · {e.from_date}{e.to_date ? ` → ${e.to_date}` : ''}</span>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Fees & ledger (finance roles only) */}
        {finance ? (
          <div style={CARD}>
            <div style={CARD_HEAD}>{t('students.section.ledger')}</div>
            <div style={{ padding: '16px 24px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('students.ledgerOutstanding')}</span>
                <span style={{ fontFamily: 'var(--font-serif)', fontSize: '26px', color: 'var(--ink)', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(dues?.total_due_paise ?? 0)}</span>
              </div>
              {dues && dues.lines.length > 0 ? (
                dues.lines.map((l) => (
                  <div key={l.id} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px', color: 'var(--ink)' }}>
                    <span>{l.label}</span>
                    <span style={{ fontVariantNumeric: 'tabular-nums', color: l.balance_paise > 0 ? 'var(--pill-unpaid-fg)' : 'var(--accent)' }}>{l.balance_paise > 0 ? `${formatMoney(l.balance_paise)} due` : 'Paid'}</span>
                  </div>
                ))
              ) : (
                <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('students.ledgerNoDues')}</div>
              )}
              {payments.length > 0 ? (
                <div style={{ marginTop: '4px', borderTop: '1px solid var(--track)', paddingTop: '10px' }}>
                  <div style={{ fontSize: '11px', color: 'var(--muted)', marginBottom: '6px' }}>{t('students.recentPayments')}</div>
                  {payments.slice(0, 5).map((pay) => (
                    <div key={pay.id} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
                      <span style={{ color: 'var(--muted)' }}>{pay.receipt_no}</span>
                      <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatMoney(pay.amount_paise)}{pay.confirmed ? '' : ' *'}</span>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>
          </div>
        ) : null}

        {/* Requests */}
        <div style={CARD}>
          <div style={CARD_HEAD}>{t('students.section.requests')}</div>
          <div>
            {requests.length === 0 ? (
              <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: '13px' }}>{t('students.noRequests')}</div>
            ) : (
              requests.map((r, i) => (
                <div key={r.id} style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', padding: '12px 24px', fontSize: '13px', ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}) }}>
                  <span>{r.kind}</span>
                  <Pill variant="neutral">{r.status}</Pill>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {dialog === 'transfer' ? (
        <TransferDialog classes={classes} currentClassId={p.class_id} busy={busy} onClose={() => setDialog(null)} onConfirm={async (cid) => { setBusy(true); try { await api.transfer_student(id, cid); setDialog(null); load() } finally { setBusy(false) } }} />
      ) : null}
      {dialog === 'left' ? (
        <LeaveDialog busy={busy} onClose={() => setDialog(null)} onConfirm={async (date, reason) => { setBusy(true); try { await api.mark_student_left(id, date, reason); setDialog(null); load() } finally { setBusy(false) } }} />
      ) : null}
    </div>
  )
}

function TransferDialog({ classes, currentClassId, busy, onClose, onConfirm }: { classes: ClassDto[]; currentClassId: string | null; busy: boolean; onClose: () => void; onConfirm: (classId: string) => void }) {
  const [cid, setCid] = useState('')
  return (
    <Modal title={t('students.transfer.title')} onClose={onClose}>
      <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>
        {t('students.transfer.toClass')}
        <select value={cid} onChange={(e) => setCid(e.target.value)} style={fieldStyle()}>
          <option value="">—</option>
          {classes.filter((c) => c.id !== currentClassId).map((c) => (
            <option key={c.id} value={c.id}>{c.display}</option>
          ))}
        </select>
      </label>
      <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
        <button type="button" style={secondaryBtn()} onClick={onClose}>{t('students.cancel')}</button>
        <button type="button" style={{ ...primaryBtn(), opacity: cid && !busy ? 1 : 0.5 }} disabled={!cid || busy} onClick={() => onConfirm(cid)}>{t('students.transfer.confirm')}</button>
      </div>
    </Modal>
  )
}

function LeaveDialog({ busy, onClose, onConfirm }: { busy: boolean; onClose: () => void; onConfirm: (date: string, reason: string) => void }) {
  const [date, setDate] = useState('')
  const [reason, setReason] = useState('')
  return (
    <Modal title={t('students.left.title')} onClose={onClose}>
      <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>
        {t('students.left.date')}
        <input type="date" value={date} onChange={(e) => setDate(e.target.value)} style={fieldStyle()} />
      </label>
      <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>
        {t('students.left.reason')}
        <input value={reason} onChange={(e) => setReason(e.target.value)} style={fieldStyle()} />
      </label>
      <div style={{ fontSize: '12px', color: 'var(--gold-text)' }}>{t('students.left.warn')}</div>
      <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
        <button type="button" style={secondaryBtn()} onClick={onClose}>{t('students.cancel')}</button>
        <button type="button" style={{ ...primaryBtn(), opacity: date && reason && !busy ? 1 : 0.5 }} disabled={!date || !reason || busy} onClick={() => onConfirm(date, reason)}>{t('students.left.confirm')}</button>
      </div>
    </Modal>
  )
}
