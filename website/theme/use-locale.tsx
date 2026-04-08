import { usePage } from '@rspress/core/runtime';
import { useMemo } from 'react';

import {
  DEFAULT_LANGUAGE,
  LANGUAGE_STORAGE_KEY,
  SiteLanguage,
  canonicalRouteFromPath,
  canonicalRouteFromRelativePath,
  chromeText,
  homeContent,
  inferLanguageFromPathname,
  localizePath,
  pageSourcePath
} from './locale-data';

function resolveLanguage(raw?: string): SiteLanguage {
  return raw === 'zh' ? 'zh' : DEFAULT_LANGUAGE;
}

export function readLanguagePreference() {
  if (typeof window === 'undefined') {
    return null;
  }

  const value = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
  return value === 'zh' ? 'zh' : value === 'en' ? 'en' : null;
}

export function rememberLanguagePreference(lang: SiteLanguage) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(LANGUAGE_STORAGE_KEY, lang);
}

export function useSiteLocale() {
  const { page } = usePage();
  const lang = resolveLanguage(page.lang);
  const route = useMemo(() => {
    const relativePath =
      typeof page._relativePath === 'string' ? page._relativePath : undefined;

    if (relativePath) {
      return canonicalRouteFromRelativePath(relativePath);
    }

    const routePath =
      typeof page.routePath === 'string' ? page.routePath : '/';

    return canonicalRouteFromPath(routePath);
  }, [page._relativePath, page.routePath]);

  return {
    chrome: chromeText[lang],
    home: homeContent[lang],
    lang,
    localizePath: (path: string) => localizePath(path, lang),
    route,
    sourcePath: pageSourcePath(
      typeof page._relativePath === 'string' ? page._relativePath : undefined
    ),
    switchPath: (targetLang: SiteLanguage) => localizePath(route, targetLang)
  };
}

export function shouldRedirectHomeToPreferredLanguage(pathname: string) {
  if (typeof window === 'undefined') {
    return null;
  }

  const preferredLanguage = readLanguagePreference();

  if (!preferredLanguage || preferredLanguage === DEFAULT_LANGUAGE) {
    return null;
  }

  if (inferLanguageFromPathname(pathname) !== DEFAULT_LANGUAGE) {
    return null;
  }

  if (canonicalRouteFromPath(pathname) !== '/') {
    return null;
  }

  return localizePath('/', preferredLanguage);
}
