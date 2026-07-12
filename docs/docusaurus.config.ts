import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'OpenSXRaw',
  tagline: 'Rust reader for SCIEX .wiff / .wiff.scan mass spectrometry files',
  favicon: 'img/favicon.ico',

  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },
  plugins: ['docusaurus-plugin-llms-txt'],
  themes: ['@docusaurus/theme-mermaid'],

  url: 'https://sigilweaver.app',
  baseUrl: '/opensxraw/docs/',

  organizationName: 'Sigilweaver',
  projectName: 'OpenSXRaw',

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          routeBasePath: '/',
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/Sigilweaver/OpenSXRaw/tree/main/docs/',
        },
        blog: false,
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          filename: 'sitemap.xml',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    metadata: [
      { name: 'keywords', content: 'OpenSXRaw, SCIEX, wiff, mass spectrometry, TripleTOF, QTRAP, Rust' },
      { name: 'description', content: 'OpenSXRaw is a Rust reader for SCIEX .wiff / .wiff.scan mass spectrometry files.' },
    ],
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Sigilweaver',
      logo: {
        alt: 'Sigilweaver logo',
        src: 'img/logo.svg',
        href: 'https://sigilweaver.app',
        target: '_self',
      },
      items: [
        {
          type: 'dropdown',
          label: 'OpenSXRaw',
          position: 'left',
          items: [
            { label: 'OpenMassSpec', href: 'https://sigilweaver.app/openmassspec/docs/' },
            { label: 'OpenTFRaw (Thermo)', href: 'https://sigilweaver.app/opentfraw/docs/' },
            { label: 'OpenWRaw (Waters)', href: 'https://sigilweaver.app/openwraw/docs/' },
            { label: 'OpenTimsTDF (Bruker)', href: 'https://sigilweaver.app/opentimstdf/docs/' },
            { label: 'OpenARaw (Agilent)', href: 'https://sigilweaver.app/openaraw/docs/' },
          ],
        },
        {
          href: 'https://github.com/Sigilweaver/OpenSXRaw',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Project',
          items: [
            { label: 'GitHub', href: 'https://github.com/Sigilweaver/OpenSXRaw' },
            { label: 'Issues', href: 'https://github.com/Sigilweaver/OpenSXRaw/issues' },
          ],
        },
        {
          title: 'Legal',
          items: [
            { label: 'Terms of Use', href: 'https://sigilweaver.app/terms' },
            { label: 'Privacy Policy', href: 'https://sigilweaver.app/privacy' },
          ],
        },
      ],
      copyright: `Copyright ${new Date().getFullYear()} Sigilweaver Holdings LLC. OpenSXRaw is Apache-2.0 licensed. Documentation licensed under <a href="https://creativecommons.org/licenses/by-sa/4.0/" target="_blank" rel="noopener noreferrer">CC-BY-SA 4.0</a>.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
