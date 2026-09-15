import js from '@eslint/js';
import globals from 'globals';

export default [
  // Paths ESLint must never lint.
  {
    ignores: ['dist', 'target', 'src-tauri/gen', 'reference'],
  },

  js.configs.recommended,

  {
    files: ['**/*.{js,mjs,cjs}'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      // Force all HTML through the safe `render()` helper.
      'no-restricted-properties': [
        'error',
        {
          property: 'innerHTML',
          message: 'Use render() from src/core/html.js',
        },
        {
          property: 'outerHTML',
          message: 'Use render() from src/core/html.js',
        },
      ],
      // No dynamic code evaluation.
      'no-restricted-syntax': [
        'error',
        {
          selector: "CallExpression[callee.name='eval']",
          message: 'eval is not allowed',
        },
      ],
    },
  },

  {
    // The safe HTML helper is the one place allowed to touch innerHTML.
    files: ['src/core/html.js'],
    rules: {
      'no-restricted-properties': 'off',
    },
  },
];
