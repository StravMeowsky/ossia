// Vite+ owns workspace formatting, lint and development checks.
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    exclude: ['**/node_modules/**', '**/target/**', 'packages/ui/tests/**'],
  },
  staged: {
    '*': 'vp check --fix',
  },
  fmt: {
    singleQuote: true,
    semi: true,
    sortImports: {},
    ignorePatterns: [
      '.docs/',
      'packages/ui/src/generated/',
      'tests/fixtures/',
      'vendor/',
      'packages/desktop/src-tauri/{target,gen,icons}/',
      'packages/*/dist/',
      'pnpm-lock.yaml',
    ],
  },
  lint: {
    ignorePatterns: [
      '.docs/',
      'packages/ui/src/generated/',
      'tests/fixtures/',
      'vendor/',
      'packages/desktop/src-tauri/{target,gen}/',
      'packages/*/dist/',
    ],
    options: { typeAware: true, typeCheck: true },
  },
});
