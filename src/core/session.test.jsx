import { describe, it, expect, vi } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import { I18nProvider } from './i18n.jsx';

// Capture the session-expired handler the provider registers.
let captured;
vi.mock('@tauri-apps/api/event', () => ({
  listen: (name, handler) => {
    if (name === 'session-expired') captured = handler;
    return Promise.resolve(() => {});
  },
}));
vi.mock('../api/commands.js', () => ({
  currentUser: vi.fn(),
  signIn: vi.fn(),
  signOut: vi.fn(),
}));

import { SessionProvider, useCurrentUser } from './session.jsx';

function Probe() {
  const { user } = useCurrentUser();
  return <div>{user ? `user:${user.name}` : 'no-user'}</div>;
}

describe('SessionProvider session-expired', () => {
  it('clears the signed-in user when the session-expired event fires', async () => {
    render(
      <I18nProvider>
        <SessionProvider initialUser={{ id: 'u1', name: 'Sunita', permissions: [], language: 'en' }}>
          <Probe />
        </SessionProvider>
      </I18nProvider>,
    );
    expect(screen.getByText('user:Sunita')).toBeInTheDocument();
    await waitFor(() => expect(typeof captured).toBe('function'));
    act(() => captured());
    await waitFor(() => expect(screen.getByText('no-user')).toBeInTheDocument());
  });
});
