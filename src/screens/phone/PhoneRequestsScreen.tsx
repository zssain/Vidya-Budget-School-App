// My requests (repair T1): the teacher's own requests (list_requests — a
// non-Principal only ever gets their own, natively and on the iPhone app) with their
// status and the Principal's note, newest first; Cancel on a pending / returned one
// (cancel_request, own requests only); and a link to Apply for leave.
import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, RequestDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import PhoneScreen, { PHONE_CARD, PHONE_SECONDARY, PhoneEmpty, PhonePill } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { formatDateShortIn } from '@/lib/format'
import { navigate } from '@/lib/router'
import { useStore } from '@/lib/store'
import { isWeb } from '@/lib/platform'
import { isAvailableOnWeb } from '@/lib/web/backend'
import { requestKindLabel } from '@/lib/teacherHome'

const STATUS_TONE: Record<string, 'ok' | 'warn' | 'bad' | 'muted' | 'accent'> = {
  pending: 'accent',
  approved: 'ok',
  rejected: 'bad',
  returned: 'warn',
  cancelled: 'muted',
}


export default function PhoneRequestsScreen() {
  const lang = useLang()
  const store = useStore()
  const myId = store.app?.state.kind === 'unlocked' ? store.app.state.staff.id : ''
  const [rows, setRows] = useState<RequestDto[] | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const canCancel = !isWeb || isAvailableOnWeb('cancel_request')
  const canLeave = !isWeb || isAvailableOnWeb('request_leave')

  const load = useCallback(() => {
    api
      .list_requests()
      .then((rs) => setRows(rs.filter((r) => r.requested_by === myId)))
      .catch(() => setRows([]))
  }, [myId])
  useEffect(load, [load])

  const cancel = async (id: string) => {
    if (!window.confirm(t('tp.req.cancelConfirm'))) return
    setErr(null)
    try {
      await api.cancel_request(id)
      setToast(t('tp.req.cancelled'))
      window.setTimeout(() => setToast(null), 2200)
      load()
    } catch (e) {
      setErr(t((e as CmdError).message_key))
    }
  }

  return (
    <PhoneScreen
      title={t('tp.req.title')}
      toast={toast}
      footer={
        canLeave ? (
          <button type="button" onClick={() => navigate('/teacher/leave')} style={{ ...PHONE_SECONDARY, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8 }}>
            {t('tp.req.applyLeave')} <Icon name="arrowUpRight" size={16} />
          </button>
        ) : undefined
      }
    >
      {err ? <span role="alert" style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span> : null}
      {rows === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : rows.length === 0 ? (
        <PhoneEmpty>{t('tp.req.none')}</PhoneEmpty>
      ) : (
        <div style={PHONE_CARD}>
          {rows.map((r, i) => (
            <div key={r.id} style={{ display: 'flex', flexDirection: 'column', gap: 6, padding: '14px 16px', borderTop: i === 0 ? 0 : '1px solid var(--track)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <b style={{ flexGrow: 1, fontWeight: 600, fontSize: 15 }}>{requestKindLabel(r.kind, t)}</b>
                <span style={{ fontSize: 12, color: 'var(--muted)' }}>{formatDateShortIn(r.created_at, lang)}</span>
                <PhonePill tone={STATUS_TONE[r.status] ?? 'muted'}>{t(`tp.req.status.${STATUS_TONE[r.status] ? r.status : 'pending'}`)}</PhonePill>
              </div>
              {r.reason ? <span style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.45 }}>{r.reason}</span> : null}
              {r.note ? <span style={{ fontSize: 13, color: 'var(--gold-text)', lineHeight: 1.45 }}>{t('tp.req.note', { note: r.note })}</span> : null}
              {canCancel && (r.status === 'pending' || r.status === 'returned') ? (
                <button type="button" onClick={() => void cancel(r.id)} style={{ alignSelf: 'flex-start', height: 36, padding: '0 14px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, fontWeight: 500 }}>
                  {t('tp.req.cancel')}
                </button>
              ) : null}
            </div>
          ))}
        </div>
      )}
    </PhoneScreen>
  )
}
