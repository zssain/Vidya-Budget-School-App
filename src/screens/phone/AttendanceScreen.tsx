// Attendance screen — a pixel-exact React translation of
// design/screens/Attendance.dc.html. Every element, order and inline style is
// copied verbatim; the interaction logic mirrors the mock's renderVals() exactly
// (mark toggle, counts, stacked bar, mark-all/undo, save-draft toast, submit lock).
// The ONLY substitutions are (docs/01-MOCK-SPEC.md §2/§5):
//   - accent: the mock's `accent` (P on-state bg/border + enabled Submit bg) ->
//     var(--accent). (Accent options are wired at :root by lib/theme.ts.)
//   - keyframes: the mock's local `vIn` (10px) -> app name vIn10; its local `vPop`
//     (scale .6->1.1) -> vPopAtt. (App keyframes live in src/styles/keyframes.css.)
//   - icons: back / wifi-off / check / clock inline SVGs -> <Icon> with the same
//     width/height/strokeWidth/colour.
//   - no hard-coded visible text: every string comes from t('att.*'), with {n}/{name}
//     interpolated.
//   - P/A buttons carry .v-active-93 (the mock's `:active { scale(.93) }`).
// v2 (Phase 11): attendance is Present/Absent only — the Leave (L) button, the
// Leave count and the Leave bar segment are removed (matches the prototype). A
// legacy L already stored is shown read-only as a muted "Leave (old)" pill.
import { useEffect, useRef, useState } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { type AttendanceData, type Mark, initialMarks } from '@/dev/fixtures/attendance'

export default function AttendanceScreen({
  data,
  initial,
  marks,
  onSubmit,
  onSaveDraft,
}: {
  data: AttendanceData
  initial?: 'default' | 'markall' | 'submitted'
  /** Real flow: existing marks for the sheet (parallel to data.names). */
  marks?: Mark[]
  /** Real flow: submit the completed sheet (parallel to data.names/studentIds). */
  onSubmit?: (marks: Mark[]) => void
  /** Real flow: save the draft. */
  onSaveDraft?: (marks: Mark[]) => void
}) {
  // Real marks when wired; otherwise the fixture rule (docs §8).
  const base = marks ?? initialMarks(data.names)

  // State mirrors the mock: st (per-student marks), prev (snapshot before "mark
  // all", enables Undo), submitted (locks the screen), toast (draft-saved).
  // `initial` seeds the gallery's fidelity states without any interaction:
  //   'markall' -> all 'P' with prev set (so Undo shows), 'submitted' -> locked.
  const [st, setSt] = useState<Mark[]>(initial === 'markall' ? base.map(() => 'P') : base)
  const [prev, setPrev] = useState<Mark[] | null>(initial === 'markall' ? base : null)
  const [submitted, setSubmitted] = useState<boolean>(initial === 'submitted')
  const [toast, setToast] = useState<boolean>(false)

  // Draft-saved timer; cleared on unmount (mock componentWillUnmount).
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current)
    }
  }, [])

  // --- derived, exactly as the mock's renderVals() ---
  const count = (v: Mark) => st.filter((x) => x === v).length
  const cP = count('P')
  const cA = count('A')
  const cU = count('')
  const hasUnmarked = cU > 0

  const accent = 'var(--accent)'

  // Stacked-bar segment style: width = count / total * 100%.
  const seg = (c: number, col: string): React.CSSProperties => ({
    height: '100%',
    transition: 'width .35s cubic-bezier(.2,.8,.2,1)',
    background: col,
    width: (c / st.length) * 100 + '%',
  })

  // P/A button styles. off = white; on: P accent/white, A red/white.
  const btn: React.CSSProperties = {
    width: '44px',
    height: '44px',
    borderRadius: '8px',
    fontSize: '14px',
    fontWeight: 600,
  }
  const off: React.CSSProperties = {
    ...btn,
    border: '1px solid #D5DDE0',
    background: '#FFFFFF',
    color: '#56657A',
  }
  const on: Record<'P' | 'A', React.CSSProperties> = {
    P: { ...btn, border: '1px solid ' + accent, background: accent, color: '#FFFFFF' },
    A: { ...btn, border: '1px solid #C0392B', background: '#C0392B', color: '#FFFFFF' },
  }
  // A legacy Leave mark (pre-v2) is shown read-only as a muted "Leave (old)" pill.
  const leaveOldPill: React.CSSProperties = {
    height: '44px',
    display: 'flex',
    alignItems: 'center',
    padding: '0 12px',
    borderRadius: '8px',
    border: '1px solid #DCC495',
    background: '#FAF4E6',
    color: '#8C6A2F',
    fontSize: '13px',
    fontWeight: 500,
    whiteSpace: 'nowrap',
  }

  // Tapping the active mark clears it (toggle to ''); any set clears `prev`.
  const set = (i: number, v: Mark) => () => {
    if (submitted) return
    const next = st.slice()
    next[i] = next[i] === v ? '' : v
    setSt(next)
    setPrev(null)
  }

  const locked = submitted
  const canMarkAll = !submitted && !prev
  const canUndo = !submitted && !!prev

  const markAll = () => {
    setPrev(st)
    setSt(st.map(() => 'P'))
  }
  const undo = () => {
    if (prev) setSt(prev)
    setPrev(null)
  }
  const saveDraft = () => {
    if (timer.current) clearTimeout(timer.current)
    setToast(true)
    timer.current = setTimeout(() => setToast(false), 2200)
    if (onSaveDraft) onSaveDraft(st)
  }
  const submit = () => {
    if (!hasUnmarked) {
      setSubmitted(true)
      setPrev(null)
      setToast(false)
      if (onSubmit) onSubmit(st)
    }
  }

  const submitStyle: React.CSSProperties = {
    height: '50px',
    borderRadius: '6px',
    border: 0,
    fontSize: '15px',
    fontWeight: 500,
    background: hasUnmarked ? '#C9D3D2' : accent,
    color: hasUnmarked ? '#56657A' : '#FFFFFF',
  }

  const editing = !submitted
  const netText = submitted ? t('att.net.waiting') : t('att.net.offline')
  const netStyle: React.CSSProperties = {
    height: '30px',
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    padding: '0 12px',
    borderRadius: '15px',
    fontSize: '12px',
    fontWeight: 500,
    background: 'rgba(197,171,122,0.14)',
    color: '#E9D6AE',
    border: '1px solid rgba(197,171,122,0.3)',
  }

  return (
    <div
      style={{
        position: 'relative',
        width: '390px',
        height: '844px',
        display: 'flex',
        flexDirection: 'column',
        background: '#F5F7F6',
        color: '#13233F',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: '14px',
        overflow: 'hidden',
      }}
    >
      <header
        style={{
          flexShrink: 0,
          background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)',
          color: '#FFFFFF',
          padding: '10px 16px 16px',
          display: 'flex',
          flexDirection: 'column',
          gap: '12px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <button
            type="button"
            aria-label="Back to home"
            onClick={() => navigate('/teacher/home')}
            style={{
              width: '44px',
              height: '44px',
              marginLeft: '-10px',
              borderRadius: '22px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#FFFFFF',
              border: 0,
              background: 'transparent',
              padding: 0,
            }}
          >
            <Icon name="back" size={22} strokeWidth={1.7} />
          </button>
          <span style={netStyle}>
            <Icon name="cloudOff" size={13} strokeWidth={2} />
            {netText}
          </span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <h1
            className="v-serif"
            style={{
              margin: 0,
              fontFamily: 'var(--font-serif)',
              fontWeight: 400,
              fontSize: '32px',
              lineHeight: 1.05,
              letterSpacing: '-0.02em',
            }}
          >
            {t('att.h1')}
          </h1>
          <span
            className="v-serif"
            style={{
              fontFamily: 'var(--font-serif)',
              fontStyle: 'italic',
              fontSize: '22px',
              lineHeight: 1.25,
              color: '#C5AB7A',
            }}
          >
            {t('att.subtitle')}
          </span>
        </div>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            borderTop: '1px solid rgba(255,255,255,0.14)',
            paddingTop: '12px',
          }}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span
              className="v-serif"
              style={{
                fontFamily: 'var(--font-serif)',
                fontSize: '26px',
                lineHeight: 1,
                fontVariantNumeric: 'lining-nums tabular-nums',
              }}
            >
              {cP}
            </span>
            <span style={{ fontSize: '11px', color: '#9FACBF' }}>{t('att.count.present')}</span>
          </div>
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '2px',
              borderLeft: '1px solid rgba(255,255,255,0.12)',
              paddingLeft: '12px',
            }}
          >
            <span
              className="v-serif"
              style={{
                fontFamily: 'var(--font-serif)',
                fontSize: '26px',
                lineHeight: 1,
                fontVariantNumeric: 'lining-nums tabular-nums',
              }}
            >
              {cA}
            </span>
            <span style={{ fontSize: '11px', color: '#9FACBF' }}>{t('att.count.absent')}</span>
          </div>
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '2px',
              borderLeft: '1px solid rgba(255,255,255,0.12)',
              paddingLeft: '12px',
            }}
          >
            <span
              className="v-serif"
              style={{
                fontFamily: 'var(--font-serif)',
                fontSize: '26px',
                lineHeight: 1,
                color: '#C5AB7A',
                fontVariantNumeric: 'lining-nums tabular-nums',
              }}
            >
              {cU}
            </span>
            <span style={{ fontSize: '11px', color: '#9FACBF' }}>{t('att.count.notMarked')}</span>
          </div>
        </div>
        <div
          style={{
            height: '4px',
            borderRadius: '2px',
            background: 'rgba(255,255,255,0.12)',
            display: 'flex',
            gap: '2px',
            overflow: 'hidden',
          }}
        >
          <div style={seg(cP, '#7DB1B5')} />
          <div style={seg(cA, '#D07A73')} />
        </div>
      </header>
      <div style={{ flexGrow: 1, overflow: 'auto', display: 'flex', flexDirection: 'column' }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '14px 16px 10px',
          }}
        >
          <span
            style={{
              fontSize: '10px',
              fontWeight: 600,
              letterSpacing: '0.16em',
              textTransform: 'uppercase',
              color: '#56657A',
            }}
          >
            {t('att.hint')}
          </span>
          {canMarkAll ? (
            <button
              type="button"
              onClick={markAll}
              style={{
                height: '36px',
                padding: '0 14px',
                borderRadius: '18px',
                border: '1px solid #0C1B38',
                background: '#0C1B38',
                color: '#FFFFFF',
                fontSize: '13px',
                fontWeight: 500,
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <Icon name="check" size={14} strokeWidth={2.2} color="#C5AB7A" />
              {t('att.markAll')}
            </button>
          ) : null}
          {canUndo ? (
            <button
              type="button"
              onClick={undo}
              style={{
                height: '36px',
                padding: '0 14px',
                borderRadius: '18px',
                border: '1px solid #C9D3D2',
                background: 'transparent',
                color: '#13233F',
                fontSize: '13px',
                fontWeight: 500,
                animation: 'vIn10 200ms ease both',
              }}
            >
              {t('att.undo')}
            </button>
          ) : null}
        </div>
        <ul
          style={{
            listStyle: 'none',
            margin: 0,
            padding: '0 16px 16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
          }}
        >
          {data.names.map((name, i) => {
            const m = st[i]
            const rowStyle: React.CSSProperties = {
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              padding: '7px 7px 7px 14px',
              borderRadius: '10px',
              transition: 'background-color .18s ease, border-color .18s ease',
              border: '1px solid ' + (m ? '#D5DDE0' : '#DCC495'),
              background: m ? '#FDFDFB' : '#FAF4E6',
            }
            return (
              <li key={i} style={rowStyle}>
                <span
                  className="v-serif"
                  style={{
                    width: '26px',
                    fontFamily: 'var(--font-serif)',
                    fontSize: '16px',
                    color: '#56657A',
                    fontVariantNumeric: 'lining-nums tabular-nums',
                  }}
                >
                  {String(i + 1)}
                </span>
                <span
                  style={{
                    flexGrow: 1,
                    fontSize: '15px',
                    minWidth: 0,
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                  }}
                >
                  {name}
                </span>
                {m === 'L' ? (
                  <span style={leaveOldPill}>{t('att.leaveOld')}</span>
                ) : (
                  <div style={{ display: 'flex', gap: '6px' }}>
                    <button
                      type="button"
                      className="v-active-93"
                      aria-label={t('att.aria.present', { name })}
                      aria-pressed={m === 'P'}
                      disabled={locked}
                      onClick={set(i, 'P')}
                      style={m === 'P' ? on.P : off}
                    >
                      P
                    </button>
                    <button
                      type="button"
                      className="v-active-93"
                      aria-label={t('att.aria.absent', { name })}
                      aria-pressed={m === 'A'}
                      disabled={locked}
                      onClick={set(i, 'A')}
                      style={m === 'A' ? on.A : off}
                    >
                      A
                    </button>
                  </div>
                )}
              </li>
            )
          })}
        </ul>
      </div>
      {editing ? (
        <div
          style={{
            flexShrink: 0,
            background: '#FDFDFB',
            borderTop: '1px solid #D5DDE0',
            padding: '12px 16px 16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
          }}
        >
          {hasUnmarked ? (
            <span style={{ fontSize: '12px', color: '#8C6A2F', textAlign: 'center' }}>
              {t('att.notMarked', { n: cU })}
            </span>
          ) : null}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.6fr', gap: '10px' }}>
            <button
              type="button"
              onClick={saveDraft}
              style={{
                height: '50px',
                borderRadius: '6px',
                border: '1px solid #C9D3D2',
                background: 'transparent',
                color: '#13233F',
                fontSize: '15px',
                fontWeight: 500,
              }}
            >
              {t('att.saveDraft')}
            </button>
            <button type="button" disabled={hasUnmarked} onClick={submit} style={submitStyle}>
              {t('att.submit')}
            </button>
          </div>
        </div>
      ) : null}
      {submitted ? (
        <div
          style={{
            flexShrink: 0,
            background: '#0C1B38',
            color: '#FFFFFF',
            padding: '16px',
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
            animation: 'vIn10 300ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          <span
            style={{
              width: '40px',
              height: '40px',
              borderRadius: '20px',
              border: '1px solid rgba(197,171,122,0.5)',
              color: '#C5AB7A',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: 0,
              animation: 'vPopAtt 400ms ease both',
            }}
          >
            <Icon name="clock" size={20} strokeWidth={1.8} />
          </span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span
              className="v-serif"
              style={{ fontFamily: 'var(--font-serif)', fontSize: '19px' }}
            >
              {t('att.submittedTitle')}
            </span>
            <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t('att.submittedSub')}</span>
          </div>
        </div>
      ) : null}
      {toast ? (
        <div
          role="status"
          style={{
            position: 'absolute',
            left: '16px',
            right: '16px',
            bottom: '108px',
            background: '#0C1B38',
            color: '#FFFFFF',
            borderRadius: '10px',
            padding: '12px 14px',
            fontSize: '14px',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
            boxShadow: '0 10px 30px rgba(11,26,51,0.3)',
            animation: 'vIn10 220ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          <Icon name="check" size={18} strokeWidth={2} color="#C5AB7A" />
          {t('att.toast')}
        </div>
      ) : null}
    </div>
  )
}
