import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright e2e for the Linkdean web app (@playwright/test 1.65).
 * Specs live in `e2e/` so Vitest's colocated `*.test.ts` files are never collected.
 *
 * Default target is the throwaway smoke stack (Vite :5199 → API :18081). Override with
 * PLAYWRIGHT_BASE_URL / PLAYWRIGHT_WEB_PORT. Reuse an already-running Vite when not on CI.
 *
 * `setup` registers a user via the API and writes `e2e/.auth/user.json`; `chromium` reuses it.
 *
 * @see https://playwright.dev/docs/test-configuration
 * @see .claude/skills/linkdean-throwaway-smoke-stack/SKILL.md
 */
const webPort = process.env.PLAYWRIGHT_WEB_PORT ?? '5199'
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? `http://127.0.0.1:${webPort}`
const apiProxy = process.env.API_PROXY_TARGET ?? 'http://127.0.0.1:18081'

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? 'github' : 'list',
  // Inventory is mostly skipped until phases land; keep collection green.
  passWithNoTests: true,
  use: {
    baseURL,
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'setup', testMatch: 'auth.setup.ts' },
    {
      name: 'chromium',
      testIgnore: 'auth.setup.ts',
      use: { ...devices['Desktop Chrome'], storageState: 'e2e/.auth/user.json' },
      dependencies: ['setup'],
    },
  ],
  webServer: {
    command: `API_PROXY_TARGET=${apiProxy} pnpm dev --host 127.0.0.1 --port ${webPort} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
