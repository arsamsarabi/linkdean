#!/usr/bin/env node
/**
 * Copy curated markdown from repo `docs/` into `apps/docs/docs/`.
 * Edit sources under docs/; run `pnpm sync` (also runs before build).
 */
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const appRoot = join(__dirname, '..');
const repoDocs = join(appRoot, '../../docs');
const outRoot = join(appRoot, 'docs');

/** @type {{ src: string, dest: string, id: string, title: string, sidebar_position: number }[]} */
const pages = [
  {
    src: 'self-hosting/quick-start.md',
    dest: 'self-hosting/quick-start.md',
    id: 'quick-start',
    title: 'Quick start',
    sidebar_position: 1,
  },
  {
    src: 'self-hosting/environment.md',
    dest: 'self-hosting/environment.md',
    id: 'environment',
    title: 'Environment variables',
    sidebar_position: 2,
  },
  {
    src: 'self-hosting/operations.md',
    dest: 'self-hosting/operations.md',
    id: 'operations',
    title: 'Operations',
    sidebar_position: 3,
  },
  {
    src: 'product/features.md',
    dest: 'features.md',
    id: 'features',
    title: 'Features',
    sidebar_position: 4,
  },
  {
    src: 'guides/fast-capture.md',
    dest: 'guides/fast-capture.md',
    id: 'fast-capture',
    title: 'Saving pages fast',
    sidebar_position: 1,
  },
  {
    src: 'product/architecture.md',
    dest: 'architecture.md',
    id: 'architecture',
    title: 'Architecture',
    sidebar_position: 6,
  },
];

const githubBlob = 'https://github.com/arsamsarabi/linkdean/blob/main';

function rewriteLinks(body) {
  return (
    body
      // Self-hosting relative links (from within self-hosting/ or rewritten cousins).
      .replace(/\]\(\.\/quick-start\.md(#[^)]*)?\)/g, '](/docs/self-hosting/quick-start$1)')
      .replace(/\]\(\.\/environment\.md(#[^)]*)?\)/g, '](/docs/self-hosting/environment$1)')
      .replace(/\]\(\.\/operations\.md(#[^)]*)?\)/g, '](/docs/self-hosting/operations$1)')
      .replace(/\]\(\.\/\.\.\/self-hosting\/quick-start\.md(#[^)]*)?\)/g, '](/docs/self-hosting/quick-start$1)')
      .replace(/\]\(\.\/\.\.\/self-hosting\/environment\.md(#[^)]*)?\)/g, '](/docs/self-hosting/environment$1)')
      .replace(/\]\(\.\/\.\.\/self-hosting\/operations\.md(#[^)]*)?\)/g, '](/docs/self-hosting/operations$1)')
      .replace(/\]\(\.\.\/self-hosting\/quick-start\.md(#[^)]*)?\)/g, '](/docs/self-hosting/quick-start$1)')
      .replace(/\]\(\.\.\/self-hosting\/environment\.md(#[^)]*)?\)/g, '](/docs/self-hosting/environment$1)')
      .replace(/\]\(\.\.\/self-hosting\/operations\.md(#[^)]*)?\)/g, '](/docs/self-hosting/operations$1)')
      // Product + guides.
      .replace(/\]\(\.\/features\.md(#[^)]*)?\)/g, '](/docs/features$1)')
      .replace(/\]\(\.\/architecture\.md(#[^)]*)?\)/g, '](/docs/architecture$1)')
      .replace(/\]\(\.\/roadmap\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/product/roadmap.md$1)`)
      .replace(/\]\(\.\/ideas\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/product/ideas.md$1)`)
      .replace(/\]\(\.\.\/product\/features\.md(#[^)]*)?\)/g, '](/docs/features$1)')
      .replace(/\]\(\.\.\/product\/architecture\.md(#[^)]*)?\)/g, '](/docs/architecture$1)')
      .replace(/\]\(\.\.\/product\/roadmap\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/product/roadmap.md$1)`)
      .replace(/\]\(\.\.\/product\/ideas\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/product/ideas.md$1)`)
      .replace(/\]\(\.\.\/guides\/fast-capture\.md(#[^)]*)?\)/g, '](/docs/guides/fast-capture$1)')
      .replace(/\]\(\.\/fast-capture\.md(#[^)]*)?\)/g, '](/docs/guides/fast-capture$1)')
      // Ops / design (repo-only).
      .replace(
        /\]\(\.\.\/ops\/managed-service\.md(#[^)]*)?\)/g,
        `](${githubBlob}/docs/ops/managed-service.md$1)`,
      )
      .replace(
        /\]\(\.\.\/design\/organizations-and-teams\.md(#[^)]*)?\)/g,
        `](${githubBlob}/docs/design/organizations-and-teams.md$1)`,
      )
      .replace(/\]\(\.\.\/CHANGELOG\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/CHANGELOG.md$1)`)
      .replace(/\]\(\.\/CHANGELOG\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/CHANGELOG.md$1)`)
      .replace(/\]\(CHANGELOG\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/CHANGELOG.md$1)`)
      // Deploy paths from product/roadmap etc.
      .replace(
        /\]\(\.\.\/\.\.\/deploy\/public-beta\/BACKUP\.md(#[^)]*)?\)/g,
        `](${githubBlob}/deploy/public-beta/BACKUP.md$1)`,
      )
      .replace(
        /\]\(\.\.\/deploy\/public-beta\/BACKUP\.md(#[^)]*)?\)/g,
        `](${githubBlob}/deploy/public-beta/BACKUP.md$1)`,
      )
      .replace(
        /\]\(\.\.\/\.\.\/deploy\/public-beta\/MONITORING\.md(#[^)]*)?\)/g,
        `](${githubBlob}/deploy/public-beta/MONITORING.md$1)`,
      )
      .replace(
        /\]\(\.\.\/deploy\/public-beta\/MONITORING\.md(#[^)]*)?\)/g,
        `](${githubBlob}/deploy/public-beta/MONITORING.md$1)`,
      )
      .replace(
        /\]\(\.\.\/\.github\/SECURITY\.md(#[^)]*)?\)/g,
        `](${githubBlob}/.github/SECURITY.md$1)`,
      )
      .replace(
        /\]\(\/\.github\/SECURITY\.md(#[^)]*)?\)/g,
        `](${githubBlob}/.github/SECURITY.md$1)`,
      )
  );
}

rmSync(join(outRoot, 'self-hosting'), { recursive: true, force: true });
rmSync(join(outRoot, 'guides'), { recursive: true, force: true });
rmSync(join(outRoot, 'fast-capture.md'), { force: true });
mkdirSync(join(outRoot, 'self-hosting'), { recursive: true });
writeFileSync(
  join(outRoot, 'self-hosting', '_category_.json'),
  `${JSON.stringify({ label: 'Self-hosting', position: 1, collapsed: false }, null, 2)}\n`,
);
mkdirSync(join(outRoot, 'guides'), { recursive: true });
writeFileSync(
  join(outRoot, 'guides', '_category_.json'),
  `${JSON.stringify({ label: 'Guides', position: 5, collapsed: false }, null, 2)}\n`,
);

for (const name of ['features.md', 'architecture.md']) {
  rmSync(join(outRoot, name), { force: true });
}

for (const page of pages) {
  const raw = readFileSync(join(repoDocs, page.src), 'utf8');
  const body = rewriteLinks(raw);
  const frontmatter = [
    '---',
    `# Generated from docs/${page.src} — do not edit here. Run: pnpm sync`,
    'format: md',
    `id: ${page.id}`,
    `title: ${page.title}`,
    `sidebar_position: ${page.sidebar_position}`,
    '---',
    '',
  ].join('\n');
  const dest = join(outRoot, page.dest);
  mkdirSync(dirname(dest), { recursive: true });
  writeFileSync(dest, frontmatter + body);
  console.log(`synced ${page.src} → docs/${page.dest}`);
}
