# Primitive Protocol 双协议架构

## Scope
- 保留 OpenAI Responses 作为 OmniLLM 标准 canonical protocol。
- 继续支持现有 `Gateway::call` / `Gateway::stream` 调用方式访问各个 LLM Provider。
- 新增 provider primitive protocol，让调用者直接发送 provider-native payload，不经过 `LlmRequest` / `LlmResponse` / `ApiRequest` / `ApiResponse` 转换。
- 统一 token budget：canonical 和 primitive 两条路径共用预算估算、预留、结算、取消和 stream/realtime partial usage 规则。
- 参考 OpenAI openai-go、Anthropic TypeScript SDK、Google go-genai、LangChain、ZeroClaw providers、Hermes providers runtime 设计 provider API 覆盖和运行时边界。

## Out Of Scope
- 替换 OpenAI Responses 标准协议。
- 删除现有 generation transcoding 或 provider compat path。
- 在第一阶段承诺所有 provider SDK 的完整 parity。
- 新增第二套 budget/accounting 系统。
- 把 primitive payload 强制转换成 provider-neutral schema。

## Constraints
- Source of truth: `contract.provider.primitive_protocol`、`capability.primitive_provider.execute`、`capability.budget.cost`。
- Canonical path 默认行为必须保持 backward compatible。
- Primitive path 必须显式进入，不能影响现有 Gateway 构造和调用。
- Primitive request/response body 是 provider-native source of truth。
- Usage extraction 只能作为 side-channel telemetry，不能改写 primitive payload。
- Unsupported primitive endpoint 必须在 key/RPM/budget/network 之前失败。
- Budget reservation 和 settlement 对每条执行路径只能发生一次。

## Current State
- Done: OpenAI Responses canonical path remains default and existing canonical tests pass.
- Done: Primitive public model and mode boundary are implemented and re-exported.
- Done: Primitive provider registry and support matrix are implemented.
- Done: Primitive non-stream execution path is implemented with raw payload preservation.
- Done: Primitive usage projection and unified budget settlement are implemented.
- Done: OpenAI, Anthropic, Gemini/Vertex, OpenAI-compatible, Bedrock planned, and Custom HTTP support statuses are represented.
- Done: Primitive SSE stream support is implemented; realtime is an explicit scaffold error.
- Done: README, website docs, skill reference, and example are updated.
- Done: Final validation includes fmt, targeted primitive/API tests, full tests, example compilation, and spec/task YAML checks.

## Milestones
- Milestone 1: Canonical guardrails and public mode boundary — done (`task-primitive-protocol-001`, `task-primitive-protocol-002`).
- Milestone 2: Primitive registry and non-stream execution — done (`task-primitive-protocol-003`, `task-primitive-protocol-004`).
- Milestone 3: Unified budget and usage projection — done (`task-primitive-protocol-005`).
- Milestone 4: Provider API family coverage — done (`task-primitive-protocol-006`, `task-primitive-protocol-007`, `task-primitive-protocol-008`).
- Milestone 5: Streaming/realtime and release readiness — done (`task-primitive-protocol-009`, `task-primitive-protocol-010`).

## Key Dependencies
- All primitive protocol task dependencies are resolved.
- Realtime remains intentionally scaffolded until a future task adds full WebSocket/WebRTC lifecycle support.
- Bedrock remains planned registry support, not dispatch-enabled support.

## Acceptance Signals
- Existing canonical Gateway/API tests pass unchanged.
- Primitive calls preserve provider-native request/response bodies without canonical conversion.
- OpenAI, Anthropic, Gemini, and OpenAI-compatible provider slices each have explicit support status and tests.
- Budget tests prove reserve/refund/settle-once semantics for success, provider error, local rejection, cancellation, stream EOF, and realtime scaffold behavior.
- Stream fixtures cover OpenAI, Anthropic, and Gemini SSE usage extraction.
- Docs and examples clearly explain when to use OpenAI Responses canonical mode versus provider primitive mode.

## Final Acceptance Criteria
- The implementation satisfies `contract.provider.primitive_protocol` and `capability.primitive_provider.execute` for non-stream calls and SSE streams, with realtime explicitly scaffolded.
- OpenAI Responses remains the standard protocol and default user-facing path.
- Provider primitive APIs are additive and do not require existing users to change code.
- Token budget is unified across both protocol forms.
- Task cards, project page, and dashboard are synchronized.
- Status: completed on 2026-05-02.
