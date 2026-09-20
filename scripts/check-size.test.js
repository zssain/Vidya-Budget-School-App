import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { LIMIT_BYTES, WARNING_BYTES, pathBytes, sizeState } from './check-size.mjs';

describe('size gate', () => {
  it('measures all files in an installed app directory', () => {
    const root = mkdtempSync(join(tmpdir(), 'vidya-size-'));
    mkdirSync(join(root, 'nested'));
    writeFileSync(join(root, 'one'), Buffer.alloc(123));
    writeFileSync(join(root, 'nested', 'two'), Buffer.alloc(456));
    expect(pathBytes(root)).toBe(579);
  });

  it('warns above 25 MB and fails above 30 MB', () => {
    expect(sizeState(WARNING_BYTES)).toBe('OK');
    expect(sizeState(WARNING_BYTES + 1)).toBe('WARN');
    expect(sizeState(LIMIT_BYTES)).toBe('WARN');
    expect(sizeState(LIMIT_BYTES + 1)).toBe('FAIL');
  });
});
