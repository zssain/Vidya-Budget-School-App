// Printing (docs/00 §PRINTING). Desktop uses Tauri's webview print API
// (WebviewWindow::print, verified present in tauri 2.11); if it errors we fall
// back to the browser window.print(). "Printed" means only that the print
// dialog opened (§7) — we never claim a page was physically printed.
//
// The caller first navigates to a hidden print route that renders ONLY the
// document with @page print CSS, waits a tick for layout, then calls print().

import * as api from './api'

/** Open the print dialog for the current window. Returns false if neither path worked. */
export async function printCurrentWindow(): Promise<boolean> {
  try {
    await api.print_page()
    return true
  } catch {
    try {
      window.print()
      return true
    } catch {
      return false
    }
  }
}
