// Teacher Home (Android phone) — a pixel-exact React translation of
// design/screens/TeacherHome.dc.html. The mock is STATIC (renderVals() returns
// {}). Every element, order and inline style is copied verbatim; the ONLY
// substitutions are (docs/01-MOCK-SPEC.md §5/§6):
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
import type { ReactNode } from 'react'
import { Icon } from '@/components/Icon'
import { t, getLang } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import logo from '@/assets/vidya-horizontal-on-dark.svg'
import logoHi from '@/assets/vidya-horizontal-hindi-on-dark.svg'
import type { TeacherHomeData, TeacherTileIcon } from '@/dev/fixtures/teacherHome'

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

export default function TeacherHomeScreen({ data }: { data: TeacherHomeData }) {
  const [attendanceTask, replyTask] = data.tasks

  return (
    <div
      style={{
        width: '390px',
        height: '844px',
        display: 'flex',
        flexDirection: 'column',
        background: 'radial-gradient(110% 45% at 85% 0%, #1A3560 0%, #0C1B38 60%)',
        color: '#FFFFFF',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
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
        {/* Header: logo + bell (gold dot) + "MI" account button. */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <img
            src={getLang() === 'hi' ? logoHi : logo}
            alt="Vidya Budget School"
            width={132}
            height={43}
            style={{ width: '132px', height: '43px', display: 'block' }}
          />
          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              type="button"
              aria-label={t('teacher.aria.notifications')}
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
            </button>
            <button
              type="button"
              aria-label={t('teacher.aria.account')}
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
              MI
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
            {t('teacher.eyebrow')}
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
              {t('teacher.h1')}
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
              {t('teacher.h1sub')}
            </span>
          </div>
        </div>

        {/* Task card. */}
        <section
          style={{
            border: '1px solid rgba(255,255,255,0.12)',
            borderRadius: '18px',
            background: 'rgba(255,255,255,0.04)',
            overflow: 'hidden',
            animation: 'vRise14 520ms 80ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          {/* V-A attendance -> Attendance. */}
          <a
            href="Attendance.dc.html"
            onClick={(e) => {
              e.preventDefault()
              navigate('/teacher/attendance/v-a')
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '14px',
              padding: '16px',
              textDecoration: 'none',
              color: '#FFFFFF',
            }}
          >
            <span
              style={{
                width: '10px',
                height: '10px',
                borderRadius: '5px',
                background: attendanceTask.dot,
                flexShrink: 0,
                animation: attendanceTask.pulse ? 'vPulse 2.2s ease-in-out infinite' : undefined,
              }}
            ></span>
            <span
              style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: '3px' }}
            >
              <span style={{ fontSize: '15px', fontWeight: 500 }}>
                {t(attendanceTask.titleKey)}
              </span>
              <span style={{ fontSize: '12px', color: '#9FACBF' }}>
                {t(attendanceTask.subKey)}
              </span>
            </span>
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
              }}
            >
              {t('teacher.start')}
              <Icon name="arrowUpRight" size={14} strokeWidth={2} />
            </span>
          </a>
          {/* Reply to the Principal. */}
          <a
            href="#"
            onClick={(e) => e.preventDefault()}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '14px',
              padding: '16px',
              textDecoration: 'none',
              color: '#FFFFFF',
              borderTop: '1px solid rgba(255,255,255,0.10)',
            }}
          >
            <span
              style={{
                width: '10px',
                height: '10px',
                borderRadius: '5px',
                background: replyTask.dot,
                flexShrink: 0,
              }}
            ></span>
            <span
              style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: '3px' }}
            >
              <span style={{ fontSize: '15px', fontWeight: 500 }}>{t(replyTask.titleKey)}</span>
              <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t(replyTask.subKey)}</span>
            </span>
            <Icon name="chevronRight" size={18} strokeWidth={1.75} color="#9FACBF" />
          </a>
        </section>

        {/* 3-column grid of 9 tiles. */}
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            gap: '10px',
          }}
        >
          {data.tiles.map((tile) => (
            <a
              key={tile.labelKey}
              href={tile.to === '/teacher/attendance/v-a' ? 'Attendance.dc.html' : '#'}
              className="tile v-active-96"
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

        {/* Footer. */}
        <div
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
            style={{ width: '6px', height: '6px', borderRadius: '3px', background: '#5FD0A0' }}
          ></span>
          {t('teacher.footer')}
        </div>
      </div>
    </div>
  )
}
