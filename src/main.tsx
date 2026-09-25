import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'

// Bundled font subsets only — never loaded from the web (docs/01-MOCK-SPEC.md §3).
// Newsreader (variable, opsz axis) is declared in styles/fonts.css.
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

import './styles/app.css'
import { loadAccent } from './lib/theme'
import { loadLang } from './lib/i18n'

loadAccent()
loadLang()

// Phase-1-only DEV flag: `VITE_DEV_START=teacher` opens on Teacher Home (used by
// the Android debug build so it lands on a phone screen). Phase 3 removes it.
if (import.meta.env.VITE_DEV_START === 'teacher' && typeof location !== 'undefined' && !location.hash) {
  location.hash = '/teacher/home'
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
