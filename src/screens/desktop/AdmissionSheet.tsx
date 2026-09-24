import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ClassDto, StudentRowDto } from '@/lib/api'

// New admission (prompts/P07 §2). Collect-fee Sheet pattern (520px right panel,
// radius 20, footer band). Full form; duplicate check shows candidates before
// saving; offline → provisional number (backend). Principal + Accountant.

const label: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }
function field(): CSSProperties {
  return { height: '46px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px', boxSizing: 'border-box' }
}

export default function AdmissionSheet({ role }: { role: string }) {
  const base = role === 'accountant' ? '/accountant/students' : '/principal/students'
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [f, setF] = useState({ name: '', classId: '', roll: '', guardianName: '', guardianMobile: '', dob: '', gender: '', address: '', transport: false, rte: false, category: '', aadhaar: 'none' })
  const [dups, setDups] = useState<StudentRowDto[] | null>(null)
  const [ack, setAck] = useState(false)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    api.list_classes().then(setClasses).catch(() => setClasses([]))
  }, [])

  const set = (patch: Partial<typeof f>) => {
    setF((prev) => ({ ...prev, ...patch }))
    setDups(null)
    setAck(false)
  }
  const close = () => navigate(base)

  const save = async () => {
    setErr(null)
    if (f.name.trim() === '') return setErr(t('students.admission.needName'))
    if (f.classId === '') return setErr(t('students.admission.needClass'))
    // Duplicate check before saving (unless already acknowledged).
    if (!ack) {
      const candidates = await api.check_duplicate_students(f.name.trim(), f.dob || undefined, f.guardianMobile || undefined).catch(() => [])
      if (candidates.length > 0) {
        setDups(candidates)
        setAck(true) // next click saves anyway
        return
      }
    }
    setBusy(true)
    try {
      const created = await api.create_student({
        name: f.name.trim(),
        class_id: f.classId,
        roll_no: f.roll ? Number(f.roll) : null,
        guardian_name: f.guardianName || null,
        guardian_mobile: f.guardianMobile || null,
        dob: f.dob || null,
        gender: f.gender || null,
        address: f.address || null,
        transport: f.transport,
        rte: f.rte,
        category: f.category || null,
        aadhaar_status: f.aadhaar,
      })
      navigate(`${base}/${created.id}`)
    } catch (e) {
      const ce = e as api.CmdError
      setErr(t(ce.message_key, ce.vars as Record<string, string | number>))
      setBusy(false)
    }
  }

  return (
    <div onClick={close} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 40 }}>
      <div
        onClick={(e) => e.stopPropagation()}
        style={{ position: 'absolute', top: 0, right: 0, width: '520px', margin: '16px', marginLeft: 0, height: 'calc(100vh - 32px)', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}
      >
        <div style={{ padding: '24px 24px 0', display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>{t('students.eyebrow')}</span>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            <span style={{ fontFamily: 'var(--font-serif)', fontSize: '30px', color: 'var(--ink)', letterSpacing: '-0.02em' }}>{t('students.admission.title')}</span>
            <span style={{ fontFamily: 'var(--font-serif)', fontStyle: 'italic', fontSize: '22px', color: 'var(--accent)' }}>{t('students.admission.sub')}</span>
          </div>
        </div>

        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: '14px', overflowY: 'auto', flex: 1 }}>
          <label style={label}>{t('students.admission.name')}<input value={f.name} onChange={(e) => set({ name: e.target.value })} style={field()} /></label>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 100px', gap: '12px' }}>
            <label style={label}>{t('students.field.class')}
              <select value={f.classId} onChange={(e) => set({ classId: e.target.value })} style={field()}>
                <option value="">—</option>
                {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
              </select>
            </label>
            <label style={label}>{t('students.field.roll')}<input inputMode="numeric" value={f.roll} onChange={(e) => set({ roll: e.target.value.replace(/\D/g, '') })} style={field()} /></label>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
            <label style={label}>{t('students.field.dob')}<input type="date" value={f.dob} onChange={(e) => set({ dob: e.target.value })} style={field()} /></label>
            <label style={label}>{t('students.field.gender')}
              <select value={f.gender} onChange={(e) => set({ gender: e.target.value })} style={field()}>
                <option value="">—</option>
                <option value="male">{t('students.gender.male')}</option>
                <option value="female">{t('students.gender.female')}</option>
                <option value="other">{t('students.gender.other')}</option>
              </select>
            </label>
          </div>
          <label style={label}>{t('students.field.guardian')}<input value={f.guardianName} onChange={(e) => set({ guardianName: e.target.value })} style={field()} /></label>
          <label style={label}>{t('students.field.mobile')}<input inputMode="numeric" value={f.guardianMobile} onChange={(e) => set({ guardianMobile: e.target.value.replace(/\D/g, '').slice(0, 10) })} style={field()} /></label>
          <label style={label}>{t('students.field.address')}<input value={f.address} onChange={(e) => set({ address: e.target.value })} style={field()} /></label>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
            <label style={label}>{t('students.field.category')}<input value={f.category} onChange={(e) => set({ category: e.target.value })} style={field()} /></label>
            <label style={label}>{t('students.field.aadhaar')}
              <select value={f.aadhaar} onChange={(e) => set({ aadhaar: e.target.value })} style={field()}>
                <option value="none">{t('students.aadhaar.none')}</option>
                <option value="submitted">{t('students.aadhaar.submitted')}</option>
                <option value="verified">{t('students.aadhaar.verified')}</option>
              </select>
            </label>
          </div>
          <div style={{ display: 'flex', gap: '20px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '14px', color: 'var(--ink)', cursor: 'pointer' }}>
              <input type="checkbox" checked={f.transport} onChange={(e) => set({ transport: e.target.checked })} />{t('students.field.transport')}
            </label>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '14px', color: 'var(--ink)', cursor: 'pointer' }}>
              <input type="checkbox" checked={f.rte} onChange={(e) => set({ rte: e.target.checked })} />{t('students.field.rte')}
            </label>
          </div>

          {dups && dups.length > 0 ? (
            <div style={{ border: '1px solid var(--gold-line)', background: 'var(--unmarked)', borderRadius: '10px', padding: '12px 14px' }}>
              <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--gold-text)' }}>{t('students.admission.dupTitle')}</div>
              <div style={{ fontSize: '12px', color: 'var(--muted)', margin: '4px 0 8px' }}>{t('students.admission.dupHint')}</div>
              {dups.map((d) => (
                <button key={d.id} type="button" onClick={() => navigate(`${base}/${d.id}`)} style={{ display: 'flex', justifyContent: 'space-between', width: '100%', border: 'none', background: 'transparent', padding: '6px 0', fontSize: '13px', color: 'var(--ink)', cursor: 'pointer', textAlign: 'left' }}>
                  <span>{d.name}</span>
                  <span style={{ color: 'var(--muted)' }}>{d.class_display ?? '—'} · {d.guardian_name ?? ''}</span>
                </button>
              ))}
            </div>
          ) : null}
          {err ? <div style={{ fontSize: '13px', color: 'var(--pill-unpaid-fg)' }}>{err}</div> : null}
        </div>

        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" onClick={close} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('students.cancel')}</button>
          <button type="button" onClick={save} disabled={busy} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.6 : 1 }}>
            {busy ? t('students.admission.saving') : ack ? t('students.admission.saveAnyway') : t('students.admission.save')}
          </button>
        </div>
      </div>
    </div>
  )
}
