// File-picker helpers over tauri-plugin-dialog. The dialog only returns a PATH;
// the actual read/write happens in a Rust command (so "success" means the file
// was written — no fake success). Resilient in a plain browser (returns null).

import { save, open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { isWeb } from './platform'

const CSV = [{ name: 'CSV', extensions: ['csv'] }]

/**
 * Open WhatsApp with a pre-filled message (docs/00 §WHATSAPP SHARE).
 * `https://wa.me/91<mobile>?text=<urlencoded>`. No mobile → open WhatsApp
 * without a number. Shares TEXT, not a file.
 */
export async function shareWhatsApp(mobile: string | null, text: string): Promise<void> {
  const num = mobile ? `91${mobile}` : ''
  const url = `https://wa.me/${num}?text=${encodeURIComponent(text)}`
  // iPhone PWA (Phase 19): there is no Tauri opener. Open the wa.me deep link
  // directly — a Home-Screen web app hands it to Safari, which opens WhatsApp. If a
  // popup is blocked in standalone mode, fall back to a same-tab navigation.
  if (isWeb) {
    const win = window.open(url, '_blank', 'noopener')
    if (!win) window.location.href = url
    return
  }
  try {
    await openUrl(url)
  } catch {
    try {
      window.open(url, '_blank')
    } catch {
      /* ignore */
    }
  }
}

/** Ask for a save path (CSV). Returns null if cancelled or unavailable. */
export async function pickSavePath(defaultName: string): Promise<string | null> {
  try {
    const p = await save({ defaultPath: defaultName, filters: CSV })
    return p ?? null
  } catch {
    return null
  }
}

/** Ask for a file to open (CSV). Returns null if cancelled or unavailable. */
export async function pickOpenPath(): Promise<string | null> {
  try {
    const r = await open({ multiple: false, directory: false, filters: CSV })
    return typeof r === 'string' ? r : null
  } catch {
    return null
  }
}

/** Ask for a Vidya backup file (`.vbak`) to restore from. Null if cancelled. */
export async function pickBackupPath(): Promise<string | null> {
  try {
    const r = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Vidya backup', extensions: ['vbak'] }],
    })
    return typeof r === 'string' ? r : null
  } catch {
    return null
  }
}
