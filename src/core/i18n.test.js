import { beforeEach, describe, expect, it } from 'vitest';
import { getLanguage, onLanguageChange, setLanguage, t } from './i18n.js';

beforeEach(() => setLanguage('en'));

describe('i18n', () => {
  it('translates a key', () => {
    expect(t('nav.home')).toBe('Home');
  });

  it('returns the key when missing', () => {
    expect(t('does.not.exist')).toBe('does.not.exist');
  });

  it('substitutes {param} placeholders', () => {
    expect(t('login.wrongSchool', { code: 'vaani' })).toBe('This computer belongs to school code @vaani.');
  });

  it('switches language and reads Hindi', () => {
    setLanguage('hi');
    expect(getLanguage()).toBe('hi');
    expect(t('nav.home')).toBe('होम');
  });

  it('notifies language-change listeners', () => {
    let got = null;
    const off = onLanguageChange((l) => (got = l));
    setLanguage('hi');
    expect(got).toBe('hi');
    off();
  });
});
