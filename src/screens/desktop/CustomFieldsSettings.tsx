// Custom fields management (P13, foundation §8.9) — Settings → Custom fields.
// Principal defines extra student/staff fields (text/number/date/choice) with a
// label; the `key` is the CSV column header. The commands re-check the permission
// and audit every change. Derived screen (cards, matching Settings).

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, CustomFieldDto, CustomFieldInput } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 14px' }
const linkBtn: React.CSSProperties = { background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }
const fieldLabel: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }
const inputStyle: React.CSSProperties = { height: 38, padding: '0 12px', borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14 }
const TYPES = ['text', 'number', 'date', 'choice'] as const

const emptyInput = (entity: string): CustomFieldInput => ({ entity, key: '', label: '', field_type: 'text', options: [], required: false })

function EntitySection({ entity, title }: { entity: string; title: string }) {
  const [fields, setFields] = useState<CustomFieldDto[]>([])
  const [editing, setEditing] = useState<{ id: string | null; input: CustomFieldInput; optionsText: string } | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_custom_fields(entity, true).then(setFields).catch(() => setFields([]))
  }, [entity])
  useEffect(() => { load() }, [load])

  const save = async () => {
    if (!editing) return
    setError(null)
    const input: CustomFieldInput = { ...editing.input, options: editing.optionsText.split('\n').map((s) => s.trim()).filter(Boolean) }
    try {
      if (editing.id) await api.update_custom_field(editing.id, input)
      else await api.create_custom_field(input)
      setEditing(null)
      load()
    } catch (e) {
      void (e as CmdError)
      setError(t('cf.error'))
    }
  }

  const toggle = async (f: CustomFieldDto) => {
    try {
      await api.set_custom_field_active(f.id, !f.active)
      load()
    } catch (e) {
      void (e as CmdError)
    }
  }

  const set = (patch: Partial<CustomFieldInput>) => setEditing((e) => (e ? { ...e, input: { ...e.input, ...patch } } : e))

  return (
    <div style={{ ...CARD, gridColumn: '1 / -1' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h3 style={{ ...H3, margin: 0 }}>{title}</h3>
        <button type="button" style={linkBtn} onClick={() => { setError(null); setEditing({ id: null, input: emptyInput(entity), optionsText: '' }) }}>+ {t('cf.add')}</button>
      </div>
      <div style={{ marginTop: 12 }}>
        {fields.length === 0 ? (
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>{t('cf.empty')}</p>
        ) : (
          fields.map((f, i) => (
            <div key={f.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '11px 0', ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}), opacity: f.active ? 1 : 0.55 }}>
              <div style={{ flexGrow: 1 }}>
                <span style={{ fontSize: 14, fontWeight: 500 }}>{f.label}</span>
                <span style={{ fontSize: 12, color: 'var(--muted)' }}> · {f.key} · {t(`cf.type.${f.field_type}`)}{f.required ? ` · ${t('cf.required')}` : ''}</span>
              </div>
              <button type="button" style={linkBtn} onClick={() => { setError(null); setEditing({ id: f.id, input: { entity, key: f.key, label: f.label, label_hi: f.label_hi, label_te: f.label_te, field_type: f.field_type, options: f.options, required: f.required }, optionsText: f.options.join('\n') }) }}>{t('cf.edit')}</button>
              <button type="button" style={linkBtn} onClick={() => toggle(f)}>{f.active ? t('cf.disable') : t('cf.enable')}</button>
            </div>
          ))
        )}
      </div>

      {editing ? (
        <div onClick={() => setEditing(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 440, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{title}</div>
            <div style={{ display: 'flex', gap: 12 }}>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('cf.key')}
                <input value={editing.input.key} disabled={!!editing.id} onChange={(e) => set({ key: e.target.value })} style={{ ...inputStyle, opacity: editing.id ? 0.6 : 1 }} />
              </label>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('cf.label')}
                <input value={editing.input.label} onChange={(e) => set({ label: e.target.value })} style={inputStyle} />
              </label>
            </div>
            <label style={fieldLabel}>{t('cf.type')}
              <select value={editing.input.field_type} disabled={!!editing.id} onChange={(e) => set({ field_type: e.target.value })} style={{ ...inputStyle, opacity: editing.id ? 0.6 : 1 }}>
                {TYPES.map((ty) => <option key={ty} value={ty}>{t(`cf.type.${ty}`)}</option>)}
              </select>
            </label>
            {editing.input.field_type === 'choice' ? (
              <label style={fieldLabel}>{t('cf.options')}
                <textarea value={editing.optionsText} onChange={(e) => setEditing((ed) => (ed ? { ...ed, optionsText: e.target.value } : ed))} rows={4} style={{ ...inputStyle, height: 'auto', padding: '8px 12px', resize: 'vertical', fontFamily: 'inherit' }} />
              </label>
            ) : null}
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
              <input type="checkbox" checked={editing.input.required ?? false} onChange={(e) => set({ required: e.target.checked })} />
              {t('cf.required')}
            </label>
            {error ? <div style={{ fontSize: 12, color: 'var(--danger)' }}>{error}</div> : null}
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setEditing(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('cf.cancel')}</button>
              <button type="button" onClick={save} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('cf.save')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}

export default function CustomFieldsSettings() {
  return (
    <>
      <EntitySection entity="student" title={t('cf.student')} />
      <EntitySection entity="staff" title={t('cf.staff')} />
    </>
  )
}
