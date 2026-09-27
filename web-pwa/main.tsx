// Entry point for the iPhone PWA (Phase 19). It mounts the SAME React app as the
// Tauri build; `platform` resolves to 'web' (set by web-pwa/vite.config.ts), so the
// app renders the phone layout and routes commands to the Web backend. Web-only init
// (service-worker registration, navigator.storage.persist()) is added in Steps 6/7.

import React from 'react'
import ReactDOM from 'react-dom/client'
import App from '@/App'

// Bundled font subsets only — never loaded from the web (docs/01-MOCK-SPEC.md §3);
// the same set the Tauri app bundles, so Hindi/Telugu render identically offline.
import '@fontsource/geist/latin-400.css'
import '@fontsource/geist/latin-500.css'
import '@fontsource/geist/latin-600.css'
import '@fontsource/geist/latin-700.css'
import '@fontsource/noto-sans-devanagari/devanagari-400.css'
import '@fontsource/noto-sans-devanagari/devanagari-500.css'
import '@fontsource/noto-sans-devanagari/devanagari-600.css'
import '@fontsource/noto-sans-devanagari/latin-400.css'
import '@fontsource/noto-sans-devanagari/latin-500.css'
import '@fontsource/noto-sans-devanagari/latin-600.css'
import '@fontsource/noto-sans-telugu/telugu-400.css'
import '@fontsource/noto-sans-telugu/telugu-500.css'
import '@fontsource/noto-sans-telugu/telugu-600.css'

import '@/styles/app.css'
import { loadAccent } from '@/lib/theme'
import { loadLang } from '@/lib/i18n'

loadAccent()
loadLang()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
