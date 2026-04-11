import { Link, useFrontmatter, usePage } from '@rspress/core/runtime';

import { getRouteLabel } from '../locale-data';
import { useSiteLocale } from '../use-locale';

type OmniDocFrontmatter = {
  description?: string;
  label?: string;
  release?: string;
  summary?: string;
  updated?: string;
};

const DEFAULT_RELEASE = 'v0.1.2';
const DEFAULT_UPDATED = 'Apr 2026';

function buildAnchor(id: string) {
  return `#${id}`;
}

export function DocIntro() {
  const { page } = usePage();
  const { frontmatter } = useFrontmatter();
  const { chrome, lang, localizePath, route, sourcePath } = useSiteLocale();
  const meta = frontmatter as OmniDocFrontmatter;
  const sectionLinks = page.toc.filter(item => item.depth === 2).slice(0, 3);
  const summary = meta.summary ?? meta.description ?? page.description ?? '';

  return (
    <section className="omn-doc-intro">
      <div className="omn-doc-intro__meta">
        <span className="omn-doc-chip omn-doc-chip--accent">
          {meta.release ?? DEFAULT_RELEASE}
        </span>
        <span className="omn-doc-chip">{meta.label ?? getRouteLabel(route, lang)}</span>
        <span className="omn-doc-chip">{meta.updated ?? DEFAULT_UPDATED}</span>
      </div>

      <div className="omn-doc-intro__grid">
        <article className="omn-doc-intro__card omn-doc-intro__card--primary">
          <span className="omn-doc-intro__kicker">{chrome.repositorySourceKicker}</span>
          <strong>{sourcePath}</strong>
          {summary ? <p>{summary}</p> : null}
        </article>

        <article className="omn-doc-intro__card">
          <span className="omn-doc-intro__kicker">{chrome.quickPathsKicker}</span>
          <div className="omn-doc-intro__links">
            {sectionLinks.length > 0 ? (
              sectionLinks.map(link => (
                <a href={buildAnchor(link.id)} key={link.id}>
                  {link.text}
                </a>
              ))
            ) : (
              <Link to={localizePath('/usage')}>{chrome.openDocIndexLabel}</Link>
            )}
          </div>
        </article>
      </div>
    </section>
  );
}

export function DocsSidebarIntro() {
  const { chrome, lang, localizePath, route } = useSiteLocale();

  return (
    <div className="omn-side-panel">
      <span className="omn-side-panel__kicker">{chrome.installPanelKicker}</span>
      <code>cargo add omnillm</code>
      <p>
        {getRouteLabel(route, lang)}{' '}
        {lang === 'zh'
          ? '配有贴近源码的运行时行为、配额限制与 provider 桥接说明。'
          : 'with source-adjacent notes for runtime behavior, quotas, and provider bridges.'}
      </p>
      <Link to={localizePath('/usage#installation')}>{chrome.installPanelLink}</Link>
    </div>
  );
}

export function DocsOutlineCta() {
  const { chrome, localizePath } = useSiteLocale();

  return (
    <div className="omn-outline-panel">
      <span className="omn-outline-panel__kicker">{chrome.needInternalsKicker}</span>
      <p>{chrome.needInternalsText}</p>
      <div className="omn-outline-panel__actions">
        <Link to={localizePath('/implementation')}>{chrome.needInternalsLink}</Link>
        <a href="https://github.com/aiomni/omnillm" rel="noreferrer" target="_blank">
          {chrome.githubLabel}
        </a>
      </div>
    </div>
  );
}
