// Recover an existing school from a .vbak backup (Welcome → Recover, §12). Reached at
// /recover. Pick the backup file + enter the recovery key → a summary of what will be
// restored → confirm → the backend installs it (fences the old PC: server_epoch + 1,
// every device must rejoin) and the app RESTARTS into the restored school. The restore
// core + commands are tested; this screen is their UI. Colours via design tokens.
import { useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, RestoreSummaryDto } from '@/lib/api'
import { pickBackupPath } from '@/lib/files'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const SERIF = "'Newsreader', Georgia, serif"

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '8px 0', fontSize: 14, borderTop: '1px solid var(--track)' }}>
      <span style={{ color: 'var(--muted)' }}>{label}</span>
      <span style={{ color: 'var(--ink)', textAlign: 'right' }}>{value}</span>
    </div>
  )
}

export default function RecoverScreen() {
  const [path, setPath] = useState<string | null>(null)
  const [recoveryKey, setRecoveryKey] = useState('')
  const [summary, setSummary] = useState<RestoreSummaryDto | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const showErr = (e: unknown) =>
    setError(t((e as CmdError).message_key, (e as CmdError).vars as Record<string, string | number>))

  const choose = async () => {
    const p = await pickBackupPath()
    if (p) {
      setPath(p)
      setSummary(null)
      setError(null)
    }
  }

  const check = () => {
    if (!path) return
    setBusy(true)
    setError(null)
    api
      .restore_summary(path, recoveryKey)
      .then(setSummary)
      .catch(showErr)
      .finally(() => setBusy(false))
  }

  const restore = () => {
    if (!path) return
    setBusy(true)
    setError(null)
    // On success the backend restarts the app (this promise never resolves); only an
    // error comes back here.
    api.restore_install(path, recoveryKey).catch((e) => {
      showErr(e)
      setBusy(false)
    })
  }

  const input: React.CSSProperties = { height: 42, borderRadius: 8, border: '1px solid var(--line-strong)', padding: '0 14px', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, width: '100%', boxSizing: 'border-box', fontFamily: 'ui-monospace, Menlo, monospace', letterSpacing: '0.04em' }
  const btn = (bg: string, fg: string): React.CSSProperties => ({ height: 44, padding: '0 20px', borderRadius: 8, border: bg === 'transparent' ? '1px solid var(--line-strong)' : 'none', background: bg, color: fg, fontSize: 14, fontWeight: 600, cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.6 : 1 })

  const fileName = path ? path.split(/[\\/]/).pop() : null

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--ink)', padding: 40, fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <div style={{ maxWidth: 560, margin: '0 auto' }}>
        <button type="button" onClick={() => navigate('/welcome')} style={{ ...btn('transparent', 'var(--ink)'), marginBottom: 20 }}>
          {t('recover.back')}
        </button>
        <h1 style={{ fontFamily: SERIF, fontSize: 34, margin: '0 0 6px' }}>{t('recover.title')}</h1>
        <p style={{ color: 'var(--muted)', fontSize: 14, margin: '0 0 24px', lineHeight: 1.5 }}>{t('recover.intro')}</p>

        {error && <div style={{ ...CARD, padding: 14, marginBottom: 16, borderColor: 'var(--danger)', color: 'var(--danger)', fontSize: 14 }}>{error}</div>}

        {!summary ? (
          <div style={{ ...CARD, display: 'flex', flexDirection: 'column', gap: 18 }}>
            <div>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>{t('recover.step1')}</div>
              <button type="button" onClick={choose} style={btn('transparent', 'var(--ink)')}>{t('recover.chooseFile')}</button>
              <div style={{ fontSize: 13, color: fileName ? 'var(--ink)' : 'var(--muted)', marginTop: 8, wordBreak: 'break-all' }}>{fileName ?? t('recover.noFile')}</div>
            </div>
            <div>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>{t('recover.step2')}</div>
              <input style={input} value={recoveryKey} onChange={(e) => setRecoveryKey(e.target.value)} placeholder={t('recover.keyPlaceholder')} aria-label={t('recover.step2')} />
            </div>
            <button type="button" onClick={check} disabled={busy || !path || recoveryKey.trim().length < 20} style={btn('var(--accent)', '#fff')}>
              {busy ? t('recover.checking') : t('recover.check')}
            </button>
          </div>
        ) : (
          <div style={CARD}>
            <div style={{ fontFamily: SERIF, fontSize: 22, marginBottom: 2 }}>{summary.school_name || '—'}</div>
            <div style={{ fontSize: 13, color: 'var(--muted)', marginBottom: 14 }}>{t('recover.fromDate', { date: summary.backup_date || '—' })}</div>
            <Row label={t('recover.students')} value={String(summary.students)} />
            <Row label={t('recover.payments')} value={String(summary.payments)} />
            {summary.last_receipt_no ? <Row label={t('recover.lastReceipt')} value={summary.last_receipt_no} /> : null}
            <Row label={t('recover.integrity')} value={summary.chain_ok ? t('recover.chainOk') : t('recover.chainBad')} />
            {summary.stale ? <p style={{ fontSize: 13, color: 'var(--gold)', margin: '12px 0 0' }}>{t('recover.staleWarn', { date: summary.backup_date })}</p> : null}
            <div style={{ display: 'flex', gap: 12, marginTop: 20 }}>
              <button type="button" onClick={() => setSummary(null)} disabled={busy} style={btn('transparent', 'var(--ink)')}>{t('recover.chooseAnother')}</button>
              <button type="button" onClick={restore} disabled={busy} style={btn('var(--accent)', '#fff')}>{busy ? t('recover.restoring') : t('recover.restoreBtn')}</button>
            </div>
            <p style={{ fontSize: 12, color: 'var(--muted)', margin: '14px 0 0', lineHeight: 1.5 }}>{t('recover.fenceNote')}</p>
          </div>
        )}
      </div>
    </div>
  )
}
