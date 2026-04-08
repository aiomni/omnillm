import { Link } from '@rspress/core/runtime';

import { getFooterLinks, localeLabels } from '../locale-data';
import { rememberLanguagePreference, useSiteLocale } from '../use-locale';

export function SiteNavTitle() {
  const { localizePath } = useSiteLocale();

  return (
    <Link className="omn-nav-brand" to={localizePath('/')}>
      <span className="omn-nav-brand__mark" aria-hidden="true" />
      <span>OMNILLM</span>
    </Link>
  );
}

function LanguageSwitcher() {
  const { chrome, lang, switchPath } = useSiteLocale();

  return (
    <div
      aria-label={chrome.languageLabel}
      className="omn-lang-switcher"
      role="group"
    >
      {(['en', 'zh'] as const).map(targetLang => {
        const label = localeLabels[targetLang];

        if (targetLang === lang) {
          return (
            <span
              aria-current="true"
              className="omn-lang-switcher__item omn-lang-switcher__item--active"
              key={targetLang}
            >
              {label}
            </span>
          );
        }

        return (
          <Link
            className="omn-lang-switcher__item"
            key={targetLang}
            onClick={() => rememberLanguagePreference(targetLang)}
            to={switchPath(targetLang)}
          >
            {label}
          </Link>
        );
      })}
    </div>
  );
}

export function NavControls() {
  const { chrome, localizePath } = useSiteLocale();

  return (
    <div className="omn-nav-controls">
      <Link className="omn-nav-install" to={localizePath('/usage#installation')}>
        {chrome.installCta}
      </Link>
      <LanguageSwitcher />
    </div>
  );
}

export function SiteFooter() {
  const { chrome, lang, localizePath } = useSiteLocale();
  const footerLinks = getFooterLinks(lang);

  return (
    <footer className="omn-site-footer">
      <div className="omn-site-footer__inner">
        <div className="omn-site-footer__lead">
          <Link className="omn-site-footer__brand" to={localizePath('/')}>
            OMNILLM
          </Link>
          <p>{chrome.footerLead}</p>
        </div>

        <nav className="omn-site-footer__nav" aria-label={chrome.footerAriaLabel}>
          {footerLinks.map(link => (
            <Link key={link.href} to={link.href}>
              {link.label}
            </Link>
          ))}
          <a href="https://github.com/aiomni/omnillm" rel="noreferrer" target="_blank">
            {chrome.githubLabel}
          </a>
        </nav>
      </div>
    </footer>
  );
}
