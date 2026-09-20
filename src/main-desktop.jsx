import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppProviders } from './app/AppProviders.jsx';
import { DesktopApp } from './app/DesktopApp.jsx';
import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/print.css';

export const DESKTOP_BUNDLE_MARKER = 'VIDYA_DESKTOP_ONLY';

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (event) => {
    const editable = event.target instanceof Element && event.target.closest('input, textarea');
    if (!editable) event.preventDefault();
  });
}

createRoot(document.getElementById('app')).render(
  <StrictMode>
    <AppProviders>
      <DesktopApp />
    </AppProviders>
  </StrictMode>,
);
