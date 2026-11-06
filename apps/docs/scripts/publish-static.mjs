#!/usr/bin/env node
/** Copy Docusaurus `build/` into `deploy/public-beta/static/docs/` for Caddy. */
import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const buildDir = join(__dirname, '../build');
const target = join(__dirname, '../../../deploy/public-beta/static/docs');

rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });
cpSync(buildDir, target, { recursive: true });
console.log(`published build/ → ${target}`);
