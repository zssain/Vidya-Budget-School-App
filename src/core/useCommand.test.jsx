import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useMutation, useQuery } from './useCommand.js';
import { AppError } from '../api/errors.js';
describe('command hooks', () => {
  it('ignores a stale response', async () => {
    let release;
    const first = new Promise((resolve) => {
      release = resolve;
    });
    const { result, rerender } = renderHook(({ fn }) => useQuery(fn, [fn]), {
      initialProps: { fn: () => first },
    });
    rerender({ fn: () => Promise.resolve('new') });
    await act(async () => {});
    expect(result.current.data).toBe('new');
    await act(async () => release('old'));
    expect(result.current.data).toBe('new');
  });
  it('maps field errors', async () => {
    const { result } = renderHook(() =>
      useMutation(() =>
        Promise.reject(new AppError({ kind: 'validation', message: 'Bad amount', field: 'amount' })),
      ),
    );
    await act(async () => {
      await expect(result.current.run()).rejects.toThrow('Bad amount');
    });
    expect(result.current.fieldError('amount')).toBe('Bad amount');
  });
});
