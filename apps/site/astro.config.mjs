// @ts-check
import { defineConfig } from 'astro/config'

// Static build for the public-beta apex host. The build lands in dist/, which the Release
// workflow copies to the guest, where Caddy serves it as https://linkdean.app.
export default defineConfig({
  site: 'https://linkdean.app',
  output: 'static',
  build: { format: 'directory' },
})
