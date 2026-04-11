---
title: 使用指南
description: 安装 OmniLLM、配置 provider 端点、发送规范化请求、处理流式结果，并按生产环境形态运行这个运行时。
label: 运行指南
release: v0.1.0
updated: 2026 年 4 月
summary: 运行时初始化、Gateway 调用、协议桥接、预算跟踪、回放脱敏与运维模式。
---

# 使用指南

这份指南说明如何把 OmniLLM 用作：

- 生成请求的运行时网关
- 支持协议之间的转码层
- 面向向量嵌入、图像、音频与重排序 API 的类型化多端点转换层
- 面向测试夹具的回放脱敏辅助工具
- 自带官方 OmniLLM Skill 的 Rust 项目

如果你想看 Skill 的安装方式，请阅读 [skill.md](./skill.md)。如果你想继续了解系统设计与源码细节，请阅读 [architecture.md](./architecture.md) 与 [implementation.md](./implementation.md)。

## 这个库提供什么

OmniLLM 目前有两条主要使用面：

1. `Gateway`
   当你需要在运行时发送生成请求，并同时获得以下能力时使用它：
   - provider 无关的请求/响应类型
   - 多 Key 负载均衡
   - 每个 Key 的 RPM 与 TPM 控制
   - 熔断保护
   - 预算跟踪
   - 规范化流式事件

2. API 与协议转换辅助工具
   当你需要以下能力时使用这一组工具：
   - 把上游原始 payload 解析成规范化类型
   - 把规范化类型重新输出成 provider 的传输格式
   - 在支持的协议之间做转码
   - 检查桥接过程和字段损耗元数据
   - 为测试用例清洗请求/响应夹具

注意：当前运行时 `Gateway` 只处理生成请求。向量嵌入、图像、音频与重排序 API 目前以规范化类型转换工具的形式提供，而不是完整的运行时传输客户端。

## 安装

把这个库加到你的 `Cargo.toml` 中：

```toml
[dependencies]
omnillm = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tokio-util = "0.7"
```

选择一种 TLS 后端：

- 默认：`rustls`
- 可选：`native-tls`

示例：

```toml
[dependencies]
omnillm = "0.1"
```

```toml
[dependencies]
omnillm = { version = "0.1", default-features = false, features = ["native-tls"] }
```

## OmniLLM Skill 集成

OmniLLM 在仓库的
[`skill/` 目录](https://github.com/aiomni/omnillm/tree/main/skill)
中附带了一份官方 agent skill。如果你想通过 Vercel Labs skills 流程把它安装到 Claude Code、Codex 或 OpenCode 中，请阅读 [技能指南](./skill.md)。

## 核心概念

这个库围绕 `LlmRequest` 和 `LlmResponse` 来统一生成请求模型。

- `LlmRequest` 是规范化的生成请求。
- `LlmResponse` 是规范化的生成响应。
- `LlmStreamEvent` 是规范化的流式事件模型。
- `CapabilitySet` 用来承载跨 provider 的工具、结构化输出、推理、内置工具等能力。
- `EndpointProtocol` 表示运行时端点配置，包括 `*_compat` 模式。
- `ProviderProtocol` 表示底层生成协议，供编解码与转码层使用。
- `ProviderEndpoint` 用于标识请求要发送到哪里，以及如何发送。

对于多端点场景：

- `ApiRequest` 和 `ApiResponse` 是跨端点家族的规范化类型封装。
- `WireFormat` 表示某一种具体的上游传输格式。
- `ConversionReport<T>` 会告诉你转换是否发生了桥接，以及是否有字段损耗。

## 快速开始

下面是一个最小但可用的运行时示例：

```rust
use omnillm::{
    GatewayBuilder, GenerationConfig, KeyConfig, LlmRequest, Message, MessageRole,
    ProviderEndpoint, RequestItem,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = GatewayBuilder::new(ProviderEndpoint::openai_responses())
        .add_key(KeyConfig::new("sk-key-1", "prod-1"))
        .budget_limit_usd(50.0)
        .build()?;

    let request = LlmRequest {
        model: "gpt-4.1-mini".into(),
        instructions: Some("Answer concisely.".into()),
        input: vec![RequestItem::from(Message::text(
            MessageRole::User,
            "Explain Rust ownership in one sentence.",
        ))],
        messages: Vec::new(),
        capabilities: Default::default(),
        generation: GenerationConfig {
            max_output_tokens: Some(128),
            ..Default::default()
        },
        metadata: Default::default(),
        vendor_extensions: Default::default(),
    };

    let response = gateway.call(request, CancellationToken::new()).await?;
    println!("{}", response.content_text);
    Ok(())
}
```

## 构建 Gateway

`GatewayBuilder` 用来配置运行时客户端：

```rust
use std::time::Duration;

use omnillm::{GatewayBuilder, KeyConfig, PoolConfig, ProviderEndpoint};

let gateway = GatewayBuilder::new(ProviderEndpoint::claude_messages())
    .add_key(
        KeyConfig::new("sk-key-1", "claude-prod-1")
            .tpm_limit(90_000)
            .rpm_limit(500),
    )
    .add_key(
        KeyConfig::new("sk-key-2", "claude-prod-2")
            .tpm_limit(90_000)
            .rpm_limit(500),
    )
    .budget_limit_usd(100.0)
    .pool_config(PoolConfig::default())
    .request_timeout(Duration::from_secs(120))
    .build()?;
```

### Builder 选项

- `add_key` / `add_keys`
  为同一个上游端点注册一个或多个 API Key。

- `budget_limit_usd`
  设置进程内预算上限。请求会在派发前预留估算成本，并在完成后按实际成本结算。

- `pool_config`
  配置获取重试策略与熔断阈值。

- `request_timeout`
  设置 `Dispatcher` 使用的 HTTP 客户端超时时间。

### Key 配置

每个 `KeyConfig` 都包含：

- 原始 key 字符串
- 人类可读的 label
- `tpm_limit`
- `rpm_limit`

建议用标签字段做可观测性标识。`gateway.pool_status()` 会返回这些标签。

## 选择 Provider Endpoint

内置的生成端点包括：

- `ProviderEndpoint::openai_responses()`
- `ProviderEndpoint::openai_chat_completions()`
- `ProviderEndpoint::claude_messages()`
- `ProviderEndpoint::gemini_generate_content()`

你也可以自己构造一个自定义端点：

```rust
use omnillm::{AuthScheme, EndpointProtocol, ProviderEndpoint};

let endpoint = ProviderEndpoint::new(
    EndpointProtocol::OpenAiResponsesCompat,
    "https://your-openai-compatible-host/v1/responses",
)
.with_auth(AuthScheme::Header {
    name: "x-api-key".into(),
})
.with_default_header("x-tenant-id", "acme-prod");
```

当 `base_url` 只是 host 或前缀时，使用非 `compat` 协议，让 OmniLLM 自动补齐标准路径。
当 `base_url` 已经是某个包装层或兼容网关暴露出来的完整请求 URL 时，使用 `*_compat` 协议。
`EndpointProtocol` 是运行时配置层；`ClaudeMessages`、`GeminiGenerateContent` 这类名字保留在 `ProviderProtocol` 上，因为它们对应的是上游 wire API 形态，供 `parse_*`、`emit_*`、`transcode_*` 使用。

### 鉴权方式

`AuthScheme` 支持：

- `Bearer`
- `Header { name }`
- `Query { name }`

如果你没有显式设置鉴权方式，`ProviderEndpoint` 会使用该协议对应的默认值。

## 构建请求

### `input` 与 `messages`

`LlmRequest` 同时支持：

- `input`：规范化执行输入
- `messages`：兼容式聊天风格视图

如果 `input` 非空，它会被视为真实输入来源；如果 `input` 为空，则回退使用 `messages`。

新代码建议优先使用 `input`。

### `instructions` 字段

`instructions` 是规范化模型里放置 system/developer 指令的顶层字段。

如果没有显式提供 `instructions`，这个库可以从聊天风格视图中的 system/developer 消息中推导出规范化指令。

### 生成控制

`GenerationConfig` 包含：

- `max_output_tokens`
- `temperature`
- `top_p`
- `top_k`
- `stop_sequences`
- `presence_penalty`
- `frequency_penalty`
- `seed`

这些都是规范化控制项。转码到能力更窄的协议时，某些字段可能会被丢弃，并通过 `ConversionReport.loss_reasons` 体现出来。

## 能力集

`CapabilitySet` 是跨 provider 的能力层。

### 自定义工具

```rust
use omnillm::{CapabilitySet, ToolDefinition};
use serde_json::json;

let capabilities = CapabilitySet {
    tools: vec![ToolDefinition {
        name: "lookup_weather".into(),
        description: Some("Get current weather for a city".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
        strict: false,
        vendor_extensions: Default::default(),
    }],
    ..Default::default()
};
```

### 结构化输出

```rust
use omnillm::{CapabilitySet, StructuredOutputConfig};
use serde_json::json;

let capabilities = CapabilitySet {
    structured_output: Some(StructuredOutputConfig {
        name: Some("summary".into()),
        schema: json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "bullets": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["title", "bullets"]
        }),
        strict: true,
    }),
    ..Default::default()
};
```

### 推理与内置工具

`CapabilitySet` 还包括：

- `reasoning`
- `builtin_tools`
- `modalities`
- `safety`
- `cache`

这些都是规范化抽象。具体是否受支持取决于目标协议；如果目标协议无法表达其中一部分能力，转换报告会把它标记为 `bridged`，并在必要时标记为 `lossy`。

## 非流式调用

单次生成调用可以使用 `Gateway::call`：

```rust
let response = gateway.call(request, CancellationToken::new()).await?;

println!("model: {}", response.model);
println!("usage total: {}", response.usage.total());
println!("text: {}", response.content_text);
```

Gateway 会按下面的顺序执行：

1. 估算 token 与成本
2. 获取一个健康且拥有足够 TPM 容量的 key
3. 检查本地预算
4. 检查本地 RPM 窗口
5. 派发上游 HTTP 请求
6. 按实际 usage 结算成本
7. 根据成功或失败结果更新 key 健康状态

## 流式调用

如果你希望收到规范化流事件，请使用 `Gateway::stream`：

```rust
use futures_util::StreamExt;
use omnillm::LlmStreamEvent;

let mut stream = gateway.stream(request, CancellationToken::new()).await?;

while let Some(event) = stream.next().await {
    match event? {
        LlmStreamEvent::ResponseStarted { model, .. } => {
            println!("started: {}", model);
        }
        LlmStreamEvent::TextDelta { delta } => {
            print!("{delta}");
        }
        LlmStreamEvent::ToolCallDelta { call_id, name, delta } => {
            println!("tool call {call_id} {name}: {delta}");
        }
        LlmStreamEvent::Usage { usage } => {
            println!("usage so far: {}", usage.total());
        }
        LlmStreamEvent::Completed { response } => {
            println!("\nfinal text: {}", response.content_text);
        }
        other => {
            println!("event: {:?}", other);
        }
    }
}
```

### 流语义

- 流会产出 `Result<LlmStreamEvent, GatewayError>`。
- 有些上游会发送一个终态 `Completed` 事件；也有些上游会以 `[DONE]` 或协议特定的停止标记结束。
- 当上游没有提供终态完成事件时，gateway 会按需合成一个 `Completed`，让调用方仍然能拿到最终规范化响应。
- 如果流在 usage 元数据出现之前就结束或失败，gateway 会回退到内部 usage 估算来结算预算，而不是把整笔预留全部退回。

### 取消

可以使用 `CancellationToken` 来停止一个进行中的请求：

```rust
let cancel = CancellationToken::new();
let child = cancel.clone();

tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    child.cancel();
});

let result = gateway.call(request, cancel).await;
```

取消会表现为 `GatewayError::Cancelled`。

## 预算跟踪

预算跟踪是进程内的，并且不依赖锁。

关键点：

- 请求会在派发前预留估算成本
- 最终成本会按实际 usage 结算
- 成功请求可能向上或向下修正结算
- 失败或被截断的流不会自动全额退款；只要可能，gateway 会依据已知或估算出的部分 usage 来结算

可观测性相关方法：

- `gateway.budget_used_usd()`
- `gateway.budget_remaining_usd()`

如果你在 `Gateway` 之外也需要这个底层原语，可以直接使用 `BudgetTracker`。

## Key 池、限流与熔断

每个 key 都会被独立跟踪。

### Key 池负责约束什么

- 使用原子 in-flight 计数器进行 TPM 预留
- 通过滑动窗口做 RPM 准入
- 用随机化选择降低竞争
- 在 provider 返回限流响应时进入冷却
- 在未授权响应后永久标记为不可用
- 在连续 provider 故障后触发熔断

### 可观测性

```rust
for status in gateway.pool_status() {
    println!(
        "{} available={} inflight={}/{} failures={}",
        status.label,
        status.available,
        status.tpm_inflight,
        status.tpm_limit,
        status.consecutive_failures,
    );
}
```

`KeyStatus` 包含：

- `label`
- `available`
- `tpm_inflight`
- `tpm_limit`
- `cool_down_until`
- `failure_cool_down_until`
- `consecutive_failures`

冷却字段使用 Unix epoch 毫秒时间戳。

## 错误处理

对外暴露的运行时错误会统一为 `GatewayError`：

- `NoAvailableKey`
- `BudgetExceeded`
- `RateLimited`
- `Unauthorized`
- `Cancelled`
- `Provider(ProviderError)`
- `Protocol(String)`
- `Http(reqwest::Error)`

一般可以这样理解这些错误：

- `NoAvailableKey`
  当前没有任何健康 key 拥有足够的本地容量。

- `BudgetExceeded`
  你的预算上限在请求派发前就拒绝了这次调用。

- `RateLimited`
  要么是本地 RPM 窗口拒绝了请求，要么是上游 429 被统一归一化成了这个错误。

- `Unauthorized`
  上游返回了 401/403，对应 key 会被标记为永久不可用。

- `Provider`
  请求传输完成了，但 provider 执行失败；或者网络异常被归一化成了 provider 侧故障。

- `Protocol`
  crate 无法按预期解析或输出目标协议 payload。

## 协议解析与输出

当你想直接处理受支持的生成协议时，可以使用这些辅助函数：

- `parse_request`
- `emit_request`
- `parse_response`
- `emit_response`
- `parse_stream_event`
- `emit_stream_event`
- `transcode_request`
- `transcode_response`
- `transcode_stream_event`
- `transcode_error`

示例：

```rust
use omnillm::{transcode_request, ProviderProtocol};

let raw_chat = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{"role": "user", "content": "Hello!"}],
  "max_tokens": 32
}"#;

let raw_responses = transcode_request(
    ProviderProtocol::OpenAiChatCompletions,
    ProviderProtocol::OpenAiResponses,
    raw_chat,
)?;
```

## 多端点 API 层

多端点 API 层是类型化且规范化的。当你想为非生成类端点家族构建转换器或请求发射器时，它会很有用。

### 支持的规范化端点家族

- 生成：`ApiRequest::Responses`
- 向量嵌入：`ApiRequest::Embeddings`
- 图像生成：`ApiRequest::ImageGenerations`
- 音频转写：`ApiRequest::AudioTranscriptions`
- 语音合成：`ApiRequest::AudioSpeech`
- 重排序：`ApiRequest::Rerank`

### 输出传输请求

```rust
use omnillm::{
    emit_transport_request, ApiRequest, EmbeddingInput, EmbeddingRequest, RequestBody, WireFormat,
};

let request = ApiRequest::Embeddings(EmbeddingRequest {
    model: "text-embedding-3-small".into(),
    input: vec![EmbeddingInput::Text { text: "hello".into() }],
    dimensions: Some(256),
    encoding_format: None,
    user: None,
    vendor_extensions: Default::default(),
});

let report = emit_transport_request(WireFormat::OpenAiEmbeddings, &request)?;
assert_eq!(report.value.path, "/embeddings");

if let RequestBody::Json { value } = report.value.body {
    println!("{}", value);
}
```

### 桥接与损耗报告

`ConversionReport<T>` 会告诉你：

- `bridged`
  表示目标 wire format 与规范化端点模型并不原生一致，需要经过桥接。

- `lossy`
  表示有一个或多个字段无法被表达出来。

- `loss_reasons`
  具体说明哪些内容被丢弃了，或发生了怎样的降级。

示例：

```rust
use omnillm::{transcode_api_request, WireFormat};

let raw = r#"{
  "model": "gpt-4.1-mini",
  "messages": [{"role": "user", "content": "Hello!"}],
  "max_tokens": 32
}"#;

let report = transcode_api_request(
    WireFormat::OpenAiChatCompletions,
    WireFormat::OpenAiResponses,
    raw,
)?;

println!("bridged={} lossy={}", report.bridged, report.lossy);
for reason in &report.loss_reasons {
    println!("loss: {}", reason);
}
```

## 内置 Provider 注册表

你可以使用内置注册表查看当前已经建模的 provider 和端点家族：

```rust
use omnillm::{embedded_provider_registry, EndpointKind, ProviderKind, WireFormat};

let registry = embedded_provider_registry();

assert!(registry.supports_endpoint(ProviderKind::OpenAi, EndpointKind::Embeddings));
assert!(registry.supports_wire_format(
    ProviderKind::OpenAi,
    WireFormat::OpenAiResponses,
));
```

这个注册表是元数据，不是运行时 dispatcher。它更适合用在能力发现、配置 UI 和校验逻辑中。

## 回放脱敏

如果你在做 record/replay 风格的测试，可以使用：

- `ReplayFixture`
- `sanitize_transport_request`
- `sanitize_transport_response`
- `sanitize_json_value`

这些辅助函数会脱敏常见敏感信息，例如：

- 鉴权头
- query token 参数
- JSON 中看起来像 key 的敏感字段
- 体积较大的二进制或 base64 blob

示例：

```rust
use omnillm::{sanitize_transport_request, HttpMethod, RequestBody, TransportRequest};
use serde_json::json;

let request = TransportRequest {
    method: HttpMethod::Post,
    path: "/responses?ak=secret".into(),
    headers: [("Authorization".into(), "Bearer secret".into())]
        .into_iter()
        .collect(),
    accept: None,
    body: RequestBody::Json {
        value: json!({
            "api_key": "secret",
            "input": "hello"
        }),
    },
};

let sanitized = sanitize_transport_request(&request);
assert_eq!(sanitized.path, "/responses?ak=<redacted:ak>");
```

## 仓库中附带的示例

在仓库根目录运行：

```sh
cargo run --example basic_usage
cargo run --example multi_endpoint_demo
cargo run --example responses_live_demo
```

它们各自展示的内容如下：

- `basic_usage`
  带预算跟踪和 Key 池状态打印的并发运行时生成调用。

- `multi_endpoint_demo`
  不发起网络请求的前提下，展示类型化请求输出、协议转码、provider 注册表查询以及回放脱敏。

- `responses_live_demo`
  一个完全通过环境变量配置、支持图像输入的实时运行时示例请求。

## 实时示例与实时测试

仓库中附带了 `.env.example`，用于实时运行时示例以及被忽略的实时测试。

典型流程：

```sh
cp .env.example .env
cargo run --example responses_live_demo
```

可选的 `ignored` 测试：

```sh
cargo test responses_vision_demo -- --ignored --nocapture
cargo test responses_function_tool_demo -- --ignored --nocapture
```

## 实践模式

### 1. OpenAI 兼容运行时 Gateway

运行时配置请使用 `ProviderEndpoint::new(...)` 配合 `EndpointProtocol`。
当需要走标准上游路径时使用官方变体；当需要命中包装层自定义完整 URL 时使用 `*_compat` 变体。

### 2. 纯转换服务

如果你在写代理、SDK 适配层或测试工具，可能根本不需要 `Gateway`。这种情况下可以直接使用 `emit_*`、`parse_*` 和 `transcode_*`。

### 3. 安全夹具采集

如果你会把请求/响应夹具落到仓库里，记得在写盘前先做脱敏。

## 故障排查

### 我遇到了 `NoAvailableKey`

可能原因：

- 所有 key 都在冷却中
- 所有 key 都已经失效
- 所有 key 在本地 TPM 上都已经饱和
- 本地熔断器已经对所有 key 打开

请先检查 `gateway.pool_status()`。

### 我比预期更早遇到了 `BudgetExceeded`

要记住 `Gateway` 会在派发前预留估算成本，之后才会进行结算。在流量尖峰期间，当前使用量在结算完成前可能会暂时偏高。

### 我遇到了 `Protocol(...)`

这通常意味着以下几种情况之一：

- 上游请求体结构发生了变化
- 你选择的协议与实际上游不匹配
- 你请求了目标协议无法编码的能力

如果你在做转码，请检查 `loss_reasons`。

### 流在没有 provider usage 元数据的情况下结束了

这在某些上游流式协议形态中是正常现象。`Gateway` 会回退到部分 usage 估算来进行预算结算，并在需要时合成终态 `Completed` 响应。

## API 表面参考

最常用的项目包括：

- 运行时生成：
  `Gateway`, `GatewayBuilder`, `KeyConfig`, `PoolConfig`, `ProviderEndpoint`, `EndpointProtocol`

- 规范化生成类型：
  `LlmRequest`, `LlmResponse`, `LlmStreamEvent`, `Message`, `RequestItem`, `CapabilitySet`

- 转换辅助函数：
  `parse_request`, `emit_request`, `parse_response`, `emit_response`, `transcode_request`, `transcode_response`

- 多端点 API：
  `ApiRequest`, `ApiResponse`, `WireFormat`, `ConversionReport`, `emit_transport_request`, `parse_transport_response`

- 回放脱敏：
  `ReplayFixture`, `sanitize_transport_request`, `sanitize_transport_response`, `sanitize_json_value`

## 推荐阅读顺序

如果你第一次接触这个 crate：

1. 先读主仓库里的 `README.md`
2. 运行 `cargo run --example basic_usage`
3. 阅读这份使用指南
4. 如果需要设计背景，再读 [architecture.md](./architecture.md)
5. 如果需要内部实现，再读 [implementation.md](./implementation.md)
