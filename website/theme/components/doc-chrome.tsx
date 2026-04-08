import { Link, useFrontmatter, usePage } from '@rspress/core/runtime';

type OmniDocFrontmatter = {
  description?: string;
  label?: string;
  release?: string;
  summary?: string;
  updated?: string;
};

const DEFAULT_RELEASE = 'v0.1.0';
const DEFAULT_UPDATED = 'Apr 2026';

function routeLabel(routePath: string) {
  if (routePath === '/usage') {
    return 'runtime guide';
  }

  if (routePath === '/architecture') {
    return 'system design';
  }

  if (routePath === '/implementation') {
    return 'source walkthrough';
  }

  return 'documentation';
}

function buildAnchor(id: string) {
  return `#${id}`;
}

export function DocIntro() {
  const { page } = usePage();
  const { frontmatter } = useFrontmatter();
  const meta = frontmatter as OmniDocFrontmatter;
  const sectionLinks = page.toc.filter(item => item.depth === 2).slice(0, 3);
  const summary = meta.summary ?? meta.description ?? page.description ?? '';
  const relativePath =
    typeof page._relativePath === 'string' ? page._relativePath : 'website/docs';

  return (
    <section className="omn-doc-intro">
      <div className="omn-doc-intro__meta">
        <span className="omn-doc-chip omn-doc-chip--accent">
          {meta.release ?? DEFAULT_RELEASE}
        </span>
        <span className="omn-doc-chip">{meta.label ?? routeLabel(page.routePath)}</span>
        <span className="omn-doc-chip">{meta.updated ?? DEFAULT_UPDATED}</span>
      </div>

      <div className="omn-doc-intro__grid">
        <article className="omn-doc-intro__card omn-doc-intro__card--primary">
          <span className="omn-doc-intro__kicker">Repository Source</span>
          <strong>{relativePath}</strong>
          {summary ? <p>{summary}</p> : null}
        </article>

        <article className="omn-doc-intro__card">
          <span className="omn-doc-intro__kicker">Quick Paths</span>
          <div className="omn-doc-intro__links">
            {sectionLinks.length > 0 ? (
              sectionLinks.map(link => (
                <a href={buildAnchor(link.id)} key={link.id}>
                  {link.text}
                </a>
              ))
            ) : (
              <Link to="/usage">Open documentation index</Link>
            )}
          </div>
        </article>
      </div>
    </section>
  );
}

export function DocsSidebarIntro() {
  const { page } = usePage();

  return (
    <div className="omn-side-panel">
      <span className="omn-side-panel__kicker">crate install</span>
      <code>cargo add omnillm</code>
      <p>
        {routeLabel(page.routePath)} with source-adjacent notes for runtime
        behavior, quotas, and provider bridges.
      </p>
      <Link to="/usage#installation">Install the crate</Link>
    </div>
  );
}

export function DocsOutlineCta() {
  return (
    <div className="omn-outline-panel">
      <span className="omn-outline-panel__kicker">Need internals?</span>
      <p>
        Cross-check the behavior with implementation notes or inspect the
        repository directly.
      </p>
      <div className="omn-outline-panel__actions">
        <Link to="/implementation">Implementation</Link>
        <a href="https://github.com/aiomni/omnillm" rel="noreferrer" target="_blank">
          GitHub
        </a>
      </div>
    </div>
  );
}
