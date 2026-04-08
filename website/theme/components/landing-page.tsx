import type { CSSProperties, ReactNode } from "react";
import { Link } from "@rspress/core/runtime";

type HeroStat = {
  label: string;
  value: string;
};

type FeatureCard = {
  eyebrow: string;
  title: string;
  copy: string;
  detail: string;
  accent: string;
};

type DocCard = {
  label: string;
  title: string;
  copy: string;
  href: string;
};

type CodeLine = {
  line: string;
  content: ReactNode;
};

const heroStats: HeroStat[] = [
  { value: "04", label: "generation protocols" },
  { value: "07", label: "provider families" },
  { value: "01", label: "bundled Claude Skill" },
];

const featureCards: FeatureCard[] = [
  {
    eyebrow: "Gateway",
    title: "Route canonical requests through one runtime surface.",
    copy: "Gateway dispatch keeps request typing, key pooling, timeouts, and usage accounting in the same execution path.",
    detail: "responses · chat · streams",
    accent: "#0969da",
  },
  {
    eyebrow: "Bridge",
    title: "Transcode across APIs without hiding downgrade behavior.",
    copy: "Loss reports stay explicit so application logic can branch on bridged or dropped fields instead of guessing.",
    detail: "typed conversion · loss metadata",
    accent: "#ffb4a6",
  },
  {
    eyebrow: "Operate",
    title: "Keep quota, replay, and provider state visible.",
    copy: "Per-key limits, budget reservation, fixture sanitization, registry metadata, and the bundled Claude Skill all live next to the crate.",
    detail: "budgets · replay · skill",
    accent: "#7fe0ff",
  },
];

const docCards: DocCard[] = [
  {
    label: "Guide",
    title: "Usage Guide",
    copy: "Install the crate, configure endpoints, send requests, stream results, and operate the runtime in production-shaped flows.",
    href: "/usage",
  },
  {
    label: "Design",
    title: "Architecture Notes",
    copy: "Read the lease lifecycle, key-pool acquisition strategy, and budget tracker model before diving into source.",
    href: "/architecture",
  },
  {
    label: "Source",
    title: "Implementation Notes",
    copy: "Walk the crate module by module when you want concrete execution paths, data structures, and internal boundaries.",
    href: "/implementation",
  },
];

const codeIndent = "    ";
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
  href: string;
  label: string;
  tone: "primary" | "secondary";
}) {
  const isExternal = props.href.startsWith("http");
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
    <Link className={className} to={props.href}>
      <span>{props.label}</span>
      {props.tone === "primary" ? <span aria-hidden="true">→</span> : null}
    </Link>
  );
}

function FeatureTile(card: FeatureCard) {
  const style = {
    "--omn-feature-accent": card.accent,
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

function DocTile(entry: DocCard) {
  return (
    <Link className="omn-home__doc-card" to={entry.href}>
      <span className="omn-home__doc-label">{entry.label}</span>
      <h3>{entry.title}</h3>
      <p>{entry.copy}</p>
      <span className="omn-home__doc-link">
        Read <span aria-hidden="true">→</span>
      </span>
    </Link>
  );
}

export function LandingPage() {
  return (
    <div className="omn-home">
      <section className="omn-home__hero">
        <div className="omn-home__intro">
          <div className="omn-home__badge-row">
            <span className="omn-home__badge omn-home__badge--accent">
              v0.1.0
            </span>
            <span className="omn-home__badge">
              provider-neutral Rust runtime
            </span>
            <span className="omn-home__badge">AI-native · Claude Skill included</span>
          </div>
          <h1>OmniLLM</h1>
          <p>
            Type-safe, high-performance LLM routing, protocol bridging, and
            budget-aware multi-key execution for Rust, with a bundled Claude
            Skill for repo-native AI workflows.
          </p>
          <div className="omn-home__actions">
            <ActionLink href="/usage" label="Get Started" tone="primary" />
            <ActionLink
              href="/usage#ai-native-skill"
              label="Install Skill"
              tone="secondary"
            />
            <ActionLink
              href="https://github.com/aiomni/omnillm"
              label="Browse Source"
              tone="secondary"
            />
          </div>
          <div className="omn-home__install">
            <code>$ cargo add omnillm</code>
            <span>crate install · skill included</span>
          </div>
        </div>

        <aside className="omn-home__hero-side">
          <div className="omn-home__signal">
            <span className="omn-home__section-kicker">AI-Native Runtime</span>
            <h2>
              One crate and one bundled Skill for routing and quota control.
            </h2>
            <p>
              Canonical request types, loss-aware transcoding, and budget
              settlement stay in one operational frame, and the repository ships
              a Claude Skill that teaches agents where OmniLLM runtime behavior
              ends and typed conversion helpers begin.
            </p>
          </div>
          <div className="omn-home__metric-grid">
            {heroStats.map((item) => (
              <article className="omn-home__metric-card" key={item.label}>
                <strong>{item.value}</strong>
                <span>{item.label}</span>
              </article>
            ))}
          </div>
        </aside>
      </section>

      <section className="omn-home__feature-grid">
        {featureCards.map((card) => (
          <FeatureTile key={card.title} {...card} />
        ))}
      </section>

      <section className="omn-home__showcase">
        <div className="omn-home__showcase-copy">
          <span className="omn-home__section-kicker">Source-Adjacent Docs And Skill</span>
          <h2>Operate the runtime and onboard AI agents from repository context.</h2>
          <p>
            Usage notes, architecture rationale, implementation walkthroughs,
            and the bundled Claude Skill live beside the crate so behavior,
            design, AI guidance, and source stay aligned.
          </p>
          <ul className="omn-home__bullet-list">
            <li>
              Canonical request and response models stay visible in the docs.
            </li>
            <li>
              Key-pool, budget, and replay tooling are documented from the
              repository state.
            </li>
            <li>
              Implementation notes point directly back to the modules that
              enforce the behavior.
            </li>
            <li>
              The bundled `skill/` package can be zipped and uploaded to
              Claude.ai for OmniLLM-aware assistance.
            </li>
          </ul>
        </div>

        <div className="omn-home__code-frame">
          <div className="omn-home__code-head">
            <div className="omn-home__code-dots" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <span>examples/basic_usage.rs</span>
          </div>
          <div className="omn-home__code-body">
            {codeLines.map((line) => (
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
          <span className="omn-home__section-kicker">Documentation</span>
          <h2>Choose the reading depth you need.</h2>
          <p>
            Start with operational usage, move into architecture, then read the
            implementation notes when you want the concrete module boundaries.
          </p>
        </div>
        <div className="omn-home__docs-grid">
          {docCards.map((entry) => (
            <DocTile key={entry.title} {...entry} />
          ))}
        </div>
      </section>

      <section className="omn-home__bottom-strip">
        <article>
          <span className="omn-home__strip-label">Focus</span>
          <strong>Gateway dispatch</strong>
          <p>Provider-neutral runtime calls with typed generation surfaces.</p>
        </article>
        <article>
          <span className="omn-home__strip-label">Safety</span>
          <strong>Loss-aware bridges</strong>
          <p>
            Transcoding stays explicit about downgraded or unsupported fields.
          </p>
        </article>
        <article>
          <span className="omn-home__strip-label">Operations</span>
          <strong>Budget-first execution</strong>
          <p>Quota reservation and settlement wrap every request lifecycle.</p>
        </article>
      </section>
    </div>
  );
}
