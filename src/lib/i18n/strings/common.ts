import type { Bundle } from '../index'

// App-wide strings shared across screens (docs/01-MOCK-SPEC.md). Screen-specific
// strings live in the per-screen modules.
const common: Bundle = {
  en: {
    'app.name': 'Vidya',
    'placeholder.title': 'Coming in a later phase',
    'nav.help': 'Help',
  },
  hi: {
    'app.name': 'Vidya',
    'placeholder.title': 'आगे किसी चरण में आएगा',
    'nav.help': 'सहायता',
  },
}
export default common
