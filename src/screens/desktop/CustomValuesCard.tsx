// Custom values on a record's profile (P13, foundation §8.9). Renders the active
// custom fields for an entity with the record's current values; the Principal
// edits (student → EditStudentDetails, staff → ManageStaff). Read-only otherwise.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, CustomFieldValueDto } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }
const CARD_HEAD: React.CSSProperties = { padding: '16px 24px', borderBottom: '1px solid var(--track)', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }
const inputStyle: React.CSSProperties = { height: 34, padding: '0 10px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 13, minWidth: 160 }

export default function CustomValuesCard({ entity, entityId, canEdit }: { entity: string; entityId: string; canEdit: boolean }) {
  const [rows, setRows] = useState<CustomFieldValueDto[]>([])
  const [draft, setDraft] = useState<Record<string, string>>({})
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(() => {
    api.get_custom_values(entity, entityId)
      .then((r) => {
        setRows(r)
        setDraft(Object.fromEntries(r.map((x) => [x.field.id, x.value ?? ''])))
      })
      .catch(() => setRows([]))
  }, [entity, entityId])
  useEffect(() => { load() }, [load])

  if (rows.length === 0) return null // nothing to show until fields are defined

  const save = async () => {
    setError(null)
    setSaved(false)
    const values = rows.map((r) => ({ field_id: r.field.id, value: draft[r.field.id] ?? '' }))
    try {
      const updated = await api.set_custom_values(entity, entityId, values)
      setRows(updated)
      setSaved(true)
    } catch (e) {
      void (e as CmdError)
      setError(t('cf.values.error'))
    }
  }

  const set = (id: string, v: string) => { setDraft((d) => ({ ...d, [id]: v })); setSaved(false) }

  return (
    <div style={CARD}>
      <div style={CARD_HEAD}>{t('cf.values.section')}</div>
      <div style={{ padding: '16px 24px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px 24px' }}>
        {rows.map((r) => {
          const f = r.field
          const val = draft[f.id] ?? ''
          return (
            <label key={f.id} style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12, color: 'var(--muted)' }}>
              {f.label}{f.required ? ' *' : ''}
              {!canEdit ? (
                <span style={{ fontSize: 13, fontWeight: 500, color: 'var(--ink)' }}>{val || '—'}</span>
              ) : f.field_type === 'choice' ? (
                <select value={val} onChange={(e) => set(f.id, e.target.value)} style={inputStyle}>
                  <option value="">{t('cf.choose')}</option>
                  {f.options.map((o) => <option key={o} value={o}>{o}</option>)}
                </select>
              ) : (
                <input type={f.field_type === 'date' ? 'date' : f.field_type === 'number' ? 'text' : 'text'} inputMode={f.field_type === 'number' ? 'decimal' : undefined} value={val} onChange={(e) => set(f.id, e.target.value)} style={inputStyle} />
              )}
            </label>
          )
        })}
      </div>
      {canEdit ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '0 24px 16px' }}>
          <button type="button" onClick={save} style={{ height: 36, padding: '0 16px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('cf.values.save')}</button>
          {saved ? <span style={{ fontSize: 13, color: 'var(--accent)' }}>{t('cf.values.saved')}</span> : null}
          {error ? <span style={{ fontSize: 12, color: 'var(--danger)' }}>{error}</span> : null}
        </div>
      ) : null}
    </div>
  )
}
