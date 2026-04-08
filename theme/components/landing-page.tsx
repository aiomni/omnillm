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

const providers = [
  'OpenAI',
  'Anthropic',
  'Gemini',
  'Vertex AI',
  'Bedrock',
  'Azure OpenAI',
  'Compatible'
];

const metrics: Metric[] = [
  {
    value: '04',
    label: 'wire protocols online',
    copy:
      'Responses, Chat Completions, Claude Messages, and Gemini GenerateContent run through one runtime surface.'
  },
  {
    value: '07',
    label: 'provider families mapped',
    copy:
      'OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Bedrock, and compatible edges are already modeled.'
  },
  {
    value: '02',
    label: 'quota passes per request',
    copy:
      'Capacity is reserved before dispatch, then reconciled against actual usage once the call settles.'
  },
  {
    value: '00',
    label: 'hidden downgrade stories',
    copy:
      'Bridge and lossy conversion semantics stay explicit so downstream systems know exactly what changed.'
  }
];

const tracks: Tile[] = [
  {
    kicker: 'Dispatch',
    title: 'Push one canonical request shape through every provider edge.',
    copy:
      'Use GatewayBuilder, key pools, RPM and TPM controls, budget ceilings, and streaming events without per-vendor glue code.',
    href: './usage#building-a-gateway'
  },
  {
    kicker: 'Bridge',
    title: 'Transcode across protocols without smearing over loss or bridge semantics.',
    copy:
      'Conversions keep downgraded and bridged metadata visible so your app can decide what to trust and what to branch on.',
    href: './usage#protocol-parsing-and-emission'
  },
  {
    kicker: 'Replay',
    title: 'Capture fixtures that are safe to ship back into test and audit loops.',
    copy:
      'Replay sanitizers strip auth headers, nested secrets, query tokens, and bulky binary noise before data lands in your corpus.',
    href: './usage#replay-sanitization'
  }
];

const pipeline: Tile[] = [
  {
    kicker: 'Acquire',
    title: 'Random-start first-fit key selection keeps hot pools from collapsing.',
    copy:
      'Requests scan from a randomized entry point and reserve quota atomically so the same key is not hit by every caller.',
    href: './architecture#keypool-acquire-and-error-reporting'
  },
  {
    kicker: 'Reserve',
    title: 'Budget is pre-occupied before transport leaves the process.',
    copy:
      'Estimated spend is held up front, then refunded or topped up when the provider returns usage metadata.',
    href: './architecture#budgettracker-fixed-point-lock-free'
  },
  {
    kicker: 'Settle',
    title: 'Transport emission and post-call settlement stay on the same visible rail.',
    copy:
      'Canonical requests fan out to the right endpoint family, then reconcile quota and usage without hiding the end state.',
    href: './usage#multi-endpoint-api-layer'
  }
];

const docsCards: Tile[] = [
  {
    kicker: 'Guide',
    title: 'Usage Guide',
    copy:
      'Start with installation, endpoint configuration, runtime calls, streaming, protocol conversion, and operating patterns.',
    href: './usage'
  },
  {
    kicker: 'Architecture',
    title: 'Architecture Notes',
    copy:
      'Understand the concurrency model, lease lifecycle, key acquisition algorithm, and the cancellation-safe critical path.',
    href: './architecture'
  },
  {
    kicker: 'Implementation',
    title: 'Implementation Notes',
    copy:
      'Trace the crate layout, core data structures, error taxonomy, and gateway execution flow file by file.',
    href: './implementation'
  }
];

const feed = [
  {
    title: 'Runtime gateway',
    copy:
      'Canonical generation calls with multi-key balancing, circuit breaking, request timeouts, and provider dispatch.'
  },
  {
    title: 'Protocol trench tools',
    copy:
      'Parse, emit, and transcode raw payloads or typed endpoint requests without losing visibility into downgrade behavior.'
  },
  {
    title: 'Provider registry',
    copy:
      'Inspect built-in provider coverage for native, compatible, and planned endpoint families directly from the crate.'
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
          <span className="omn-home__eyebrow">OmniLLM runtime rail</span>
          <h1 className="omn-home__title">
            One fast lane for every model request.
          </h1>
          <p className="omn-home__lede">
            OmniLLM turns provider sprawl into a single Rust runtime: route,
            transcode, lease quota, balance keys, and settle spend without
            hiding what changed on the wire.
          </p>
          <div className="omn-home__actions">
            <ActionLink href="./usage" label="Open the docs" tone="primary" />
            <ActionLink
              href="./architecture"
              label="See the runtime path"
              tone="secondary"
            />
          </div>
        </div>

        <aside className="omn-home__signal">
          <div className="omn-home__signal-head">
            <span>Dispatch board</span>
            <span className="omn-home__signal-status">armed</span>
          </div>
          <h2 className="omn-home__signal-title">
            Acquire, emit, stream, settle.
          </h2>
          <p className="omn-home__signal-copy">
            The runtime keeps the path visible from key selection to usage
            reconciliation, so transport decisions and budget movement never
            disappear behind vendor clients.
          </p>
          <div className="omn-home__signal-rail">
            <div className="omn-home__rail-item">
              <span className="omn-home__rail-step">A1</span>
              <div className="omn-home__rail-copy">
                <strong>Acquire a healthy key</strong>
                <span>Randomized scans keep contention moving.</span>
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
                <span>Usage data closes the loop after completion.</span>
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

      <section className="omn-home__provider-strip" aria-label="Mapped providers">
        <span className="omn-home__provider-label">Mapped edges</span>
        <div className="omn-home__provider-marquee">
          <div className="omn-home__provider-track">
            {[...providers, ...providers].map((provider, index) => (
              <span className="omn-home__provider-chip" key={`${provider}-${index}`}>
                {provider}
              </span>
            ))}
          </div>
        </div>
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
            <span className="omn-home__section-tag">Control blocks</span>
            <h2>Route, bridge, and replay from the same hard-edged surface.</h2>
          </div>
          <p>
            The same runtime that ships requests also exposes the failure model:
            compatibility boundaries, bridge semantics, and replay-safe traces
            stay visible instead of being buried under SDK glue.
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
            <span className="omn-home__section-tag">Runtime lanes</span>
            <h2>The execution path stays in order from acquire to settle.</h2>
          </div>
          <p>
            Key leases, budget pre-occupation, transport emission, and usage
            settlement are not separate subsystems glued together later. They
            are one control loop, surfaced here as one readable rail.
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
            <span className="omn-home__section-tag">Surface area</span>
            <h2>Start broad, then drop straight into file-level material.</h2>
          </div>
          <p>
            The homepage acts like a control surface. Once you have the shape of
            the system, the guide, architecture notes, and implementation
            walkthrough take you deeper without changing tools or context.
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
            <h2>Three entry points. One in-repo documentation system.</h2>
          </div>
          <p>
            Markdown stays the source of truth. The public site now presents it
            with a darker product-led front door without forcing the crate docs
            to move into a CMS workflow.
          </p>
        </div>
        <div className="omn-home__docs">
          {docsCards.map(card => (
            <DocCard key={card.title} {...card} />
          ))}
        </div>
      </section>

      <section className="omn-home__cta">
        <span className="omn-home__section-tag">Ship from repo</span>
        <h2>Keep the runtime and the docs in the same lane.</h2>
        <p>
          The repository already includes the Pages workflow, the custom theme,
          and this new landing surface. Edit docs next to the Rust code, push,
          and publish without switching systems or losing the engineering
          context around the crate.
        </p>
        <div className="omn-home__cta-actions">
          <ActionLink
            href="./implementation"
            label="Inspect internals"
            tone="primary"
          />
          <ActionLink
            href="https://github.com/aiomni/omnillm"
            label="Inspect the repo"
            tone="secondary"
          />
        </div>
      </section>
    </div>
  );
}
