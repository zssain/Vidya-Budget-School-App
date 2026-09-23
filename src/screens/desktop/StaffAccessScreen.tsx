// Staff & access (prompts/P04 Step 5), derived from the Collect-fee table + Sheet
// (docs §6.2). Staff list with role/state + assignments, Add staff (→ invite with
// QR + link), Suspend/Remove, class-teacher assignment, effective-access preview.
// A second Principal can never be created here (enforced server-side too).

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, ClassDto, InviteDto, StaffFullDto } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16 }
const SERIF = "'Newsreader', Georgia, serif"

function errMsg(e: CmdError): string {
  return t(e.message_key, e.vars as Record<string, string | number>)
}

function rolePill(role: string): React.CSSProperties {
  const map: Record<string, [string, string]> = {
    principal: ['var(--accent-12)', 'var(--accent)'],
    accountant: ['#F4ECDC', '#6B5220'],
    teacher: ['#E7E3F1', '#4A3B78'],
  }
  const [bg, fg] = map[role] ?? ['var(--panel)', 'var(--muted)']
  return { display: 'inline-flex', alignItems: 'center', height: 22, padding: '0 10px', borderRadius: 4, fontSize: 12, background: bg, color: fg }
}

export default function StaffAccessScreen() {
  const [staff, setStaff] = useState<StaffFullDto[]>([])
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [access, setAccess] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)
  const [invite, setInvite] = useState<InviteDto | null>(null)
  const [showAdd, setShowAdd] = useState(false)
  const [form, setForm] = useState({ name: '', role: 'teacher', mobile: '', email: '' })

  const load = () => {
    api.list_staff_access().then(setStaff).catch((e) => setError(errMsg(e as CmdError)))
    api.list_classes().then(setClasses).catch(() => {})
  }
  useEffect(load, [])

  useEffect(() => {
    if (selectedId) api.effective_access(selectedId).then(setAccess).catch(() => setAccess([]))
    else setAccess([])
  }, [selectedId])

  const guard = (p: Promise<unknown>) => {
    setError(null)
    p.then(() => load()).catch((e) => setError(errMsg(e as CmdError)))
  }

  const submitAdd = () => {
    setError(null)
    api
      .add_staff({ name: form.name, role: form.role, mobile: form.mobile, google_email: form.email || null })
      .then((inv) => {
        setInvite(inv)
        setShowAdd(false)
        setForm({ name: '', role: 'teacher', mobile: '', email: '' })
        load()
      })
      .catch((e) => setError(errMsg(e as CmdError)))
  }

  const input: React.CSSProperties = { height: 40, borderRadius: 6, border: '1px solid var(--line-strong)', padding: '0 12px', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, width: '100%', boxSizing: 'border-box' }
  const btn = (bg: string, fg: string): React.CSSProperties => ({ height: 34, padding: '0 14px', borderRadius: 6, border: bg === 'transparent' ? '1px solid var(--line-strong)' : 'none', background: bg, color: fg, fontSize: 13, fontWeight: 500, cursor: 'pointer' })

  const selected = staff.find((s) => s.id === selectedId)

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--ink)', padding: 40, fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end', marginBottom: 24 }}>
        <h1 style={{ fontFamily: SERIF, fontSize: 40, letterSpacing: '-0.025em', margin: 0 }}>{t('sa.title')}</h1>
        <button onClick={() => setShowAdd((v) => !v)} style={btn('var(--accent)', '#fff')}>{t('sa.add')}</button>
      </div>
      {error && <p style={{ color: 'var(--danger)', fontSize: 13 }}>{error}</p>}

      {showAdd && (
        <div style={{ ...CARD, padding: 20, marginBottom: 20, maxWidth: 520 }}>
          <div style={{ display: 'grid', gap: 12 }}>
            <label style={{ fontSize: 12, color: 'var(--muted)' }}>{t('sa.name')}<input style={input} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} /></label>
            <label style={{ fontSize: 12, color: 'var(--muted)' }}>{t('sa.role')}
              <select style={input} value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })}>
                <option value="teacher">Teacher</option>
                <option value="accountant">Accountant</option>
              </select>
            </label>
            <label style={{ fontSize: 12, color: 'var(--muted)' }}>{t('sa.mobile')}<input style={input} inputMode="numeric" value={form.mobile} onChange={(e) => setForm({ ...form, mobile: e.target.value })} /></label>
            <label style={{ fontSize: 12, color: 'var(--muted)' }}>{t('sa.email')}<input style={input} value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} /></label>
            <div style={{ display: 'flex', gap: 10 }}>
              <button onClick={submitAdd} style={btn('var(--accent)', '#fff')}>{t('sa.save')}</button>
              <button onClick={() => setShowAdd(false)} style={btn('transparent', 'var(--ink)')}>{t('sa.cancel')}</button>
            </div>
          </div>
        </div>
      )}

      <div style={{ display: 'flex', gap: 20 }}>
        {/* Staff list */}
        <div style={{ ...CARD, flex: 1, overflow: 'hidden' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr auto', padding: '12px 24px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
            <span>{t('sa.name')}</span><span>{t('sa.role')}</span><span>State</span><span />
          </div>
          {staff.map((s) => (
            <div key={s.id} onClick={() => setSelectedId(s.id)} style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr auto', alignItems: 'center', gap: 8, padding: '14px 24px', borderBottom: '1px solid var(--track)', cursor: 'pointer', background: selectedId === s.id ? 'var(--accent-6)' : 'transparent' }}>
              <div><div style={{ fontWeight: 500 }}>{s.name}</div><div style={{ fontSize: 12, color: 'var(--muted)' }}>{s.class_teacher_of.join(', ') || '—'}</div></div>
              <span><span style={rolePill(s.role)}>{s.role}</span></span>
              <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t(`sa.state.${s.state}`)}</span>
              <span style={{ display: 'flex', gap: 8 }}>
                {s.role !== 'principal' && s.state === 'active' && <button onClick={(e) => { e.stopPropagation(); guard(api.suspend_staff(s.id)) }} style={btn('transparent', 'var(--ink)')}>{t('sa.suspend')}</button>}
                {s.role !== 'principal' && <button onClick={(e) => { e.stopPropagation(); guard(api.remove_staff(s.id)) }} style={btn('transparent', 'var(--danger)')}>{t('sa.remove')}</button>}
                {s.state === 'invited' && <button onClick={(e) => { e.stopPropagation(); api.create_invite(s.id).then(setInvite).catch((x) => setError(errMsg(x as CmdError))) }} style={btn('transparent', 'var(--ink)')}>{t('sa.invite')}</button>}
              </span>
            </div>
          ))}
        </div>

        {/* Detail: effective access + assignments */}
        <div style={{ width: 360 }}>
          {selected && (
            <div style={{ ...CARD, padding: 20 }}>
              <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)', marginBottom: 8 }}>{t('sa.access')}</div>
              <div style={{ fontFamily: SERIF, fontSize: 22, marginBottom: 12 }}>{selected.name}</div>
              <ul style={{ margin: 0, paddingLeft: 18, fontSize: 14, color: 'var(--ink)' }}>{access.map((l, i) => <li key={i} style={{ marginBottom: 4 }}>{l}</li>)}</ul>
              {selected.role === 'teacher' && (
                <div style={{ marginTop: 16 }}>
                  <div style={{ fontSize: 12, color: 'var(--muted)', marginBottom: 8 }}>{t('sa.assignments')}</div>
                  {classes.map((c) => (
                    <button key={c.id} onClick={() => guard(api.set_class_teacher(c.id, selected.id))} style={{ ...btn('transparent', 'var(--ink)'), marginRight: 6, marginBottom: 6 }}>{c.display}</button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Invite result (code + link + QR) */}
      {invite && (
        <div role="dialog" style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 20 }} onClick={() => setInvite(null)}>
          <div style={{ ...CARD, padding: 24, width: 380, textAlign: 'center' }} onClick={(e) => e.stopPropagation()}>
            <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>{t('sa.inviteCode')}</div>
            <div style={{ fontFamily: 'monospace', fontSize: 26, letterSpacing: '0.1em', margin: '8px 0 16px' }}>{invite.code}</div>
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: 12 }} dangerouslySetInnerHTML={{ __html: invite.qr_svg }} />
            <div style={{ fontSize: 12, color: 'var(--muted)', wordBreak: 'break-all', marginBottom: 8 }}>{invite.link}</div>
            <div style={{ fontSize: 12, color: 'var(--muted)' }}>{t('sa.fingerprint')}: {invite.short_fingerprint || '—'}</div>
            <div style={{ marginTop: 16, display: 'flex', gap: 10, justifyContent: 'center' }}>
              <button onClick={() => { guard(api.revoke_invite(invite.staff_id)); setInvite(null) }} style={btn('transparent', 'var(--danger)')}>{t('sa.revokeInvite')}</button>
              <button onClick={() => setInvite(null)} style={btn('var(--accent)', '#fff')}>{t('sa.cancel')}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
