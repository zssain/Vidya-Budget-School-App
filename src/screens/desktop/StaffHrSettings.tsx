// Settings → Staff HR (P17, §10.5). A card in the Settings grid (like
// PaymentsSettings / PrivacySettings), shown only when the `hr` module is on.
// Real controls: school start time + late grace + "allow check-in away" (via
// get_hr_settings / set_hr_settings), and the leave types with yearly quota +
// paid/unpaid + active (list_leave_types / save_leave_type). **[OWNER defaults]**
// Casual 12 paid, Sick 6 paid, Unpaid; start 09:00, grace 0, away off.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, LeaveTypeDto } from '@/lib/api'
import { t, useLang } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24, gridColumn: '1 / -1' }
const SERIF = "'Newsreader', Georgia, serif"
const FIELD: React.CSSProperties = { height: 42, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', padding: '0 12px', fontSize: 14, color: 'var(--ink)', boxSizing: 'border-box' }

function Toggle({ on, onClick, label }: { on: boolean; onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={onClick}
      style={{ width: 46, height: 26, borderRadius: 13, cursor: 'pointer', position: 'relative', flexShrink: 0, border: `1px solid ${on ? 'var(--accent)' : 'var(--line-strong)'}`, background: on ? 'var(--accent)' : 'var(--white)', transition: 'background-color .18s ease' }}
    >
      <span style={{ position: 'absolute', top: 2, left: on ? 22 : 2, width: 20, height: 20, borderRadius: 10, background: 'var(--white)', boxShadow: '0 1px 2px rgba(11,26,51,0.3)', transition: 'left .18s ease' }} />
    </button>
  )
}

// One editable leave-type row (draft rows have id === undefined until saved).
interface Draft {
  id?: string
  name: string
  name_hi: string | null
  name_te: string | null
  quota: string // text so "" = unlimited
  paid: boolean
  active: boolean
}

function toDraft(l: LeaveTypeDto): Draft {
  return { id: l.id, name: l.name, name_hi: l.name_hi, name_te: l.name_te, quota: l.yearly_quota == null ? '' : String(l.yearly_quota), paid: l.paid, active: l.active }
}

export default function StaffHrSettings() {
  useLang()
  const [start, setStart] = useState('09:00')
  const [grace, setGrace] = useState('0')
  const [allowAway, setAllowAway] = useState(false)
  const [status, setStatus] = useState<'idle' | 'saved' | 'invalid'>('idle')
  const [types, setTypes] = useState<Draft[]>([])

  const loadTypes = () => api.list_leave_types().then((rows) => setTypes(rows.map(toDraft))).catch(() => setTypes([]))

  useEffect(() => {
    api.get_hr_settings().then((s) => { setStart(s.start_time); setGrace(String(s.grace_min)); setAllowAway(s.allow_away) }).catch(() => {})
    loadTypes()
  }, [])

  const saveSettings = async () => {
    setStatus('idle')
    try {
      await api.set_hr_settings({ start_time: start.trim(), grace_min: Math.max(0, Number(grace) || 0), allow_away: allowAway })
      setStatus('saved')
    } catch (e) {
      setStatus((e as CmdError).code === 'VALIDATION' ? 'invalid' : 'idle')
    }
  }

  const patch = (i: number, p: Partial<Draft>) => setTypes((ts) => ts.map((t, j) => (j === i ? { ...t, ...p } : t)))

  const saveType = async (i: number) => {
    const d = types[i]
    if (!d.name.trim()) return
    const quota = d.quota.trim() === '' ? null : Math.max(0, Number(d.quota) || 0)
    try {
      await api.save_leave_type({ id: d.id, name: d.name.trim(), name_hi: d.name_hi, name_te: d.name_te, yearly_quota: quota, paid: d.paid, active: d.active })
      loadTypes()
    } catch (e) {
      void (e as CmdError)
    }
  }

  const addRow = () => setTypes((ts) => [...ts, { name: '', name_hi: null, name_te: null, quota: '', paid: true, active: true }])

  return (
    <div style={CARD}>
      <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('staffhr.settings.eyebrow')}</div>
      <h3 style={{ fontFamily: SERIF, fontSize: 22, fontWeight: 400, margin: '2px 0 16px' }}>{t('staffhr.settings.title')}</h3>

      {/* Check-in policy */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 24, alignItems: 'flex-end' }}>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('staffhr.settings.start')}</span>
          <input type="time" value={start} onChange={(e) => setStart(e.target.value)} style={{ ...FIELD, width: 130 }} />
        </label>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('staffhr.settings.grace')}</span>
          <input inputMode="numeric" value={grace} onChange={(e) => setGrace(e.target.value.replace(/[^0-9]/g, ''))} style={{ ...FIELD, width: 90 }} />
        </label>
        <span style={{ display: 'flex', alignItems: 'center', gap: 12, height: 42 }}>
          <Toggle on={allowAway} onClick={() => setAllowAway((v) => !v)} label={t('staffhr.settings.allowAway')} />
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('staffhr.settings.allowAway')}</span>
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
          <button type="button" onClick={saveSettings} style={{ height: 42, padding: '0 20px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 14, fontWeight: 500, cursor: 'pointer' }}>{t('staffhr.settings.save')}</button>
          {status === 'saved' ? <span style={{ fontSize: 13, color: 'var(--accent)' }}>{t('staffhr.settings.saved')}</span> : null}
          {status === 'invalid' ? <span style={{ fontSize: 13, color: 'var(--danger)' }}>{t('staffhr.settings.invalid')}</span> : null}
        </div>
      </div>
      <p style={{ fontSize: 12, color: 'var(--muted)', margin: '10px 0 0' }}>{t('staffhr.settings.startHint')} {t('staffhr.settings.allowAwayHint')}</p>

      {/* Leave types */}
      <h4 style={{ fontSize: 15, fontWeight: 600, margin: '22px 0 4px' }}>{t('staffhr.leave.title')}</h4>
      <p style={{ fontSize: 12, color: 'var(--muted)', margin: '0 0 12px' }}>{t('staffhr.leave.hint')}</p>
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {types.map((d, i) => (
          <div key={d.id ?? `new-${i}`} style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 12, padding: '12px 0', borderTop: '1px solid var(--track)' }}>
            <input value={d.name} placeholder={t('staffhr.leave.namePlaceholder')} onChange={(e) => patch(i, { name: e.target.value })} style={{ ...FIELD, flex: '1 1 180px', minWidth: 140 }} aria-label={t('staffhr.leave.name')} />
            <label style={{ display: 'flex', flexDirection: 'column', fontSize: 11, color: 'var(--muted)', gap: 2 }}>
              {t('staffhr.leave.quota')}
              <input inputMode="numeric" value={d.quota} placeholder={t('staffhr.leave.unlimited')} onChange={(e) => patch(i, { quota: e.target.value.replace(/[^0-9]/g, '') })} style={{ ...FIELD, width: 110, height: 36 }} />
            </label>
            <button type="button" onClick={() => patch(i, { paid: !d.paid })} style={{ height: 30, padding: '0 12px', borderRadius: 15, fontSize: 12, fontWeight: 500, cursor: 'pointer', border: `1px solid ${d.paid ? 'var(--accent)' : 'var(--line-strong)'}`, background: d.paid ? 'var(--accent-12)' : 'var(--white)', color: d.paid ? 'var(--accent)' : 'var(--muted)' }}>
              {d.paid ? t('staffhr.leave.paid') : t('staffhr.leave.unpaid')}
            </button>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Toggle on={d.active} onClick={() => patch(i, { active: !d.active })} label={t('staffhr.leave.active')} />
              <span style={{ fontSize: 12, color: 'var(--muted)' }}>{d.active ? t('staffhr.leave.active') : t('staffhr.leave.inactive')}</span>
            </span>
            <button type="button" onClick={() => saveType(i)} style={{ height: 34, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('staffhr.leave.save')}</button>
          </div>
        ))}
      </div>
      <button type="button" onClick={addRow} style={{ marginTop: 14, height: 40, padding: '0 16px', borderRadius: 6, border: '1px dashed var(--line-strong)', background: 'transparent', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>+ {t('staffhr.leave.add')}</button>
    </div>
  )
}
