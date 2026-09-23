import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { GradeBandDto } from '@/lib/api'

// Grade scale editor (prompts/P07 §7): edit bands with validation (no gaps or
// overlaps, 0–100). % shown to one decimal (tenths under the hood).

function field(): CSSProperties {
  return { height: '38px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 10px', width: '100%', boxSizing: 'border-box' }
}

// A band edited in whole-percent (UI) → tenths (backend).
interface Band {
  min: string
  max: string
  grade: string
  point: string
}

export default function GradeScaleScreen() {
  const [bands, setBands] = useState<Band[]>([])
  const [err, setErr] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)

  useEffect(() => {
    api.list_grade_bands().then((bs) => setBands(bs.map(toUi))).catch(() => setBands([]))
  }, [])

  const toUi = (b: GradeBandDto): Band => ({ min: String(b.min_pct / 10), max: String(b.max_pct / 10), grade: b.grade, point: b.grade_point == null ? '' : String(b.grade_point) })

  const setAt = (i: number, patch: Partial<Band>) => setBands((prev) => prev.map((b, j) => (j === i ? { ...b, ...patch } : b)))
  const remove = (i: number) => setBands((prev) => prev.filter((_, j) => j !== i))
  const add = () => setBands((prev) => [...prev, { min: '', max: '', grade: '', point: '' }])

  const save = async () => {
    setErr(null)
    const payload: GradeBandDto[] = bands.map((b) => ({
      min_pct: Math.round(Number(b.min) * 10),
      max_pct: Math.round(Number(b.max) * 10),
      grade: b.grade.trim(),
      grade_point: b.point.trim() === '' ? null : Number(b.point),
    }))
    try {
      const saved = await api.update_grade_bands(payload)
      setBands(saved.map(toUi))
      setToast(t('grade.saved'))
      window.setTimeout(() => setToast(null), 2600)
    } catch (e) {
      const ce = e as api.CmdError
      const reason = (ce.vars as { rule?: string } | undefined)?.rule
      const key = reason ? `grade.err.${reason}` : ''
      const msg = key && t(key) !== key ? t(key) : t('grade.err.range')
      // vars for VALIDATION carry {field, rule}; map the rule to a message.
      setErr(msg)
    }
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle eyebrow={t('grade.eyebrow')} title={t('grade.title')} sub={t('grade.sub')} />

      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr 80px', gap: '12px', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('grade.min')}</span>
          <span>{t('grade.max')}</span>
          <span>{t('grade.grade')}</span>
          <span>{t('grade.point')}</span>
          <span />
        </div>
        {bands.map((b, i) => (
          <div key={i} style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr 80px', gap: '12px', alignItems: 'center', padding: '8px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none' }}>
            <input inputMode="decimal" value={b.min} onChange={(e) => setAt(i, { min: e.target.value.replace(/[^0-9.]/g, '') })} style={field()} aria-label={t('grade.min')} />
            <input inputMode="decimal" value={b.max} onChange={(e) => setAt(i, { max: e.target.value.replace(/[^0-9.]/g, '') })} style={field()} aria-label={t('grade.max')} />
            <input value={b.grade} onChange={(e) => setAt(i, { grade: e.target.value })} style={field()} aria-label={t('grade.grade')} />
            <input inputMode="numeric" value={b.point} onChange={(e) => setAt(i, { point: e.target.value.replace(/\D/g, '') })} style={field()} aria-label={t('grade.point')} />
            <button type="button" onClick={() => remove(i)} style={{ height: '38px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--muted)', fontSize: '12px', cursor: 'pointer' }}>{t('grade.remove')}</button>
          </div>
        ))}
      </div>

      {err ? <div style={{ color: 'var(--pill-unpaid-fg)', fontSize: '14px' }}>{err}</div> : null}

      <div style={{ display: 'flex', gap: '12px' }}>
        <button type="button" onClick={add} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('grade.addBand')}</button>
        <button type="button" onClick={save} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('grade.save')}</button>
      </div>

      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}
