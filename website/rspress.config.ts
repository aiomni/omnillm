import { defineConfig } from '@rspress/core';

import {
  getNav,
  getSidebar,
  localeMetadata,
  themeLocaleMetadata
} from './theme/locale-data';

const base = process.env.RSPRESS_BASE ?? '/';

export default defineConfig({
  root: 'docs',
  base,
  lang: 'en',
  locales: localeMetadata as unknown as { description: string; lang: string; title: string }[],
  title: 'OmniLLM',
  description:
    'Provider-neutral Rust runtime for LLM routing, protocol transcoding, and budget-aware multi-key execution.',
  icon: '/favicon.svg',
  logo: '/omnillm-mark.svg',
  themeConfig: {
    darkMode: false,
    nav: getNav('en'),
    sidebar: getSidebar('en'),
    locales: themeLocaleMetadata as unknown as Array<{
      lang: string;
      nav: ReturnType<typeof getNav>;
      sidebar: ReturnType<typeof getSidebar>;
    }>,
    editLink: {
      docRepoBaseUrl: 'https://github.com/aiomni/omnillm/edit/main/website/docs',
      text: 'Edit this page on GitHub'
    }
  }
});
