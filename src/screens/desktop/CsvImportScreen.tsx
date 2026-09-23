import { useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { pickOpenPath, pickSavePath } from '@/lib/files'
import * as api from '@/lib/api'
import type { ImportPreviewDto, ImportResultDto } from '@/lib/api'

// CSV import (prompts/P07 §3): download template → choose file → dry-run preview
// (valid count, row errors with column + reason, duplicate candidates) → import
// N students in ONE transaction → result with any skipped rows.

function btn(primary: boolean): CSSProperties {
  return {
    height: '44px',
    padding: '0 18px',
    borderRadius: '6px',
    border: `1px solid ${primary ? 'var(--accent)' : 'var(--line-strong)'}`,
    background: primary ? 'var(--accent)' : 'var(--surface)',
    color: primary ? 'var(--white)' : 'var(--ink)',
    fontSize: '14px',
    fontWeight: 500,
    cursor: 'pointer',
    display: 'inline-flex',
    alignItems: 'center',
    gap: '8px',
  }
}

const HEAD: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '60px 1.4fr 1fr 2fr',
  gap: '16px',
  padding: '12px 24px',
  fontSize: '10px',
  fontWeight: 600,
  letterSpacing: '0.16em',
  textTransform: 'uppercase',
  color: 'var(--muted)',
  borderBottom: '1px solid var(--track)',
}

function errText(reason: string): string {
  const key = `csv.err.${reason}`
  const s = t(key)
  return s === key ? reason : s
}

export default function CsvImportScreen({ role }: { role: string }) {
  const base = role === 'accountant' ? '/accountant/students' : '/principal/students'
  const [path, setPath] = useState<string | null>(null)
  const [preview, setPreview] = useState<ImportPreviewDto | null>(null)
  const [result, setResult] = useState<ImportResultDto | null>(null)
  const [busy, setBusy] = useState(false)
  const [toast, setToast] = useState<string | null>(null)

  const flash = (m: string) => {
    setToast(m)
    window.setTimeout(() => setToast(null), 2600)
  }

  const downloadTemplate = async () => {
    const p = await pickSavePath('students-template.csv')
    if (!p) return
    try {
      await api.students_csv_template(p)
      flash(t('csv.templateSaved'))
    } catch {
      /* ignore */
    }
  }

  const chooseFile = async () => {
    const p = await pickOpenPath()
    if (!p) return
    setPath(p)
    setResult(null)
    setPreview(null)
    setBusy(true)
    try {
      setPreview(await api.import_students_dry_run(p))
    } catch (e) {
      const ce = e as api.CmdError
      flash(t(ce.message_key, ce.vars as Record<string, string | number>))
    } finally {
      setBusy(false)
    }
  }

  const doImport = async () => {
    if (!path) return
    setBusy(true)
    try {
      const r = await api.import_students_commit(path)
      setResult(r)
      flash(t('csv.imported', { n: r.imported, skipped: r.skipped.length }))
    } catch (e) {
      const ce = e as api.CmdError
      flash(t(ce.message_key, ce.vars as Record<string, string | number>))
    } finally {
      setBusy(false)
    }
  }

  const fatal = preview?.error
  const fatalMsg = fatal === 'too_many_rows' ? t('csv.tooManyRows') : fatal ? t('csv.notUtf8') : null

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <button type="button" onClick={() => navigate(base)} style={{ display: 'flex', alignItems: 'center', gap: '6px', border: 'none', background: 'transparent', color: 'var(--muted)', fontSize: '13px', cursor: 'pointer', alignSelf: 'flex-start' }}>
        <Icon name="back" size={16} strokeWidth={1.75} />
        {t('students.back')}
      </button>

      <PageTitle
        eyebrow={t('students.eyebrow')}
        title={t('csv.title')}
        sub={t('csv.sub')}
        actions={
          <>
            <button type="button" style={btn(false)} onClick={downloadTemplate}>{t('csv.downloadTemplate')}</button>
            <button type="button" style={btn(false)} onClick={chooseFile}>{t('csv.chooseFile')}</button>
          </>
        }
      />

      {fatalMsg ? <div style={{ color: 'var(--pill-unpaid-fg)', fontSize: '14px' }}>{fatalMsg}</div> : null}

      {preview && !fatalMsg ? (
        <>
          <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
            <span style={{ fontSize: '14px', color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>
              {t('csv.previewTotal', { total: preview.total, valid: preview.valid })}
            </span>
            <button type="button" disabled={preview.valid === 0 || busy || result != null} style={{ ...btn(true), marginLeft: 'auto', opacity: preview.valid === 0 || busy || result != null ? 0.5 : 1 }} onClick={doImport}>
              {t('csv.importN', { n: preview.valid })}
            </button>
          </div>

          <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
            <div style={HEAD}>
              <span>{t('csv.col.row')}</span>
              <span>{t('csv.col.name')}</span>
              <span>{t('csv.col.class')}</span>
              <span>{t('csv.col.issues')}</span>
            </div>
            {preview.rows.map((r) => (
              <div key={r.row} style={{ display: 'grid', gridTemplateColumns: '60px 1.4fr 1fr 2fr', gap: '16px', alignItems: 'center', padding: '12px 24px', borderTop: '1px solid var(--track)', fontSize: '14px' }}>
                <span style={{ color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{r.row}</span>
                <span style={{ fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.name || '—'}</span>
                <span style={{ color: 'var(--muted)' }}>{r.class || '—'}</span>
                <span style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                  {r.errors.length === 0 && !r.duplicate ? (
                    <Pill variant="paid">{t('csv.ok')}</Pill>
                  ) : (
                    <>
                      {r.errors.map((e, i) => (
                        <Pill key={i} variant="unpaid">{e.column}: {errText(e.reason)}</Pill>
                      ))}
                      {r.duplicate ? <Pill variant="partpaid">{t('csv.duplicate')}</Pill> : null}
                    </>
                  )}
                </span>
              </div>
            ))}
          </div>
        </>
      ) : null}

      {result ? (
        <div style={{ fontSize: '14px', color: 'var(--ink)' }}>
          {t('csv.imported', { n: result.imported, skipped: result.skipped.length })}
        </div>
      ) : null}

      {toast ? (
        <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div>
      ) : null}
    </div>
  )
}
