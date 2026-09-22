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

import './styles/app.css'
import { loadAccent } from './lib/theme'

loadAccent()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
