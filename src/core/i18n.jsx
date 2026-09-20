import { createContext, useContext, useMemo, useState } from 'react';
import en from '../locales/en.json';
import hi from '../locales/hi.json';

const dictionaries = { en, hi };
const I18nContext = createContext(null);
function lookup(source, key) {
  return key.split('.').reduce((value, part) => value?.[part], source);
}
export function translate(language, key, params = {}) {
  let value = lookup(dictionaries[language] || en, key);
  if (typeof value !== 'string') {
    if (import.meta.env?.DEV) console.warn(`Missing translation: ${key}`);
    return key;
  }
  return value.replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? `{${name}}`));
}
export function I18nProvider({ children, initialLanguage = 'en' }) {
  const [language, setLanguage] = useState(initialLanguage);
  const value = useMemo(
    () => ({ language, setLanguage, t: (key, params) => translate(language, key, params) }),
    [language],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
export function useT() {
  return useContext(I18nContext).t;
}
export function useLanguage() {
  const { language, setLanguage } = useContext(I18nContext);
  return { language, setLanguage };
}
