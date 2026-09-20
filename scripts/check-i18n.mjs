#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Linter } from 'eslint';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const enPath = join(root, 'src/locales/en.json');
const hiPath = join(root, 'src/locales/hi.json');
const flatten = (object, prefix = '') =>
  Object.entries(object).flatMap(([name, value]) => {
    const key = prefix ? `${prefix}.${name}` : name;
    return value && typeof value === 'object' && !Array.isArray(value) ? flatten(value, key) : [key];
  });
if (!existsSync(enPath) || !existsSync(hiPath)) process.exit(0);
const en = new Set(flatten(JSON.parse(readFileSync(enPath, 'utf8'))));
const hi = new Set(flatten(JSON.parse(readFileSync(hiPath, 'utf8'))));
const errors = [];
for (const key of en) if (!hi.has(key)) errors.push(`missing in hi.json: ${key}`);
for (const key of hi) if (!en.has(key)) errors.push(`missing in en.json: ${key}`);

const files = [];
function walk(directory) {
  for (const item of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, item.name);
    if (item.isDirectory()) walk(path);
    else if (extname(path) === '.jsx') files.push(path);
  }
}
walk(join(root, 'src'));
const englishWords = (text) => (String(text).match(/[A-Za-z]+/g) || []).length >= 2;
const untranslatedRule = {
  create(context) {
    const source = context.sourceCode;
    const allowed = (node) =>
      source.getText(node).includes('i18n-ignore') ||
      source.lines
        .slice(node.loc.start.line - 1, node.loc.end.line)
        .some((line) => line.includes('i18n-ignore'));
    return {
      JSXText(node) {
        if (englishWords(node.value.trim()) && !allowed(node))
          context.report({ node, message: 'Visible JSX text must use t().' });
      },
      JSXAttribute(node) {
        if (
          !['title', 'placeholder', 'aria-label', 'alt'].includes(node.name.name) ||
          node.value?.type !== 'Literal'
        )
          return;
        if (englishWords(node.value.value) && !allowed(node))
          context.report({ node, message: 'Visible JSX attribute must use t().' });
      },
    };
  },
};
for (const path of files) {
  const source = readFileSync(path, 'utf8');
  const linter = new Linter();
  const messages = linter.verify(
    source,
    [
      {
        files: ['**/*.jsx'],
        languageOptions: {
          ecmaVersion: 2022,
          sourceType: 'module',
          parserOptions: { ecmaFeatures: { jsx: true } },
        },
        plugins: { vidya: { rules: { untranslated: untranslatedRule } } },
        rules: { 'vidya/untranslated': 'error' },
      },
    ],
    { filename: path },
  );
  for (const message of messages)
    errors.push(`${path.slice(root.length + 1)}:${message.line}: ${message.message}`);
  for (const match of source.matchAll(/\bt\(\s*['"]([^'"]+)['"]/g))
    if (!en.has(match[1])) errors.push(`${path.slice(root.length + 1)}: missing key ${match[1]}`);
}
if (errors.length) {
  console.error('i18n check failed:');
  errors.forEach((error) => console.error(`  - ${error}`));
  process.exit(1);
}
console.log(`check-i18n: OK (${en.size} keys in both languages; ${files.length} JSX files scanned)`);
