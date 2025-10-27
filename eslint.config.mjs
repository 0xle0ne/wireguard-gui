import { defineConfig, globalIgnores } from 'eslint/config';
import nextCoreWebVitals from 'eslint-config-next/core-web-vitals';
import _import from 'eslint-plugin-import';
// import { fixupPluginRules } from '@eslint/compat';
import tsParser from '@typescript-eslint/parser';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import js from '@eslint/js';
import { FlatCompat } from '@eslint/eslintrc';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  globalIgnores([
    'dist/*',
    '**/.cache',
    '**/public',
    '**/node_modules',
    '**/*.esm.js',
    '**/*.config.js',
    'src-tauri/*',
    'out/*',
    'eslint.config.mjs',
    'ts.config.json',
  ]),
  {
    extends: [...nextCoreWebVitals, ...compat.extends('prettier')],

    plugins: {
      import: _import,
    },

    languageOptions: {
      ecmaVersion: 5,
      sourceType: 'script',
      parserOptions: {
        project: './tsconfig.json',
        tsConfigRootDir: './',
      },
    },

    settings: {
      'import/parsers': {
        '@typescript-eslint/parser': ['.ts', '.tsx'],
      },

      'import/resolver': {
        typescript: {
          alwaysTryTypes: true,
        },
      },

      tailwindcss: {
        callees: ['cn'],
        config: './tailwind.config.js',
      },

      next: {
        rootDir: ['./'],
      },
    },

    rules: {
      'import/no-unresolved': 'error',
      '@next/next/no-html-link-for-pages': 'off',
      'react/jsx-key': 'off',
      'tailwindcss/no-custom-classname': 'off',
      'comma-dangle': 'off',
      '@typescript-eslint/comma-dangle': 'off',
      indent: 'off',
      '@typescript-eslint/indent': 'off',
    },
  },
  {
    files: ['**/*.ts', '**/*.tsx'],

    languageOptions: {
      parser: tsParser,
    },
  },
]);
