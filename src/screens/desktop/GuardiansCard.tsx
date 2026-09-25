// Guardians section of the student profile (P13, foundation §8.2). Displays the
// student's guardians (primary first) and, for the Principal, lets them add (up to
// two), edit, set-primary and remove. The commands re-check the permission and
// audit every change; each returns the fresh list.

import { useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, GuardianDto, GuardianEditInput } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }
const CARD_HEAD: React.CSSProperties = { padding: '14px 24px', borderBottom: '1px solid var(--line)', fontSize: 15, fontWeight: 600, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }
const linkBtn: React.CSSProperties = { background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }
const fieldLabel: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }
const inputStyle: React.CSSProperties = { height: 38, padding: '0 12px', borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14 }

const emptyInput = (): GuardianEditInput => ({ name: '', relation: '', mobile: '', email: '', language: 'en', whatsapp_ok: true })

export default function GuardiansCard({
  studentId,
  guardians,
  canEdit,
  onChanged,
}: {
  studentId: string
  guardians: GuardianDto[]
  canEdit: boolean
  onChanged: (list: GuardianDto[]) => void
}) {
  const [editing, setEditing] = useState<{ id: string | null; input: GuardianEditInput } | null>(null)
  const [error, setError] = useState<string | null>(null)

  const save = async () => {
    if (!editing) return
    setError(null)
    try {
      const list = editing.id
        ? await api.update_guardian(studentId, editing.id, editing.input)
        : await api.add_guardian(studentId, editing.input)
      onChanged(list)
      setEditing(null)
    } catch (e) {
      void (e as CmdError)
      setError(t('gd.error'))
    }
  }

  const makePrimary = async (g: GuardianDto) => {
    try {
      onChanged(await api.set_primary_guardian(studentId, g.id))
    } catch (e) {
      void (e as CmdError)
    }
  }

  const remove = async (g: GuardianDto) => {
    // eslint-disable-next-line no-alert
    if (!window.confirm(t('gd.confirmRemove'))) return
    try {
      onChanged(await api.remove_guardian(studentId, g.id))
    } catch (e) {
      void (e as CmdError)
    }
  }

  const set = (patch: Partial<GuardianEditInput>) => setEditing((e) => (e ? { ...e, input: { ...e.input, ...patch } } : e))
  const canAdd = canEdit && guardians.length < 2

  return (
    <div style={CARD}>
      <div style={CARD_HEAD}>
        <span>{t('gd.section')}</span>
        {canAdd ? (
          <button type="button" style={linkBtn} onClick={() => { setError(null); setEditing({ id: null, input: emptyInput() }) }}>+ {t('gd.add')}</button>
        ) : null}
      </div>
      <div>
        {guardians.length === 0 ? (
          <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: 13 }}>{t('gd.empty')}</div>
        ) : (
          guardians.map((g, i) => (
            <div key={g.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 24px', ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}) }}>
              <div style={{ flexGrow: 1 }}>
                <div style={{ fontSize: 14, fontWeight: 500, display: 'flex', gap: 8, alignItems: 'center' }}>
                  {g.name}
                  {g.is_primary ? <span style={{ fontSize: 11, fontWeight: 500, color: 'var(--white)', background: 'var(--accent)', borderRadius: 4, padding: '1px 7px' }}>{t('gd.primary')}</span> : null}
                </div>
                <div style={{ fontSize: 12, color: 'var(--muted)' }}>
                  {[g.relation, g.mobile, g.email].filter(Boolean).join(' · ') || '—'}
                </div>
              </div>
              {canEdit ? (
                <>
                  {!g.is_primary ? <button type="button" style={linkBtn} onClick={() => makePrimary(g)}>{t('gd.makePrimary')}</button> : null}
                  <button type="button" style={linkBtn} onClick={() => { setError(null); setEditing({ id: g.id, input: { name: g.name, relation: g.relation ?? '', mobile: g.mobile ?? '', email: g.email ?? '', language: g.language, whatsapp_ok: g.whatsapp_ok } }) }}>{t('gd.edit')}</button>
                  <button type="button" style={{ ...linkBtn, color: 'var(--danger)' }} onClick={() => remove(g)}>{t('gd.remove')}</button>
                </>
              ) : null}
            </div>
          ))
        )}
      </div>

      {editing ? (
        <div onClick={() => setEditing(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 440, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{t('gd.section')}</div>
            <label style={fieldLabel}>{t('gd.name')}
              <input value={editing.input.name} onChange={(e) => set({ name: e.target.value })} style={inputStyle} />
            </label>
            <div style={{ display: 'flex', gap: 12 }}>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('gd.relation')}
                <input value={editing.input.relation ?? ''} onChange={(e) => set({ relation: e.target.value })} style={inputStyle} />
              </label>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('gd.mobile')}
                <input value={editing.input.mobile ?? ''} onChange={(e) => set({ mobile: e.target.value })} style={inputStyle} inputMode="numeric" />
              </label>
            </div>
            <label style={fieldLabel}>{t('gd.email')}
              <input value={editing.input.email ?? ''} onChange={(e) => set({ email: e.target.value })} style={inputStyle} inputMode="email" />
            </label>
            <label style={fieldLabel}>{t('gd.language')}
              <select value={editing.input.language ?? 'en'} onChange={(e) => set({ language: e.target.value })} style={inputStyle}>
                <option value="en">{t('gd.lang.en')}</option>
                <option value="hi">{t('gd.lang.hi')}</option>
                <option value="te">{t('gd.lang.te')}</option>
              </select>
            </label>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
              <input type="checkbox" checked={editing.input.whatsapp_ok ?? true} onChange={(e) => set({ whatsapp_ok: e.target.checked })} />
              {t('gd.whatsapp')}
            </label>
            {error ? <div style={{ fontSize: 12, color: 'var(--danger)' }}>{error}</div> : null}
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setEditing(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('gd.cancel')}</button>
              <button type="button" onClick={save} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('gd.save')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}
