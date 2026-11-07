// @ts-check
import { defineConfig } from 'astro/config'

// Static build for the public-beta apex host. Caddy serves
// deploy/public-beta/static/site as https://linkdean.app.
export default defineConfig({
  site: 'https://linkdean.app',
  output: 'static',
  outDir: '../../deploy/public-beta/static/site',
  build: { format: 'directory' },
})
