import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Linkdean Docs',
  tagline: 'Self-host and operate Linkdean',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://docs.linkdean.app',
  baseUrl: '/',

  organizationName: 'arsamsarabi',
  projectName: 'linkdean',

  onBrokenLinks: 'throw',

  markdown: {
    format: 'detect',
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: ({ docPath }) => {
            const sourceByDoc: Record<string, string> = {
              'self-hosting/install.md': 'docs/INSTALL.md',
              'self-hosting/operations.md': 'docs/SELF_HOSTED.md',
              'features.md': 'docs/FEATURES.md',
              'architecture.md': 'docs/ARCHITECTURE.md',
              'intro.md': 'apps/docs/docs/intro.md',
            };
            const path =
              sourceByDoc[docPath] ?? `apps/docs/docs/${docPath}`;
            return `https://github.com/arsamsarabi/linkdean/edit/main/${path}`;
          },
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/social-card.png',
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Linkdean',
      logo: {
        alt: 'Linkdean',
        src: 'img/logo.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://app.linkdean.app',
          label: 'App',
          position: 'right',
        },
        {
          href: 'https://github.com/arsamsarabi/linkdean',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'Install', to: '/docs/self-hosting/install' },
            { label: 'Features', to: '/docs/features' },
            { label: 'Architecture', to: '/docs/architecture' },
          ],
        },
        {
          title: 'Product',
          items: [
            { label: 'Marketing site', href: 'https://linkdean.app' },
            { label: 'App', href: 'https://app.linkdean.app' },
            {
              label: 'Terms',
              href: 'https://app.linkdean.app/terms',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/arsamsarabi/linkdean',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Linkdean. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json', 'toml', 'yaml'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
