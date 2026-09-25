// Fees → Dues (prototype `feesadmin` state 2) + reminder sheet (state 3), P14 Step 5.
// Every control is real: list_dues / queue_fee_reminders / preview_fee_reminder /
// record_message. Reuses the UPI QR (qr_svg) built in Step 1. Vidya never handles
// money — parents pay the school directly; statuses are honest (email = queued;
// WhatsApp = tapped).

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import * as api from '@/lib/api'
import type { BulkReminderDto, DuesListDto, DuesRowDto, ReminderPreviewDto } from '@/lib/api'
import { formatMoney } from '@/lib/format'
import { shareWhatsApp } from '@/lib/files'
import { t } from '@/lib/i18n'

const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }
const smallBtn = (primary = false): CSSProperties => ({ height: 34, padding: '0 12px', borderRadius: 6, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: `1px solid ${primary ? 'var(--accent)' : 'var(--line-strong)'}`, background: primary ? 'var(--accent)' : 'var(--surface)', color: primary ? 'var(--white)' : 'var(--ink)' })
const COLS = '1.7fr 0.6fr 1.3fr 0.8fr 1.3fr'

function Strip({ data }: { data: DuesListDto }) {
  const cells: [string, string][] = [
    [formatMoney(data.strip.total_due_paise), t('fees.dues.strip.total')],
    [String(data.strip.students_with_dues), t('fees.dues.strip.students')],
    [String(data.strip.unpaid_dues), t('fees.dues.strip.unpaid')],
    [formatMoney(data.strip.collected_today_paise), t('fees.dues.strip.collected')],
  ]
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', background: 'var(--panel)', borderRadius: 16 }}>
      {cells.map(([v, l], i) => (
        <div key={l} style={{ padding: '22px 24px', borderLeft: i ? '1px solid var(--line-stat)' : 'none' }}>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 40, lineHeight: 1, letterSpacing: '-0.02em', fontVariantNumeric: 'tabular-nums' }}>{v}</div>
          <div style={{ fontSize: 14, color: 'var(--muted)', marginTop: 8 }}>{l}</div>
        </div>
      ))}
    </div>
  )
}

export default function FeesDues() {
  const [data, setData] = useState<DuesListDto | null>(null)
  const [reminder, setReminder] = useState<{ row: DuesRowDto; channel: 'email' | 'wa' } | null>(null)
  const [bulk, setBulk] = useState<BulkReminderDto | null>(null)

  const load = useCallback(() => { api.list_dues().then(setData).catch(() => setData(null)) }, [])
  useEffect(load, [load])

  const emailAll = async () => {
    if (!data) return
    const ids = Array.from(new Set(data.rows.filter((r) => r.emailable).map((r) => r.student_id)))
    if (ids.length === 0) return
    try {
      const res = await api.queue_fee_reminders(ids)
      setBulk(res)
      load()
    } catch { /* honest: nothing queued */ }
  }

  if (!data) return <div style={{ padding: 48, color: 'var(--muted)' }}>…</div>

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button
          type="button"
          disabled={data.emailable_students === 0}
          onClick={emailAll}
          style={{ ...smallBtn(true), height: 40, opacity: data.emailable_students === 0 ? 0.5 : 1 }}
        >
          {data.emailable_students === 0 ? t('fees.dues.emailAllNone') : t('fees.dues.emailAll', { n: data.emailable_students })}
        </button>
      </div>

      {bulk ? (
        <div style={{ fontSize: 13, color: 'var(--accent)', background: 'var(--accent-6)', border: '1px solid var(--line)', borderRadius: 8, padding: '10px 14px' }}>
          {t('fees.dues.emailed', { n: bulk.queued, noEmail: bulk.skipped_no_email, noConsent: bulk.skipped_no_consent })}
        </div>
      ) : null}

      <Strip data={data} />

      <div style={CARD}>
        <div style={{ display: 'grid', gridTemplateColumns: COLS, gap: 16, padding: '12px 24px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('fees.dues.col.student')}</span>
          <span>{t('fees.dues.col.class')}</span>
          <span>{t('fees.dues.col.instalment')}</span>
          <span style={{ textAlign: 'right' }}>{t('fees.dues.col.amount')}</span>
          <span style={{ textAlign: 'right' }}>{t('fees.dues.col.remind')}</span>
        </div>
        {data.rows.length === 0 ? (
          <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('fees.dues.none')}</div>
        ) : (
          data.rows.map((r, i) => (
            <div key={r.due_id} style={{ display: 'grid', gridTemplateColumns: COLS, gap: 16, alignItems: 'center', padding: '14px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: 14 }}>
              <span>
                <div style={{ fontWeight: 500 }}>{r.student_name}</div>
                <div style={{ fontSize: 12, color: 'var(--muted)' }}>{r.guardian_name ? `${t('fees.dues.guardian')} · ${r.guardian_name}` : t('fees.dues.noGuardian')}</div>
              </span>
              <span style={{ color: 'var(--muted)' }}>{r.class_display ?? '—'}</span>
              <span>{r.fee_head}</span>
              <span style={{ textAlign: 'right', fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.balance_paise)}</span>
              <span style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <button type="button" style={smallBtn()} onClick={() => setReminder({ row: r, channel: 'email' })}>{t('fees.dues.email')}</button>
                <button type="button" style={smallBtn()} onClick={() => setReminder({ row: r, channel: 'wa' })}>{t('fees.dues.whatsapp')}</button>
              </span>
            </div>
          ))
        )}
      </div>

      {reminder ? (
        <ReminderSheet row={reminder.row} initialChannel={reminder.channel} onClose={() => setReminder(null)} onSent={() => { setReminder(null); load() }} />
      ) : null}
    </div>
  )
}

function ReminderSheet({ row, initialChannel, onClose, onSent }: { row: DuesRowDto; initialChannel: 'email' | 'wa'; onClose: () => void; onSent: () => void }) {
  const [channel, setChannel] = useState<'email' | 'wa'>(initialChannel)
  const [lang, setLang] = useState<'en' | 'hi' | 'te'>((row.guardian_language as 'en' | 'hi' | 'te') || 'en')
  const [preview, setPreview] = useState<ReminderPreviewDto | null>(null)
  const [qr, setQr] = useState<string | null>(null)
  const [school, setSchool] = useState('')
  const [busy, setBusy] = useState(false)
  const [done, setDone] = useState(false)

  useEffect(() => { api.get_school().then((s) => setSchool(s.name)).catch(() => setSchool('')) }, [])
  useEffect(() => {
    api.preview_fee_reminder(row.student_id, lang).then(setPreview).catch(() => setPreview(null))
  }, [row.student_id, lang])
  useEffect(() => {
    if (preview?.upi_link) api.qr_svg(preview.upi_link).then(setQr).catch(() => setQr(null))
    else setQr(null)
  }, [preview?.upi_link])

  const send = async () => {
    if (!preview) return
    setBusy(true)
    try {
      if (channel === 'email') {
        await api.queue_fee_reminders([row.student_id], lang)
        setDone(true)
      } else {
        await api.record_message({ channel: 'wa_tap', kind: 'fee_reminder', language: lang, to_guardian_id: row.guardian_id, to_address: preview.guardian_mobile, body: preview.body, related_table: 'student', related_id: row.student_id })
        await shareWhatsApp(preview.guardian_mobile, preview.body)
        onSent()
      }
    } catch { /* honest: nothing sent */ } finally { setBusy(false) }
  }

  const chipStyle = (on: boolean): CSSProperties => ({ height: 36, padding: '0 16px', borderRadius: 18, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: `1.5px solid ${on ? 'var(--accent)' : 'var(--line-strong)'}`, background: on ? 'var(--accent)' : 'var(--white)', color: on ? 'var(--white)' : 'var(--ink)' })
  const segStyle = (on: boolean): CSSProperties => ({ flex: 1, height: 44, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: 'none', background: on ? 'var(--accent)' : 'transparent', color: on ? 'var(--white)' : 'var(--ink)' })

  const emailBlocked = channel === 'email' && (!preview?.has_email || !preview?.has_consent)

  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'flex', justifyContent: 'flex-end', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: 520, margin: 16, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 12px' }}>
          <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>{t('fees.rem.eyebrow')}</div>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 24 }}>{row.student_name} · {row.class_display ?? ''}</div>
          <div style={{ fontSize: 13, color: 'var(--muted)' }}>{row.guardian_name ?? ''}{row.guardian_email ? ` · ${row.guardian_email}` : ''}</div>
        </div>

        <div style={{ padding: '0 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1, minHeight: 0, overflowY: 'auto' }}>
          <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: 6, overflow: 'hidden' }}>
            <button type="button" style={segStyle(channel === 'email')} onClick={() => setChannel('email')}>{t('fees.rem.chEmail')}</button>
            <button type="button" style={{ ...segStyle(channel === 'wa'), borderLeft: '1px solid var(--line-strong)' }} onClick={() => setChannel('wa')}>{t('fees.rem.chWhatsApp')}</button>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            {(['en', 'hi', 'te'] as const).map((l) => (
              <button key={l} type="button" style={chipStyle(lang === l)} onClick={() => setLang(l)}>{l === 'en' ? 'English' : l === 'hi' ? 'हिंदी' : 'తెలుగు'}</button>
            ))}
          </div>

          {preview ? (
            <div style={{ borderRadius: 12, border: '1px solid var(--line)', background: 'var(--white)', padding: 18, display: 'flex', flexDirection: 'column', gap: 10, fontSize: 14, lineHeight: 1.55 }}>
              {channel === 'email' ? <span style={{ fontSize: 12, color: 'var(--muted)' }}>{t('fees.rem.from', { school })}</span> : null}
              {preview.subject ? <span style={{ fontWeight: 600 }}>{preview.subject}</span> : null}
              <span style={{ whiteSpace: 'pre-wrap' }}>{preview.body}</span>
              {preview.upi_link && qr ? (
                <div style={{ display: 'flex', alignItems: 'center', gap: 14, padding: 10, borderRadius: 8, background: 'var(--bg)' }}>
                  <span style={{ width: 88, height: 88, flexShrink: 0 }} dangerouslySetInnerHTML={{ __html: qr }} />
                  <span style={{ fontSize: 13 }}><b>{t('fees.rem.scan')} {formatMoney(row.balance_paise)}</b></span>
                </div>
              ) : null}
            </div>
          ) : <div style={{ color: 'var(--muted)', padding: 20 }}>…</div>}

          {emailBlocked ? (
            <div style={{ fontSize: 13, color: 'var(--gold-text)' }}>{!preview?.has_email ? t('fees.rem.noEmail') : t('fees.rem.noConsent')}</div>
          ) : null}
        </div>

        <div style={{ padding: '14px 24px', background: 'var(--panel)', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {done ? (
            <div style={{ textAlign: 'center', fontSize: 14, color: 'var(--accent)' }}>{t('fees.rem.sent')}</div>
          ) : (
            <button type="button" disabled={busy || emailBlocked || !preview} onClick={send} style={{ height: 50, borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 15, fontWeight: 500, cursor: 'pointer', opacity: busy || emailBlocked || !preview ? 0.5 : 1 }}>
              {channel === 'email' ? t('fees.rem.sendEmail') : t('fees.rem.sendWhatsApp')}
            </button>
          )}
          <span style={{ fontSize: 12, color: 'var(--muted)', textAlign: 'center' }}>{channel === 'email' ? t('fees.rem.emailNote') : t('fees.rem.waNote')}</span>
          <button type="button" onClick={onClose} style={{ height: 34, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('fees.rem.close')}</button>
        </div>
      </div>
    </div>
  )
}
