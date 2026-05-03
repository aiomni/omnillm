import { Link } from '@rspress/core/runtime';
import { useEffect } from 'react';

import { rememberLanguagePreference, useSiteLocale } from '../use-locale';

const gatewayCode = `let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
    .add_key(KeyConfig::new("sk-key-1", "prod-1").rpm_limit(500))
    .budget_limit_usd(50.0)
    .request_timeout(Duration::from_secs(45))
    .build()?;

let response = gateway.call(request, CancellationToken::new()).await?;`;

const transcodeCode = `let report = transcode_api_request(
    WireFormat::OpenAiChatCompletions,
    WireFormat::OpenAiResponses,
    raw_chat,
)?;

assert!(report.bridged);
assert!(!report.lossy);
for reason in report.loss_reasons {
    tracing::warn!(%reason, "provider bridge dropped data");
}`;

const replayCode = `let sanitized = sanitize_transport_request(recorded_request);
let fixture = ReplayFixture::from_transport(sanitized, response);
fixture.write_json("tests/fixtures/openai_responses.basic.json")?;`;

const primitiveCode = `let request = PrimitiveRequest::json(
    ProviderPrimitiveKind::ImagesGenerate,
    serde_json::json!({
        "model": "gpt-image-1",
        "prompt": "A blueprint-style gateway diagram"
    }),
);

let raw = gateway.primitive_call(request, CancellationToken::new()).await?;`;

const skillInstallCode = `# Codex / Claude Code / OpenCode
npx @vercel-labs/skills install github:aiomni/omnillm/skill

# Rust runtime
cargo add omnillm`;

type ApiGroup = {
  copy: string;
  items: string[];
  kicker: string;
  title: string;
};

type ManualCopy = {
  about: string;
  aboutLabel: string;
  apiLead: string;
  apiRows: Array<[string, string]>;
  apiGroups: ApiGroup[];
  apiTitle: string;
  checklistItems: string[];
  checklistLead: string;
  checklistTitle: string;
  contents: string;
  dek: string;
  dualLead: string;
  dualRows: Array<[string, string]>;
  dualTitle: string;
  endpointLead: string;
  endpointRows: Array<[string, string]>;
  endpointTitle: string;
  eyebrow: string;
  featureStrip: Array<[string, string]>;
  heroMeta: Array<[string, string]>;
  issue: [string, string];
  marginNotes: Array<[string, string]>;
  marginTitle: string;
  nav: [string, string, string, string];
  primitiveLead: string;
  primitiveTail: string;
  primitiveTitle: string;
  figureCaption: string;
  providerLead: string;
  providerRows: Array<[string, string]>;
  providerTitle: string;
  quickCallouts: Array<[string, string]>;
  quickLead: string;
  quickRunin: [string, string];
  quickTitle: string;
  quote: string;
  quoteCite: string;
  replayLead: string;
  replaySteps: string[];
  replayTitle: string;
  sectionFolios: Record<string, string>;
  skillLead: string;
  skillSteps: string[];
  skillTitle: string;
  toc: Array<[string, string, string]>;
  transcodeLead: string;
  transcodePoints: Array<[string, string]>;
  transcodeTitle: string;
  signalsLead: string;
  signalsRows: Array<[string, string]>;
  signalsRunin: [string, string];
  signalsTitle: string;
  title: string;
};

const copies: Record<'en' | 'zh', ManualCopy> = {
  en: {
    about:
      'This field manual turns OmniLLM’s runtime, protocol bridge, primitive provider support, replay fixtures, docs, and bundled Skill into one readable entry point for Rust teams shipping production LLM systems.',
    aboutLabel: 'About this manual.',
    apiLead:
      'Treat the API as layers. Start with the stable application contract, choose the endpoint family that matches the job, drop to primitive mode only when the raw provider contract matters, then keep transport, replay, and operational signals attached to every call.',
    apiRows: [
      ['Application contract', 'LlmRequest, Message, RequestItem, LlmResponse, and LlmStreamEvent keep generation portable across provider wire formats.'],
      ['Gateway runtime', 'GatewayBuilder, ProviderEndpoint, KeyConfig, Gateway::call, and Gateway::stream bind endpoint choice to key pools, limits, timeout, cancellation, and budget settlement.'],
      ['Endpoint families', 'EmbeddingRequest, image generation, audio transcription, audio speech, and rerank request families add typed APIs beyond text generation.'],
      ['Primitive APIs', 'PrimitiveRequest plus primitive_call, primitive_stream, and primitive_realtime preserve raw provider JSON, SSE, realtime, or media payloads.'],
      ['Bridge and transport', 'WireFormat, EndpointProtocol, ProviderProtocol, ConversionReport, and emit_transport_request make conversion and HTTP emission inspectable.'],
      ['Operations and tests', 'pool_status, budget_remaining_usd, ReplayFixture, sanitizer helpers, and OmniLLM error classes make production behavior reviewable.']
    ],
    apiGroups: [
      {
        kicker: 'Layer 01',
        title: 'Canonical generation contract',
        copy:
          'Use this layer when product code wants provider-neutral generation and stable response handling.',
        items: [
          'LlmRequest carries messages, model options, tools, and provider-neutral generation intent.',
          'Message, MessageRole, and RequestItem keep chat content typed instead of passing raw JSON around.',
          'Gateway::call returns LlmResponse for non-streaming generation.',
          'Gateway::stream returns LlmStreamEvent for canonical streaming behavior.'
        ]
      },
      {
        kicker: 'Layer 02',
        title: 'Gateway construction and runtime policy',
        copy:
          'Use this layer to bind endpoint choice to operational controls before the first request is sent.',
        items: [
          'GatewayBuilder selects ProviderEndpoint and wires key pools, budgets, retry posture, and request timeout.',
          'KeyConfig labels keys, applies RPM limits, and makes pool status readable for operators.',
          'CancellationToken is part of every call path, including streaming and primitive calls.',
          'BudgetTracker pre-reserves and settles usage without introducing a second budget subsystem.'
        ]
      },
      {
        kicker: 'Layer 03',
        title: 'Typed non-generation endpoint families',
        copy:
          'Use this layer when the task is still portable enough to deserve a typed OmniLLM request family.',
        items: [
          'EmbeddingRequest and EmbeddingResponse cover vector generation and OpenAI-compatible embeddings emission.',
          'Image generation request families let media workloads share gateway keys, timeout, and budget controls.',
          'Audio transcription and speech request families keep media endpoints inside the same runtime posture.',
          'Rerank request families model retrieval ranking without pretending it is text generation.'
        ]
      },
      {
        kicker: 'Layer 04',
        title: 'Provider primitive APIs',
        copy:
          'Use this layer when the provider API shape is itself the product contract and must be preserved.',
        items: [
          'PrimitiveRequest carries raw provider-native request payloads without LlmRequest or ApiRequest conversion.',
          'primitive_call handles one-shot provider-native APIs such as images, audio, token counting, or metadata.',
          'primitive_stream keeps provider-native SSE events intact when canonical stream events would lose detail.',
          'primitive_realtime is reserved for realtime transports such as OpenAI Realtime and Gemini Live.'
        ]
      },
      {
        kicker: 'Layer 05',
        title: 'Protocol bridge and transport emission',
        copy:
          'Use this layer when you need to inspect, test, or report the exact wire shape before dispatch.',
        items: [
          'EndpointProtocol describes runtime endpoint behavior; ProviderProtocol describes low-level provider wire shape.',
          'WireFormat names the source and target body format for explicit transcoding.',
          'ConversionReport<T> reports bridged, lossy, and loss_reasons instead of hiding downgrade behavior.',
          'emit_transport_request exposes method, path, headers, and body so tests can assert the emitted request.'
        ]
      },
      {
        kicker: 'Layer 06',
        title: 'Operational and replay surfaces',
        copy:
          'Use this layer to turn gateway behavior into observable service behavior.',
        items: [
          'pool_status reports key availability, limiter pressure, inflight work, and circuit state.',
          'budget_remaining_usd shows remaining budget after reservation and settlement accounting.',
          'ReplayFixture plus sanitizer helpers produce safe request/response fixtures for protocol regression tests.',
          'NoAvailableKey, BudgetExceeded, and Protocol(...) separate pool, spend, and conversion failures.'
        ]
      }
    ],
    apiTitle: 'API surface is layered by intent.',
    checklistItems: [
      'Choose the canonical protocol for portable generation before reaching for provider primitives.',
      'Configure every production key with explicit labels, RPM limits, cooldown behavior, and budget assumptions.',
      'Use CancellationToken and request timeouts for every gateway path, including primitive and stream calls.',
      'Treat ConversionReport as part of acceptance criteria when transcoding between provider formats.',
      'Record replay fixtures only after sanitization has removed auth headers, query secrets, and large binary fields.',
      'Expose pool_status, budget_remaining_usd, and OmniLLM error classes to operators before launch.'
    ],
    checklistLead:
      'The runtime gives you the primitives, but production readiness comes from making budget, cancellation, replay, and provider behavior visible in your service boundary.',
    checklistTitle: 'Before shipping, make the operational contract explicit.',
    contents: 'Contents',
    dek:
      'OmniLLM gives Rust teams one runtime for canonical generation, provider-native primitive APIs, protocol conversion, multi-key load balancing, rate limits, circuit breaking, replay-safe testing, and lock-free budget tracking.',
    dualLead:
      'OmniLLM exposes a normalized canonical mode and an explicit primitive mode. The first keeps generation portable. The second preserves provider-native request and response bodies for APIs that do not map cleanly onto a single generation schema.',
    dualRows: [
      ['Canonical Responses', 'Gateway::call and Gateway::stream use LlmRequest, LlmResponse, and LlmStreamEvent for provider-neutral generation.'],
      ['Provider Primitive', 'primitive_call, primitive_stream, and primitive_realtime preserve raw provider payloads for Images, Audio, Realtime, Count Tokens, Gemini Live, and compatible wrappers.'],
      ['Shared Ledger', 'Both modes settle through BudgetTracker, so raw provider usage participates in pre-reserve and settle accounting.'],
      ['Shared Protection', 'Both modes use the same key pool, RPM protection, timeout model, and circuit state instead of creating a second execution stack.']
    ],
    dualTitle: 'Two modes, one budget ledger.',
    endpointLead:
      'Runtime configuration uses EndpointProtocol. Low-level parsing, emission, and transcoding use ProviderProtocol. Names such as ClaudeMessages and GeminiGenerateContent are wire-shape identifiers, not marketing preferences.',
    endpointRows: [
      ['Official endpoints', 'Use official endpoint variants when OmniLLM should derive the standard upstream path from a host or prefix.'],
      ['Compatible endpoints', 'Use *_compat variants when the upstream wrapper already exposes the full request URL.'],
      ['Wire formats', 'Use WireFormat when converting raw API bodies between OpenAI, Anthropic, Gemini, and compatible request shapes.'],
      ['Transport profile', 'EndpointProtocol chooses the runtime request shape while application code keeps typed OmniLLM surfaces.']
    ],
    endpointTitle: 'Endpoint names describe wire shape.',
    eyebrow: 'AI-native production library',
    featureStrip: [
      ['Chapter 01', 'Build the gateway'],
      ['Chapter 02', 'Choose a protocol mode'],
      ['Chapter 03', 'Read the API surface'],
      ['Chapter 04', 'Operate with fixtures']
    ],
    heroMeta: [
      ['For Rust developers', 'Written for teams integrating LLM providers into production systems rather than experimenting with one-off SDK calls.'],
      ['Two protocol postures', 'Use canonical Responses semantics by default. Drop to primitive mode when raw provider APIs matter.'],
      ['Operational by design', 'Budgets, key pools, circuit state, replay fixtures, conversion reports, and agent instructions are first-class surfaces.']
    ],
    issue: ['Field Manual', 'Issue 01 · Rust Runtime'],
    marginNotes: [
      ['Providers', 'OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Bedrock, and OpenAI-compatible endpoints are represented through the embedded provider registry.'],
      ['Prompt cache', 'Typed cache policy exposes key, retention, breakpoint, and provider-specific bridge behavior without making budget estimates assume cache hits.'],
      ['Primitive tiering', 'P0 covers core generation and media endpoints. P3 includes WebSocket realtime support for OpenAI Realtime and Gemini Live.'],
      ['Docs path', 'Start with Usage, use Architecture for runtime invariants, then read Implementation when you need module boundaries.']
    ],
    marginTitle: 'Margin Notes',
    nav: ['Docs', 'API', 'Skill', 'GitHub'],
    primitiveLead:
      'Primitive calls are additive and explicit. They are the escape hatch for provider APIs where the raw provider contract is part of your product behavior.',
    primitiveTail:
      'keep primitive paths additive instead of forcing raw provider APIs through canonical generation conversion.',
    primitiveTitle: 'Primitive mode keeps raw provider APIs intact.',
    figureCaption:
      'EndpointProtocol chooses the runtime profile while WireFormat and ProviderProtocol describe conversion and transport boundaries.',
    providerLead:
      'Provider support is expressed as endpoint and primitive capabilities rather than SDK parity claims. This keeps the registry honest about transport shape, request path, response preservation, and settlement behavior.',
    providerRows: [
      ['OpenAI / Azure OpenAI', 'Canonical Responses, Chat Completions compatibility, embeddings, images, audio, realtime primitives, and OpenAI-compatible wrappers.'],
      ['Anthropic Claude', 'Messages wire shape, canonical generation bridge, tool/message conversion, and primitive extension points.'],
      ['Gemini / Vertex AI', 'GenerateContent profiles, Gemini-family wire conversion, Vertex-style deployment posture, and Live API primitive scope.'],
      ['Bedrock', 'Provider registry integration and runtime routing hooks for cloud-hosted model families.'],
      ['Compatible providers', 'Explicit compat endpoints for wrappers that already expose OpenAI-shaped URLs or provider-native proxy paths.']
    ],
    providerTitle: 'Provider coverage is capability-scoped.',
    quickCallouts: [
      ['Install', 'Add the crate, configure an endpoint, and label every key so pool status is understandable in production.'],
      ['Request', 'Keep product code centered on LlmRequest, Message, RequestItem, and LlmResponse unless raw provider payloads are required.'],
      ['Protect', 'Let the runtime handle key selection, RPM pressure, timeout, circuit state, cancellation, and budget reservation.'],
      ['Verify', 'Use replay fixtures and conversion reports to make provider behavior reviewable before rolling changes forward.']
    ],
    quickLead:
      'The default OmniLLM path is intentionally conservative: one typed generation request, one gateway, multiple provider backends. Application code stays centered on LlmRequest and LlmResponse while the runtime handles operational concerns.',
    quickRunin: ['Default', 'Use Gateway::call for non-streaming generation and Gateway::stream for canonical streaming. This is the right entry point when your product wants provider-neutral generation behavior without hand-writing provider adapters.'],
    quickTitle: 'Start with the canonical path.',
    quote: 'Normalize the application contract. Preserve the provider contract when the provider API is the product.',
    quoteCite: 'OmniLLM protocol posture',
    replayLead:
      'Record/replay tests need reviewable artifacts that do not leak secrets. OmniLLM ships ReplayFixture, sanitize_transport_request, sanitize_transport_response, and sanitize_json_value for safe fixture workflows.',
    replaySteps: [
      'Record real transport requests and responses only in controlled integration runs.',
      'Sanitize auth headers, query tokens, JSON secrets, and large binary or base64 payload fields.',
      'Review fixture diffs as API contracts instead of opaque snapshots.',
      'Replay against deterministic fixtures before modifying protocol bridges or provider profiles.'
    ],
    replayTitle: 'Fixtures should be useful and safe.',
    sectionFolios: {
      api: 'API Reference',
      checklist: 'Production',
      dual: 'Architecture',
      endpoint: 'Runtime Profiles',
      primitive: 'Native APIs',
      providers: 'Registry',
      quick: 'Usage Guide',
      replay: 'Testing',
      signals: 'Operations',
      skill: 'AI-native Project',
      transcode: 'Conversion'
    },
    signalsLead:
      'Production services can inspect gateway.pool_status() and gateway.budget_remaining_usd(). These surfaces expose key availability, inflight token pressure, RPM pressure, circuit state, and remaining budget after pre-reserve plus settlement accounting.',
    signalsRows: [
      ['pool_status()', 'Shows key availability, limiter pressure, circuit state, and whether the pool can accept more work.'],
      ['budget_remaining_usd()', 'Reports remaining budget after reservation and settlement, so operators see real spend pressure.'],
      ['NoAvailableKey', 'Indicates pool exhaustion, cooldown, or circuit-open state rather than a provider protocol failure.'],
      ['BudgetExceeded', 'Indicates spend policy blocked dispatch before or during settlement.'],
      ['Protocol(...)', 'Indicates bridge, parsing, emission, or provider wire-shape mismatch.']
    ],
    signalsRunin: ['Signals', 'When a request fails, OmniLLM-specific errors tell operators whether the failure belongs to pool availability, spend policy, or protocol conversion.'],
    signalsTitle: 'Runtime status belongs in the interface.',
    skillLead:
      'The bundled OmniLLM Skill gives coding agents repository-native signals instead of generic Rust SDK guesses. It is tuned to GatewayBuilder, ProviderEndpoint, EndpointProtocol, WireFormat, ReplayFixture, primitive calls, and OmniLLM runtime errors.',
    skillSteps: [
      'Install the Skill into Claude Code, Codex, OpenCode, or Claude-compatible skill runners.',
      'Ask for gateway setup, endpoint selection, protocol transcoding, replay fixture generation, or OmniLLM-specific error debugging.',
      'Verify answers against real examples, tests, endpoint profiles, conversion reports, and runtime surfaces.'
    ],
    skillTitle: 'The Skill teaches agents the real library.',
    toc: [
      ['01', 'Quick Start', '#quick-start'],
      ['02', 'Protocol Modes', '#dual-protocol'],
      ['03', 'Endpoint Profiles', '#endpoint-profiles'],
      ['04', 'Primitive APIs', '#primitive-apis'],
      ['05', 'Transcoding', '#transcoding'],
      ['06', 'API Surfaces', '#api-surfaces'],
      ['07', 'Provider Coverage', '#provider-coverage'],
      ['08', 'Replay Testing', '#replay-sanitization'],
      ['09', 'Observability', '#observability'],
      ['10', 'Ship Checklist', '#production-checklist'],
      ['11', 'OmniLLM Skill', '#skill-guide']
    ],
    transcodeLead:
      'Transcoding returns explicit bridge metadata through ConversionReport<T>. Callers can inspect bridged, lossy, and loss_reasons instead of guessing which provider-specific fields were dropped.',
    transcodePoints: [
      ['bridged', 'The request crossed provider formats instead of staying native.'],
      ['lossy', 'At least one field could not be represented safely in the target wire shape.'],
      ['loss_reasons', 'Human-readable reasons make test assertions and operator logs actionable.'],
      ['emit_transport_request', 'Turns typed requests into inspectable method, path, headers, and body before dispatch.']
    ],
    transcodeTitle: 'Loss is reported, not hidden.',
    title: 'The provider-neutral LLM field manual.'
  },
  zh: {
    about:
      '这份 Field Manual 把 OmniLLM 的运行时、协议桥接、provider primitive 支持、回放夹具、文档与内置 Skill 汇总成一个面向 Rust 团队的生产接入入口。',
    aboutLabel: '关于这份手册。',
    apiLead:
      '把 API 当成多层结构来读：先用稳定的应用契约，再选择任务对应的 endpoint family；只有当原始 provider 契约本身很重要时才进入 primitive mode，并且每条调用路径都保留 transport、replay 与运维信号。',
    apiRows: [
      ['应用契约', 'LlmRequest、Message、RequestItem、LlmResponse 与 LlmStreamEvent 让生成行为跨 provider wire format 保持可移植。'],
      ['Gateway runtime', 'GatewayBuilder、ProviderEndpoint、KeyConfig、Gateway::call 与 Gateway::stream 把端点选择和 key pool、限流、timeout、cancellation、budget settlement 绑定在一起。'],
      ['端点家族', 'EmbeddingRequest、图像生成、音频转写、语音与 rerank request family 为文本生成之外的任务提供类型化 API。'],
      ['Primitive APIs', 'PrimitiveRequest 搭配 primitive_call、primitive_stream、primitive_realtime 保留原始 provider JSON、SSE、realtime 或 media payload。'],
      ['Bridge 与 transport', 'WireFormat、EndpointProtocol、ProviderProtocol、ConversionReport 与 emit_transport_request 让转换和 HTTP 发射可检查。'],
      ['运维与测试', 'pool_status、budget_remaining_usd、ReplayFixture、sanitizer helpers 与 OmniLLM error classes 让生产行为可审查。']
    ],
    apiGroups: [
      {
        kicker: 'Layer 01',
        title: 'Canonical generation contract',
        copy:
          '当业务代码需要 provider-neutral generation 与稳定响应处理时，使用这一层。',
        items: [
          'LlmRequest 承载 messages、model options、tools 与 provider-neutral generation intent。',
          'Message、MessageRole、RequestItem 让聊天内容保持类型化，而不是到处传 raw JSON。',
          'Gateway::call 返回非流式生成的 LlmResponse。',
          'Gateway::stream 返回 canonical streaming 的 LlmStreamEvent。'
        ]
      },
      {
        kicker: 'Layer 02',
        title: 'Gateway construction and runtime policy',
        copy:
          '在发送第一条请求前，用这一层把 endpoint choice 绑定到运维控制。',
        items: [
          'GatewayBuilder 选择 ProviderEndpoint，并连接 key pools、budgets、retry posture 与 request timeout。',
          'KeyConfig 为 key 设置 label、RPM limit，让 pool status 对运维可读。',
          'CancellationToken 出现在每条调用路径，包括 streaming 与 primitive calls。',
          'BudgetTracker 执行 pre-reserve 与 settlement，不引入第二套预算系统。'
        ]
      },
      {
        kicker: 'Layer 03',
        title: 'Typed non-generation endpoint families',
        copy:
          '当任务仍足够通用、值得抽象为 typed OmniLLM request family 时，使用这一层。',
        items: [
          'EmbeddingRequest 与 EmbeddingResponse 覆盖向量生成和 OpenAI-compatible embeddings emission。',
          '图像生成请求族让 media workload 共享 gateway key、timeout 与 budget controls。',
          '音频转写和语音请求族让媒体端点保持在同一套 runtime posture 中。',
          'Rerank request family 表达检索排序，而不是把它伪装成文本生成。'
        ]
      },
      {
        kicker: 'Layer 04',
        title: 'Provider primitive APIs',
        copy:
          '当 provider API shape 本身就是产品契约、不能被规范化掉时，使用这一层。',
        items: [
          'PrimitiveRequest 承载 provider-native request payload，不经过 LlmRequest 或 ApiRequest conversion。',
          'primitive_call 处理 images、audio、token counting、metadata 等一次性 provider-native API。',
          'primitive_stream 在 canonical stream events 会丢细节时保留 provider-native SSE events。',
          'primitive_realtime 面向 OpenAI Realtime 与 Gemini Live 这类 realtime transport。'
        ]
      },
      {
        kicker: 'Layer 05',
        title: 'Protocol bridge and transport emission',
        copy:
          '当你需要在 dispatch 前检查、测试或报告准确 wire shape 时，使用这一层。',
        items: [
          'EndpointProtocol 描述 runtime endpoint behavior；ProviderProtocol 描述低层 provider wire shape。',
          'WireFormat 明确 transcode 的 source body format 与 target body format。',
          'ConversionReport<T> 报告 bridged、lossy、loss_reasons，不隐藏降级行为。',
          'emit_transport_request 暴露 method、path、headers、body，方便测试断言发射出的请求。'
        ]
      },
      {
        kicker: 'Layer 06',
        title: 'Operational and replay surfaces',
        copy:
          '用这一层把 gateway 行为转化成服务可观测行为。',
        items: [
          'pool_status 报告 key availability、limiter pressure、inflight work 与 circuit state。',
          'budget_remaining_usd 展示 reservation 与 settlement 之后的剩余预算。',
          'ReplayFixture 搭配 sanitizer helpers 生成安全 request/response fixture，用于 protocol regression tests。',
          'NoAvailableKey、BudgetExceeded、Protocol(...) 区分 pool、spend 与 conversion failure。'
        ]
      }
    ],
    apiTitle: 'API surface 按使用意图分层。',
    checklistItems: [
      '优先为可移植生成选择 canonical protocol，再决定是否进入 provider primitive。',
      '为每个生产 Key 配置明确 label、RPM 限制、冷却行为与预算假设。',
      '所有 gateway 路径都使用 CancellationToken 与 request timeout，包括 primitive 和 stream。',
      '把 ConversionReport 纳入协议转码验收标准，不要默认所有字段都能无损桥接。',
      '回放夹具必须先脱敏 auth header、query secret 与大体积二进制字段。',
      '上线前把 pool_status、budget_remaining_usd 与 OmniLLM 错误类别暴露给运维。'
    ],
    checklistLead:
      '运行时提供基础能力，但生产可用性来自把预算、取消、回放与 provider 行为显式放进服务边界。',
    checklistTitle: '上线前，先明确运行契约。',
    contents: '目录',
    dek:
      'OmniLLM 为 Rust 团队提供一个统一运行时，覆盖 canonical generation、provider 原生 primitive API、协议转换、多 Key 负载均衡、限流、熔断、安全回放测试与 lock-free 预算追踪。',
    dualLead:
      'OmniLLM 提供规范化 canonical 模式与显式 primitive 模式。前者让生成行为可移植，后者为无法自然映射到单一生成 schema 的 provider API 保留原始请求与响应。',
    dualRows: [
      ['Canonical Responses', 'Gateway::call 与 Gateway::stream 使用 LlmRequest、LlmResponse、LlmStreamEvent 做 provider-neutral generation。'],
      ['Provider Primitive', 'primitive_call、primitive_stream、primitive_realtime 为 Images、Audio、Realtime、Count Tokens、Gemini Live 与兼容包装器保留原生 payload。'],
      ['共享预算账本', '两种模式都通过 BudgetTracker 结算，原始 provider usage 也参与预留与结算。'],
      ['共享保护机制', '两种模式共享 key pool、RPM protection、timeout model 与 circuit state，不引入第二套执行栈。']
    ],
    dualTitle: '两种模式，共用一套预算账本。',
    endpointLead:
      '运行时配置使用 EndpointProtocol；底层解析、发射与转码使用 ProviderProtocol。ClaudeMessages、GeminiGenerateContent 这类名称描述的是 wire shape，而不是营销偏好。',
    endpointRows: [
      ['官方端点', '当 OmniLLM 需要从 host 或 prefix 推导标准 upstream path 时，使用官方端点变体。'],
      ['兼容端点', '当上游 wrapper 已经暴露完整请求 URL 时，使用 *_compat 变体。'],
      ['Wire formats', '在 OpenAI、Anthropic、Gemini 与兼容请求形态之间转换 raw API body 时使用 WireFormat。'],
      ['Transport profile', 'EndpointProtocol 选择运行时请求形态，应用代码仍保留 typed OmniLLM surface。']
    ],
    endpointTitle: '端点名称描述 wire shape。',
    eyebrow: 'AI-native production library',
    featureStrip: [
      ['Chapter 01', '构建 gateway'],
      ['Chapter 02', '选择协议模式'],
      ['Chapter 03', '阅读 API surface'],
      ['Chapter 04', '用 fixture 运维']
    ],
    heroMeta: [
      ['面向 Rust 开发者', '为把 LLM provider 接入生产系统的团队编写，而不是只演示一次性 SDK 调用。'],
      ['两种协议姿态', '默认使用 canonical Responses 语义；当原始 provider API 本身很重要时，再进入 primitive mode。'],
      ['按生产运维设计', '预算、Key 池、熔断状态、回放夹具、转换报告与 agent 指令都是一等接口。']
    ],
    issue: ['Field Manual', 'Issue 01 · Rust Runtime'],
    marginNotes: [
      ['Providers', 'OpenAI、Azure OpenAI、Anthropic、Gemini、Vertex AI、Bedrock 与 OpenAI-compatible 端点通过内置 provider registry 表达。'],
      ['Prompt cache', '类型化 cache policy 暴露 key、retention、breakpoint 与 provider-specific bridge 行为，但预算估算不默认假设 cache hit。'],
      ['Primitive tiering', 'P0 覆盖核心生成与媒体端点；P3 包含 OpenAI Realtime 与 Gemini Live 的 WebSocket realtime 支持。'],
      ['Docs path', '先读 Usage，再用 Architecture 理解运行时不变量，最后在需要模块边界时读 Implementation。']
    ],
    marginTitle: '边栏笔记',
    nav: ['文档', 'API', 'Skill', 'GitHub'],
    primitiveLead:
      'Primitive 调用是显式增量能力。它适用于那些 provider 原生 API 契约就是产品行为一部分的场景。',
    primitiveTail:
      '保持 primitive path 的增量语义，而不是把原始 provider API 强行塞进 canonical generation conversion。',
    primitiveTitle: 'Primitive mode 保留 provider 原生 API。',
    figureCaption:
      'EndpointProtocol 选择运行时 profile；WireFormat 与 ProviderProtocol 描述转换和传输边界。',
    providerLead:
      'Provider 支持以端点能力与 primitive 能力表达，而不是宣称完整 SDK parity。这样 registry 能更准确地描述 transport shape、request path、response preservation 与 settlement 行为。',
    providerRows: [
      ['OpenAI / Azure OpenAI', 'Canonical Responses、Chat Completions 兼容、embeddings、images、audio、realtime primitives 与 OpenAI-compatible wrapper。'],
      ['Anthropic Claude', 'Messages wire shape、canonical generation bridge、tool/message conversion 与 primitive extension point。'],
      ['Gemini / Vertex AI', 'GenerateContent profile、Gemini-family wire conversion、Vertex-style deployment posture 与 Live API primitive scope。'],
      ['Bedrock', '用于云上模型家族的 provider registry 集成与 runtime routing hook。'],
      ['Compatible providers', '为已经暴露 OpenAI-shaped URL 或 provider-native proxy path 的 wrapper 提供明确 compat endpoint。']
    ],
    providerTitle: 'Provider 覆盖按能力声明。',
    quickCallouts: [
      ['安装', '添加 crate，配置 endpoint，并为每个 key 设置 label，方便生产环境理解 pool status。'],
      ['请求', '除非确实需要 raw provider payload，否则业务代码保持在 LlmRequest、Message、RequestItem 与 LlmResponse。'],
      ['保护', '让运行时接管 key selection、RPM pressure、timeout、circuit state、cancellation 与 budget reservation。'],
      ['验证', '使用 replay fixture 与 conversion report，让 provider 行为在发布前可审查。']
    ],
    quickLead:
      'OmniLLM 默认路径刻意保持保守：一个类型化生成请求、一个 gateway、多个 provider 后端。应用代码围绕 LlmRequest 与 LlmResponse，运行时处理运维问题。',
    quickRunin: ['默认', '非流式生成使用 Gateway::call；canonical streaming 使用 Gateway::stream。当产品需要 provider-neutral generation，而不是手写每个 provider adapter 时，这是首选入口。'],
    quickTitle: '从 canonical path 开始。',
    quote: '规范化应用契约；当 provider API 本身就是产品时，保留 provider 契约。',
    quoteCite: 'OmniLLM protocol posture',
    replayLead:
      'Record/replay 测试需要可审查且不泄露密钥的工件。OmniLLM 提供 ReplayFixture、sanitize_transport_request、sanitize_transport_response 与 sanitize_json_value 来支持安全 fixture 工作流。',
    replaySteps: [
      '只在受控集成运行中记录真实 transport request 与 response。',
      '脱敏 auth header、query token、JSON secret，以及大体积 binary/base64 字段。',
      '把 fixture diff 当成 API contract 审查，而不是不透明 snapshot。',
      '修改 protocol bridge 或 provider profile 前，先用 deterministic fixture replay。'
    ],
    replayTitle: 'Fixture 要既有用又安全。',
    sectionFolios: {
      api: 'API Reference',
      checklist: 'Production',
      dual: 'Architecture',
      endpoint: 'Runtime Profiles',
      primitive: 'Native APIs',
      providers: 'Registry',
      quick: 'Usage Guide',
      replay: 'Testing',
      signals: 'Operations',
      skill: 'AI-native Project',
      transcode: 'Conversion'
    },
    signalsLead:
      '生产服务可以检查 gateway.pool_status() 与 gateway.budget_remaining_usd()。这些接口暴露 Key 可用性、inflight token pressure、RPM pressure、circuit state，以及预留与结算后的剩余预算。',
    signalsRows: [
      ['pool_status()', '展示 key availability、limiter pressure、circuit state，以及当前 pool 是否还能接收更多工作。'],
      ['budget_remaining_usd()', '报告 reservation 与 settlement 后的剩余预算，让运维看到真实 spend pressure。'],
      ['NoAvailableKey', '表示 pool exhaustion、cooldown 或 circuit-open，而不是 provider protocol failure。'],
      ['BudgetExceeded', '表示 spend policy 在分发前或结算过程中阻止了请求。'],
      ['Protocol(...)', '表示 bridge、parsing、emission 或 provider wire-shape mismatch。']
    ],
    signalsRunin: ['信号', '请求失败时，OmniLLM-specific errors 会告诉运维问题属于 pool availability、spend policy 还是 protocol conversion。'],
    signalsTitle: '运行时状态应当出现在接口里。',
    skillLead:
      '内置 OmniLLM Skill 给 coding agent 仓库级信号，而不是泛泛的 Rust SDK 猜测。它围绕 GatewayBuilder、ProviderEndpoint、EndpointProtocol、WireFormat、ReplayFixture、primitive calls 与 OmniLLM runtime errors 调优。',
    skillSteps: [
      '把 Skill 安装到 Claude Code、Codex、OpenCode 或兼容 Claude Skill 的运行器。',
      '让 agent 协助 gateway setup、endpoint selection、protocol transcoding、replay fixture generation 或 OmniLLM error debugging。',
      '用真实 examples、tests、endpoint profiles、conversion reports 与 runtime surfaces 校验回答。'
    ],
    skillTitle: 'Skill 会教 agent 使用真实库边界。',
    toc: [
      ['01', 'Quick Start', '#quick-start'],
      ['02', 'Protocol Modes', '#dual-protocol'],
      ['03', 'Endpoint Profiles', '#endpoint-profiles'],
      ['04', 'Primitive APIs', '#primitive-apis'],
      ['05', 'Transcoding', '#transcoding'],
      ['06', 'API Surfaces', '#api-surfaces'],
      ['07', 'Provider Coverage', '#provider-coverage'],
      ['08', 'Replay Testing', '#replay-sanitization'],
      ['09', 'Observability', '#observability'],
      ['10', 'Ship Checklist', '#production-checklist'],
      ['11', 'OmniLLM Skill', '#skill-guide']
    ],
    transcodeLead:
      'Transcoding 会通过 ConversionReport<T> 返回显式 bridge metadata。调用方可以检查 bridged、lossy 与 loss_reasons，而不是猜测哪些 provider-specific 字段被丢弃。',
    transcodePoints: [
      ['bridged', '请求跨过了 provider format，而不是停留在 native shape。'],
      ['lossy', '至少有一个字段无法安全表示到目标 wire shape。'],
      ['loss_reasons', '可读原因让测试断言与运维日志可行动。'],
      ['emit_transport_request', '在分发前把 typed request 变成可检查的 method、path、headers 与 body。']
    ],
    transcodeTitle: '损耗要报告，不能隐藏。',
    title: '面向 provider-neutral LLM 的 Rust 现场手册。'
  }
};

function CodePlate(props: { children: string; file: string; label: string }) {
  return (
    <div className="plate">
      <div className="plate-head">
        <span>{props.file}</span>
        <span>{props.label}</span>
      </div>
      <pre>
        <code>{props.children}</code>
      </pre>
    </div>
  );
}

function CalloutGrid(props: { items: Array<[string, string]> }) {
  return (
    <div className="callout-grid">
      {props.items.map(([title, body]) => (
        <article className="callout" key={title}>
          <span>{title}</span>
          <p>{body}</p>
        </article>
      ))}
    </div>
  );
}

function SpecRows(props: { rows: Array<[string, string]> }) {
  return (
    <table className="spec-table">
      <tbody>
        {props.rows.map(([title, body]) => (
          <tr key={title}>
            <th>{title}</th>
            <td>{body}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ApiSurfaceMap(props: { groups: ApiGroup[] }) {
  return (
    <div className="api-surface-map">
      {props.groups.map(group => (
        <article className="api-surface-card" key={group.title}>
          <span className="api-surface-card__kicker">{group.kicker}</span>
          <h3>{group.title}</h3>
          <p>{group.copy}</p>
          <ul>
            {group.items.map(item => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </article>
      ))}
    </div>
  );
}

function NumberedList(props: { items: string[] }) {
  return (
    <ol className="numbered">
      {props.items.map(item => (
        <li key={item}>
          <span>{item}</span>
        </li>
      ))}
    </ol>
  );
}

function InlineCode(props: { children: string }) {
  return <code className="inline-code">{props.children}</code>;
}

function ManualLanguageSwitcher() {
  const { lang, switchPath } = useSiteLocale();

  return (
    <div
      aria-label={lang === 'zh' ? '语言切换' : 'Language switcher'}
      className="manual-lang-switcher"
      role="group"
    >
      {([
        ['en', 'EN'],
        ['zh', '中文']
      ] as const).map(([targetLang, label]) => {
        if (targetLang === lang) {
          return (
            <span
              aria-current="true"
              className="manual-lang-switcher__item manual-lang-switcher__item--active"
              key={targetLang}
            >
              {label}
            </span>
          );
        }

        return (
          <Link
            className="manual-lang-switcher__item"
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

export function LandingPage() {
  const { lang } = useSiteLocale();
  const copy = copies[lang];

  useEffect(() => {
    document.body.classList.add('omn-field-page');

    return () => {
      document.body.classList.remove('omn-field-page');
    };
  }, []);

  return (
    <main className="omn-field-manual">
      <div className="page">
        <header className="masthead">
          <a className="brand" href="#top">
            OmniLLM
          </a>
          <p className="issue">
            {copy.issue[0]}
            <br />
            {copy.issue[1]}
          </p>
          <nav className="nav" aria-label="Primary">
            <a href="#quick-start">{copy.nav[0]}</a>
            <a href="#api-surfaces">{copy.nav[1]}</a>
            <a href="#skill-guide">{copy.nav[2]}</a>
            <a href="https://github.com/aiomni/omnillm" rel="noreferrer" target="_blank">
              {copy.nav[3]}
            </a>
            <ManualLanguageSwitcher />
          </nav>
        </header>

        <section className="cover" id="top" data-od-id="hero">
          <div className="cover-kicker">
            <p className="eyebrow">{copy.eyebrow}</p>
            <div className="cover-rule" aria-hidden="true" />
          </div>
          <div className="cover-headline">
            <h1 data-od-id="headline">{copy.title}</h1>
          </div>
          <div className="cover-body">
            <p className="dek" data-od-id="body">
              {copy.dek}
            </p>
          </div>
          <aside className="cover-meta" aria-label="Edition notes">
            {copy.heroMeta.map(([title, body]) => (
              <div className="meta-block" key={title}>
                <b>{title}</b>
                <p>{body}</p>
              </div>
            ))}
          </aside>
        </section>

        <nav className="feature-strip" aria-label="Featured chapters">
          {copy.featureStrip.map(([label, title], index) => (
            <a href={copy.toc[index]?.[2] ?? '#quick-start'} key={label}>
              <span>{label}</span>
              <strong>{title}</strong>
            </a>
          ))}
        </nav>

        <div className="issue-grid">
          <aside className="contents" data-od-id="docs-nav">
            <p className="folio">{copy.contents}</p>
            <ol className="toc-list">
              {copy.toc.map(([number, label, href]) => (
                <li key={number}>
                  <a href={href}>
                    <span>{number}</span>
                    <span>{label}</span>
                  </a>
                </li>
              ))}
            </ol>
          </aside>

          <article>
            <section id="quick-start">
              <p className="folio">{copy.sectionFolios.quick}</p>
              <h2>{copy.quickTitle}</h2>
              <p className="lead">{copy.quickLead}</p>
              <CalloutGrid items={copy.quickCallouts} />
              <p>
                <span className="runin">{copy.quickRunin[0]}</span>
                {copy.quickRunin[1]}
              </p>
              <CodePlate file="gateway.rs" label="canonical generation">
                {gatewayCode}
              </CodePlate>
            </section>

            <section id="dual-protocol">
              <p className="folio">{copy.sectionFolios.dual}</p>
              <h2>{copy.dualTitle}</h2>
              <p>{copy.dualLead}</p>
              <blockquote className="quote" data-od-id="pull-quote">
                {copy.quote}
                <cite>{copy.quoteCite}</cite>
              </blockquote>
              <SpecRows rows={copy.dualRows} />
            </section>

            <section id="endpoint-profiles">
              <p className="folio">{copy.sectionFolios.endpoint}</p>
              <h2>{copy.endpointTitle}</h2>
              <p>{copy.endpointLead}</p>
              <SpecRows rows={copy.endpointRows} />
              <figure className="figure" data-od-id="endpoint-figure">
                <div className="figure-canvas" aria-hidden="true" />
                <figcaption className="caption">
                  <span className="figure-label">{lang === 'zh' ? '图 1.' : 'Figure 1.'}</span>{' '}
                  {copy.figureCaption}
                </figcaption>
              </figure>
            </section>

            <section id="primitive-apis">
              <p className="folio">{copy.sectionFolios.primitive}</p>
              <h2>{copy.primitiveTitle}</h2>
              <p>
                {copy.primitiveLead} <InlineCode>PrimitiveRequest</InlineCode>,{' '}
                <InlineCode>primitive_call</InlineCode>,{' '}
                <InlineCode>primitive_stream</InlineCode>,
                {lang === 'zh' ? ' 以及 ' : ' and '}
                <InlineCode>primitive_realtime</InlineCode> {copy.primitiveTail}
              </p>
              <CodePlate file="primitive.rs" label="raw provider payload">
                {primitiveCode}
              </CodePlate>
            </section>

            <section id="transcoding">
              <p className="folio">{copy.sectionFolios.transcode}</p>
              <h2>{copy.transcodeTitle}</h2>
              <p>{copy.transcodeLead}</p>
              <CalloutGrid items={copy.transcodePoints} />
              <CodePlate file="transcode.rs" label="bridge metadata">
                {transcodeCode}
              </CodePlate>
            </section>

            <section id="api-surfaces" data-od-id="api-surfaces">
              <p className="folio">{copy.sectionFolios.api}</p>
              <h2>{copy.apiTitle}</h2>
              <p>{copy.apiLead}</p>
              <SpecRows rows={copy.apiRows} />
              <ApiSurfaceMap groups={copy.apiGroups} />
            </section>

            <section id="provider-coverage">
              <p className="folio">{copy.sectionFolios.providers}</p>
              <h2>{copy.providerTitle}</h2>
              <p>{copy.providerLead}</p>
              <SpecRows rows={copy.providerRows} />
            </section>

            <section id="replay-sanitization">
              <p className="folio">{copy.sectionFolios.replay}</p>
              <h2>{copy.replayTitle}</h2>
              <p>{copy.replayLead}</p>
              <NumberedList items={copy.replaySteps} />
              <CodePlate file="replay.rs" label="sanitized fixture">
                {replayCode}
              </CodePlate>
            </section>

            <section id="observability">
              <p className="folio">{copy.sectionFolios.signals}</p>
              <h2>{copy.signalsTitle}</h2>
              <p>{copy.signalsLead}</p>
              <SpecRows rows={copy.signalsRows} />
              <p>
                <span className="runin">{copy.signalsRunin[0]}</span>
                {copy.signalsRunin[1]}
              </p>
            </section>

            <section id="production-checklist">
              <p className="folio">{copy.sectionFolios.checklist}</p>
              <h2>{copy.checklistTitle}</h2>
              <p>{copy.checklistLead}</p>
              <NumberedList items={copy.checklistItems} />
            </section>

            <section id="skill-guide">
              <p className="folio">{copy.sectionFolios.skill}</p>
              <h2>{copy.skillTitle}</h2>
              <p>{copy.skillLead}</p>
              <NumberedList items={copy.skillSteps} />
              <CodePlate file="install.sh" label="runtime + skill">
                {skillInstallCode}
              </CodePlate>
              <div className="end-matter">
                <div className="avatar">OL</div>
                <p>
                  <strong>{copy.aboutLabel}</strong> {copy.about}
                </p>
              </div>
            </section>
          </article>

          <aside className="margin-notes" data-od-id="docs-toc">
            <p className="folio">{copy.marginTitle}</p>
            {copy.marginNotes.map(([title, body]) => (
              <p className="side-note" key={title}>
                <span className="note-title">{title}</span>
                {body}
              </p>
            ))}
          </aside>
        </div>
      </div>
    </main>
  );
}
