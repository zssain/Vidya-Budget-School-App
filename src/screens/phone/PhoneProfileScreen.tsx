// Profile (repair T1): who is signed in (name + role from the session), the app
// language (English / हिंदी / తెలుగు — the same setting as the desktop Settings),
// Lock (the `lock` command, then back to the PIN screen) and About.
import { Icon } from '@/components/Icon'
import PhoneScreen, { PHONE_CARD, PHONE_SECONDARY } from '@/components/PhoneScreen'
import * as api from '@/lib/api'
import { setLang, t, useLang, type Lang } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { refreshAppState, useStore } from '@/lib/store'
import { initials } from '@/lib/teacherHome'

const LANGS: { id: Lang; label: string }[] = [
  { id: 'en', label: 'English' },
  { id: 'hi', label: 'हिंदी' },
  { id: 'te', label: 'తెలుగు' },
]

export default function PhoneProfileScreen() {
  const lang = useLang()
  const store = useStore()
  const staff = store.app?.state.kind === 'unlocked' ? store.app.state.staff : null
  const role = staff?.role ?? 'teacher'

  const doLock = () => {
    api
      .lock()
      .catch(() => {})
      .finally(() => {
        navigate('/teacher/home')
        void refreshAppState()
      })
  }

  return (
    <PhoneScreen title={t('tp.prof.title')}>
      <div style={{ ...PHONE_CARD, display: 'flex', alignItems: 'center', gap: 14, padding: 16 }}>
        <span style={{ width: 52, height: 52, borderRadius: 26, background: 'var(--navy-raised)', color: 'var(--gold-light)', fontWeight: 600, fontSize: 17, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
          {initials(staff?.name ?? '')}
        </span>
        <span style={{ display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0 }}>
          <b style={{ fontWeight: 600, fontSize: 17 }}>{staff?.name ?? ''}</b>
          <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t(`tp.prof.role.${role === 'principal' || role === 'accountant' ? role : 'teacher'}`)}</span>
        </span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <span style={{ fontSize: 13, fontWeight: 500 }}>{t('tp.prof.language')}</span>
        <div role="radiogroup" aria-label={t('tp.prof.language')} style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', border: '1px solid var(--line-strong)', borderRadius: 6, overflow: 'hidden' }}>
          {LANGS.map((l, i) => {
            const on = l.id === lang
            return (
              <button key={l.id} type="button" role="radio" aria-checked={on} onClick={() => setLang(l.id)} style={{ height: 44, fontSize: 14, fontWeight: 500, border: 0, borderLeft: i === 0 ? 0 : '1px solid var(--line-strong)', background: on ? 'var(--accent)' : 'var(--white)', color: on ? 'var(--white)' : 'var(--ink)' }}>
                {l.label}
              </button>
            )
          })}
        </div>
      </div>

      <button type="button" onClick={() => navigate('/teacher/about')} style={{ ...PHONE_SECONDARY, display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 16px' }}>
        {t('tp.prof.about')} <Icon name="chevronRight" size={18} strokeWidth={1.6} />
      </button>

      <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        <button type="button" onClick={doLock} style={{ ...PHONE_SECONDARY, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8 }}>
          <Icon name="shield" size={18} strokeWidth={1.7} /> {t('tp.prof.lock')}
        </button>
        <span style={{ fontSize: 12, color: 'var(--muted)', textAlign: 'center' }}>{t('tp.prof.lockSub')}</span>
      </div>
    </PhoneScreen>
  )
}
