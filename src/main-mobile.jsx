import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppProviders } from './app/AppProviders.jsx';
import { MobileApp } from './app/MobileApp.jsx';
import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/print.css';

createRoot(document.getElementById('app')).render(
  <StrictMode>
    <AppProviders>
      <MobileApp />
    </AppProviders>
  </StrictMode>,
);
