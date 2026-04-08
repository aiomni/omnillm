import './index.css';

import { preloadLink } from '@rspress/core/runtime';
import { useEffect } from 'react';
import { Layout as BasicLayout } from '@rspress/core/theme-original';
import {
  DocIntro,
  DocsOutlineCta,
  DocsSidebarIntro
} from './components/doc-chrome';
import {
  NavControls,
  SiteFooter,
  SiteNavTitle
} from './components/site-chrome';
import { localizePath, SUPPORTED_LANGUAGES } from './locale-data';
import { shouldRedirectHomeToPreferredLanguage } from './use-locale';

const PRELOAD_ROUTES = SUPPORTED_LANGUAGES.flatMap(lang => [
  localizePath('/', lang),
  localizePath('/usage', lang),
  localizePath('/skill', lang),
  localizePath('/architecture', lang),
  localizePath('/implementation', lang)
]);
let didWarmRoutes = false;

function normalizePreloadPath(path: string) {
  if (!path || path === '/') {
    return '/';
  }

  return path.replace(/\/+$/, '') || '/';
}

function warmRoutes(currentPath: string) {
  if (
    didWarmRoutes ||
    typeof window === 'undefined' ||
    process.env.NODE_ENV !== 'production'
  ) {
    return;
  }

  didWarmRoutes = true;
  const normalizedCurrentPath = normalizePreloadPath(currentPath);

  for (const route of PRELOAD_ROUTES) {
    if (normalizePreloadPath(route) === normalizedCurrentPath) {
      continue;
    }

    preloadLink(route);
  }
}

function HomeLanguageRedirect() {
  useEffect(() => {
    const target = shouldRedirectHomeToPreferredLanguage(window.location.pathname);

    if (!target || target === window.location.pathname) {
      return;
    }

    window.location.replace(target);
  }, []);

  return null;
}

function RoutePreloader() {
  useEffect(() => {
    const timer = window.setTimeout(() => {
      warmRoutes(window.location.pathname);
    }, 0);

    return () => window.clearTimeout(timer);
  }, []);

  return null;
}

const Layout = () => {
  return (
    <>
      <HomeLanguageRedirect />
      <RoutePreloader />
      <BasicLayout
        navTitle={<SiteNavTitle />}
        beforeNavMenu={<NavControls />}
        beforeDocContent={<DocIntro />}
        beforeSidebar={<DocsSidebarIntro />}
        afterOutline={<DocsOutlineCta />}
        bottom={<SiteFooter />}
      />
    </>
  );
};

export { Layout };
export * from '@rspress/core/theme-original';
