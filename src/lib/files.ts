// File-picker helpers over tauri-plugin-dialog. The dialog only returns a PATH;
// the actual read/write happens in a Rust command (so "success" means the file
// was written — no fake success). Resilient in a plain browser (returns null).

import { save, open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'

const CSV = [{ name: 'CSV', extensions: ['csv'] }]

/**
 * Open WhatsApp with a pre-filled message (docs/00 §WHATSAPP SHARE).
 * `https://wa.me/91<mobile>?text=<urlencoded>`. No mobile → open WhatsApp
 * without a number. Shares TEXT, not a file.
 */
export async function shareWhatsApp(mobile: string | null, text: string): Promise<void> {
  const num = mobile ? `91${mobile}` : ''
  const url = `https://wa.me/${num}?text=${encodeURIComponent(text)}`
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
