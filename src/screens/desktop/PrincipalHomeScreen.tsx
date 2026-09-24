import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import type { PrincipalHomeData } from '@/dev/fixtures/principalHome'
import logo from '@/assets/vidya-horizontal-on-dark.svg'

// Principal Home — pixel-exact translation of design/screens/Main.dc.html.
// Every element, order and inline style is copied verbatim into style={{...}};
// the renderVals() computed styles (stat cells, approval pills, class/fee bars)
// are reproduced here exactly. Substitutions (only these):
//   accent → var(--accent); accentSoft rgba(0.10) → var(--accent-10);
//   attendance-correction pill rgba(0.12)/accent → var(--accent-12)/var(--accent);
//   @keyframes vRise (12px) → the app name vRise12.
// Visible English text comes from t(); DATA comes from the fixture.

const EASE = 'cubic-bezier(.2,.8,.2,1)'

// --- renderVals() equivalents --------------------------------------------------

// cell(i): stat-strip cell. First cell has no left border.
function cellStyle(i: number): CSSProperties {
  return {
    display: 'flex',
    flexDirection: 'column',
    gap: '10px',
    padding: '22px 24px',
    textDecoration: 'none',
    color: 'inherit',
    ...(i > 0 ? { borderLeft: '1px solid #C9D6D5' } : {}),
  }
}

// pill(bg, fg): 164×26 approval badge. min-width (not width) so Hindi never
// truncates (P08 Part F); English content ≤164px still renders at exactly 164px,
// so the fidelity baseline is unchanged.
function pillStyle(bg: string, fg: string): CSSProperties {
  return {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: '164px',
    height: '26px',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 500,
    whiteSpace: 'nowrap',
    background: bg,
    color: fg,
  }
}

// class bar: vFill 800ms (300 + i·60)ms; width p%; background accent.
function classBarStyle(pct: number | null, i: number): CSSProperties {
  return {
    height: '100%',
    borderRadius: '3px',
    transformOrigin: 'left',
    animation: `vFill 800ms ${300 + i * 60}ms ${EASE} both`,
    width: `${pct || 0}%`,
    background: 'var(--accent)',
  }
}

// class pct tone: gold when not submitted, else muted.
function classToneStyle(pct: number | null): CSSProperties {
  return pct == null
    ? { color: '#8C6A2F', fontWeight: 600 }
    : { color: '#56657A' }
}

// fee bar: vGrow 700ms (300 + i·70)ms; height round(v/55000·96); gold on Today.
function feeBarStyle(day: string, value: number, i: number): CSSProperties {
  return {
    width: '100%',
    maxWidth: '36px',
    borderRadius: '4px 4px 2px 2px',
    transformOrigin: 'bottom',
    animation: `vGrow 700ms ${300 + i * 70}ms ${EASE} both`,
    height: `${Math.round((value / 55000) * 96)}px`,
    background: day === 'Today' ? '#C5AB7A' : 'rgba(125,177,181,0.55)',
  }
}

// `chrome` (default true) renders the pixel-exact standalone mock (own sidebar +
// header) used by the fidelity gallery. The real app passes chrome={false}: the
// shared desktop AppShell owns the sidebar + header, and only the main content
// column renders here. The content JSX is identical in both modes.
export default function PrincipalHomeScreen({ data, chrome = true }: { data: PrincipalHomeData; chrome?: boolean }) {
  return (
    <div
      style={
        chrome
          ? {
              width: '1440px',
              height: '1080px',
              display: 'flex',
              background: '#F5F7F6',
              color: '#13233F',
              fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
              fontSize: '14px',
              overflow: 'hidden',
            }
          : {
              display: 'flex',
              width: '100%',
              minHeight: '100%',
              background: '#F5F7F6',
              color: '#13233F',
              fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
              fontSize: '14px',
            }
      }
    >
      {/* Sidebar */}
      {chrome && (
      <aside
        style={{
          width: '256px',
          flexShrink: 0,
          background: '#0C1B38',
          color: '#FFFFFF',
          display: 'flex',
          flexDirection: 'column',
          padding: '26px 16px 20px',
          boxSizing: 'border-box',
          gap: '28px',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', padding: '0 10px' }}>
          <img
            src={logo}
            alt={t('home.logoAlt')}
            width={160}
            height={52}
            style={{ width: '160px', height: '52px', display: 'block' }}
          />
          <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t('home.school')}</span>
        </div>
        <nav aria-label="Main" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          {/* Overview */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                letterSpacing: '0.16em',
                textTransform: 'uppercase',
                color: '#8E9AAE',
                padding: '0 10px 8px',
              }}
            >
              {t('home.nav.section.overview')}
            </div>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                background: 'rgba(255,255,255,0.07)',
                color: '#FFFFFF',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="home" size={18} strokeWidth={1.6} color="#C5AB7A" />
              <span>{t('home.nav.home')}</span>
              <span
                style={{
                  marginLeft: 'auto',
                  width: '5px',
                  height: '5px',
                  borderRadius: '3px',
                  background: '#C5AB7A',
                }}
              />
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="approvals" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.approvals')}</span>
              <span
                style={{
                  marginLeft: 'auto',
                  minWidth: '20px',
                  height: '20px',
                  padding: '0 6px',
                  borderRadius: '10px',
                  background: '#C5AB7A',
                  color: '#0C1B38',
                  fontSize: '11px',
                  fontWeight: 700,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxSizing: 'border-box',
                }}
              >
                4
              </span>
            </a>
          </div>
          {/* Academics */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                letterSpacing: '0.16em',
                textTransform: 'uppercase',
                color: '#8E9AAE',
                padding: '0 10px 8px',
              }}
            >
              {t('home.nav.section.academics')}
            </div>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="students" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.students')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="attendance" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.attendance')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="marks" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.marks')}</span>
            </a>
          </div>
          {/* Finance */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                letterSpacing: '0.16em',
                textTransform: 'uppercase',
                color: '#8E9AAE',
                padding: '0 10px 8px',
              }}
            >
              {t('home.nav.section.finance')}
            </div>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="fees" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.fees')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="daybook" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.daybook')}</span>
            </a>
          </div>
          {/* School */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                letterSpacing: '0.16em',
                textTransform: 'uppercase',
                color: '#8E9AAE',
                padding: '0 10px 8px',
              }}
            >
              {t('home.nav.section.school')}
            </div>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="staff" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.staff')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="sync" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.sync')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="backups" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.backups')}</span>
            </a>
            <a
              href="#"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '11px',
                height: '38px',
                padding: '0 10px',
                borderRadius: '6px',
                color: '#C9D2DE',
                textDecoration: 'none',
                fontWeight: 500,
              }}
            >
              <Icon name="settings" size={18} strokeWidth={1.6} />
              <span>{t('home.nav.settings')}</span>
            </a>
          </div>
        </nav>
        <div
          style={{
            marginTop: 'auto',
            display: 'flex',
            flexDirection: 'column',
            gap: '14px',
          }}
        >
          <div
            style={{
              border: '1px solid rgba(255,255,255,0.12)',
              borderRadius: '10px',
              padding: '12px 14px',
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                fontSize: '13px',
                fontWeight: 500,
              }}
            >
              <span
                style={{
                  width: '7px',
                  height: '7px',
                  borderRadius: '4px',
                  background: '#5FD0A0',
                  boxShadow: '0 0 0 3px rgba(95,208,160,0.18)',
                }}
              />
              {t('home.serverOnline')}
            </div>
            <div style={{ fontSize: '12px', color: '#9FACBF' }}>{t('home.thisPc')}</div>
          </div>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              padding: '0 6px',
            }}
          >
            <div
              style={{
                width: '34px',
                height: '34px',
                borderRadius: '17px',
                background: '#1C3358',
                color: '#E6D3A8',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontWeight: 600,
                fontSize: '13px',
              }}
            >
              {t('home.user.initials')}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column' }}>
              <span style={{ fontWeight: 500, fontSize: '13px' }}>{t('home.user.name')}</span>
              <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t('home.user.role')}</span>
            </div>
          </div>
        </div>
      </aside>
      )}

      {/* Main column */}
      <div
        style={{
          flexGrow: 1,
          display: 'flex',
          flexDirection: 'column',
          minWidth: 0,
        }}
      >
        {/* Header */}
        {chrome && (
        <header
          style={{
            height: '64px',
            flexShrink: 0,
            borderBottom: '1px solid #D5DDE0',
            display: 'flex',
            alignItems: 'center',
            gap: '14px',
            padding: '0 40px',
            boxSizing: 'border-box',
          }}
        >
          <button
            type="button"
            style={{
              height: '36px',
              padding: '0 12px',
              borderRadius: '6px',
              border: '1px solid #D5DDE0',
              background: '#FDFDFB',
              color: '#13233F',
              fontSize: '13px',
              fontWeight: 500,
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
            }}
          >
            {t('home.session')}
            <Icon name="chevronDown" size={15} strokeWidth={1.75} />
          </button>
          <div style={{ position: 'relative', width: '400px' }}>
            <Icon
              name="search"
              size={16}
              strokeWidth={1.75}
              color="#56657A"
              style={{ position: 'absolute', left: '12px', top: '11px' }}
            />
            <input
              type="search"
              aria-label={t('home.searchLabel')}
              placeholder={t('home.searchPlaceholder')}
              style={{
                width: '100%',
                height: '38px',
                boxSizing: 'border-box',
                borderRadius: '6px',
                border: '1px solid #D5DDE0',
                background: '#FDFDFB',
                padding: '0 16px 0 36px',
                fontFamily: 'inherit',
                fontSize: '14px',
                color: '#13233F',
              }}
            />
          </div>
          <div
            style={{
              marginLeft: 'auto',
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
            }}
          >
            <span
              style={{
                height: '34px',
                padding: '0 12px',
                borderRadius: '17px',
                background: 'var(--accent-10)',
                color: 'var(--accent)',
                fontSize: '13px',
                fontWeight: 500,
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
              }}
            >
              <span
                style={{
                  width: '6px',
                  height: '6px',
                  borderRadius: '3px',
                  background: 'var(--accent)',
                }}
              />
              {t('home.syncPill')}
            </span>
            <button
              type="button"
              aria-label={t('home.bellLabel')}
              style={{
                position: 'relative',
                width: '38px',
                height: '38px',
                borderRadius: '19px',
                border: '1px solid #D5DDE0',
                background: '#FDFDFB',
                color: '#13233F',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <Icon name="bell" size={17} strokeWidth={1.6} />
              <span
                style={{
                  position: 'absolute',
                  top: '7px',
                  right: '8px',
                  width: '7px',
                  height: '7px',
                  borderRadius: '4px',
                  background: '#C5AB7A',
                }}
              />
            </button>
          </div>
        </header>
        )}

        {/* Main */}
        <main
          style={{
            flexGrow: 1,
            padding: '32px 40px',
            display: 'flex',
            flexDirection: 'column',
            gap: '24px',
            boxSizing: 'border-box',
            minHeight: 0,
          }}
        >
          {/* Title + actions */}
          <div
            style={{
              display: 'flex',
              alignItems: 'flex-end',
              justifyContent: 'space-between',
              gap: '24px',
              animation: `vRise12 520ms ${EASE} both`,
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <span
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  fontSize: '11px',
                  fontWeight: 600,
                  letterSpacing: '0.14em',
                  textTransform: 'uppercase',
                  color: 'var(--accent)',
                }}
              >
                <span
                  style={{
                    width: '5px',
                    height: '5px',
                    borderRadius: '3px',
                    background: 'var(--accent)',
                  }}
                />
                {t('home.eyebrow')}
              </span>
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                <h1
                  className="v-serif"
                  style={{
                    margin: 0,
                    fontFamily: 'var(--font-serif)',
                    fontWeight: 400,
                    fontSize: '46px',
                    lineHeight: 1.05,
                    letterSpacing: '-0.025em',
                  }}
                >
                  {t('home.h1')}
                </h1>
                <span
                  className="v-serif"
                  style={{
                    fontFamily: 'var(--font-serif)',
                    fontStyle: 'italic',
                    fontSize: '46px',
                    lineHeight: 1.1,
                    letterSpacing: '-0.025em',
                    color: 'var(--accent)',
                  }}
                >
                  {t('home.h1sub')}
                </span>
              </div>
            </div>
            <div style={{ display: 'flex', gap: '10px' }}>
              <button
                type="button"
                style={{
                  height: '44px',
                  padding: '0 18px',
                  borderRadius: '6px',
                  border: '1px solid #C9D3D2',
                  background: 'transparent',
                  color: '#13233F',
                  fontSize: '14px',
                  fontWeight: 500,
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                }}
              >
                <Icon name="plus" size={16} strokeWidth={1.75} />
                {t('home.newAdmission')}
              </button>
              <button
                type="button"
                style={{
                  height: '44px',
                  padding: '0 20px',
                  borderRadius: '6px',
                  border: 0,
                  background: 'var(--accent)',
                  color: '#FFFFFF',
                  fontSize: '14px',
                  fontWeight: 500,
                  display: 'flex',
                  alignItems: 'center',
                  gap: '14px',
                }}
              >
                {t('home.reviewApprovals')}
                <Icon name="arrowUpRight" size={16} strokeWidth={1.75} />
              </button>
            </div>
          </div>

          {/* Stat strip */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(4, minmax(0, 1fr))',
              background: '#E2EAEB',
              borderRadius: '16px',
              animation: `vRise12 520ms 80ms ${EASE} both`,
            }}
          >
            {data.stats.map((k, i) => (
              <a key={i} href="#" style={cellStyle(i)}>
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'flex-start',
                  }}
                >
                  <span
                    className="v-serif"
                    style={{
                      fontFamily: 'var(--font-serif)',
                      fontSize: '40px',
                      lineHeight: 1,
                      letterSpacing: '-0.02em',
                      fontVariantNumeric: 'lining-nums tabular-nums',
                      color: '#13233F',
                    }}
                  >
                    {k.value}
                  </span>
                  <Icon name="arrowUpRight" size={14} strokeWidth={1.75} color="#13233F" />
                </div>
                <span style={{ fontSize: '14px', color: '#13233F' }}>{k.label}</span>
                <span style={{ fontSize: '12px', color: '#56657A' }}>{k.note}</span>
              </a>
            ))}
          </div>

          {/* Approvals + Needs attention */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '2fr 1fr',
              gap: '24px',
              animation: `vRise12 520ms 160ms ${EASE} both`,
            }}
          >
            <section
              style={{
                background: '#FDFDFB',
                border: '1px solid #D5DDE0',
                borderRadius: '16px',
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'flex-end',
                  justifyContent: 'space-between',
                  padding: '20px 24px 16px',
                }}
              >
                <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                  <span
                    style={{
                      fontSize: '10px',
                      fontWeight: 600,
                      letterSpacing: '0.16em',
                      textTransform: 'uppercase',
                      color: '#56657A',
                    }}
                  >
                    {t('home.approvals.eyebrow')}
                  </span>
                  <h2
                    className="v-serif"
                    style={{
                      margin: 0,
                      fontFamily: 'var(--font-serif)',
                      fontWeight: 400,
                      fontSize: '26px',
                      letterSpacing: '-0.015em',
                    }}
                  >
                    {t('home.approvals.title')}
                  </h2>
                </div>
                <a
                  href="#"
                  style={{
                    fontSize: '13px',
                    textDecoration: 'none',
                    color: '#13233F',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    borderBottom: '1px solid #13233F',
                    paddingBottom: '2px',
                  }}
                >
                  {t('home.approvals.openAll')}
                  <Icon name="arrowUpRight" size={13} strokeWidth={1.75} />
                </a>
              </div>
              {data.approvals.map((r, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '16px',
                    padding: '14px 24px',
                    borderTop: '1px solid #E8EDEC',
                  }}
                >
                  <span style={pillStyle(r.badge.bg, r.badge.fg)}>{r.type}</span>
                  <div
                    style={{
                      flexGrow: 1,
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '3px',
                      minWidth: 0,
                    }}
                  >
                    <span
                      style={{
                        fontWeight: 500,
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {r.what}
                    </span>
                    <span style={{ fontSize: '12px', color: '#56657A' }}>
                      {r.who} · {r.age}
                    </span>
                  </div>
                  <button
                    type="button"
                    style={{
                      height: '34px',
                      padding: '0 14px',
                      borderRadius: '6px',
                      border: '1px solid #C9D3D2',
                      background: 'transparent',
                      color: '#13233F',
                      fontSize: '13px',
                      fontWeight: 500,
                    }}
                  >
                    {t('home.approvals.review')}
                  </button>
                </div>
              ))}
            </section>

            <section
              style={{
                background: '#FDFDFB',
                border: '1px solid #D5DDE0',
                borderRadius: '16px',
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  padding: '20px 24px 12px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
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
                  {t('home.needs.eyebrow')}
                </span>
                <h2
                  className="v-serif"
                  style={{
                    margin: 0,
                    fontFamily: 'var(--font-serif)',
                    fontWeight: 400,
                    fontSize: '26px',
                    letterSpacing: '-0.015em',
                  }}
                >
                  {t('home.needs.title')}
                </h2>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', flexGrow: 1 }}>
                <div
                  style={{
                    display: 'flex',
                    gap: '12px',
                    padding: '14px 24px',
                    borderTop: '1px solid #E8EDEC',
                  }}
                >
                  <span
                    style={{
                      width: '8px',
                      height: '8px',
                      marginTop: '6px',
                      borderRadius: '4px',
                      background: '#C5AB7A',
                      flexShrink: 0,
                    }}
                  />
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
                    <span style={{ fontWeight: 500 }}>{t('home.needs.attendance.title')}</span>
                    <span style={{ fontSize: '12px', color: '#56657A' }}>
                      {t('home.needs.attendance.sub')}
                    </span>
                  </div>
                </div>
                <div
                  style={{
                    display: 'flex',
                    gap: '12px',
                    padding: '14px 24px',
                    borderTop: '1px solid #E8EDEC',
                  }}
                >
                  <span
                    style={{
                      width: '8px',
                      height: '8px',
                      marginTop: '6px',
                      borderRadius: '4px',
                      background: '#C0392B',
                      flexShrink: 0,
                    }}
                  />
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
                    <span style={{ fontWeight: 500 }}>{t('home.needs.conflict.title')}</span>
                    <span style={{ fontSize: '12px', color: '#56657A' }}>
                      {t('home.needs.conflict.sub')}
                    </span>
                  </div>
                </div>
              </div>
              <div
                style={{
                  background: '#E2EAEB',
                  padding: '14px 24px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '3px',
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
                  {t('home.lastBackup')}
                </span>
                <span style={{ fontSize: '13px', color: '#13233F' }}>
                  {t('home.lastBackupValue')}
                </span>
              </div>
            </section>
          </div>

          {/* Attendance by class + Fee collection */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '3fr 2fr',
              gap: '24px',
              minHeight: 0,
              animation: `vRise12 520ms 240ms ${EASE} both`,
            }}
          >
            <section
              style={{
                background: '#FDFDFB',
                border: '1px solid #D5DDE0',
                borderRadius: '16px',
                padding: '18px 24px',
                display: 'flex',
                flexDirection: 'column',
                gap: '10px',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'baseline',
                }}
              >
                <h2
                  className="v-serif"
                  style={{
                    margin: 0,
                    fontFamily: 'var(--font-serif)',
                    fontWeight: 400,
                    fontSize: '22px',
                  }}
                >
                  {t('home.attByClass')}
                </h2>
                <a
                  href="#"
                  style={{ fontSize: '13px', textDecoration: 'none', color: 'var(--accent)' }}
                >
                  {t('home.fullRegister')}
                </a>
              </div>
              {data.classes.map((c, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '14px',
                    height: '24px',
                  }}
                >
                  <span style={{ width: '60px', fontSize: '13px' }}>{c.name}</span>
                  <div
                    style={{
                      flexGrow: 1,
                      height: '6px',
                      borderRadius: '3px',
                      background: '#E8EDEC',
                      overflow: 'hidden',
                    }}
                  >
                    <div style={classBarStyle(c.pct, i)} />
                  </div>
                  <span
                    style={{
                      width: '110px',
                      textAlign: 'right',
                      fontSize: '12px',
                      fontVariantNumeric: 'tabular-nums',
                      ...classToneStyle(c.pct),
                    }}
                  >
                    {c.pct == null
                      ? t('home.classNotSubmitted')
                      : t('home.classPresent', { pct: c.pct })}
                  </span>
                </div>
              ))}
            </section>

            <section
              style={{
                background: '#0C1B38',
                color: '#FFFFFF',
                borderRadius: '16px',
                padding: '18px 24px',
                display: 'flex',
                flexDirection: 'column',
                gap: '10px',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'baseline',
                }}
              >
                <h2
                  className="v-serif"
                  style={{
                    margin: 0,
                    fontFamily: 'var(--font-serif)',
                    fontWeight: 400,
                    fontSize: '22px',
                  }}
                >
                  {t('home.feeCollection')}
                </h2>
                <span style={{ fontSize: '12px', color: '#9FACBF' }}>{t('home.last6')}</span>
              </div>
              <span
                className="v-serif"
                style={{
                  fontFamily: 'var(--font-serif)',
                  fontStyle: 'italic',
                  fontSize: '28px',
                  color: '#C5AB7A',
                  fontVariantNumeric: 'lining-nums',
                }}
              >
                {data.feeTotal}
              </span>
              <div
                style={{
                  flexGrow: 1,
                  display: 'flex',
                  alignItems: 'flex-end',
                  gap: '14px',
                  minHeight: '110px',
                }}
              >
                {data.days.map((d, i) => (
                  <div
                    key={i}
                    style={{
                      flex: 1,
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      gap: '6px',
                    }}
                  >
                    <div style={feeBarStyle(d.day, d.value, i)} />
                    <span style={{ fontSize: '11px', color: '#9FACBF' }}>{d.day}</span>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </main>
      </div>
    </div>
  )
}
