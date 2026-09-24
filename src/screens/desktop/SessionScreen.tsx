import { useEffect, useState } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { SchoolDto } from '@/lib/api'

// Session switcher (prompts/P07 Shared). One session exists until a rollover
// (Phase 8). Past sessions render read-only with the gold banner; the read-only
// enforcement (SESSION_READ_ONLY) is set on the session row and surfaced by the
// AppShell banner. Full multi-session switching lands with Phase 8 rollover.
export default function SessionScreen() {
  const [school, setSchool] = useState<SchoolDto | null>(null)
  useEffect(() => {
    api.get_school().then(setSchool).catch(() => setSchool(null))
  }, [])

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle eyebrow={t('session.eyebrow')} title={t('session.title')} />
      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', padding: '20px 24px', display: 'flex', alignItems: 'center', gap: '16px' }}>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: '11px', color: 'var(--muted)' }}>{t('session.current')}</div>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: '26px', color: 'var(--ink)' }}>{school?.session_label ?? '—'}</div>
        </div>
        {school?.session_read_only ? <Pill variant="neutral">{t('session.readOnly')}</Pill> : null}
      </div>
      <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('session.note')}</div>
    </div>
  )
}
