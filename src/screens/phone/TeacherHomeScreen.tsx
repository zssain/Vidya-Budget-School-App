// Teacher Home (phone: Android app + iPhone PWA) — a React translation of
// design/screens/TeacherHome.dc.html. The mock is STATIC; since repair T1 the
// layout and inline styles stay the mock's but every line is real data from a
// TeacherHomeView (lib/teacherHome.ts, built by TeacherHomeContainer): the
// greeting + date, one task row per class still to submit today (+ returned /
// decided requests), real tile targets and badges, the bell's unread circulars and
// the sync footer. The other substitutions are (docs/01-MOCK-SPEC.md §5/§6):
//   - keyframes: the mock's local vRise (14px) -> app name vRise14; vPulse ->
//     vPulse. (App keyframes live in src/styles/keyframes.css.)
//   - logo: the /_blob asset is imported from src/assets (Vite bundles it).
//   - bell / chevronRight / arrowUpRight SVGs -> <Icon> with the same
//     width/height/strokeWidth/colour.
//   - tile icons: the five whose two-tone geometry already lives in
//     src/lib/icons.ts (reportCard, myClasses, inbox, cloudSync, profile) render
//     via <Icon>; the four whose icons.ts entry holds the MONO sidebar geometry
//     (attendance, marks, students, requests) are inlined verbatim from the mock
//     so the two-tone teal/gold split is exact — fidelity wins.
//   - no hard-coded visible text: every string comes from t('teacher.*').
//   - links/tiles navigate via @/lib/router; :active scale(.96) via .v-active-96.
import { useEffect, useState, type ReactNode } from 'react'
import * as api from '@/lib/api'
import type { StaffDayDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { t, getLang } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { isWeb } from '@/lib/platform'
import type { TeacherHomeView, TeacherTileIcon } from '@/lib/teacherHome'
import logo from '@/assets/vidya-horizontal-on-dark.svg'
import logoHi from '@/assets/vidya-horizontal-hindi-on-dark.svg'
import { SyncProblemsBanner } from './SyncProblems'

// The four two-tone tile SVGs whose icons.ts entry is the mono sidebar geometry.
// Copied verbatim (paths + per-part teal #7DB1B5 / gold #C5AB7A stroke) from the
// mock's tile <svg>s, rendered at 34px / stroke-width 1.3.
const INLINE_TILE: Record<'attendance' | 'marks' | 'students' | 'requests', ReactNode> = {
  attendance: (
    <svg
      width="34"
      height="34"
      viewBox="0 0 24 24"
      fill="none"
      strokeWidth={1.3}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="3.5" y="5" width="17" height="15" rx="2" stroke="#7DB1B5" />
      <path d="M8 3v4M16 3v4M3.5 10h17" stroke="#7DB1B5" />
      <path d="m9 15 2 2 4-4" stroke="#C5AB7A" />
    </svg>
  ),
  marks: (
    <svg
      width="34"
      height="34"
      viewBox="0 0 24 24"
      fill="none"
      strokeWidth={1.3}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="5" y="4" width="14" height="17" rx="2" stroke="#7DB1B5" />
      <path d="M9 4V3h6v1" stroke="#7DB1B5" />
      <path d="M9 10h6M9 14h6M9 18h3" stroke="#C5AB7A" />
    </svg>
  ),
  students: (
    <svg
      width="34"
      height="34"
      viewBox="0 0 24 24"
      fill="none"
      strokeWidth={1.3}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="9" cy="8" r="3.5" stroke="#7DB1B5" />
      <path d="M2.5 20a6.5 6.5 0 0 1 13 0" stroke="#7DB1B5" />
      <path d="M16 4.5a3.5 3.5 0 0 1 0 7M18 14a6 6 0 0 1 3.5 6" stroke="#C5AB7A" />
    </svg>
  ),
  requests: (
    <svg
      width="34"
      height="34"
      viewBox="0 0 24 24"
      fill="none"
      strokeWidth={1.3}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M5 5h14l1 8v6H4v-6z" stroke="#7DB1B5" />
      <path d="M4 13h4l2 3h4l2-3h4" stroke="#C5AB7A" />
    </svg>
  ),
}

/** Draw a tile's glyph: <Icon> for the five two-tone names icons.ts stores, an
 *  inlined mock SVG for the four whose icons.ts entry is mono. */
function TileGlyph({ icon }: { icon: TeacherTileIcon }) {
  switch (icon) {
    case 'icon:reportCard':
      return <Icon name="reportCard" size={34} strokeWidth={1.3} />
    case 'icon:myClasses':
      return <Icon name="myClasses" size={34} strokeWidth={1.3} />
    case 'icon:inbox':
      return <Icon name="inbox" size={34} strokeWidth={1.3} />
    case 'icon:cloudSync':
      return <Icon name="cloudSync" size={34} strokeWidth={1.3} />
    case 'icon:profile':
      return <Icon name="profile" size={34} strokeWidth={1.3} />
    case 'inline:attendance':
      return INLINE_TILE.attendance
    case 'inline:marks':
      return INLINE_TILE.marks
    case 'inline:students':
      return INLINE_TILE.students
    case 'inline:requests':
      return INLINE_TILE.requests
  }
}

// Staff-HR check-in card (P17 Step 2) — a card on the Teacher Home (§10.5 "Home
// tile or card for staff"). Self-contained: it reads my_staff_day; if the HR
// module is off the call errors and the card hides. NOTE: this adds a card the
// original TeacherHome mock does not draw — the Teacher-Home fidelity baseline
// must be regenerated with owner approval (P11 attendance-change precedent).
function fmtTime12(min: number | null | undefined): string {
  if (min == null) return '—'
  const h = Math.floor(min / 60)
  const m = min % 60
  const ampm = h < 12 ? 'AM' : 'PM'
  const h12 = h % 12 === 0 ? 12 : h % 12
  return `${h12}:${String(m).padStart(2, '0')} ${ampm}`
}

function CheckInBanner() {
  const [day, setDay] = useState<StaffDayDto | null>(null)
  const [hidden, setHidden] = useState(false)
  useEffect(() => {
    const d = new Date()
    const today = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    api.my_staff_day(today).then(setDay).catch(() => setHidden(true))
  }, [])
  if (hidden) return null
  const today = day?.today ?? null
  const checkedIn = !!today && today.check_in_min != null
  const onLeave = today?.status === 'leave'
  const away = today?.status === 'away_pending'
  const title = onLeave ? t('staffhr.day.onLeaveToday') : checkedIn ? t('staffhr.home.checkedIn', { time: fmtTime12(today!.check_in_min) }) : t('staffhr.home.checkIn')
  const sub = onLeave ? '' : away ? t('staffhr.home.away') : checkedIn ? '' : t('staffhr.home.checkInSub')
  return (
    <button
      type="button"
      className="v-active-96"
      onClick={() => navigate('/teacher/checkin')}
      style={{ display: 'flex', alignItems: 'center', gap: 14, width: '100%', textAlign: 'left', padding: '14px 16px', borderRadius: 16, border: '1px solid rgba(255,255,255,0.12)', background: 'rgba(255,255,255,0.05)', color: '#FFFFFF' }}
    >
      <span style={{ width: 42, height: 42, borderRadius: 21, flexShrink: 0, border: '1px solid rgba(197,171,122,0.5)', color: '#C5AB7A', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Icon name={checkedIn || onLeave ? 'check' : 'clock'} size={20} strokeWidth={1.7} />
      </span>
      <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0 }}>
        <b style={{ fontWeight: 600, fontSize: 15 }}>{title}</b>
        {sub ? <span style={{ fontSize: 12, color: '#9FACBF' }}>{sub}</span> : null}
      </span>
      <Icon name="chevronRight" size={18} strokeWidth={1.6} />
    </button>
  )
}

export default function TeacherHomeScreen({ view }: { view: TeacherHomeView }) {
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        background: 'radial-gradient(110% 45% at 85% 0%, #1A3560 0%, #0C1B38 60%)',
        color: '#FFFFFF',
        fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif",
        fontSize: '14px',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          flexGrow: 1,
          overflow: 'auto',
          padding: '18px 16px 24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        {/* Header: logo + bell (gold dot = unread circulars) + initials account button. */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <img
            src={getLang() === 'hi' ? logoHi : logo}
            alt="Vidya School Management"
            width={132}
            height={43}
            style={{ width: '132px', height: '43px', display: 'block' }}
          />
          <div style={{ display: 'flex', gap: '8px' }}>
            {view.tiles.some((tile) => tile.id === 'inbox') && (
              <button
                type="button"
                aria-label={view.unread > 0 ? t('teacher.aria.notificationsUnread', { n: view.unread }) : t('teacher.aria.notifications')}
                onClick={() => navigate('/teacher/inbox')}
                style={{
                  position: 'relative',
                  width: '44px',
                  height: '44px',
                  borderRadius: '22px',
                  border: '1px solid rgba(255,255,255,0.14)',
                  background: 'rgba(255,255,255,0.04)',
                  color: '#FFFFFF',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Icon name="bell" size={19} strokeWidth={1.6} />
                {view.unread > 0 ? (
                  <span
                    style={{
                      position: 'absolute',
                      top: '10px',
                      right: '11px',
                      width: '7px',
                      height: '7px',
                      borderRadius: '4px',
                      background: '#C5AB7A',
                    }}
                  ></span>
                ) : null}
              </button>
            )}
            <button
              type="button"
              aria-label={t('teacher.aria.account')}
              onClick={() => navigate('/teacher/profile')}
              style={{
                width: '44px',
                height: '44px',
                borderRadius: '22px',
                border: 0,
                background: '#1C3358',
                color: '#E6D3A8',
                fontWeight: 600,
                fontSize: '14px',
              }}
            >
              {view.initials}
            </button>
          </div>
        </div>

        {/* Greeting block. */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '10px',
            animation: 'vRise14 520ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          <span
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              fontSize: '10px',
              fontWeight: 600,
              letterSpacing: '0.16em',
              textTransform: 'uppercase',
              color: '#9FACBF',
            }}
          >
            <span
              style={{ width: '5px', height: '5px', borderRadius: '3px', background: '#C5AB7A' }}
            ></span>
            {view.eyebrow}
          </span>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            <h1
              className="v-serif"
              style={{
                margin: 0,
                fontFamily: 'var(--font-serif)',
                fontWeight: 400,
                fontSize: '34px',
                lineHeight: 1.05,
                letterSpacing: '-0.02em',
              }}
            >
              {view.greeting}
            </h1>
            <span
              className="v-serif"
              style={{
                fontFamily: 'var(--font-serif)',
                fontStyle: 'italic',
                fontSize: '34px',
                lineHeight: 1.1,
                letterSpacing: '-0.02em',
                color: '#C5AB7A',
              }}
            >
              {view.dueLine}
            </span>
          </div>
        </div>

        {/* Staff-HR check-in card (P17). Hidden when the HR module is off. */}
        <CheckInBanner />

        {/* Changes the school computer refused (B1). Hidden when none. */}
        {isWeb && <SyncProblemsBanner />}

        {/* Task card: one row per real task (attendance due today, returned/decided
            requests), or a single "All caught up" row. */}
        <section
          style={{
            border: '1px solid rgba(255,255,255,0.12)',
            borderRadius: '18px',
            background: 'rgba(255,255,255,0.04)',
            overflow: 'hidden',
            animation: 'vRise14 520ms 80ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          {view.tasks.length === 0 ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '14px', padding: '16px', color: '#FFFFFF' }}>
              <span style={{ width: '10px', height: '10px', borderRadius: '5px', background: '#5FD0A0', flexShrink: 0 }}></span>
              <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: '3px' }}>
                <span style={{ fontSize: '15px', fontWeight: 500 }}>{t('teacher.task.none')}</span>
                <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t('teacher.task.noneSub')}</span>
              </span>
            </div>
          ) : (
            view.tasks.map((task, i) => (
              <a
                key={task.key}
                href={`#${task.to}`}
                onClick={(e) => {
                  e.preventDefault()
                  navigate(task.to)
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '14px',
                  padding: '16px',
                  textDecoration: 'none',
                  color: '#FFFFFF',
                  borderTop: i > 0 ? '1px solid rgba(255,255,255,0.10)' : undefined,
                }}
              >
                <span
                  style={{
                    width: '10px',
                    height: '10px',
                    borderRadius: '5px',
                    background: task.dot,
                    flexShrink: 0,
                    animation: task.pulse ? 'vPulse 2.2s ease-in-out infinite' : undefined,
                  }}
                ></span>
                <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: '3px', minWidth: 0 }}>
                  <span style={{ fontSize: '15px', fontWeight: 500 }}>{task.title}</span>
                  <span style={{ fontSize: '12px', color: '#9FACBF' }}>{task.sub}</span>
                </span>
                {task.action ? (
                  <span
                    className="v-active-96"
                    style={{
                      height: '38px',
                      padding: '0 14px',
                      borderRadius: '19px',
                      background: '#C5AB7A',
                      color: '#0C1B38',
                      fontSize: '13px',
                      fontWeight: 600,
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                      flexShrink: 0,
                    }}
                  >
                    {task.action.label}
                    <Icon name="arrowUpRight" size={14} strokeWidth={2} />
                  </span>
                ) : (
                  <Icon name="chevronRight" size={18} strokeWidth={1.75} color="#9FACBF" />
                )}
              </a>
            ))
          )}
        </section>

        {/* 3-column grid of tiles (a tile whose commands this platform lacks is hidden). */}
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            gap: '10px',
          }}
        >
          {view.tiles.map((tile) => (
            <a
              key={tile.id}
              href={`#${tile.to}`}
              className="tile v-active-96"
              data-tile={tile.id}
              onClick={(e) => {
                e.preventDefault()
                navigate(tile.to)
              }}
              style={{
                position: tile.badge ? 'relative' : undefined,
                height: '112px',
                borderRadius: '16px',
                background: 'rgba(255,255,255,0.05)',
                border: '1px solid rgba(255,255,255,0.08)',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '10px',
                textDecoration: 'none',
                color: '#E8EDF4',
                fontSize: '13px',
                textAlign: 'center',
                animation: `vRise14 480ms ${tile.delay} cubic-bezier(.2,.8,.2,1) both`,
              }}
            >
              {tile.badge ? (
                <span
                  style={{
                    position: 'absolute',
                    top: '10px',
                    right: '10px',
                    minWidth: '20px',
                    height: '20px',
                    borderRadius: '10px',
                    background: '#C5AB7A',
                    color: '#0C1B38',
                    fontSize: '11px',
                    fontWeight: 700,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  {tile.badge}
                </span>
              ) : null}
              <TileGlyph icon={tile.icon} />
              {t(tile.labelKey)}
            </a>
          ))}
        </div>

        {/* Footer: the real sync state (hidden when there is nothing true to say). */}
        {view.footer ? (
          <div
            role="status"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              fontSize: '12px',
              color: '#9FACBF',
            }}
          >
            <span
              style={{ width: '6px', height: '6px', borderRadius: '3px', background: view.footer.tone === 'ok' ? '#5FD0A0' : '#C5AB7A' }}
            ></span>
            {view.footer.text}
          </div>
        ) : null}
      </div>
    </div>
  )
}
