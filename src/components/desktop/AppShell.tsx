import { useEffect, useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { Icon } from '@/components/Icon'
import { navForRole, type NavItem, type NavSection, type Role } from '@/lib/nav'
import { navigate } from '@/lib/router'
import { useStore } from '@/lib/store'
import * as api from '@/lib/api'
import type { SchoolDto } from '@/lib/api'
import { t } from '@/lib/i18n'
import logoDark from '@/assets/vidya-horizontal-on-dark.svg'

// The single desktop chrome (docs §6.2): the mock sidebar (Main.dc.html /
// FeeCollection.dc.html) + the mock header, with REAL navigation, an active
// item, a live count badge, the school/user identity, and a read-only banner
// for past sessions. Every derived desktop screen renders as its `children`;
// the two full mock screens (Home, Collect fee) render here with chrome={false}.
//
// Colours are tokens only (docs §15). Values equal the mock's verbatim sidebar.

// --- initials from a name (up to two words) --------------------------------
function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length === 0) return '·'
  const first = parts[0][0] ?? ''
  const second = parts.length > 1 ? parts[parts.length - 1][0] ?? '' : ''
  return (first + second).toUpperCase()
}

function roleLabel(role: string): string {
  return role.length > 0 ? role[0].toUpperCase() + role.slice(1) : role
}

// --- sidebar item ----------------------------------------------------------
function itemStyle(active: boolean): CSSProperties {
  return {
    display: 'flex',
    alignItems: 'center',
    gap: '11px',
    height: '38px',
    padding: '0 10px',
    borderRadius: '6px',
    fontWeight: 500,
    fontSize: '14px',
    textDecoration: 'none',
    cursor: 'pointer',
    background: active ? 'rgba(255,255,255,0.07)' : 'transparent',
    color: active ? 'var(--white)' : 'var(--on-navy)',
  }
}

function SidebarItem({ item, active, badge }: { item: NavItem; active: boolean; badge: number }) {
  return (
    <a
      href={`#${item.path}`}
      aria-current={active ? 'page' : undefined}
      style={itemStyle(active)}
      onClick={(e) => {
        e.preventDefault()
        navigate(item.path)
      }}
    >
      <Icon name={item.icon} size={18} strokeWidth={1.6} color={active ? 'var(--gold)' : undefined} />
      <span>{t(item.labelKey)}</span>
      {item.badge != null && badge > 0 ? (
        <span
          data-sidebar-badge={badge}
          style={{
            marginLeft: 'auto',
            minWidth: '20px',
            height: '20px',
            padding: '0 6px',
            borderRadius: '10px',
            background: 'var(--gold)',
            color: 'var(--navy)',
            fontSize: '11px',
            fontWeight: 700,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            boxSizing: 'border-box',
          }}
        >
          {badge}
        </span>
      ) : active ? (
        <span style={{ marginLeft: 'auto', width: '5px', height: '5px', borderRadius: '3px', background: 'var(--gold)' }} />
      ) : null}
    </a>
  )
}

export interface AppShellProps {
  role: Role
  /** Which nav item is highlighted (a screen passes its own key). */
  active: string
  children: ReactNode
}

export default function AppShell({ role, active, children }: AppShellProps) {
  const store = useStore()
  const [school, setSchool] = useState<SchoolDto | null>(null)
  const [badge, setBadge] = useState(0)

  useEffect(() => {
    api.get_school().then(setSchool).catch(() => setSchool(null))
    // Badge: Principal → pending approvals; Accountant → own pending requests.
    api
      .list_requests('pending')
      .then((rows) => setBadge(rows.length))
      .catch(() => setBadge(0))
  }, [role])

  // Global search palette on Ctrl/Cmd+K.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        navigate('/search')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const sections: NavSection[] = navForRole(role)
  const user = store.app?.state.kind === 'unlocked' ? store.app.state.staff : null
  const schoolName = school?.name ?? t('app.name')
  const sessionLabel = school?.session_label ?? ''
  const readOnly = school?.session_read_only ?? false

  return (
    <div
      style={{
        display: 'flex',
        minHeight: '100vh',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: '14px',
      }}
    >
      {/* Sidebar */}
      <aside
        style={{
          width: '256px',
          flexShrink: 0,
          background: 'var(--navy)',
          color: 'var(--white)',
          display: 'flex',
          flexDirection: 'column',
          padding: '26px 16px 20px',
          boxSizing: 'border-box',
          gap: '28px',
          position: 'sticky',
          top: 0,
          height: '100vh',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', padding: '0 10px' }}>
          <img src={logoDark} alt={schoolName} width={160} height={52} style={{ width: '160px', height: '52px', display: 'block' }} />
          <span style={{ fontSize: '12px', color: 'var(--on-navy-muted)' }}>{schoolName}</span>
        </div>

        <nav aria-label={t('shell.mainNav')} style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          {sections.map((section, si) => (
            <div key={si} style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
              {section.labelKey != null ? (
                <div
                  style={{
                    fontSize: '10px',
                    fontWeight: 600,
                    letterSpacing: '0.16em',
                    textTransform: 'uppercase',
                    color: 'var(--on-navy-label)',
                    padding: '0 10px 8px',
                  }}
                >
                  {t(section.labelKey)}
                </div>
              ) : null}
              {section.items.map((item) => (
                <SidebarItem key={item.key} item={item} active={item.key === active} badge={badge} />
              ))}
            </div>
          ))}
        </nav>

        <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: '14px' }}>
          {role === 'principal' ? (
            <div style={{ border: '1px solid rgba(255,255,255,0.12)', borderRadius: '10px', padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', fontWeight: 500 }}>
                <span style={{ width: '7px', height: '7px', borderRadius: '4px', background: 'var(--online)', boxShadow: '0 0 0 3px rgba(95,208,160,0.18)' }} />
                {t('shell.serverOnline')}
              </div>
              <div style={{ fontSize: '12px', color: 'var(--on-navy-muted)' }}>{t('shell.thisPc')}</div>
            </div>
          ) : null}
          {user != null ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px', padding: '0 6px' }}>
              <div style={{ width: '34px', height: '34px', borderRadius: '17px', background: 'var(--navy-raised)', color: 'var(--gold-light)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 600, fontSize: '13px' }}>
                {initials(user.name)}
              </div>
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                <span style={{ fontWeight: 500, fontSize: '13px' }}>{user.name}</span>
                <span style={{ fontSize: '12px', color: 'var(--on-navy-muted)' }}>{roleLabel(user.role)}</span>
              </div>
            </div>
          ) : null}
        </div>
      </aside>

      {/* Main column */}
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        {/* Header */}
        <header
          style={{
            height: '64px',
            flexShrink: 0,
            borderBottom: '1px solid var(--line)',
            display: 'flex',
            alignItems: 'center',
            gap: '14px',
            padding: '0 40px',
            boxSizing: 'border-box',
            position: 'sticky',
            top: 0,
            background: 'var(--bg)',
            zIndex: 5,
          }}
        >
          <button
            type="button"
            style={{ height: '36px', padding: '0 12px', borderRadius: '6px', border: '1px solid var(--line)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}
            onClick={() => navigate('/session')}
          >
            {sessionLabel !== '' ? sessionLabel : t('nav.settings')}
            <Icon name="chevronDown" size={15} strokeWidth={1.75} />
          </button>
          <div style={{ position: 'relative', width: '400px' }}>
            <Icon name="search" size={16} strokeWidth={1.75} color="var(--muted)" style={{ position: 'absolute', left: '12px', top: '11px' }} />
            <input
              type="search"
              aria-label={t('shell.searchLabel')}
              placeholder={t('shell.searchPlaceholder')}
              onFocus={() => navigate('/search')}
              style={{ width: '100%', height: '38px', boxSizing: 'border-box', borderRadius: '6px', border: '1px solid var(--line)', background: 'var(--surface)', padding: '0 16px 0 36px', fontFamily: 'inherit', fontSize: '14px', color: 'var(--ink)' }}
            />
          </div>
          <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: '10px' }}>
            <span style={{ height: '34px', padding: '0 12px', borderRadius: '17px', background: 'var(--accent-10)', color: 'var(--accent)', fontSize: '13px', fontWeight: 500, display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span style={{ width: '6px', height: '6px', borderRadius: '3px', background: 'var(--accent)' }} />
              {t('shell.syncPill')}
            </span>
            <button
              type="button"
              aria-label={t('shell.bellLabel')}
              onClick={() => navigate('/inbox')}
              style={{ position: 'relative', width: '38px', height: '38px', borderRadius: '19px', border: '1px solid var(--line)', background: 'var(--surface)', color: 'var(--ink)', display: 'flex', alignItems: 'center', justifyContent: 'center', cursor: 'pointer' }}
            >
              <Icon name="bell" size={17} strokeWidth={1.6} />
            </button>
          </div>
        </header>

        {/* Read-only (past-session) banner */}
        {readOnly ? (
          <div
            role="status"
            style={{ background: 'var(--gold)', color: 'var(--navy)', textAlign: 'center', padding: '8px 16px', fontSize: '13px', fontWeight: 500 }}
          >
            {t('shell.readOnlyBanner', { session: sessionLabel })}
          </div>
        ) : null}

        {/* Content */}
        <div style={{ flexGrow: 1, minWidth: 0 }}>{children}</div>
      </div>
    </div>
  )
}
