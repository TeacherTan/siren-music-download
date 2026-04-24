import js from '@eslint/js';
import globals from 'globals';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import sveltePlugin from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import prettierConfig from 'eslint-config-prettier';

const sharedGlobals = {
  ...globals.browser,
  ...globals.node,
  ...globals.es2021,
};

const tsRules = {
  ...tseslint.configs.recommended.rules,
  '@typescript-eslint/no-unused-vars': [
    'warn',
    { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
  ],
  '@typescript-eslint/no-explicit-any': 'warn',
  'no-unused-vars': 'off',
  'no-undef': 'off',
};

export default [
  {
    ignores: [
      'node_modules/**',
      'dist/**',
      'target/**',
      'src-tauri/target/**',
      '.svelte-kit/**',
      'build/**',
      '.claude/**',
    ],
  },
  {
    files: ['**/*.{js,mjs,cjs}'],
    ...js.configs.recommended,
    languageOptions: {
      ...js.configs.recommended.languageOptions,
      globals: sharedGlobals,
    },
    rules: {
      ...js.configs.recommended.rules,
      'no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
      globals: sharedGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: tsRules,
  },
  ...sveltePlugin.configs['flat/recommended'],
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsparser,
        extraFileExtensions: ['.svelte'],
      },
      globals: sharedGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tsRules,
      'no-unused-expressions': 'off',
      '@typescript-eslint/no-unused-expressions': 'off',
      'no-useless-assignment': 'warn',
      'no-unsafe-finally': 'warn',
      'svelte/no-unused-svelte-ignore': 'warn',
      'svelte/no-useless-children-snippet': 'warn',
    },
  },
  {
    files: ['**/*.svelte.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
      globals: sharedGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tsRules,
      'svelte/prefer-svelte-reactivity': 'off',
    },
  },
  prettierConfig,
];
