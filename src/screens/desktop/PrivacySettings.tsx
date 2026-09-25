// Settings → Privacy (P13, DPDP §9): retention setting, the data-incident log with
// the 72-hour reporting reminder, and the export/erase history. Principal only —
// the commands re-check the permission and audit changes.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, IncidentDto, IncidentInput, PrivacyActionDto } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24, gridColumn: '1 / -1' }
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 14px' }
const linkBtn: React.CSSProperties = { background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }
const fieldLabel: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }
const inputStyle: React.CSSProperties = { height: 38, padding: '0 12px', borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14 }

const emptyIncident = (): IncidentInput => ({ occurred_on: '', description: '', action_taken: '', reported_to_board: false, reported_on: '' })

export default function PrivacySettings() {
  const [retention, setRetention] = useState('keep')
  const [incidents, setIncidents] = useState<IncidentDto[]>([])
  const [actions, setActions] = useState<PrivacyActionDto[]>([])
  const [editing, setEditing] = useState<IncidentInput | null>(null)

  const load = useCallback(() => {
    api.get_retention().then(setRetention).catch(() => {})
    api.list_incidents().then(setIncidents).catch(() => setIncidents([]))
    api.list_privacy_actions().then(setActions).catch(() => setActions([]))
  }, [])
  useEffect(() => { load() }, [load])

  const chooseRetention = async (v: string) => {
    setRetention(v)
    try { await api.set_retention(v) } catch (e) { void (e as CmdError) }
  }

  const saveIncident = async () => {
    if (!editing) return
    try {
      await api.add_incident(editing)
      setEditing(null)
      load()
    } catch (e) {
      void (e as CmdError)
    }
  }

  return (
    <div style={CARD}>
      <h3 style={H3}>{t('pv.section')}</h3>
      <p style={{ fontSize: 13, color: 'var(--muted)', margin: '0 0 16px' }}>{t('pv.hint')}</p>

      {/* Retention */}
      <div style={{ borderTop: '1px solid var(--track)', paddingTop: 14 }}>
        <div style={{ fontSize: 14, fontWeight: 500, marginBottom: 8 }}>{t('pv.retention')}</div>
        <div style={{ display: 'flex', gap: 10 }}>
          {(['keep', 'review'] as const).map((v) => (
            <button key={v} type="button" onClick={() => chooseRetention(v)}
              style={{ height: 36, padding: '0 16px', borderRadius: 8, fontSize: 13, cursor: 'pointer', border: `1px solid ${retention === v ? 'var(--accent)' : 'var(--line-strong)'}`, background: retention === v ? 'var(--accent)' : 'var(--white)', color: retention === v ? 'var(--white)' : 'var(--ink)' }}>
              {t(`pv.retention.${v}`)}
            </button>
          ))}
        </div>
      </div>

      {/* Incident log */}
      <div style={{ borderTop: '1px solid var(--track)', paddingTop: 14, marginTop: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ fontSize: 14, fontWeight: 500 }}>{t('pv.incidents')}</div>
          <button type="button" style={linkBtn} onClick={() => setEditing(emptyIncident())}>+ {t('pv.incident.add')}</button>
        </div>
        <div style={{ fontSize: 12, color: 'var(--gold-text)', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', borderRadius: 8, padding: '8px 12px', margin: '10px 0' }}>{t('pv.incident.72h')}</div>
        {incidents.length === 0 ? (
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>{t('pv.incident.empty')}</p>
        ) : (
          incidents.map((i, idx) => (
            <div key={i.id} style={{ padding: '10px 0', fontSize: 13, ...(idx > 0 ? { borderTop: '1px solid var(--track)' } : {}) }}>
              <div style={{ fontWeight: 500 }}>{i.description}</div>
              <div style={{ fontSize: 12, color: 'var(--muted)' }}>
                {i.occurred_on ?? '—'}{i.action_taken ? ` · ${i.action_taken}` : ''}{i.reported_to_board ? ` · ${t('pv.incident.reported')}${i.reported_on ? ` (${i.reported_on})` : ''}` : ''}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Export / erase history */}
      <div style={{ borderTop: '1px solid var(--track)', paddingTop: 14, marginTop: 16 }}>
        <div style={{ fontSize: 14, fontWeight: 500, marginBottom: 8 }}>{t('pv.actions')}</div>
        {actions.length === 0 ? (
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>{t('pv.actions.empty')}</p>
        ) : (
          actions.map((a) => (
            <div key={a.id} style={{ display: 'flex', justifyContent: 'space-between', gap: 12, padding: '6px 0', fontSize: 13 }}>
              <span>{a.kind === 'erase' ? t('cs.erase') : t('cs.export')} · {a.note ?? ''}</span>
              <span style={{ color: 'var(--muted)' }}>{a.performed_at.slice(0, 10)}</span>
            </div>
          ))
        )}
      </div>

      {editing ? (
        <div onClick={() => setEditing(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 460, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{t('pv.incident.add')}</div>
            <label style={fieldLabel}>{t('pv.incident.date')}
              <input type="date" value={editing.occurred_on ?? ''} onChange={(e) => setEditing({ ...editing, occurred_on: e.target.value })} style={inputStyle} />
            </label>
            <label style={fieldLabel}>{t('pv.incident.description')}
              <textarea value={editing.description} onChange={(e) => setEditing({ ...editing, description: e.target.value })} rows={3} style={{ ...inputStyle, height: 'auto', padding: '8px 12px', resize: 'vertical', fontFamily: 'inherit' }} />
            </label>
            <label style={fieldLabel}>{t('pv.incident.action')}
              <input value={editing.action_taken ?? ''} onChange={(e) => setEditing({ ...editing, action_taken: e.target.value })} style={inputStyle} />
            </label>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
              <input type="checkbox" checked={editing.reported_to_board ?? false} onChange={(e) => setEditing({ ...editing, reported_to_board: e.target.checked })} />
              {t('pv.incident.reported')}
            </label>
            {editing.reported_to_board ? (
              <label style={fieldLabel}>{t('pv.incident.reportedOn')}
                <input type="date" value={editing.reported_on ?? ''} onChange={(e) => setEditing({ ...editing, reported_on: e.target.value })} style={inputStyle} />
              </label>
            ) : null}
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setEditing(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('pv.incident.cancel')}</button>
              <button type="button" onClick={saveIncident} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('pv.incident.save')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}
