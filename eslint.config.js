import js from '@eslint/js';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import globals from 'globals';

const restrictedSyntax = [
  'error',
  {
    selector: "CallExpression[callee.name='eval']",
    message: 'Dynamic code evaluation is not allowed.',
  },
  {
    selector: "NewExpression[callee.name='Function']",
    message: 'Dynamic code evaluation is not allowed.',
  },
  {
    selector: "JSXAttribute[name.name='dangerouslySetInnerHTML']",
    message: 'Render with React components (UI_GUIDE.md).',
  },
];

export default [
  { ignores: ['dist', 'dist-mobile', 'target', 'src-tauri/gen', 'reference'] },
  js.configs.recommended,
  react.configs.flat.recommended,
  react.configs.flat['jsx-runtime'],
  reactHooks.configs.flat.recommended,
  {
    files: ['**/*.{js,jsx,mjs,cjs}'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.node },
    },
    settings: { react: { version: 'detect' } },
    rules: {
      'react/no-danger': 'error',
      'react/jsx-no-target-blank': 'error',
      'react/prop-types': 'off',
      'no-restricted-properties': [
        'error',
        {
          property: 'innerHTML',
          message: 'Render with React components (UI_GUIDE.md).',
        },
        {
          property: 'outerHTML',
          message: 'Render with React components (UI_GUIDE.md).',
        },
        {
          property: 'insertAdjacentHTML',
          message: 'Render with React components (UI_GUIDE.md).',
        },
        {
          object: 'document',
          property: 'write',
          message: 'Render with React components (UI_GUIDE.md).',
        },
      ],
      'no-restricted-syntax': restrictedSyntax,
    },
  },
];
