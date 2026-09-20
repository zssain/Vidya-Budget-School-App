import { createContext, useCallback, useContext, useMemo, useState } from 'react';
const RouterContext = createContext(null);
export function RouterProvider({ children, initialView = 'home' }) {
  const [stack, setStack] = useState([{ viewId: initialView, params: {} }]);
  const current = stack.at(-1);
  const go = useCallback((viewId, params = {}) => setStack((old) => [...old, { viewId, params }]), []);
  const back = useCallback(() => setStack((old) => (old.length > 1 ? old.slice(0, -1) : old)), []);
  const value = useMemo(() => ({ ...current, go, back }), [current, go, back]);
  return <RouterContext.Provider value={value}>{children}</RouterContext.Provider>;
}
export function useRouter() {
  return useContext(RouterContext);
}
