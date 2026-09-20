import { createContext, useCallback, useContext, useMemo, useState } from 'react';
import * as commands from '../api/commands.js';
const SessionContext = createContext(null);
export function SessionProvider({ children, initialUser = null }) {
  const [user, setUser] = useState(initialUser);
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
