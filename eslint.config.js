import eslint from '@eslint/js';
import globals from 'globals';
import * as sveltePlugin from 'eslint-plugin-svelte';
import tseslint from 'typescript-eslint';

export default [
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  ...sveltePlugin.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    rules: {
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      'svelte/no-navigation-without-resolve': 'off',
    },
  },
  {
    ignores: [
      'build/',
      '.svelte-kit/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
    ],
  },
];
