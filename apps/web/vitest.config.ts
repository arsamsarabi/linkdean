import path from 'node:path'
import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

const rootDir = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(rootDir, 'src'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: false,
    setupFiles: ['./src/test/setup.ts'],
    // Playwright journeys live in e2e/; Vitest must not load them (they call test.describe from @playwright/test).
    exclude: ['**/node_modules/**', '**/dist/**', 'e2e/**'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
})
