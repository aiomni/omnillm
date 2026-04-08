export type SiteLanguage = 'en' | 'zh';

export type NavLinkConfig = {
  activeMatch?: string;
  link: string;
  text: string;
};

export type SidebarGroupConfig = {
  items: Array<{
    link: string;
    text: string;
  }>;
  text: string;
};

type HeroStat = {
  label: string;
  value: string;
};

type FeatureCard = {
  accent: string;
  copy: string;
  detail: string;
  eyebrow: string;
  title: string;
};

type DocCard = {
  copy: string;
  href: string;
  label: string;
  title: string;
};

type StripCard = {
  copy: string;
  label: string;
  title: string;
};

type HomeContent = {
  badgeProviderNeutral: string;
  badgeSkill: string;
  bottomStrip: StripCard[];
  browseSourceLabel: string;
  codeFileLabel: string;
  docCards: DocCard[];
  docsDescription: string;
  docsKicker: string;
  docsTitle: string;
  featureCards: FeatureCard[];
  getStartedLabel: string;
  heroCopy: string;
  heroSignalCopy: string;
  heroSignalKicker: string;
  heroSignalTitle: string;
  heroStats: HeroStat[];
  installHint: string;
  installSkillLabel: string;
  metaDescription: string;
  readLabel: string;
  showcaseBullets: string[];
  showcaseCopy: string;
  showcaseKicker: string;
  showcaseTitle: string;
};

type ChromeText = {
  documentationFallbackLabel: string;
  footerAriaLabel: string;
  footerLead: string;
  githubLabel: string;
  installCta: string;
  installPanelKicker: string;
  installPanelLink: string;
  languageLabel: string;
  needInternalsKicker: string;
  needInternalsLink: string;
  needInternalsText: string;
  openDocIndexLabel: string;
  quickPathsKicker: string;
  repositorySourceKicker: string;
};

export const DEFAULT_LANGUAGE: SiteLanguage = 'en';
export const LANGUAGE_STORAGE_KEY = 'omnillm.language';
export const SUPPORTED_LANGUAGES: SiteLanguage[] = ['en', 'zh'];

function rootActiveMatch(lang: SiteLanguage) {
  if (lang === 'zh') {
    return '^/zh(?:/|/index(?:\\.html)?)?$';
  }

  return '^(?:/|/index(?:\\.html)?)$';
}

function isExternalPath(path: string) {
  return /^(?:https?:)?\/\//.test(path) || path.startsWith('mailto:') || path.startsWith('tel:');
}

function splitHash(path: string) {
  const [pathname, hash = ''] = path.split('#');
  return {
    hash,
    pathname
  };
}

function normalizeInternalPath(path: string) {
  const trimmed = path.trim();

  if (!trimmed || trimmed === '/') {
    return '/';
  }

  const withLeadingSlash = trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
  const withoutHtml = withLeadingSlash
    .replace(/\/index\.html?$/i, '/')
    .replace(/\.html?$/i, '')
    .replace(/\/+$/, '');

  return withoutHtml || '/';
}

export function stripLanguagePrefix(path: string) {
  const normalized = normalizeInternalPath(path);

  if (normalized === '/zh') {
    return '/';
  }

  if (normalized.startsWith('/zh/')) {
    return normalized.slice(3) || '/';
  }

  return normalized;
}

export function localizePath(path: string, lang: SiteLanguage) {
  if (!path || path.startsWith('#') || isExternalPath(path)) {
    return path;
  }

  const { hash, pathname } = splitHash(path);
  const canonicalPath = stripLanguagePrefix(pathname);
  const localizedPath =
    lang === 'zh'
      ? canonicalPath === '/'
        ? '/zh/'
        : `/zh${canonicalPath}`
      : canonicalPath;

  return hash ? `${localizedPath}#${hash}` : localizedPath;
}

export function inferLanguageFromPathname(pathname: string): SiteLanguage {
  return stripLanguagePrefix(pathname) !== normalizeInternalPath(pathname) ? 'zh' : 'en';
}

export function canonicalRouteFromRelativePath(relativePath?: string) {
  if (!relativePath) {
    return '/';
  }

  const withoutLanguage = relativePath.replace(/^(en|zh)\//, '');

  if (/^index\.mdx?$/i.test(withoutLanguage)) {
    return '/';
  }

  return normalizeInternalPath(withoutLanguage.replace(/\.mdx?$/i, ''));
}

export function canonicalRouteFromPath(pathname: string) {
  return stripLanguagePrefix(pathname);
}

export function pageSourcePath(relativePath?: string) {
  if (!relativePath) {
    return 'website/docs';
  }

  return `website/docs/${relativePath}`;
}

export function getRouteLabel(route: string, lang: SiteLanguage) {
  const labels = ROUTE_LABELS[lang];
  return labels[route] ?? labels.default;
}

export function getFooterLinks(lang: SiteLanguage) {
  return [
    { href: localizePath('/', lang), label: lang === 'zh' ? '概览' : 'Overview' },
    { href: localizePath('/usage', lang), label: lang === 'zh' ? '使用' : 'Usage' },
    {
      href: localizePath('/architecture', lang),
      label: lang === 'zh' ? '架构' : 'Architecture'
    },
    {
      href: localizePath('/implementation', lang),
      label: lang === 'zh' ? '实现' : 'Implementation'
    }
  ];
}

export function getNav(lang: SiteLanguage): NavLinkConfig[] {
  return [
    {
      activeMatch: rootActiveMatch(lang),
      link: localizePath('/', lang),
      text: lang === 'zh' ? '概览' : 'Overview'
    },
    {
      link: localizePath('/usage', lang),
      text: lang === 'zh' ? '使用' : 'Usage'
    },
    {
      link: localizePath('/architecture', lang),
      text: lang === 'zh' ? '架构' : 'Architecture'
    },
    {
      link: localizePath('/implementation', lang),
      text: lang === 'zh' ? '实现' : 'Implementation'
    },
    {
      link: 'https://github.com/aiomni/omnillm',
      text: 'GitHub'
    }
  ];
}

export function getSidebar(lang: SiteLanguage): Record<string, SidebarGroupConfig[]> {
  return {
    '/': [
      {
        text: lang === 'zh' ? '从这里开始' : 'Start Here',
        items: [
          { text: lang === 'zh' ? '概览' : 'Overview', link: localizePath('/', lang) },
          {
            text: lang === 'zh' ? '使用指南' : 'Usage Guide',
            link: localizePath('/usage', lang)
          },
          {
            text: lang === 'zh' ? '技能指南' : 'Skill Guide',
            link: localizePath('/skill', lang)
          }
        ]
      },
      {
        text: lang === 'zh' ? '深入阅读' : 'Deep Dive',
        items: [
          {
            text: lang === 'zh' ? '架构说明' : 'Architecture Notes',
            link: localizePath('/architecture', lang)
          },
          {
            text: lang === 'zh' ? '实现说明' : 'Implementation Notes',
            link: localizePath('/implementation', lang)
          }
        ]
      }
    ]
  };
}

export const localeLabels: Record<SiteLanguage, string> = {
  en: 'EN',
  zh: '中文'
};

export const localeMetadata = [
  {
    description:
      'Provider-neutral Rust runtime for LLM routing, protocol transcoding, and budget-aware multi-key execution.',
    label: 'English',
    lang: 'en',
    title: 'OmniLLM'
  },
  {
    description: '面向 LLM 路由、协议转码与预算感知多 Key 执行的 provider-neutral Rust 运行时。',
    label: '简体中文',
    lang: 'zh',
    title: 'OmniLLM'
  }
] as const;

export const themeLocaleMetadata = [
  {
    label: 'English',
    lang: 'en',
    nav: getNav('en'),
    sidebar: getSidebar('en')
  },
  {
    label: '简体中文',
    lang: 'zh',
    nav: getNav('zh'),
    sidebar: getSidebar('zh')
  }
] as const;

export const chromeText: Record<SiteLanguage, ChromeText> = {
  en: {
    documentationFallbackLabel: 'documentation',
    footerAriaLabel: 'Footer',
    footerLead:
      'Provider-neutral Rust runtime for routing, transcoding, replay-safe tracing, and budget-aware execution.',
    githubLabel: 'GitHub',
    installCta: 'Install',
    installPanelKicker: 'crate install',
    installPanelLink: 'Install the crate',
    languageLabel: 'Language',
    needInternalsKicker: 'Need internals?',
    needInternalsLink: 'Implementation',
    needInternalsText:
      'Cross-check the behavior with implementation notes or inspect the repository directly.',
    openDocIndexLabel: 'Open documentation index',
    quickPathsKicker: 'Quick Paths',
    repositorySourceKicker: 'Repository Source'
  },
  zh: {
    documentationFallbackLabel: '文档',
    footerAriaLabel: '页脚',
    footerLead:
      '为 Rust 提供面向多模型路由、协议转码、可回放追踪与预算感知执行的 provider-neutral 运行时。',
    githubLabel: 'GitHub',
    installCta: '安装',
    installPanelKicker: 'crate 安装',
    installPanelLink: '安装 crate',
    languageLabel: '语言',
    needInternalsKicker: '需要内部实现？',
    needInternalsLink: '实现说明',
    needInternalsText: '可以对照实现说明中的行为解析，或直接查看仓库源码。',
    openDocIndexLabel: '打开文档索引',
    quickPathsKicker: '快速跳转',
    repositorySourceKicker: '仓库源码'
  }
};

export const homeContent: Record<SiteLanguage, HomeContent> = {
  en: {
    badgeProviderNeutral: 'provider-neutral Rust runtime',
    badgeSkill: 'bundled OmniLLM Skill',
    bottomStrip: [
      {
        copy: 'Provider-neutral runtime calls with typed generation surfaces.',
        label: 'Focus',
        title: 'Gateway dispatch'
      },
      {
        copy: 'Transcoding stays explicit about downgraded or unsupported fields.',
        label: 'Safety',
        title: 'Loss-aware bridges'
      },
      {
        copy: 'Quota reservation and settlement wrap every request lifecycle.',
        label: 'Operations',
        title: 'Budget-first execution'
      }
    ],
    browseSourceLabel: 'Browse Source',
    codeFileLabel: 'examples/basic_usage.rs',
    docCards: [
      {
        copy:
          'Install the crate, configure endpoints, send requests, stream results, and operate the runtime in production-shaped flows.',
        href: '/usage',
        label: 'Guide',
        title: 'Usage Guide'
      },
      {
        copy:
          "Install the OmniLLM Skill in Claude Code, Codex, OpenCode, or Claude and keep agents aligned with the crate's real boundaries.",
        href: '/skill',
        label: 'Skill',
        title: 'Skill Guide'
      },
      {
        copy:
          'Read the lease lifecycle, key-pool acquisition strategy, and budget tracker model before diving into source.',
        href: '/architecture',
        label: 'Design',
        title: 'Architecture Notes'
      },
      {
        copy:
          'Walk the crate module by module when you want concrete execution paths, data structures, and internal boundaries.',
        href: '/implementation',
        label: 'Source',
        title: 'Implementation Notes'
      }
    ],
    docsDescription:
      'Start with operational usage, move into architecture, then read the implementation notes when you want the concrete module boundaries.',
    docsKicker: 'Documentation',
    docsTitle: 'Choose the reading depth you need.',
    featureCards: [
      {
        accent: '#0969da',
        copy:
          'Gateway dispatch keeps request typing, key pooling, timeouts, and usage accounting in the same execution path.',
        detail: 'responses · chat · streams',
        eyebrow: 'Gateway',
        title: 'Route canonical requests through one runtime surface.'
      },
      {
        accent: '#ffb4a6',
        copy:
          'Loss reports stay explicit so application logic can branch on bridged or dropped fields instead of guessing.',
        detail: 'typed conversion · loss metadata',
        eyebrow: 'Bridge',
        title: 'Transcode across APIs without hiding downgrade behavior.'
      },
      {
        accent: '#7fe0ff',
        copy:
          'Per-key limits, budget reservation, fixture sanitization, registry metadata, and the bundled OmniLLM Skill all live next to the crate.',
        detail: 'budgets · replay · skill',
        eyebrow: 'Operate',
        title: 'Keep quota, replay, and provider state visible.'
      }
    ],
    getStartedLabel: 'Get Started',
    heroCopy:
      'Type-safe, high-performance LLM routing, protocol bridging, and budget-aware multi-key execution for Rust, with a bundled OmniLLM Skill in the repository.',
    heroSignalCopy:
      'Canonical request types, loss-aware transcoding, and budget settlement stay in one operational frame, and the repository also includes the OmniLLM Skill for repo-native assistance.',
    heroSignalKicker: 'AI-Native Runtime',
    heroSignalTitle: 'One runtime crate, plus an OmniLLM Skill.',
    heroStats: [
      { value: '04', label: 'generation protocols' },
      { value: '07', label: 'provider families' },
      { value: '01', label: 'bundled OmniLLM Skill' }
    ],
    installHint: 'crate install · skill included',
    installSkillLabel: 'Install Skill',
    metaDescription:
      'AI-native provider-neutral Rust runtime for LLM routing, protocol transcoding, bundled OmniLLM Skill workflows, and budget-aware multi-key execution.',
    readLabel: 'Read',
    showcaseBullets: [
      'Canonical request and response models stay visible in the docs.',
      'Key-pool, budget, and replay tooling are documented from the repository state.',
      'Implementation notes point directly back to the modules that enforce the behavior.',
      'The Skill Guide covers Claude Code, Codex, OpenCode, and Claude installation paths.'
    ],
    showcaseCopy:
      'Usage notes, architecture rationale, implementation walkthroughs, and the bundled OmniLLM Skill live beside the crate so behavior, design, AI guidance, and source stay aligned.',
    showcaseKicker: 'Source-Adjacent Docs And Skill',
    showcaseTitle: 'Operate the runtime and onboard AI agents from repository context.'
  },
  zh: {
    badgeProviderNeutral: 'provider-neutral Rust 运行时',
    badgeSkill: '内置 OmniLLM Skill',
    bottomStrip: [
      {
        copy: '用类型化生成接口统一发起 provider-neutral 运行时调用。',
        label: '重点',
        title: 'Gateway 调度'
      },
      {
        copy: '转码时会明确暴露降级或不支持的字段，而不是静默吞掉。',
        label: '安全性',
        title: '感知损耗的桥接'
      },
      {
        copy: '配额预留与结算覆盖每一次请求的完整生命周期。',
        label: '运维',
        title: '预算优先执行'
      }
    ],
    browseSourceLabel: '查看源码',
    codeFileLabel: 'examples/basic_usage.rs',
    docCards: [
      {
        copy:
          '安装 crate、配置端点、发起请求、处理流式结果，并按生产环境习惯运行 OmniLLM。',
        href: '/usage',
        label: '指南',
        title: '使用指南'
      },
      {
        copy:
          '在 Claude Code、Codex、OpenCode 或 Claude 中安装 OmniLLM Skill，让 agent 与 crate 的真实边界保持一致。',
        href: '/skill',
        label: '技能',
        title: '技能指南'
      },
      {
        copy: '在阅读源码之前，先了解租约生命周期、Key 池获取策略与预算跟踪模型。',
        href: '/architecture',
        label: '设计',
        title: '架构说明'
      },
      {
        copy: '当你需要具体执行路径、数据结构与内部边界时，按模块阅读 crate 实现。',
        href: '/implementation',
        label: '源码',
        title: '实现说明'
      }
    ],
    docsDescription: '可以先看运行与接入，再看架构，最后在需要具体模块边界时深入实现说明。',
    docsKicker: '文档',
    docsTitle: '按你需要的深度阅读 OmniLLM。',
    featureCards: [
      {
        accent: '#0969da',
        copy: 'Gateway 调度把请求类型、Key 池、超时与用量结算放在同一条执行路径上。',
        detail: 'responses · chat · streams',
        eyebrow: 'Gateway',
        title: '通过统一运行时接口路由规范化请求。'
      },
      {
        accent: '#ffb4a6',
        copy: '转换损耗会显式呈现，让业务逻辑能基于桥接与丢失字段作出判断，而不是猜测。',
        detail: '类型化转换 · 损耗元数据',
        eyebrow: 'Bridge',
        title: '跨 API 转码，但不隐藏降级行为。'
      },
      {
        accent: '#7fe0ff',
        copy:
          '单 Key 限流、预算预留、回放脱敏、注册表元数据与 OmniLLM Skill 都与 crate 放在同一仓库。',
        detail: '预算 · 回放 · skill',
        eyebrow: 'Operate',
        title: '让配额、回放和 provider 状态始终可见。'
      }
    ],
    getStartedLabel: '开始使用',
    heroCopy:
      '为 Rust 提供类型安全、高性能的 LLM 路由、协议桥接与预算感知多 Key 执行能力，仓库内同时附带 OmniLLM Skill。',
    heroSignalCopy:
      '规范化请求类型、感知损耗的转码与预算结算都保持在同一个运行时框架中，仓库还额外提供面向代码库协作的 OmniLLM Skill。',
    heroSignalKicker: 'AI 原生运行时',
    heroSignalTitle: '一个运行时 crate，再加一个 OmniLLM Skill。',
    heroStats: [
      { value: '04', label: '生成协议' },
      { value: '07', label: 'provider 家族' },
      { value: '01', label: '内置 OmniLLM Skill' }
    ],
    installHint: 'crate 安装 · 已包含 skill',
    installSkillLabel: '安装 Skill',
    metaDescription:
      '面向 LLM 路由、协议转码、OmniLLM Skill 工作流与预算感知多 Key 执行的 AI 原生 provider-neutral Rust 运行时。',
    readLabel: '阅读',
    showcaseBullets: [
      '文档会直接展示规范化请求与响应模型。',
      'Key 池、预算与回放工具都按仓库中的真实状态编写说明。',
      '实现说明会直接指回真正约束行为的源码模块。',
      '技能指南覆盖 Claude Code、Codex、OpenCode 与 Claude 的安装路径。'
    ],
    showcaseCopy:
      '使用说明、架构动机、实现走读与内置 OmniLLM Skill 都与 crate 同仓维护，让行为、设计、AI 指引与源码始终保持对齐。',
    showcaseKicker: '贴近源码的文档与 Skill',
    showcaseTitle: '从仓库上下文中运行运行时，并为 AI agent 完成接入。'
  }
};

const ROUTE_LABELS: Record<SiteLanguage, Record<string, string>> = {
  en: {
    '/architecture': 'system design',
    '/implementation': 'source walkthrough',
    '/skill': 'skill guide',
    '/usage': 'runtime guide',
    default: chromeText.en.documentationFallbackLabel
  },
  zh: {
    '/architecture': '架构说明',
    '/implementation': '实现走读',
    '/skill': '技能指南',
    '/usage': '运行指南',
    default: chromeText.zh.documentationFallbackLabel
  }
};
