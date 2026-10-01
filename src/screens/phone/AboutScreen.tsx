// About screen (Phase 19, Step 7) — a small phone screen reached from the account
// button on Teacher Home. Shows the product name + version, and on the iPhone PWA
// (isWeb) the honest limits from §18 / phase-19-spike so staff know exactly how the
// web app behaves (Drive-only sync, no background sync, camera/gallery photos, no
// push yet, on-device storage retention). All copy via t(); colours via CSS tokens.
// Unlike the pixel-exact mock screens this is new, so it fills the viewport
// responsively (100dvh) rather than the fixed 390x844 mock frame.
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { isWeb } from '@/lib/platform'

// Injected by Vite's `define` (both the Tauri and web-pwa configs); same guard as
// SettingsScreen so a non-string value falls back to a dash.
declare const __APP_VERSION__: string

// The honest-limits rows, in display order. Each maps to a `.t` (title) and `.d`
// (description) key in strings/about.ts.
const LIMITS = ['sync', 'background', 'photos', 'notifications', 'storage'] as const

function LimitRow({ id }: { id: (typeof LIMITS)[number] }) {
  return (
    <div style={{ display: 'flex', gap: 12, padding: '14px 0', borderTop: '1px solid var(--track)' }}>
      <span style={{ marginTop: 2, color: 'var(--gold)', flexShrink: 0 }}>
        <Icon name="check" size={18} strokeWidth={1.8} />
      </span>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}>
        <span style={{ fontSize: 14, fontWeight: 600, color: 'var(--ink)' }}>{t(`about.iphone.${id}.t`)}</span>
        <span style={{ fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{t(`about.iphone.${id}.d`)}</span>
      </div>
    </div>
  )
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '12px 0', fontSize: 14, borderTop: '1px solid var(--track)' }}>
      <span style={{ color: 'var(--muted)' }}>{label}</span>
      <span style={{ color: 'var(--ink)', textAlign: 'right' }}>{value}</span>
    </div>
  )
}

const CARD: React.CSSProperties = {
  background: 'var(--surface)',
  border: '1px solid var(--line)',
  borderRadius: 16,
  padding: '4px 18px 14px',
}

export default function AboutScreen() {
  const version = typeof __APP_VERSION__ === 'string' ? __APP_VERSION__ : '—'
  return (
    <div
      style={{
        minHeight: '100dvh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif",
        fontSize: 14,
      }}
    >
      <header
        style={{
          flexShrink: 0,
          background: 'var(--navy)',
          color: 'var(--white)',
          padding: 'calc(10px + env(safe-area-inset-top)) 16px 18px',
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        <button
          type="button"
          aria-label={t('about.back')}
          onClick={() => navigate('/teacher/home')}
          style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--white)', border: 0, background: 'transparent' }}
        >
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 28 }}>{t('about.title')}</h1>
      </header>

      <div style={{ flexGrow: 1, padding: '16px 16px calc(24px + env(safe-area-inset-bottom))', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Product + version + support. */}
        <div style={CARD}>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22, padding: '14px 0 2px' }}>{t('about.product')}</div>
          <p style={{ margin: '0 0 6px', fontSize: 13, color: 'var(--muted)' }}>{t('about.tagline')}</p>
          <InfoRow label={t('about.version')} value={version} />
          <InfoRow label={t('about.support')} value="mohammedzuhairhussain28@gmail.com" />
          <InfoRow label={t('about.website')} value="neverworks.org" />
        </div>

        {/* iPhone honest-limits — only on the PWA. */}
        {isWeb && (
          <div style={CARD}>
            <h2 style={{ fontSize: 15, fontWeight: 600, margin: '16px 0 6px' }}>{t('about.iphone.title')}</h2>
            <p style={{ margin: '0 0 4px', fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{t('about.iphone.intro')}</p>
            {LIMITS.map((id) => (
              <LimitRow key={id} id={id} />
            ))}
          </div>
        )}

        <p style={{ margin: 0, fontSize: 12, color: 'var(--muted)' }}>{t('about.developer')}</p>
        <p style={{ margin: '-8px 0 0', fontSize: 12, color: 'var(--muted)' }}>{t('about.copyright')}</p>
      </div>
    </div>
  )
}
