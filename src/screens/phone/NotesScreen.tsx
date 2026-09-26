// Homework & notes (P16 Step 3, prototype `notes`). Teacher phone screen: pick a
// class + subject they teach, write homework or class notes, attach photos/PDFs
// (metadata; ≤ 10 MB each / 20 MB total, validated in vidya-core), then Share with
// parents → a share sheet (WhatsApp / Email / Save in Vidya). A recent-history list
// shows the class's notes; a teacher may delete their own within 24 h.
//
// NOTE: the Android native share sheet + Drive upload of the files themselves are
// deferred (no Android toolchain / live Drive in this environment, P14/P15
// precedent). Email queues through the P14 pipeline; WhatsApp opens the group text.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { HomeworkNoteDto, AttachmentMeta, CmdError } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { navigate } from '@/lib/router'
import { shareWhatsApp } from '@/lib/files'
import { t, useLang } from '@/lib/i18n'

interface ClassSubjectOpt {
  class_id: string
  class_display: string | null
  class_subject_id: string
  subject_name: string
}

export default function NotesScreen() {
  useLang()
  const [opts, setOpts] = useState<ClassSubjectOpt[]>([])
  const [sel, setSel] = useState<string>('') // "classId|classSubjectId"
  const [kind, setKind] = useState<'homework' | 'notes'>('homework')
  const [text, setText] = useState('')
  const [files, setFiles] = useState<AttachmentMeta[]>([])
  const [history, setHistory] = useState<HomeworkNoteDto[]>([])
  const [shareId, setShareId] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    api.my_timetable().then((tt) => {
      const seen = new Set<string>()
      const list: ClassSubjectOpt[] = []
      for (const s of tt.slots) {
        const key = s.class_subject_id
        if (seen.has(key)) continue
        seen.add(key)
        list.push({ class_id: s.class_id, class_display: s.class_display, class_subject_id: s.class_subject_id, subject_name: s.subject_name })
      }
      setOpts(list)
      if (list.length > 0 && !sel) setSel(`${list[0].class_id}|${list[0].class_subject_id}`)
    }).catch(() => setOpts([]))
  }, [sel])

  const classId = sel.split('|')[0]
  const classSubjectId = sel.split('|')[1]

  const loadHistory = useCallback(() => {
    if (!classId) return
    api.list_homework_notes(classId).then(setHistory).catch(() => setHistory([]))
  }, [classId])
  useEffect(loadHistory, [loadHistory])

  const onPick = (e: React.ChangeEvent<HTMLInputElement>) => {
    const picked = Array.from(e.target.files ?? []).map((f) => ({ name: f.name, size: f.size, mime: f.type || 'application/octet-stream' }))
    setFiles((prev) => [...prev, ...picked])
    e.target.value = ''
  }

  const share = () => {
    setErr(null)
    if (text.trim().length === 0 && files.length === 0) { setErr(t('notes.needContent')); return }
    api.save_homework_note({ class_id: classId, class_subject_id: classSubjectId, kind, text: text.trim(), attachments: files })
      .then((n) => { setShareId(n.id); loadHistory() })
      .catch((e) => setErr((e as CmdError).code === 'VALIDATION' ? t('notes.tooBig') : t('notes.needContent')))
  }

  const doEmail = () => {
    if (!shareId) return
    api.email_homework_note(shareId)
      .then((r) => { setShareId(null); setToast(t('notes.emailQueued', { queued: r.queued, skip: r.skipped_no_email + r.skipped_no_consent })); resetCompose() })
      .catch(() => setShareId(null))
  }
  const doWhatsApp = () => {
    void shareWhatsApp(null, text.trim() || t('notes.title'))
    setShareId(null); setToast(t('notes.waOpened')); resetCompose()
  }
  const doSave = () => { setShareId(null); setToast(t('notes.saved')); resetCompose() }
  const resetCompose = () => { setText(''); setFiles([]) }

  const del = (id: string) => { api.delete_homework_note(id).then(loadHistory).catch(() => {}) }

  const inputStyle = { borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 15, padding: '10px 12px', boxSizing: 'border-box' as const, width: '100%' }

  return (
    <div style={{ position: 'relative', width: '390px', height: '844px', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: '10px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button type="button" aria-label={t('notes.back')} onClick={() => navigate('/teacher/home')} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('notes.title')}</h1>
      </header>

      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <select aria-label={t('notes.pick')} value={sel} onChange={(e) => setSel(e.target.value)} style={{ ...inputStyle, height: 46 }}>
          {opts.length === 0 && <option value="">{t('notes.pickClass')}</option>}
          {opts.map((o) => <option key={o.class_subject_id} value={`${o.class_id}|${o.class_subject_id}`}>{o.class_display} · {o.subject_name}</option>)}
        </select>

        <div style={{ display: 'flex', gap: 8 }}>
          {(['homework', 'notes'] as const).map((k) => {
            const on = kind === k
            return (
              <button key={k} type="button" onClick={() => setKind(k)} style={{ height: 36, padding: '0 16px', borderRadius: 18, border: `1px solid ${on ? 'var(--navy)' : 'var(--line-strong)'}`, background: on ? 'var(--navy)' : 'transparent', color: on ? 'var(--white)' : 'var(--ink)', fontSize: 13, fontWeight: 500 }}>
                {k === 'homework' ? t('notes.homework') : t('notes.classnotes')}
              </button>
            )
          })}
        </div>

        <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder={t('notes.text')} style={{ ...inputStyle, minHeight: 96, resize: 'none', fontFamily: 'inherit' }} />

        {files.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {files.map((f, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px', borderRadius: 10, background: 'var(--white)', border: '1px solid var(--line)', fontSize: 13 }}>
                <span style={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{f.name}</span>
                <span style={{ color: 'var(--muted)', fontSize: 12 }}>{Math.round(f.size / 1024)} KB</span>
                <button type="button" aria-label="remove" onClick={() => setFiles(files.filter((_, j) => j !== i))} style={{ border: 0, background: 'transparent', color: 'var(--muted)' }}><Icon name="close" size={16} /></button>
              </div>
            ))}
          </div>
        )}

        <label style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, height: 44, borderRadius: 10, border: '1px dashed var(--line-strong)', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>
          + {t('notes.attach')}
          <input type="file" accept="image/*,application/pdf" multiple onChange={onPick} style={{ display: 'none' }} />
        </label>

        <span style={{ fontSize: 12, color: 'var(--muted)', lineHeight: 1.5 }}>{t('notes.warning')}</span>
        {err && <span style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span>}

        {history.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('notes.history')}</span>
            {history.map((h) => (
              <div key={h.id} style={{ borderRadius: 12, background: 'var(--surface)', border: '1px solid var(--line)', padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 6 }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}>
                  <b style={{ fontWeight: 600, fontSize: 13 }}>{h.kind === 'homework' ? t('notes.homework') : t('notes.classnotes')}</b>
                  {h.subject_name && <span style={{ fontSize: 12, color: 'var(--muted)' }}>· {h.subject_name}</span>}
                  <span style={{ marginLeft: 'auto', fontSize: 11, color: 'var(--muted)' }}>{h.created_by_name}</span>
                </div>
                <span style={{ fontSize: 13, lineHeight: 1.4 }}>{h.text}</span>
                {h.attachments.length > 0 && <span style={{ fontSize: 12, color: 'var(--muted)' }}>{h.attachments.length} file(s)</span>}
                {h.can_delete && <button type="button" onClick={() => del(h.id)} style={{ alignSelf: 'flex-start', border: 0, background: 'transparent', color: 'var(--pill-unpaid-fg)', fontSize: 12, padding: 0 }}>{t('notes.delete')}</button>}
              </div>
            ))}
          </div>
        )}
      </div>

      <div style={{ flexShrink: 0, background: '#FDFDFB', borderTop: '1px solid #D5DDE0', padding: '12px 16px 16px' }}>
        <button type="button" onClick={share} disabled={!classId} style={{ width: '100%', height: 52, borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: '#FFFFFF', fontSize: 15, fontWeight: 500, opacity: classId ? 1 : 0.5 }}>{t('notes.share')}</button>
      </div>

      {shareId && (
        <>
          <div style={{ position: 'absolute', inset: 0, background: 'rgba(11,26,51,0.45)' }} onClick={() => setShareId(null)} />
          <div style={{ position: 'absolute', left: 0, right: 0, bottom: 0, background: 'var(--white)', borderRadius: '22px 22px 0 0', padding: '14px 18px 26px', display: 'flex', flexDirection: 'column', gap: 16 }}>
            <span style={{ alignSelf: 'center', width: 40, height: 4, borderRadius: 2, background: 'var(--line)' }} />
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('notes.shareTo')}</span>
            <div style={{ display: 'flex', gap: 18, justifyContent: 'space-around' }}>
              {[
                { label: t('notes.whatsapp'), bg: 'var(--online)', icon: 'inbox' as const, fn: doWhatsApp },
                { label: t('notes.email'), bg: 'var(--accent)', icon: 'inbox' as const, fn: doEmail },
                { label: t('notes.save'), bg: 'var(--navy)', icon: 'check' as const, fn: doSave },
              ].map((o) => (
                <button key={o.label} type="button" onClick={o.fn} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, fontSize: 12, border: 0, background: 'transparent', color: 'var(--ink)' }}>
                  <span style={{ width: 54, height: 54, borderRadius: 27, background: o.bg, color: '#FFFFFF', display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name={o.icon} size={24} strokeWidth={1.8} /></span>
                  {o.label}
                </button>
              ))}
            </div>
          </div>
        </>
      )}

      {toast && (
        <div role="status" onAnimationEnd={() => {}} style={{ position: 'absolute', left: 16, right: 16, bottom: 90, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 14px', fontSize: 14, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Icon name="check" size={16} strokeWidth={2.2} color="var(--gold)" />{toast}
        </div>
      )}
    </div>
  )
}
