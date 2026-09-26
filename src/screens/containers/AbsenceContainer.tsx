// Absence alerts — phone "Absent today" (prototype `absence`), P14 Step 4. Shown
// after a class teacher submits attendance. Lists the absentees with the primary
// guardian and a preview of the `absence_alert` message; Email queues a message
// (the school PC sends it later — honest status "queued", NOT "sent", §3 rule 13),
// WhatsApp opens the share sheet / wa.me and records a `tapped` row.
//
// NOTE (fidelity): the prototype's "Email sent" label is optimistic; the honest
// status is "queued" because the phone cannot send email (the school PC does).
// This deviation from the prototype copy is required by rule 13 and noted in the
// handoff. Phone pixel-fidelity is only assertable on the canonical machine.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { AbsenceListDto, AbsentStudentDto, CmdError } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { navigate } from '@/lib/router'
import { shareWhatsApp } from '@/lib/files'
import { t, useLang } from '@/lib/i18n'

type Status = 'idle' | 'queued' | 'tapped' | 'skipped'

export default function AbsenceContainer({ classId, date }: { classId: string; date: string }) {
  useLang()
  const [list, setList] = useState<AbsenceListDto | null>(null)
  const [status, setStatus] = useState<Record<string, Status>>({})
  const [error, setError] = useState<CmdError | null>(null)

  useEffect(() => {
    api.list_absent(classId, date).then(setList).catch((e) => setError(e as CmdError))
  }, [classId, date])

  const emailable = (s: AbsentStudentDto) => !!(s.guardian_email && s.guardian_email.trim()) && s.has_messages_consent

  const doEmail = async (s: AbsentStudentDto) => {
    if (!emailable(s)) {
      setStatus((m) => ({ ...m, [s.student_id]: 'skipped' }))
      return
    }
    try {
      await api.record_message({ channel: 'email', kind: 'absence_alert', language: s.guardian_language ?? 'en', to_guardian_id: s.guardian_id, to_address: s.guardian_email, body: s.preview, related_table: 'student', related_id: s.student_id })
      setStatus((m) => ({ ...m, [s.student_id]: 'queued' }))
    } catch { /* honest: nothing queued */ }
  }

  const doWhatsApp = async (s: AbsentStudentDto) => {
    try {
      await api.record_message({ channel: 'wa_tap', kind: 'absence_alert', language: s.guardian_language ?? 'en', to_guardian_id: s.guardian_id, to_address: s.guardian_mobile, body: s.preview, related_table: 'student', related_id: s.student_id })
    } catch { /* still open the share below */ }
    await shareWhatsApp(s.guardian_mobile, s.preview)
    setStatus((m) => ({ ...m, [s.student_id]: 'tapped' }))
  }

  const emailAll = async () => {
    if (!list) return
    for (const s of list.students) await doEmail(s)
  }

  if (error) return <Center color="var(--danger)">{t(error.message_key, error.vars as Record<string, string | number>)}</Center>
  if (!list) return <Center color="var(--muted)">…</Center>

  const absentCount = list.students.length
  const canEmailAny = list.students.some(emailable)

  return (
    <div style={{ position: 'relative', width: '390px', height: '844px', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      {/* navy header (matches the phone page header) */}
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: '10px 16px 16px', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <button type="button" aria-label={t('absence.back')} onClick={() => navigate('/teacher/home')} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('absence.title')}</h1>
          <span style={{ fontFamily: 'var(--font-serif)', fontStyle: 'italic', fontSize: 22, lineHeight: 1.25, color: '#C5AB7A' }}>{list.class_display ?? ''} · {date}</span>
        </div>
        <div style={{ borderTop: '1px solid rgba(255,255,255,0.14)', paddingTop: 12, fontSize: 13, color: '#9FACBF' }}>
          {absentCount === 1 ? t('absence.oneAbsent') : t('absence.nAbsent', { n: absentCount })}
        </div>
      </header>

      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
        {!list.submitted ? (
          <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('absence.notSubmitted')}</span>
        ) : list.students.length === 0 ? (
          <span style={{ fontSize: 15, color: 'var(--muted)', textAlign: 'center', marginTop: 40 }}>{t('absence.none')}</span>
        ) : (
          <>
            <span style={{ fontSize: 13, color: 'var(--muted)', lineHeight: 1.5 }}>{t('absence.intro')}</span>
            {list.students.map((s) => {
              const st = status[s.student_id] ?? 'idle'
              return (
                <div key={s.student_id} style={{ borderRadius: 14, background: 'var(--surface)', border: '1px solid var(--line)', padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10 }}>
                  <div>
                    <b style={{ fontWeight: 600, fontSize: 15 }}>{s.student_name}</b>
                    <div style={{ fontSize: 12, color: 'var(--muted)' }}>{s.guardian_name ? `${t('absence.guardian')} · ${s.guardian_name}` : t('absence.noGuardian')}</div>
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                    {st === 'queued' ? (
                      <StatusChip icon="check" text={t('absence.queued')} />
                    ) : st === 'skipped' ? (
                      <StatusChip icon="close" text={s.guardian_email ? t('absence.noConsent') : t('absence.noEmail')} muted />
                    ) : (
                      <PhoneBtn text={t('absence.email')} onClick={() => doEmail(s)} />
                    )}
                    <PhoneBtn text={st === 'tapped' ? t('absence.opened') : t('absence.whatsapp')} onClick={() => doWhatsApp(s)} />
                  </div>
                </div>
              )
            })}
            <div style={{ borderRadius: 12, background: 'var(--panel)', padding: '12px 14px', fontSize: 13, lineHeight: 1.5 }}>
              <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('absence.message')}</div>
              <div style={{ marginTop: 6 }}>{list.students[0]?.preview}</div>
            </div>
          </>
        )}
      </div>

      {list.submitted && list.students.length > 0 ? (
        <div style={{ flexShrink: 0, background: '#FDFDFB', borderTop: '1px solid #D5DDE0', padding: '12px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          <button type="button" disabled={!canEmailAny} onClick={emailAll} style={{ height: 52, borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: '#FFFFFF', fontSize: 15, fontWeight: 500, opacity: canEmailAny ? 1 : 0.5 }}>
            {t('absence.emailAll')}
          </button>
          <span style={{ fontSize: 12, color: 'var(--muted)', textAlign: 'center' }}>{t('absence.queuedNote')}</span>
        </div>
      ) : null}
    </div>
  )
}

function Center({ children, color }: { children: React.ReactNode; color: string }) {
  return <div style={{ minHeight: '100vh', display: 'grid', placeItems: 'center', background: 'var(--bg)', color, fontSize: 14 }}>{children}</div>
}

function PhoneBtn({ text, onClick }: { text: string; onClick: () => void }) {
  return (
    <button type="button" onClick={onClick} style={{ height: 40, borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, fontWeight: 500, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6 }}>
      {text}
    </button>
  )
}

function StatusChip({ icon, text, muted }: { icon: 'check' | 'close'; text: string; muted?: boolean }) {
  return (
    <span style={{ height: 40, borderRadius: 8, background: muted ? 'var(--panel)' : 'var(--accent-10)', color: muted ? 'var(--muted)' : 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6, fontSize: 13, fontWeight: 600 }}>
      <Icon name={icon} size={14} strokeWidth={2.2} />{text}
    </span>
  )
}
