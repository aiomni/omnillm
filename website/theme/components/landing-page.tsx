import type { CSSProperties, ReactNode } from 'react';
import { Link } from '@rspress/core/runtime';

import { homeContent } from '../locale-data';
import { useSiteLocale } from '../use-locale';

type CodeLine = {
  line: string;
  content: ReactNode;
};

const codeIndent = '    ';
const codeChainIndent = `${codeIndent}${codeIndent}`;

const codeLines: CodeLine[] = [
  {
    line: "1",
    content: (
      <>
        <span className="omn-code__keyword">use</span>{" "}
        <span className="omn-code__symbol">omnillm</span>
        <span className="omn-code__plain">::</span>
        <span className="omn-code__plain">
          {"{"}GatewayBuilder, KeyConfig, LlmRequest, Message,
        </span>
      </>
    ),
  },
  {
    line: "2",
    content: (
      <>
        <span className="omn-code__plain">
          {codeIndent}
          MessageRole, ProviderEndpoint, RequestItem{"}"};
        </span>
      </>
    ),
  },
  {
    line: "3",
    content: (
      <>
        <span className="omn-code__keyword">use</span>{" "}
        <span className="omn-code__symbol">tokio_util</span>
        <span className="omn-code__plain">::sync::CancellationToken;</span>
      </>
    ),
  },
  {
    line: "4",
    content: <span className="omn-code__plain" />,
  },
  {
    line: "5",
    content: (
      <>
        <span className="omn-code__attribute">#[tokio::main]</span>
      </>
    ),
  },
  {
    line: "6",
    content: (
      <>
        <span className="omn-code__keyword">async fn</span>{" "}
        <span className="omn-code__function">main</span>
        <span className="omn-code__plain">
          () -&gt; Result&lt;(), Box&lt;dyn std::error::Error&gt;&gt; {"{"}
        </span>
      </>
    ),
  },
  {
    line: "7",
    content: (
      <>
        <span className="omn-code__plain">{codeIndent}</span>
        <span className="omn-code__keyword">let</span>{" "}
        <span className="omn-code__plain">gateway = GatewayBuilder::new(</span>
        <span className="omn-code__symbol">ProviderEndpoint</span>
        <span className="omn-code__plain">::openai_responses())</span>
      </>
    ),
  },
  {
    line: "8",
    content: (
      <>
        <span className="omn-code__plain">{codeChainIndent}.add_key(KeyConfig::new(</span>
        <span className="omn-code__string">"sk-key-1"</span>
        <span className="omn-code__plain">, </span>
        <span className="omn-code__string">"prod-1"</span>
        <span className="omn-code__plain">))</span>
      </>
    ),
  },
  {
    line: "9",
    content: (
      <>
        <span className="omn-code__plain">{codeChainIndent}.build()?;</span>
      </>
    ),
  },
  {
    line: "10",
    content: (
      <>
        <span className="omn-code__plain">{codeIndent}</span>
        <span className="omn-code__keyword">let</span>{" "}
        <span className="omn-code__plain">
          request = LlmRequest::from(Message::text(
        </span>
        <span className="omn-code__symbol">MessageRole</span>
        <span className="omn-code__plain">::User, </span>
        <span className="omn-code__string">
          "Explain Rust ownership in one sentence."
        </span>
        <span className="omn-code__plain">));</span>
      </>
    ),
  },
  {
    line: "11",
    content: (
      <>
        <span className="omn-code__plain">{codeIndent}</span>
        <span className="omn-code__keyword">let</span>{" "}
        <span className="omn-code__plain">
          response = gateway.call(request, CancellationToken::new()).await?;
        </span>
      </>
    ),
  },
  {
    line: "12",
    content: (
      <>
        <span className="omn-code__plain">{codeIndent}</span>
        <span className="omn-code__macro">println!</span>
        <span className="omn-code__plain">(</span>
        <span className="omn-code__string">"{}"</span>
        <span className="omn-code__plain">, response.content_text);</span>
      </>
    ),
  },
  {
    line: "13",
    content: (
      <>
        <span className="omn-code__plain">{codeIndent}Ok(())</span>
      </>
    ),
  },
  {
    line: "14",
    content: (
      <>
        <span className="omn-code__plain">{"}"}</span>
      </>
    ),
  },
];

function ActionLink(props: {
  label: string;
  href: string;
  tone: 'primary' | 'secondary';
}) {
  const { localizePath } = useSiteLocale();
  const isExternal = props.href.startsWith('http');
  const className = `omn-home__action omn-home__action--${props.tone}`;

  if (isExternal) {
    return (
      <a
        className={className}
        href={props.href}
        rel="noreferrer"
        target="_blank"
      >
        <span>{props.label}</span>
        {props.tone === "primary" ? <span aria-hidden="true">→</span> : null}
      </a>
    );
  }

  return (
    <Link className={className} to={localizePath(props.href)}>
      <span>{props.label}</span>
      {props.tone === "primary" ? <span aria-hidden="true">→</span> : null}
    </Link>
  );
}

function FeatureTile(card: (typeof homeContent)['en']['featureCards'][number]) {
  const style = {
    '--omn-feature-accent': card.accent
  } as CSSProperties;

  return (
    <article className="omn-home__feature-card" style={style}>
      <span className="omn-home__feature-eyebrow">{card.eyebrow}</span>
      <h3>{card.title}</h3>
      <p>{card.copy}</p>
      <span className="omn-home__feature-detail">{card.detail}</span>
    </article>
  );
}

function DocTile(entry: (typeof homeContent)['en']['docCards'][number]) {
  const { localizePath, home } = useSiteLocale();

  return (
    <Link className="omn-home__doc-card" to={localizePath(entry.href)}>
      <span className="omn-home__doc-label">{entry.label}</span>
      <h3>{entry.title}</h3>
      <p>{entry.copy}</p>
      <span className="omn-home__doc-link">
        {home.readLabel} <span aria-hidden="true">→</span>
      </span>
    </Link>
  );
}

export function LandingPage() {
  const { home } = useSiteLocale();

  return (
    <div className="omn-home">
      <section className="omn-home__hero">
        <div className="omn-home__intro">
          <div className="omn-home__badge-row">
            <span className="omn-home__badge omn-home__badge--accent">
              v0.1.0
            </span>
            <span className="omn-home__badge">{home.badgeProviderNeutral}</span>
            <span className="omn-home__badge">{home.badgeSkill}</span>
          </div>
          <h1>OmniLLM</h1>
          <p>{home.heroCopy}</p>
          <div className="omn-home__actions">
            <ActionLink href="/usage" label={home.getStartedLabel} tone="primary" />
            <ActionLink href="/skill" label={home.installSkillLabel} tone="secondary" />
            <ActionLink
              href="https://github.com/aiomni/omnillm"
              label={home.browseSourceLabel}
              tone="secondary"
            />
          </div>
          <div className="omn-home__install">
            <code>$ cargo add omnillm</code>
            <span>{home.installHint}</span>
          </div>
        </div>

        <aside className="omn-home__hero-side">
          <div className="omn-home__signal">
            <span className="omn-home__section-kicker">{home.heroSignalKicker}</span>
            <h2>{home.heroSignalTitle}</h2>
            <p>{home.heroSignalCopy}</p>
          </div>
          <div className="omn-home__metric-grid">
            {home.heroStats.map(item => (
              <article className="omn-home__metric-card" key={item.label}>
                <strong>{item.value}</strong>
                <span>{item.label}</span>
              </article>
            ))}
          </div>
        </aside>
      </section>

      <section className="omn-home__feature-grid">
        {home.featureCards.map(card => (
          <FeatureTile key={card.title} {...card} />
        ))}
      </section>

      <section className="omn-home__showcase">
        <div className="omn-home__showcase-copy">
          <span className="omn-home__section-kicker">{home.showcaseKicker}</span>
          <h2>{home.showcaseTitle}</h2>
          <p>{home.showcaseCopy}</p>
          <ul className="omn-home__bullet-list">
            {home.showcaseBullets.map(item => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>

        <div className="omn-home__code-frame">
          <div className="omn-home__code-head">
            <div className="omn-home__code-dots" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <span>{home.codeFileLabel}</span>
          </div>
          <div className="omn-home__code-body">
            {codeLines.map(line => (
              <div className="omn-home__code-line" key={line.line}>
                <span className="omn-home__code-number">{line.line}</span>
                <code>{line.content}</code>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="omn-home__docs">
        <div className="omn-home__docs-head">
          <span className="omn-home__section-kicker">{home.docsKicker}</span>
          <h2>{home.docsTitle}</h2>
          <p>{home.docsDescription}</p>
        </div>
        <div className="omn-home__docs-grid">
          {home.docCards.map(entry => (
            <DocTile key={entry.title} {...entry} />
          ))}
        </div>
      </section>

      <section className="omn-home__bottom-strip">
        {home.bottomStrip.map(entry => (
          <article key={entry.title}>
            <span className="omn-home__strip-label">{entry.label}</span>
            <strong>{entry.title}</strong>
            <p>{entry.copy}</p>
          </article>
        ))}
      </section>
    </div>
  );
}
