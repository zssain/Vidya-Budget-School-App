import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ClassDto, StudentRowDto } from '@/lib/api'

// Students list (prompts/P07 §1). Collect-fee table pattern: 10px uppercase
// headers, 14px/24px rows, top borders --track, status pill. Filters (class,
// section, status), FTS search, real pagination with the total count.

const PAGE = 25
type Status = 'active' | 'left' | 'all'

const GRID = '2.4fr 1fr 0.6fr 1.6fr 0.9fr'

const HEAD: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: GRID,
  gap: '16px',
  padding: '12px 24px',
  fontSize: '10px',
  fontWeight: 600,
  letterSpacing: '0.16em',
  textTransform: 'uppercase',
  color: 'var(--muted)',
  borderBottom: '1px solid var(--track)',
}

const ELLIPSIS: CSSProperties = { overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }

function fieldStyle(): CSSProperties {
  return {
    height: '38px',
    borderRadius: '6px',
    border: '1px solid var(--line-strong)',
    background: 'var(--surface)',
    color: 'var(--ink)',
    fontSize: '13px',
    fontFamily: 'inherit',
    padding: '0 12px',
    cursor: 'pointer',
  }
}

function segItem(selected: boolean): CSSProperties {
  return {
    minWidth: '64px',
    padding: '0 14px',
    height: '38px',
    fontSize: '13px',
    fontWeight: 500,
    border: 'none',
    borderRight: '1px solid var(--line-strong)',
    background: selected ? 'var(--accent)' : 'transparent',
    color: selected ? 'var(--white)' : 'var(--ink)',
    cursor: 'pointer',
  }
}

export default function StudentsScreen({ role }: { role: string }) {
  const base = role === 'accountant' ? '/accountant/students' : '/principal/students'
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [classId, setClassId] = useState('')
  const [section, setSection] = useState('')
  const [status, setStatus] = useState<Status>('active')
  const [query, setQuery] = useState('')
  const [debounced, setDebounced] = useState('')
  const [offset, setOffset] = useState(0)
  const [rows, setRows] = useState<StudentRowDto[]>([])
  const [total, setTotal] = useState(0)
  const [error, setError] = useState(false)

  useEffect(() => {
    api.list_classes().then(setClasses).catch(() => setClasses([]))
  }, [])

  // Debounce the search box (FTS on the server).
  useEffect(() => {
    const id = window.setTimeout(() => setDebounced(query.trim()), 250)
    return () => window.clearTimeout(id)
  }, [query])

  // Reset to page 1 whenever a filter changes.
  useEffect(() => {
    setOffset(0)
  }, [classId, section, status, debounced])

  const fetchPage = useCallback(() => {
    api
      .list_students_page({
        class_id: classId || null,
        section: section || null,
        status: status === 'all' ? null : status,
        query: debounced || null,
        limit: PAGE,
        offset,
      })
      .then((p) => {
        setRows(p.rows)
        setTotal(p.total)
        setError(false)
      })
      .catch(() => {
        setRows([])
        setTotal(0)
        setError(true)
      })
  }, [classId, section, status, debounced, offset])

  useEffect(fetchPage, [fetchPage])

  const sections = useMemo(() => {
    const set = new Set<string>()
    for (const c of classes) if (c.section) set.add(c.section)
    return Array.from(set).sort()
  }, [classes])

  const from = total === 0 ? 0 : offset + 1
  const to = Math.min(offset + PAGE, total)

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('students.eyebrow')}
        title={t('students.title')}
        sub={t('students.sub')}
        actions={
          <button
            type="button"
            onClick={() => navigate(`${base}/new`)}
            style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}
          >
            <Icon name="plus" size={16} strokeWidth={1.75} />
            {t('students.newAdmission')}
          </button>
        }
      />

      {/* Filters */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flexWrap: 'wrap' }}>
        <select aria-label={t('students.col.class')} value={classId} onChange={(e) => setClassId(e.target.value)} style={fieldStyle()}>
          <option value="">{t('students.filter.allClasses')}</option>
          {classes.map((c) => (
            <option key={c.id} value={c.id}>
              {c.display}
            </option>
          ))}
        </select>
        <select aria-label={t('students.col.status')} value={section} onChange={(e) => setSection(e.target.value)} style={fieldStyle()}>
          <option value="">{t('students.filter.allSections')}</option>
          {sections.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <div style={{ display: 'inline-flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
          {(['active', 'left', 'all'] as Status[]).map((s) => (
            <button key={s} type="button" aria-pressed={status === s} style={segItem(status === s)} onClick={() => setStatus(s)}>
              {t(`students.status.${s}`)}
            </button>
          ))}
        </div>
        <div style={{ position: 'relative', marginLeft: 'auto', width: '320px' }}>
          <Icon name="search" size={16} strokeWidth={1.75} color="var(--muted)" style={{ position: 'absolute', left: '12px', top: '11px' }} />
          <input
            type="search"
            aria-label={t('students.search')}
            placeholder={t('students.search')}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{ width: '100%', height: '38px', boxSizing: 'border-box', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', padding: '0 12px 0 36px', fontFamily: 'inherit', fontSize: '14px', color: 'var(--ink)' }}
          />
        </div>
      </div>

      {/* Table */}
      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        <div style={HEAD}>
          <span>{t('students.col.student')}</span>
          <span>{t('students.col.class')}</span>
          <span>{t('students.col.roll')}</span>
          <span>{t('students.col.guardian')}</span>
          <span>{t('students.col.status')}</span>
        </div>
        {rows.length === 0 ? (
          <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)', fontSize: '14px' }}>
            {error ? t('students.loadError') : t('students.none')}
          </div>
        ) : (
          rows.map((s) => {
            const adm = s.admission_no ?? s.provisional_no ?? t('students.admissionPending')
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => navigate(`${base}/${s.id}`)}
                style={{ display: 'grid', gridTemplateColumns: GRID, gap: '16px', width: '100%', alignItems: 'center', textAlign: 'left', padding: '14px 24px', border: 'none', borderTop: '1px solid var(--track)', background: 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '14px' }}
              >
                <span style={{ minWidth: 0 }}>
                  <span style={{ display: 'block', fontWeight: 500, ...ELLIPSIS }}>{s.name}</span>
                  <span style={{ display: 'block', fontSize: '12px', color: s.admission_no ? 'var(--muted)' : 'var(--gold-text)' }}>{adm}</span>
                </span>
                <span style={{ color: 'var(--muted)' }}>{s.class_display ?? '—'}</span>
                <span style={{ color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{s.roll_no ?? '—'}</span>
                <span style={{ minWidth: 0, ...ELLIPSIS }}>{s.guardian_name ?? '—'}</span>
                <span>
                  <Pill variant={s.status === 'left' ? 'unpaid' : 'paid'}>
                    {s.status === 'left' ? t('students.pill.left') : t('students.pill.active')}
                  </Pill>
                </span>
              </button>
            )
          })
        )}
      </div>

      {/* Pagination */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span style={{ fontSize: '13px', color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>
          {t('students.showing', { from, to, total })}
        </span>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            type="button"
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - PAGE))}
            style={{ ...fieldStyle(), padding: '0 16px', opacity: offset === 0 ? 0.5 : 1, cursor: offset === 0 ? 'default' : 'pointer' }}
          >
            {t('students.prev')}
          </button>
          <button
            type="button"
            disabled={to >= total}
            onClick={() => setOffset(offset + PAGE)}
            style={{ ...fieldStyle(), padding: '0 16px', opacity: to >= total ? 0.5 : 1, cursor: to >= total ? 'default' : 'pointer' }}
          >
            {t('students.next')}
          </button>
        </div>
      </div>
    </div>
  )
}
