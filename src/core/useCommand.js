import { useCallback, useEffect, useRef, useState } from 'react';
import { toAppError } from '../api/errors.js';

export function useQuery(fn, deps = []) {
  const call = useRef(0);
  const [state, setState] = useState({ data: undefined, error: null, loading: true });
  const reload = useCallback(async () => {
    const id = ++call.current;
    setState((old) => ({ ...old, error: null, loading: true }));
    try {
      const data = await fn();
      if (id === call.current) setState({ data, error: null, loading: false });
      return data;
    } catch (error) {
      if (id === call.current) setState((old) => ({ ...old, error: toAppError(error), loading: false }));
      return undefined;
    }
    // `deps` is deliberately supplied by the caller, matching useEffect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  useEffect(() => {
    void reload();
    return () => {
      call.current += 1;
    };
  }, [reload]);
  return { ...state, reload };
}
export function useMutation(fn) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState(null);
  const run = useCallback(
    async (...args) => {
      setPending(true);
      setError(null);
      try {
        return await fn(...args);
      } catch (caught) {
        const appError = toAppError(caught);
        setError(appError);
        throw appError;
      } finally {
        setPending(false);
      }
    },
    [fn],
  );
  return { run, pending, error, fieldError: (name) => (error?.field === name ? error.message : undefined) };
}
