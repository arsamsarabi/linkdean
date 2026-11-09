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
    src: 'INSTALL.md',
    dest: 'self-hosting/install.md',
    id: 'install',
    title: 'Install',
    sidebar_position: 1,
  },
  {
    src: 'SELF_HOSTED.md',
    dest: 'self-hosting/operations.md',
    id: 'operations',
    title: 'Operations',
    sidebar_position: 2,
  },
  {
    src: 'FEATURES.md',
    dest: 'features.md',
    id: 'features',
    title: 'Features',
    sidebar_position: 3,
  },
  {
    src: 'ARCHITECTURE.md',
    dest: 'architecture.md',
    id: 'architecture',
    title: 'Architecture',
    sidebar_position: 4,
  },
];

const githubBlob = 'https://github.com/arsamsarabi/linkdean/blob/main';

function rewriteLinks(body) {
  return (
    body
      .replace(/\]\(\.\/INSTALL\.md(#[^)]*)?\)/g, '](/docs/self-hosting/install$1)')
      .replace(/\]\(INSTALL\.md(#[^)]*)?\)/g, '](/docs/self-hosting/install$1)')
      .replace(/\]\(\.\/SELF_HOSTED\.md(#[^)]*)?\)/g, '](/docs/self-hosting/operations$1)')
      .replace(/\]\(SELF_HOSTED\.md(#[^)]*)?\)/g, '](/docs/self-hosting/operations$1)')
      .replace(/\]\(\.\/FEATURES\.md(#[^)]*)?\)/g, '](/docs/features$1)')
      .replace(/\]\(FEATURES\.md(#[^)]*)?\)/g, '](/docs/features$1)')
      .replace(/\]\(\.\/ARCHITECTURE\.md(#[^)]*)?\)/g, '](/docs/architecture$1)')
      .replace(/\]\(ARCHITECTURE\.md(#[^)]*)?\)/g, '](/docs/architecture$1)')
      // Not published on the docs site — send readers to GitHub.
      .replace(/\]\(\.\/ROADMAP\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/ROADMAP.md$1)`)
      .replace(/\]\(ROADMAP\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/ROADMAP.md$1)`)
      .replace(/\]\(\.\/IDEAS\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/IDEAS.md$1)`)
      .replace(/\]\(IDEAS\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/IDEAS.md$1)`)
      .replace(/\]\(\.\/CHANGELOG\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/CHANGELOG.md$1)`)
      .replace(/\]\(CHANGELOG\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/CHANGELOG.md$1)`)
      .replace(/\]\(\.\/MANAGED_SERVICE\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/MANAGED_SERVICE.md$1)`)
      .replace(/\]\(MANAGED_SERVICE\.md(#[^)]*)?\)/g, `](${githubBlob}/docs/MANAGED_SERVICE.md$1)`)
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
// The wipe takes the folder's sidebar label with it, so write it back.
mkdirSync(join(outRoot, 'self-hosting'), { recursive: true });
writeFileSync(
  join(outRoot, 'self-hosting', '_category_.json'),
  `${JSON.stringify({ label: 'Self-hosting', position: 1, collapsed: false }, null, 2)}\n`,
);
for (const name of ['features.md', 'architecture.md', 'intro.md']) {
  // intro is hand-written; keep it. Wipe only synced leaves below.
  if (name === 'intro.md') continue;
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
