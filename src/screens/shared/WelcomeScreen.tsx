// Welcome screen — a pixel-exact React translation of design/screens/Welcome.dc.html.
// Every element, order and inline style is copied verbatim; the ONLY substitutions
// are (docs/01-MOCK-SPEC.md §2/§5):
//   - accent: {{accent}} -> var(--accent); ring rgba(0.12) -> var(--accent-12);
//     selected-radio bg rgba(0.06) -> var(--accent-6).
//   - keyframes: the mock's local vRise (14px) -> app name vRise14; local vIn (6px)
//     -> vIn6. (App keyframes live in src/styles/keyframes.css.)
//   - logos: the two /_blob assets are imported from src/assets (Vite bundles them).
//   - help / shield / arrow SVGs -> <Icon> with the same width/height/strokeWidth.
//   - no hard-coded visible text: every string comes from t('welcome.*').
import { useState } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { isPhone } from '@/lib/platform'
import wordmark from '@/assets/vidya-horizontal-on-light.svg'
import doorMark from '@/assets/dwaar-mark-on-dark.svg'
import type { WelcomeData } from '@/dev/fixtures/welcome'

type Mode = 'setup' | 'join' | 'recover'

const CTA_KEY: Record<Mode, string> = {
  setup: 'welcome.cta.setup',
  join: 'welcome.cta.join',
  recover: 'welcome.cta.recover',
}

export default function WelcomeScreen({
  data,
  initial,
  onActivate,
}: {
  data: WelcomeData
  initial?: 'setup' | 'join' | 'recover'
  /** Real flow: "Activate and continue" → activate the licence with this code. */
  onActivate?: (code: string) => void
}) {
  const [mode, setMode] = useState<Mode>(initial ?? 'setup')
  const [code, setCode] = useState('')

  const isSetup = mode === 'setup'
  const isJoin = mode === 'join'
  const isRecover = mode === 'recover'

  // Left panel <section>. On phone it is rendered full-width (flex-grow) and the
  // navy right panel is hidden (docs §10.3); otherwise it keeps its fixed 600px.
  const leftSection = (
    <section
      style={{
        width: isPhone ? undefined : '600px',
        flexShrink: isPhone ? undefined : 0,
        flexGrow: isPhone ? 1 : undefined,
        background: '#F6F8F7',
        borderRadius: '22px',
        boxShadow: '0 1px 2px rgba(11,26,51,0.06)',
        padding: '36px 56px',
        boxSizing: 'border-box',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <img
          src={wordmark}
          alt="Vidya Budget School"
          style={{ width: '190px', height: '62px', display: 'block' }}
        />
        <a
          href="#"
          style={{
            fontSize: '13px',
            color: '#56657A',
            textDecoration: 'none',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
          }}
        >
          <Icon name="help" size={15} strokeWidth={1.75} />
          {t('welcome.help')}
        </a>
      </div>
      <div
        style={{
          flexGrow: 1,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          gap: '22px',
          maxWidth: '420px',
          animation: 'vRise14 520ms cubic-bezier(.2,.8,.2,1) both',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
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
            ></span>
            {t('welcome.eyebrow')}
          </span>
          <span
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              fontSize: '12px',
              color: '#56657A',
            }}
          >
            <Icon name="shield" size={14} strokeWidth={1.75} />
            {t('welcome.encrypted')}
          </span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
          <h1
            className="v-serif"
            style={{
              margin: 0,
              fontFamily: "'Newsreader', Georgia, serif",
              fontWeight: 400,
              fontSize: '60px',
              lineHeight: 1,
              letterSpacing: '-0.03em',
            }}
          >
            {t('welcome.h1')}
          </h1>
          <p style={{ margin: 0, fontSize: '15px', lineHeight: 1.55, color: '#56657A' }}>
            {t('welcome.intro')}
          </p>
        </div>
        <div
          role="radiogroup"
          aria-label={t('welcome.radiogroup.label')}
          style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}
        >
          {data.options.map((o) => {
            const on = mode === o.id
            return (
              <button
                key={o.id}
                type="button"
                role="radio"
                aria-checked={on}
                onClick={() => setMode(o.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '14px',
                  padding: '14px 16px',
                  borderRadius: '8px',
                  fontFamily: 'inherit',
                  ...(on
                    ? { border: '1.5px solid var(--accent)', background: 'var(--accent-6)' }
                    : { border: '1.5px solid #D5DDE0', background: '#FFFFFF' }),
                }}
              >
                <span
                  style={{
                    width: '18px',
                    height: '18px',
                    borderRadius: '9px',
                    flexShrink: 0,
                    boxSizing: 'border-box',
                    transition: 'border-width .18s ease',
                    ...(on
                      ? { border: '5px solid var(--accent)', background: '#FFFFFF' }
                      : { border: '1.5px solid #B7C2C9', background: '#FFFFFF' }),
                  }}
                ></span>
                <span
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '3px',
                    textAlign: 'left',
                  }}
                >
                  <span style={{ fontSize: '15px', fontWeight: 600, color: '#13233F' }}>
                    {t(o.titleKey)}
                  </span>
                  <span style={{ fontSize: '13px', color: '#56657A' }}>{t(o.subKey)}</span>
                </span>
              </button>
            )
          })}
        </div>
        {isSetup ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              animation: 'vIn6 260ms ease both',
            }}
          >
            <label htmlFor="code" style={{ fontSize: '13px', fontWeight: 500 }}>
              {t('welcome.field.codeLabel')}
            </label>
            <input
              id="code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder={t('welcome.field.codePlaceholder')}
              style={{
                height: '48px',
                boxSizing: 'border-box',
                borderRadius: '6px',
                border: '1.5px solid var(--accent)',
                boxShadow: '0 0 0 4px var(--accent-12)',
                background: '#FFFFFF',
                padding: '0 14px',
                fontFamily: "'Geist', monospace",
                fontSize: '15px',
                letterSpacing: '0.06em',
                color: '#13233F',
              }}
            />
            <span style={{ fontSize: '12px', color: '#56657A' }}>{t('welcome.field.codeHint')}</span>
          </div>
        ) : null}
        {isJoin ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              animation: 'vIn6 260ms ease both',
            }}
          >
            <label htmlFor="inv" style={{ fontSize: '13px', fontWeight: 500 }}>
              {t('welcome.field.invLabel')}
            </label>
            <input
              id="inv"
              placeholder={t('welcome.field.invPlaceholder')}
              style={{
                height: '48px',
                boxSizing: 'border-box',
                borderRadius: '6px',
                border: '1.5px solid var(--accent)',
                boxShadow: '0 0 0 4px var(--accent-12)',
                background: '#FFFFFF',
                padding: '0 14px',
                fontFamily: 'inherit',
                fontSize: '15px',
                color: '#13233F',
              }}
            />
            <span style={{ fontSize: '12px', color: '#56657A' }}>{t('welcome.field.invHint')}</span>
          </div>
        ) : null}
        {isRecover ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              padding: '14px 16px',
              borderRadius: '8px',
              background: '#F4ECDC',
              color: '#6B5220',
              fontSize: '13px',
              lineHeight: 1.5,
              animation: 'vIn6 260ms ease both',
            }}
          >
            <span style={{ fontWeight: 600 }}>{t('welcome.recover.title')}</span>
            <span>{t('welcome.recover.body')}</span>
          </div>
        ) : null}
        <button
          type="button"
          onClick={() => {
            if (isSetup && onActivate) onActivate(code)
          }}
          style={{
            height: '50px',
            borderRadius: '6px',
            border: 0,
            background: 'var(--accent)',
            color: '#FFFFFF',
            fontSize: '15px',
            fontWeight: 500,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '0 18px',
          }}
        >
          {t(CTA_KEY[mode])}
          <Icon name="arrowUpRight" size={18} strokeWidth={1.75} />
        </button>
        <div style={{ height: '1px', background: '#D5DDE0' }}></div>
        <span style={{ fontSize: '13px', color: '#56657A', textAlign: 'center' }}>
          {t('welcome.signin.prompt')}
          <a
            href="#"
            onClick={(e) => {
              e.preventDefault()
              navigate('/principal/home')
            }}
            style={{ color: '#13233F', fontWeight: 600, textDecoration: 'none' }}
          >
            {t('welcome.signin.link')}
          </a>
        </span>
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          fontSize: '12px',
          color: '#56657A',
        }}
      >
        <span>{t('welcome.footer.copyright')}</span>
        <span style={{ display: 'flex', gap: '20px' }}>
          <a href="#" style={{ color: '#56657A', textDecoration: 'none' }}>
            {t('welcome.footer.privacy')}
          </a>
          <a href="#" style={{ color: '#56657A', textDecoration: 'none' }}>
            {t('welcome.footer.terms')}
          </a>
        </span>
      </div>
    </section>
  )

  const rightSection = (
    <section
      style={{
        flexGrow: 1,
        position: 'relative',
        overflow: 'hidden',
        borderRadius: '22px',
        background:
          'radial-gradient(120% 80% at 80% 10%, #1A3560 0%, #0C1B38 55%, #08152B 100%)',
        color: '#FFFFFF',
        padding: '44px 52px 32px',
        boxSizing: 'border-box',
        display: 'flex',
        flexDirection: 'column',
        gap: '26px',
      }}
    >
      <span
        style={{
          fontSize: '11px',
          fontWeight: 600,
          letterSpacing: '0.16em',
          textTransform: 'uppercase',
          color: '#9FACBF',
        }}
      >
        {t('welcome.right.eyebrow')}
      </span>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '4px',
          animation: 'vRise14 600ms 120ms cubic-bezier(.2,.8,.2,1) both',
        }}
      >
        <h2
          className="v-serif"
          style={{
            margin: 0,
            fontFamily: "'Newsreader', Georgia, serif",
            fontWeight: 400,
            fontSize: '64px',
            lineHeight: 1.02,
            letterSpacing: '-0.03em',
          }}
        >
          {t('welcome.right.h2')}
        </h2>
        <span
          className="v-serif"
          style={{
            fontFamily: "'Newsreader', Georgia, serif",
            fontStyle: 'italic',
            fontWeight: 400,
            fontSize: '64px',
            lineHeight: 1.05,
            letterSpacing: '-0.03em',
            color: '#C5AB7A',
          }}
        >
          {t('welcome.right.h2Italic')}
        </span>
      </div>
      <p
        style={{
          margin: 0,
          fontSize: '15px',
          lineHeight: 1.6,
          color: '#C9D2DE',
          maxWidth: '460px',
        }}
      >
        {t('welcome.right.body')}
      </p>
      <div
        style={{
          flexGrow: 1,
          border: '1px solid rgba(255,255,255,0.14)',
          borderRadius: '22px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: 0,
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '22px',
            animation: 'vRise14 700ms 200ms cubic-bezier(.2,.8,.2,1) both',
          }}
        >
          <img
            src={doorMark}
            alt=""
            style={{ width: '220px', height: '220px', display: 'block' }}
          />
          <span
            className="v-serif"
            style={{
              fontFamily: "'Newsreader', Georgia, serif",
              fontStyle: 'italic',
              fontSize: '22px',
              color: '#C5AB7A',
            }}
          >
            {t('welcome.right.doorCaption')}
          </span>
        </div>
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          paddingTop: '16px',
          borderTop: '1px solid rgba(255,255,255,0.14)',
          fontSize: '13px',
          color: '#C9D2DE',
        }}
      >
        <span>{t('welcome.right.foot.attendance')}</span>
        <span>{t('welcome.right.foot.marks')}</span>
        <span>{t('welcome.right.foot.fees')}</span>
        <span>{t('welcome.right.foot.approvals')}</span>
      </div>
    </section>
  )

  return (
    <div
      style={{
        width: '1440px',
        height: '960px',
        boxSizing: 'border-box',
        padding: '20px',
        display: 'flex',
        gap: '20px',
        background: '#E2EAEB',
        color: '#13233F',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: '14px',
      }}
    >
      {leftSection}
      {isPhone ? null : rightSection}
    </div>
  )
}
