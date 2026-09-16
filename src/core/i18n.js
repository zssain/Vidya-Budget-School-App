// Translation. All user-facing text goes through t() (see src/AGENTS.md).
// Keys are dot-grouped by area and live in src/locales/{en,hi}.json with
// identical key sets (checked by scripts/check-i18n.mjs).

import en from '../locales/en.json';
import hi from '../locales/hi.json';

const BUNDLES = { en, hi };
let current = 'en';
const listeners = new Set();

const isDev = typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.DEV;

export function getLanguage() {
  return current;
}

export function setLanguage(lang) {
  if (lang !== 'en' && lang !== 'hi') return;
  if (lang === current) return;
  current = lang;
  for (const fn of listeners) fn(lang);
}

export function onLanguageChange(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

function lookup(bundle, key) {
  return key.split('.').reduce((o, k) => (o && typeof o === 'object' ? o[k] : undefined), bundle);
}

/** Translate a key, substituting {param} placeholders. Missing keys return the key. */
export function t(key, params) {
  let value = lookup(BUNDLES[current], key);
  if (value === undefined) value = lookup(BUNDLES.en, key);
  if (typeof value !== 'string') {
    if (isDev) console.warn(`i18n: missing key "${key}"`);
    return key;
  }
  if (params) {
    value = value.replace(/\{(\w+)\}/g, (m, name) => (name in params ? String(params[name]) : m));
  }
  return value;
}
