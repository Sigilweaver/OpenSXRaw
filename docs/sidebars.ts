import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    'install',
    'quickstart',
    {
      type: 'category',
      label: 'Guide',
      collapsed: false,
      items: [
        'guide/reader',
        'guide/scan-data',
        'guide/instrument-families',
        'guide/mzml-export',
        'guide/python-api',
      ],
    },
    {
      type: 'category',
      label: 'Format Specification',
      link: { type: 'doc', id: 'format/overview' },
      items: ['format/overview', 'format/legacy-wiff-cfbf', 'format/legacy-wiff-scan', 'format/wiff2-container'],
    },
    'changelog',
    'license',
  ],
};

export default sidebars;
