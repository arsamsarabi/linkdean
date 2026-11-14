// @ts-check
import { defineConfig } from 'astro/config'

// Static build for the public-beta apex host. The build lands in dist/, which the
// edge image (deploy/docker/Dockerfile.static) serves as https://linkdean.app.
export default defineConfig({
  site: 'https://linkdean.app',
  output: 'static',
  build: { format: 'directory' },
})
