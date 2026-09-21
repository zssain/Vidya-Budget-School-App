import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import * as commands from '../api/commands.js';
import { clearToken } from '../api/session.js';
import { useLanguage } from './i18n.jsx';
const SessionContext = createContext(null);
export function SessionProvider({ children, initialUser = null }) {
  const [user, setUser] = useState(initialUser);
  const { setLanguage } = useLanguage();
  const refresh = useCallback(async () => {
    const next = await commands.currentUser();
    setUser(next);
    return next;
  }, []);
  const signIn = useCallback(async (input) => {
    const result = await commands.signIn(input);
    if (result.user) setUser(result.user);
    return result;
  }, []);
  const signOut = useCallback(async () => {
    await commands.signOut();
    setUser(null);
  }, []);
  // Follow the signed-in user's language.
  useEffect(() => {
    if (user?.language) setLanguage(user.language);
  }, [user?.language, setLanguage]);
  // Return to the login screen when the desktop idle-watcher signs us out.
  useEffect(() => {
    let active = true;
    let unlisten;
    import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen('session-expired', () => {
          setUser(null);
          clearToken();
        }),
      )
      .then((fn) => {
        if (active) unlisten = fn;
        else fn();
      })
      .catch(() => {
        // Not running under Tauri (e.g. tests): no event bridge.
      });
    return () => {
      active = false;
      if (unlisten) unlisten();
    };
  }, []);
  const permissions = useMemo(() => user?.permissions || [], [user]);
  const value = useMemo(
    () => ({
      user,
      permissions,
      can: (action) => permissions.includes(action),
      signIn,
      signOut,
      refresh,
      setUser,
    }),
    [user, permissions, signIn, signOut, refresh],
  );
  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}
export function useCurrentUser() {
  return useContext(SessionContext);
}
