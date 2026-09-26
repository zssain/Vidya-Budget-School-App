// Circulars & notices (prototype `circulars` states 1-2), P14 Step 6. Principal
// composes a circular (title, message, audience, languages, channels), saves it as
// a draft, and sends it — the server assigns CIR/<session>/NNN. The Sent view shows
// the channel summary and staff "Read by N of M". Every control is real
// (list_circulars / save_circular / send_circular). Under the `circulars` module.

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import * as api from '@/lib/api'
import type { CircularDto, CircularInput, CmdError } from '@/lib/api'
import { t, useLang } from '@/lib/i18n'

const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }
const field = (): CSSProperties => ({ height: 46, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, fontFamily: 'inherit', padding: '0 14px', boxSizing: 'border-box', width: '100%' })
const chip = (on: boolean): CSSProperties => ({ height: 36, padding: '0 16px', borderRadius: 18, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: `1.5px solid ${on ? 'var(--accent)' : 'var(--line-strong)'}`, background: on ? 'var(--accent)' : 'var(--white)', color: on ? 'var(--white)' : 'var(--ink)' })
const primaryBtn = (): CSSProperties => ({ height: 52, borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 15, fontWeight: 500, cursor: 'pointer', width: '100%' })
const secondaryBtn = (): CSSProperties => ({ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, fontWeight: 500, cursor: 'pointer' })

type Audience = 'whole_school' | 'classes' | 'staff'
const CHANNELS: [string, string][] = [
  ['staff_app', 'chStaffApp'],
  ['wa_groups', 'chWaGroups'],
  ['email', 'chEmail'],
  ['print', 'chPrint'],
]

export default function CircularsScreen() {
  useLang()
  const [list, setList] = useState<CircularDto[] | null>(null)
  const [moduleOff, setModuleOff] = useState(false)
  const [sel, setSel] = useState<CircularDto | 'new' | null>('new')

  // compose state
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [audience, setAudience] = useState<Audience>('whole_school')
  const [langs, setLangs] = useState<Record<'en' | 'hi' | 'te', boolean>>({ en: true, hi: false, te: false })
  const [draftId, setDraftId] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_circulars().then((c) => { setList(c); setModuleOff(false) }).catch((e) => {
      if ((e as CmdError).code === 'MODULE_OFF') setModuleOff(true)
      setList([])
    })
  }, [])
  useEffect(load, [load])

  const startNew = () => { setSel('new'); setDraftId(null); setTitle(''); setBody(''); setAudience('whole_school'); setLangs({ en: true, hi: false, te: false }) }
  const openCircular = (c: CircularDto) => {
    setSel(c)
    if (c.status === 'draft') {
      setDraftId(c.id); setTitle(c.title); setBody(c.body)
      setAudience(((c.audience_json && JSON.parse(c.audience_json).kind) || 'whole_school') as Audience)
    }
  }

  const composeInput = (): CircularInput => ({
    id: draftId, title, body,
    audience_json: JSON.stringify({ kind: audience }),
    channels_json: JSON.stringify(CHANNELS.map(([k]) => k)),
    languages_json: JSON.stringify(Object.entries(langs).filter(([, on]) => on).map(([l]) => l)),
  })

  const saveDraft = async () => {
    try { const d = await api.save_circular(composeInput()); setDraftId(d.id); load() } catch { /* noop */ }
  }
  const send = async () => {
    try {
      const d = await api.save_circular(composeInput())
      const sent = await api.send_circular(d.id)
      setSel(sent); load()
    } catch { /* noop */ }
  }

  const eyebrow = t('circulars.eyebrow')
  const selectedDto: CircularDto | null = sel && sel !== 'new' ? sel : null
  const composing = sel === 'new' || selectedDto?.status === 'draft'

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 22, minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={eyebrow}
        title={composing ? t('circulars.newTitle') : t('circulars.sentTitle')}
        sub={composing ? t('circulars.newSub') : t('circulars.sentSub')}
        actions={<button type="button" style={secondaryBtn()} onClick={startNew}>{t('circulars.new')}</button>}
      />

      {moduleOff ? (
        <div style={{ ...CARD, padding: 40, textAlign: 'center', color: 'var(--muted)' }}>{t('circulars.moduleOff')}</div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '260px 1fr', gap: 24, flexGrow: 1, minHeight: 0 }}>
          {/* list */}
          <div style={{ ...CARD, display: 'flex', flexDirection: 'column' }}>
            <div style={{ padding: '14px 16px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>{t('circulars.all')}</div>
            <div style={{ overflowY: 'auto' }}>
              {(list ?? []).length === 0 ? (
                <div style={{ padding: 24, fontSize: 13, color: 'var(--muted)' }}>{t('circulars.none')}</div>
              ) : (
                (list ?? []).map((c) => (
                  <button key={c.id} type="button" onClick={() => openCircular(c)} style={{ width: '100%', textAlign: 'left', padding: '12px 16px', border: 'none', borderBottom: '1px solid var(--track)', background: sel !== 'new' && sel?.id === c.id ? 'var(--accent-6)' : 'transparent', cursor: 'pointer' }}>
                    <div style={{ fontSize: 14, fontWeight: 500, color: 'var(--ink)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.title}</div>
                    <div style={{ fontSize: 12, color: 'var(--muted)' }}>{c.status === 'sent' ? (c.number ?? t('circulars.sent')) : t('circulars.draft')}</div>
                  </button>
                ))
              )}
            </div>
          </div>

          {/* compose or sent */}
          {composing ? (
            <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1fr', gap: 24 }}>
              <div style={{ ...CARD, padding: 24, display: 'flex', flexDirection: 'column', gap: 16 }}>
                <label style={{ display: 'flex', flexDirection: 'column', gap: 6, fontSize: 13, fontWeight: 500 }}>{t('circulars.title')}
                  <input value={title} onChange={(e) => setTitle(e.target.value)} style={field()} />
                </label>
                <label style={{ display: 'flex', flexDirection: 'column', gap: 6, fontSize: 13, fontWeight: 500 }}>{t('circulars.message')}
                  <textarea value={body} onChange={(e) => setBody(e.target.value)} style={{ ...field(), height: 130, padding: '10px 14px', resize: 'vertical' }} />
                </label>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                  <span style={{ fontSize: 13, fontWeight: 500 }}>{t('circulars.sendTo')}</span>
                  <div style={{ display: 'flex', gap: 8 }}>
                    {(['whole_school', 'classes', 'staff'] as Audience[]).map((a) => (
                      <button key={a} type="button" style={chip(audience === a)} onClick={() => setAudience(a)}>{t(`circulars.aud.${a}`)}</button>
                    ))}
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ fontSize: 13, fontWeight: 500, marginRight: 6 }}>{t('circulars.languages')}</span>
                  {(['en', 'hi', 'te'] as const).map((l) => (
                    <button key={l} type="button" style={chip(langs[l])} onClick={() => setLangs((m) => ({ ...m, [l]: !m[l] }))}>{l === 'en' ? 'English' : l === 'hi' ? 'हिंदी' : 'తెలుగు'}</button>
                  ))}
                </div>
              </div>
              <div style={{ ...CARD, display: 'flex', flexDirection: 'column' }}>
                <div style={{ padding: '20px 24px 16px' }}>
                  <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('circulars.howToSend')}</div>
                  <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('circulars.channels')}</div>
                </div>
                {CHANNELS.map(([k, key]) => (
                  <div key={k} style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '13px 24px', borderTop: '1px solid var(--track)' }}>
                    <span style={{ width: 22, height: 22, borderRadius: 5, background: 'var(--accent)', color: 'var(--white)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 13 }}>✓</span>
                    <div style={{ flexGrow: 1 }}><b style={{ fontWeight: 500 }}>{t(`circulars.${key}`)}</b><div style={{ fontSize: 12, color: 'var(--muted)' }}>{t(`circulars.${key}Sub`)}</div></div>
                  </div>
                ))}
                <div style={{ padding: '16px 24px 20px', marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: 8 }}>
                  <button type="button" style={primaryBtn()} disabled={title.trim() === ''} onClick={send}>{t('circulars.send')}</button>
                  <button type="button" style={{ ...secondaryBtn(), height: 44 }} disabled={title.trim() === ''} onClick={saveDraft}>{t('circulars.saveDraft')}</button>
                  <span style={{ fontSize: 12, color: 'var(--muted)', textAlign: 'center' }}>{t('circulars.numberNote')}</span>
                </div>
              </div>
            </div>
          ) : selectedDto ? (
            <SentView c={selectedDto} />
          ) : null}
        </div>
      )}
    </div>
  )
}

function SentView({ c }: { c: CircularDto }) {
  const pct = c.staff_count > 0 ? Math.round((c.read_count / c.staff_count) * 100) : 0
  const channels: string[] = c.channels_json ? JSON.parse(c.channels_json) : []
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 24 }}>
      <section style={{ borderRadius: 16, background: 'radial-gradient(120% 70% at 50% 0%, #1A3560 0%, #0C1B38 60%)', color: 'var(--white)', padding: 32, display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div style={{ fontFamily: 'var(--font-serif)', fontSize: 30, letterSpacing: '-0.02em' }}>{c.title}</div>
        <div style={{ fontSize: 13, color: 'var(--on-navy-muted)' }}>{c.number}</div>
        {channels.map((ch) => (
          <div key={ch} style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid rgba(255,255,255,0.14)', paddingTop: 10, fontSize: 14 }}>
            <span style={{ color: 'var(--on-navy-muted)' }}>{t(`circulars.ch.${ch}`)}</span>
            <span>{t('circulars.chDone')}</span>
          </div>
        ))}
      </section>
      <div style={{ ...CARD, display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '20px 24px 8px' }}>
          <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('circulars.staff')}</div>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('circulars.readBy', { n: c.read_count, m: c.staff_count })}</div>
        </div>
        <div style={{ padding: '8px 24px 20px' }}>
          <div style={{ height: 8, borderRadius: 4, background: 'var(--track)', overflow: 'hidden' }}>
            <div style={{ width: `${pct}%`, height: '100%', background: 'var(--accent)', borderRadius: 4 }} />
          </div>
          <div style={{ fontSize: 13, color: 'var(--muted)', marginTop: 12 }}>{t('circulars.readNote')}</div>
        </div>
      </div>
    </div>
  )
}
