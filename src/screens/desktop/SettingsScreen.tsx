// Settings (docs §6.2, prompts/P08 Part E). Derived screen: eyebrow + serif h1 +
// cards. Every control here is REAL — School (get_school), Appearance (set_accent),
// Language (i18n), Academics/Data links (navigate), Security (verify_audit_chain),
// Licence (app_state), About (build version). Sections whose own editors live
// elsewhere link to them rather than duplicating.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { AuditChainDto, CmdError, ModuleRow, SchoolDto } from '@/lib/api'
import { getLang, setLang, t, useLang } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { ACCENT_OPTIONS, loadAccent, setAccent } from '@/lib/theme'
import { useStore } from '@/lib/store'
import CustomFieldsSettings from './CustomFieldsSettings'

declare const __APP_VERSION__: string

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const SERIF = "'Newsreader', Georgia, serif"
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 14px' }

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '8px 0', fontSize: 14, borderTop: '1px solid var(--track)' }}>
      <span style={{ color: 'var(--muted)' }}>{label}</span>
      <span style={{ color: 'var(--ink)', textAlign: 'right' }}>{value}</span>
    </div>
  )
}

function LinkRow({ title, sub, onClick }: { title: string; sub: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{ width: '100%', display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 16, padding: '14px 0', background: 'transparent', border: 'none', borderTop: '1px solid var(--track)', cursor: 'pointer', textAlign: 'left' }}
    >
      <span style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--ink)' }}>{title}</span>
        <span style={{ fontSize: 12, color: 'var(--muted)' }}>{sub}</span>
      </span>
      <span style={{ fontSize: 13, color: 'var(--accent)', fontWeight: 500 }}>{t('settings.open')} →</span>
    </button>
  )
}

export default function SettingsScreen() {
  useLang()
  const store = useStore()
  const [school, setSchool] = useState<SchoolDto | null>(null)
  const [accent, setAccentState] = useState<string>(loadAccent())
  const [chain, setChain] = useState<AuditChainDto | null>(null)
  const [modules, setModules] = useState<ModuleRow[]>([])
  const [confirmMod, setConfirmMod] = useState<{ key: string; next: boolean } | null>(null)

  const loadModules = useCallback(() => {
    api.list_modules().then(setModules).catch(() => setModules([]))
  }, [])

  useEffect(() => {
    api.get_school().then(setSchool).catch(() => {})
    api.verify_audit_chain().then(setChain).catch(() => {})
    loadModules()
  }, [loadModules])

  const applyModule = async () => {
    if (!confirmMod) return
    try {
      await api.set_module(confirmMod.key, confirmMod.next)
    } catch (e) {
      void (e as CmdError)
    }
    setConfirmMod(null)
    loadModules()
  }

  const chooseAccent = (hex: string) => {
    setAccent(hex)
    setAccentState(hex)
    api.set_accent(hex).catch((e) => void (e as CmdError))
  }
  const chooseLang = (next: 'en' | 'hi') => setLang(next)

  const lang = getLang()
  const licence = store.app?.licence_status ?? 'active'
  const licenceLabel =
    licence === 'revoked' ? t('settings.licence.revoked') : licence === 'moved' ? t('settings.licence.moved') : t('settings.licence.active')

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--ink)', padding: 40, fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <div style={{ marginBottom: 24 }}>
        <h1 style={{ fontFamily: SERIF, fontSize: 40, letterSpacing: '-0.025em', margin: 0 }}>{t('settings.title')}</h1>
        <p style={{ color: 'var(--muted)', fontSize: 14, margin: '6px 0 0' }}>{t('settings.subtitle')}</p>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 20, maxWidth: 900 }}>
        {/* School */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.school.title')}</h3>
          <Row label={t('settings.school.name')} value={school?.name ?? '—'} />
          <Row label={t('settings.school.board')} value={school?.board ?? '—'} />
          <Row label={t('settings.school.session')} value={school?.session_label ?? '—'} />
          <Row label={t('settings.school.phone')} value={school?.phone ?? '—'} />
        </div>

        {/* Appearance */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.appearance.title')}</h3>
          <div style={{ fontSize: 13, color: 'var(--muted)', marginBottom: 12 }}>{t('settings.appearance.accent')}</div>
          <div style={{ display: 'flex', gap: 12 }}>
            {ACCENT_OPTIONS.map((hex) => (
              <button
                key={hex}
                onClick={() => chooseAccent(hex)}
                aria-label={hex}
                style={{ width: 40, height: 40, borderRadius: 10, background: hex, cursor: 'pointer', border: accent.toLowerCase() === hex.toLowerCase() ? '3px solid var(--ink)' : '3px solid transparent', boxShadow: '0 0 0 1px var(--line)' }}
              />
            ))}
          </div>
        </div>

        {/* Language */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.language.title')}</h3>
          <div style={{ display: 'flex', gap: 10, marginBottom: 8 }}>
            {(['en', 'hi'] as const).map((l) => (
              <button
                key={l}
                onClick={() => chooseLang(l)}
                style={{ height: 40, padding: '0 18px', borderRadius: 8, fontSize: 14, fontWeight: 500, cursor: 'pointer', border: '1px solid var(--line-strong)', background: lang === l ? 'var(--accent)' : 'var(--white)', color: lang === l ? '#fff' : 'var(--ink)' }}
              >
                {l === 'en' ? 'English' : 'हिन्दी'}
              </button>
            ))}
          </div>
          <div style={{ fontSize: 12, color: 'var(--muted)' }}>{t('settings.language.hint')}</div>
        </div>

        {/* Modules (Languages & modules — prototype settings state 3) */}
        <div style={{ ...CARD, gridColumn: '1 / -1' }}>
          <h3 style={H3}>{t('settings.modules.title')}</h3>
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: '0 0 6px' }}>{t('settings.modules.hint')}</p>
          {/* Core — always on, not switchable. */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '13px 0', borderTop: '1px solid var(--track)' }}>
            <div style={{ flexGrow: 1 }}>
              <span style={{ fontSize: 14, fontWeight: 500 }}>{t('settings.modules.core')}</span>
              <div style={{ fontSize: 12, color: 'var(--muted)' }}>{t('settings.modules.coreTag')}</div>
            </div>
            <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('settings.modules.on')}</span>
          </div>
          {(['accounts', 'classroom', 'hr', 'circulars', 'wa_auto', 'store'] as const).map((key) => {
            const row = modules.find((m) => m.key === key)
            if (!row) return null
            const optional = key === 'wa_auto' || key === 'store'
            return (
              <div key={key} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '13px 0', borderTop: '1px solid var(--track)' }}>
                <div style={{ flexGrow: 1 }}>
                  <span style={{ fontSize: 14, fontWeight: 500 }}>{t(`settings.modules.${key}`)}</span>
                  <div style={{ fontSize: 12, color: 'var(--muted)' }}>{t(`settings.modules.${key}Sub`)}</div>
                </div>
                {optional ? (
                  <span style={{ fontSize: 11, fontWeight: 500, color: 'var(--pill-partpaid-fg)', background: 'var(--pill-partpaid-bg)', borderRadius: 4, padding: '2px 8px' }}>{t('settings.modules.optional')}</span>
                ) : null}
                <button
                  type="button"
                  role="switch"
                  aria-checked={row.enabled}
                  aria-label={t(`settings.modules.${key}`)}
                  onClick={() => setConfirmMod({ key, next: !row.enabled })}
                  style={{ minWidth: 52, height: 32, borderRadius: 16, cursor: 'pointer', fontSize: 13, fontWeight: 500, border: `1px solid ${row.enabled ? 'var(--accent)' : 'var(--line-strong)'}`, background: row.enabled ? 'var(--accent)' : 'var(--white)', color: row.enabled ? 'var(--white)' : 'var(--muted)' }}
                >
                  {row.enabled ? t('settings.modules.on') : t('settings.modules.off')}
                </button>
              </div>
            )
          })}
        </div>

        {/* Custom fields (P13): Principal-defined student/staff fields. */}
        <CustomFieldsSettings />

        {/* Academics */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.academics.title')}</h3>
          <LinkRow title={t('settings.academics.gradeScale')} sub={t('settings.academics.gradeScaleSub')} onClick={() => navigate('/principal/grade-scale')} />
          <LinkRow title={t('settings.academics.session')} sub={t('settings.academics.sessionSub')} onClick={() => navigate('/session')} />
        </div>

        {/* Backups & data */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.data.title')}</h3>
          <LinkRow title={t('settings.data.backups')} sub={t('settings.data.backupsSub')} onClick={() => navigate('/principal/backups')} />
          <LinkRow title={t('settings.data.sync')} sub={t('settings.data.syncSub')} onClick={() => navigate('/sync')} />
        </div>

        {/* Security */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.security.title')}</h3>
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: '0 0 10px' }}>{t('settings.security.encrypted')}</p>
          <p style={{ fontSize: 13, margin: 0, color: chain == null ? 'var(--muted)' : chain.ok ? 'var(--accent)' : 'var(--danger)' }}>
            {chain == null
              ? t('settings.security.chainChecking')
              : chain.ok
                ? t('settings.security.chainOk')
                : t('settings.security.chainBad', { seq: chain.first_bad_seq ?? 0 })}
          </p>
        </div>

        {/* Licence */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.licence.title')}</h3>
          <Row label={t('settings.licence.status')} value={licenceLabel} />
          <p style={{ fontSize: 12, color: 'var(--muted)', margin: '10px 0 0' }}>{t('settings.licence.note')}</p>
        </div>

        {/* About */}
        <div style={CARD}>
          <h3 style={H3}>{t('settings.about.title')}</h3>
          <div style={{ fontFamily: SERIF, fontSize: 20 }}>{t('settings.about.product')}</div>
          <Row label={t('settings.about.version')} value={typeof __APP_VERSION__ === 'string' ? __APP_VERSION__ : '—'} />
          <Row label={t('settings.about.support')} value="mohammedzuhairhussain28@gmail.com" />
          <Row label={t('settings.about.website')} value="vidya.zuhairhussain.com" />
          <p style={{ fontSize: 12, color: 'var(--muted)', margin: '10px 0 0' }}>{t('settings.about.tagline')}</p>
          <p style={{ fontSize: 12, color: 'var(--muted)', margin: '10px 0 0' }}>{t('settings.about.developer')}</p>
          <p style={{ fontSize: 12, color: 'var(--muted)', margin: '2px 0 0' }}>{t('settings.about.copyright')}</p>
        </div>
      </div>

      {confirmMod ? (
        <div onClick={() => setConfirmMod(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 420, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: SERIF, fontSize: 22 }}>
              {t(confirmMod.next ? 'settings.modules.confirmOnTitle' : 'settings.modules.confirmOffTitle', { module: t(`settings.modules.${confirmMod.key}`) })}
            </div>
            <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>
              {t(confirmMod.next ? 'settings.modules.confirmOnBody' : 'settings.modules.confirmOffBody')}
            </p>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setConfirmMod(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('settings.modules.cancel')}</button>
              <button type="button" onClick={applyModule} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('settings.modules.confirm')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}
