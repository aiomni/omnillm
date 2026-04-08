import { defineConfig } from '@rspress/core';

const base = process.env.RSPRESS_BASE ?? '/';

export default defineConfig({
  root: 'docs',
  base,
  title: 'OmniLLM',
  description:
    'Provider-neutral Rust runtime for LLM routing, protocol transcoding, and budget-aware multi-key execution.',
  icon: '/favicon.svg',
  logo: '/omnillm-mark.svg',
  themeConfig: {
    darkMode: false,
    nav: [
      { text: 'Overview', link: '/', activeMatch: '^(?:/|/index(?:\\.html)?)$' },
      { text: 'Usage', link: '/usage' },
      { text: 'Architecture', link: '/architecture' },
      { text: 'Implementation', link: '/implementation' },
      { text: 'GitHub', link: 'https://github.com/aiomni/omnillm' }
    ],
    sidebar: {
      '/': [
        {
          text: 'Start Here',
          items: [
            { text: 'Overview', link: '/' },
            { text: 'Usage Guide', link: '/usage' },
            { text: 'Skill Guide', link: '/skill' }
          ]
        },
        {
          text: 'Deep Dive',
          items: [
            { text: 'Architecture Notes', link: '/architecture' },
            { text: 'Implementation Notes', link: '/implementation' }
          ]
        }
      ]
    },
    editLink: {
      docRepoBaseUrl: 'https://github.com/aiomni/omnillm/edit/main/website/docs',
      text: 'Edit this page on GitHub'
    }
  }
});
