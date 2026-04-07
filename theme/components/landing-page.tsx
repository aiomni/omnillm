type Metric = {
  value: string;
  label: string;
  copy: string;
};

type Tile = {
  kicker: string;
  title: string;
  copy: string;
  href: string;
};

const metrics: Metric[] = [
  {
    value: '4',
    label: 'wire protocols online',
    copy: 'OpenAI Responses, Chat Completions, Claude Messages, and Gemini GenerateContent.'
  },
  {
    value: '6',
    label: 'canonical endpoint families',
    copy: 'Generation, embeddings, image, audio speech, audio transcription, and rerank.'
  },
  {
    value: '7',
    label: 'provider families mapped',
    copy: 'OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Bedrock, and compatible endpoints.'
  },
  {
    value: '2',
    label: 'safety loops per request',
    copy: 'Quota is pre-reserved before dispatch, then settled to actual usage after completion.'
  }
];

const tracks: Tile[] = [
  {
    kicker: 'Run',
    title: 'Push one canonical request model through every provider edge.',
    copy:
      'Use GatewayBuilder, key pools, RPM and TPM controls, budget limits, and streaming events without hand-writing per-vendor call paths.',
    href: './usage#building-a-gateway'
  },
  {
    kicker: 'Bridge',
    title: 'Transcode across protocols without hiding loss or bridge semantics.',
    copy:
      'Protocol and multi-endpoint conversions keep bridged and lossy metadata explicit so downstream systems know exactly what changed.',
    href: './usage#protocol-parsing-and-emission'
  },
  {
    kicker: 'Audit',
    title: 'Capture replay fixtures without leaking secrets or binary noise.',
    copy:
      'The replay sanitizers redact auth headers, query tokens, nested secrets, and large binary payloads before they reach your test corpus.',
    href: './usage#replay-sanitization'
  }
];

const pipeline: Tile[] = [
  {
    kicker: '01 Acquire',
    title: 'Random-start first-fit key selection',
    copy:
      'Requests scan from a random entry point and reserve quota atomically so hot pools do not collapse onto the same least-loaded key.',
    href: './architecture#keypool-acquire-and-error-reporting'
  },
  {
    kicker: '02 Reserve',
    title: 'Budget pre-occupation before transport',
    copy:
      'Estimated cost is reserved before a request leaves the process, then refunded or topped up once usage metadata lands.',
    href: './architecture#budgettracker-fixed-point-lock-free'
  },
  {
    kicker: '03 Dispatch',
    title: 'Protocol-aware transport emission',
    copy:
      'Canonical requests fan out to the correct wire format and endpoint family without pushing vendor-specific conditionals into application code.',
    href: './usage#multi-endpoint-api-layer'
  }
];

const docsCards: Tile[] = [
  {
    kicker: 'Guide',
    title: 'Usage Guide',
    copy:
      'Start here for installation, endpoint configuration, runtime calls, streaming, protocol conversion, and practical operating patterns.',
    href: './usage'
  },
  {
    kicker: 'Internals',
    title: 'Architecture Notes',
    copy:
      'Understand the concurrency model, quota lease lifecycle, key pool acquisition algorithm, and why the critical path is cancellation-safe.',
    href: './architecture'
  },
  {
    kicker: 'Source Walkthrough',
    title: 'Implementation Notes',
    copy:
      'Trace the crate layout, core data structures, error taxonomy, and the gateway execution path file by file.',
    href: './implementation'
  }
];

const feed = [
  {
    title: 'Runtime gateway',
    copy: 'Canonical generation calls with multi-key balancing, per-key limits, circuit breaking, and request timeouts.'
  },
  {
    title: 'Protocol trench tools',
    copy: 'Parse, emit, and transcode raw payloads or typed endpoint requests without losing visibility into downgrade behavior.'
  },
  {
    title: 'Provider registry',
    copy: 'Inspect built-in provider coverage for native, compatible, and planned endpoint families directly in the crate.'
  }
];

function ActionLink(props: {
  href: string;
  label: string;
  tone: 'primary' | 'secondary';
}) {
  const isExternal = props.href.startsWith('http');

  return (
    <a
      className={`omn-home__action omn-home__action--${props.tone}`}
      href={props.href}
      target={isExternal ? '_blank' : undefined}
      rel={isExternal ? 'noreferrer' : undefined}
    >
      {props.label}
    </a>
  );
}

function LinkCard({ kicker, title, copy, href }: Tile) {
  return (
    <a className="omn-home__tile" href={href}>
      <span className="omn-home__tile-kicker">{kicker}</span>
      <h3>{title}</h3>
      <p>{copy}</p>
      <span className="omn-home__tile-link">Open track</span>
    </a>
  );
}

function DocCard({ kicker, title, copy, href }: Tile) {
  return (
    <a className="omn-home__doc-card" href={href}>
      <span className="omn-home__doc-card-kicker">{kicker}</span>
      <h3>{title}</h3>
      <p>{copy}</p>
      <span className="omn-home__doc-card-link">Read now</span>
    </a>
  );
}

export function LandingPage() {
  return (
    <div className="omn-home">
      <section className="omn-home__hero">
        <div className="omn-home__hero-copy">
          <span className="omn-home__eyebrow">OmniLLM control plane</span>
          <h1 className="omn-home__title">
            Route every model call through one hard-edged Rust runtime.
          </h1>
          <p className="omn-home__lede">
            OmniLLM gives you a provider-neutral request surface, multi-key load
            balancing, protocol bridging, circuit breaking, and budget-aware
            execution in one crate. The site is tuned like a launch console:
            bold overview up front, deep docs once you cross the threshold.
          </p>
          <div className="omn-home__actions">
            <ActionLink href="./usage" label="Enter the docs" tone="primary" />
            <ActionLink
              href="https://github.com/aiomni/omnillm"
              label="Inspect the repo"
              tone="secondary"
            />
          </div>
        </div>

        <aside className="omn-home__signal">
          <div className="omn-home__signal-head">
            <span>Signal rail</span>
            <span className="omn-home__signal-status">greenline</span>
          </div>
          <h2 className="omn-home__signal-title">
            One request path. Multiple providers. No hidden downgrade story.
          </h2>
          <p className="omn-home__signal-copy">
            The runtime acquires quota, reserves budget, emits transport,
            streams or settles usage, then releases capacity through RAII.
          </p>
          <div className="omn-home__signal-rail">
            <div className="omn-home__rail-item">
              <span className="omn-home__rail-step">A1</span>
              <div className="omn-home__rail-copy">
                <strong>Acquire a healthy key</strong>
                <span>Randomized selection keeps contention moving.</span>
              </div>
            </div>
            <div className="omn-home__rail-item">
              <span className="omn-home__rail-step">B2</span>
              <div className="omn-home__rail-copy">
                <strong>Reserve quota and budget</strong>
                <span>Pre-flight accounting happens before transport.</span>
              </div>
            </div>
            <div className="omn-home__rail-item">
              <span className="omn-home__rail-step">C3</span>
              <div className="omn-home__rail-copy">
                <strong>Dispatch and settle</strong>
                <span>Actual usage reconciles state after completion.</span>
              </div>
            </div>
          </div>
          <div className="omn-home__protocols">
            <span className="omn-home__protocol">Responses</span>
            <span className="omn-home__protocol">Chat</span>
            <span className="omn-home__protocol">Claude</span>
            <span className="omn-home__protocol">Gemini</span>
          </div>
        </aside>
      </section>

      <section className="omn-home__metrics" aria-label="Project metrics">
        {metrics.map(metric => (
          <article className="omn-home__metric" key={metric.label}>
            <strong className="omn-home__metric-value">{metric.value}</strong>
            <span className="omn-home__metric-label">{metric.label}</span>
            <p className="omn-home__metric-copy">{metric.copy}</p>
          </article>
        ))}
      </section>

      <section className="omn-home__section">
        <div className="omn-home__section-head">
          <div>
            <span className="omn-home__section-tag">Mission tracks</span>
            <h2>Operate, bridge, and audit from the same surface.</h2>
          </div>
          <p>
            The front page borrows from product launch sites rather than default
            doc portals. The tone is fast, sharp, and directional, but every
            tile leads straight into concrete technical material.
          </p>
        </div>
        <div className="omn-home__tracks">
          {tracks.map(track => (
            <LinkCard key={track.title} {...track} />
          ))}
        </div>
      </section>

      <section className="omn-home__section">
        <div className="omn-home__section-head">
          <div>
            <span className="omn-home__section-tag">Control loop</span>
            <h2>The execution path stays explicit from acquire to settle.</h2>
          </div>
          <p>
            The same critical path that powers the crate is surfaced here as a
            visible runway. It keeps the site grounded in the engineering model
            instead of collapsing into generic marketing copy.
          </p>
        </div>
        <div className="omn-home__tracks">
          {pipeline.map(item => (
            <LinkCard key={item.title} {...item} />
          ))}
        </div>
      </section>

      <section className="omn-home__section">
        <div className="omn-home__section-head">
          <div>
            <span className="omn-home__section-tag">System feed</span>
            <h2>Pick your altitude, then dive into the file-level detail.</h2>
          </div>
          <p>
            Use the launchpad as an operator view, then pivot into the guide,
            architecture notes, or implementation walkthrough depending on how
            close to the metal you need to get.
          </p>
        </div>
        <div className="omn-home__feed">
          <div className="omn-home__feed-head">
            <span>Runtime channels</span>
            <span>always-on</span>
          </div>
          {feed.map(item => (
            <div className="omn-home__feed-item" key={item.title}>
              <strong>{item.title}</strong>
              <span>{item.copy}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="omn-home__section">
        <div className="omn-home__section-head">
          <div>
            <span className="omn-home__section-tag">Docs runway</span>
            <h2>Three entry points. One documentation system.</h2>
          </div>
          <p>
            Existing Markdown stays the source of truth. Rspress turns it into
            the public docs surface without forcing the Rust-focused docs to be
            rewritten into a CMS format.
          </p>
        </div>
        <div className="omn-home__docs">
          {docsCards.map(card => (
            <DocCard key={card.title} {...card} />
          ))}
        </div>
      </section>

      <section className="omn-home__cta">
        <span className="omn-home__section-tag">Ship it</span>
        <h2>Deploy the site to GitHub Pages and keep the docs in-repo.</h2>
        <p>
          The repository now includes a Pages workflow, a custom Rspress theme,
          and a launch-style homepage tuned for OmniLLM instead of a generic
          documentation template. Edit Markdown, push to main, and the public
          site can update from the same source tree as the crate.
        </p>
        <div className="omn-home__cta-actions">
          <ActionLink href="./implementation" label="Inspect internals" tone="primary" />
          <ActionLink href="./usage#examples-included-in-this-repository" label="Run examples" tone="secondary" />
        </div>
      </section>
    </div>
  );
}
