import { describe, it, expect, afterEach } from 'vitest';
import { screen, cleanup } from '@testing-library/react';
import { renderApp } from '../test/render.jsx';
import { StartupError } from './StartupError.jsx';
import en from '../locales/en.json';

afterEach(cleanup);

describe('StartupError', () => {
  it('shows the right message for each failure kind', () => {
    for (const kind of ['DataFolder', 'SecureStorage', 'WrongKey', 'Migration', 'Unknown']) {
      renderApp(<StartupError kind={kind} platform="macos" version="0.1.0" />);
      expect(screen.getByText(en.startup.error[kind])).toBeInTheDocument();
      cleanup();
    }
  });

  it('falls back to the Unknown message for an unrecognised kind', () => {
    renderApp(<StartupError kind="Nonsense" platform="macos" version="0.1.0" />);
    expect(screen.getByText(en.startup.error.Unknown)).toBeInTheDocument();
  });
});
