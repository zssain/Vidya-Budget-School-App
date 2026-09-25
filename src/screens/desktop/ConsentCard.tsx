// Consent + export/erase on the student profile (P13, DPDP §9). Accountant &
// Principal record/withdraw consent; the Principal can export the student's data
// (JSON) or erase personal fields on request (with a confirmation that explains
// what is kept). Commands re-check the permission and audit every change.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, ConsentDto } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }
const CARD_HEAD: React.CSSProperties = { padding: '14px 24px', borderBottom: '1px solid var(--line)', fontSize: 15, fontWeight: 600, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }
const linkBtn: React.CSSProperties = { background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }
const inputStyle: React.CSSProperties = { height: 34, padding: '0 10px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 13 }

export default function ConsentCard({ studentId, canManage, onErased }: { studentId: string; canManage: boolean; onErased?: () => void }) {
  const [rows, setRows] = useState<ConsentDto[]>([])
  const [adding, setAdding] = useState<{ purpose: string; method: string } | null>(null)
  const [exportJson, setExportJson] = useState<string | null>(null)
  const [confirmErase, setConfirmErase] = useState(false)

  const load = useCallback(() => {
    api.list_consent(studentId).then(setRows).catch(() => setRows([]))
  }, [studentId])
  useEffect(() => { load() }, [load])

  const record = async () => {
    if (!adding) return
    try {
      setRows(await api.record_consent(studentId, null, adding.purpose, adding.method))
      setAdding(null)
    } catch (e) { void (e as CmdError) }
  }
  const withdraw = async (id: string) => {
    try { setRows(await api.withdraw_consent(id)) } catch (e) { void (e as CmdError) }
  }
  const doExport = async () => {
    try { setExportJson(await api.export_student(studentId)) } catch (e) { void (e as CmdError) }
  }
  const doErase = async () => {
    try {
      await api.erase_student(studentId)
      setConfirmErase(false)
      onErased?.()
    } catch (e) { void (e as CmdError) }
  }

  return (
    <div style={CARD}>
      <div style={CARD_HEAD}>
        <span>{t('cs.section')}</span>
        <button type="button" style={linkBtn} onClick={() => setAdding({ purpose: 'school_records', method: 'signed_form' })}>+ {t('cs.record')}</button>
      </div>
      <div>
        {rows.length === 0 ? (
          <div style={{ padding: '16px 24px', fontSize: 13, color: 'var(--muted)' }}>{t('cs.empty')}</div>
        ) : (
          rows.map((r, i) => (
            <div key={r.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '11px 24px', ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}), opacity: r.withdrawn_at ? 0.55 : 1 }}>
              <div style={{ flexGrow: 1, fontSize: 13 }}>
                <span style={{ fontWeight: 500 }}>{t(`cs.purpose.${r.purpose}`)}</span>
                <span style={{ color: 'var(--muted)' }}> · {t(`cs.method.${r.method}`)} · {r.recorded_at.slice(0, 10)}</span>
                {r.withdrawn_at ? <span style={{ color: 'var(--danger)' }}> · {t('cs.withdrawn')}</span> : null}
              </div>
              {!r.withdrawn_at ? <button type="button" style={linkBtn} onClick={() => withdraw(r.id)}>{t('cs.withdraw')}</button> : null}
            </div>
          ))
        )}
      </div>
      {canManage ? (
        <div style={{ display: 'flex', gap: 16, padding: '12px 24px', borderTop: '1px solid var(--track)' }}>
          <button type="button" style={linkBtn} onClick={doExport}>{t('cs.export')}</button>
          <button type="button" style={{ ...linkBtn, color: 'var(--danger)' }} onClick={() => setConfirmErase(true)}>{t('cs.erase')}</button>
        </div>
      ) : null}

      {exportJson != null ? (
        <div style={{ padding: '0 24px 16px' }}>
          <div style={{ fontSize: 12, color: 'var(--muted)', margin: '4px 0' }}>{t('cs.exported')}</div>
          <textarea readOnly value={exportJson} rows={8} style={{ width: '100%', boxSizing: 'border-box', fontFamily: 'monospace', fontSize: 11, border: '1px solid var(--line-strong)', borderRadius: 6, padding: 8, background: 'var(--white)', color: 'var(--ink)' }} />
        </div>
      ) : null}

      {adding ? (
        <div onClick={() => setAdding(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 400, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{t('cs.record')}</div>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }}>{t('cs.purpose')}
              <select value={adding.purpose} onChange={(e) => setAdding({ ...adding, purpose: e.target.value })} style={inputStyle}>
                <option value="school_records">{t('cs.purpose.school_records')}</option>
                <option value="messages">{t('cs.purpose.messages')}</option>
              </select>
            </label>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }}>{t('cs.method')}
              <select value={adding.method} onChange={(e) => setAdding({ ...adding, method: e.target.value })} style={inputStyle}>
                <option value="signed_form">{t('cs.method.signed_form')}</option>
                <option value="in_person">{t('cs.method.in_person')}</option>
              </select>
            </label>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setAdding(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('cs.cancel')}</button>
              <button type="button" onClick={record} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('cs.save')}</button>
            </div>
          </div>
        </div>
      ) : null}

      {confirmErase ? (
        <div onClick={() => setConfirmErase(false)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 440, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{t('cs.erase.title')}</div>
            <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>{t('cs.erase.body')}</p>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setConfirmErase(false)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('cs.cancel')}</button>
              <button type="button" onClick={doErase} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--danger)', background: 'var(--danger)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('cs.erase.confirm')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}
