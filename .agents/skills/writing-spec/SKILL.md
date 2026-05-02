---
name: writing-spec
description: Use when asked to design, create, draft, research, validate, or maintain a Spec System for an AI Coding project, including spec coding, feature drafts, new feature discovery, requirements research, candidate patches, test plans, source of truth, current specs, Snapshots, Patches, Spec Evolution, Spec Graphs, structured Specs, Capability Specs, Contract Specs, Guides, semantics, strong or weak semantics, design principles, SOLID, KISS, DRY, Separation of Concerns, Least Privilege, LLM context pollution, archive/log separation, or turning project rules into one valid Spec truth. Also use when reviewing existing specs or drafts for multiple current truths, narrative-doc pollution, mixed Spec and Guide content, Patch Log replay, stale specs entering LLM context, unclear Spec boundaries, unpromoted Drafts, or candidate Patch conflicts.
---

# Writing Spec

维护一个可演进、可裁剪、single source of truth 的 Spec System。重点不是“写文档”，而是让 AI Coding 项目始终有一个 LLM 可以安全读取的 Current State。

写作风格：遵循本 skill 的 Language Policy；保留关键 technical terms，例如 `Spec`、`LLM`、`Snapshot`、`Patch`、`Compiler`、`Graph Cut`、`Contract`。表达要清楚直接，像工程同学在写内部 engineering rules。

## Language Policy

- If the user's request is in English, respond in English and write specs, plans, task cards, reviews, and other generated artifacts in English.
- If the user's request is in Chinese or another language, respond in that same language and write specs, plans, task cards, reviews, and other generated artifacts in that language.
- If the user explicitly requests a language, or the surrounding context specifies a language, use that language instead of inferring the language from the request.

## Code Writing Boundary

- Do not write, modify, patch, or generate production code, test code, scripts, or other source files unless the user explicitly asks for direct code implementation or code edits.
- When the user asks only for Spec work, produce Spec artifacts, analysis, candidate Spec Patches, validation notes, or review feedback without changing source code.
- It is acceptable to reference code paths, describe interfaces, or include clearly labeled illustrative pseudocode when needed for Spec clarity, but do not apply code changes without explicit user confirmation.
- If implementation would be useful but the user did not ask for coding, ask for confirmation before writing code.

## 核心模型

默认采用这个 target model：

```text
Spec System =
  Current State Snapshot   # 强语义，LLM 默认只读这里
+ Draft Workspace          # 非权威，新功能调研/方案/测试/候选 Patch
+ Guide Layer              # 弱语义，按需解释，不参与默认决策
+ Patch Log                # 系统写入历史，不进默认 context
+ Archive                  # 旧 Snapshot，弱索引，不参与当前决策
+ Spec Compiler            # apply + merge + validate
```

Hard rules：
- `omni-coding/specs/current/` 是唯一有效 Spec。任何时刻，LLM 读到的 Spec 必须是当前唯一正确版本。
- `omni-coding/specs/drafts/` 是非权威 Draft Workspace，用于新功能调研、用户对齐、测试计划、验证证据和候选 Patch；默认不作为当前事实。
- `omni-coding/specs/guides/` 是弱语义 Guide，只能辅助理解，不参与默认决策。
- `omni-coding/specs/log/` 和 `omni-coding/specs/archive/` 默认绝不注入 context，除非用户明确要求历史分析。
- LLM 永远不要 replay Patch Log。Patch 是写入路径，Snapshot 是读取路径。
- Draft 不得直接改变 `current/`。只有用户确认 accept/promote 后，候选 Patch 才能经过 Spec Compiler 写入 `current/`。
- 不允许多个 current truth，例如 `api.md`、`api_v2.md`、`api_final.md`。
- 不允许把 Spec 写成 narrative doc。Spec Object 必须结构化、可 diff、可局部更新。
- 不要把 `Spec` 和 `Guide/Tips/Rationale` 混在一起。建议、背景、推理、examples 放在 Guide，或者放在 `current/` 之外。

## 推荐 Directory Layout

如果目标 repo 没有 Spec System，创建这个 shared `omni-coding/` minimal layout：

```text
omni-coding/
├── specs/
│   ├── current/
│   │   ├── capability/
│   │   ├── system/
│   │   └── contract/
│   ├── drafts/
│   │   ├── open/
│   │   ├── accepted/
│   │   ├── rejected/
│   │   └── superseded/
│   ├── guides/
│   │   ├── capability/
│   │   ├── system/
│   │   └── contract/
│   ├── log/
│   └── archive/
├── plans/
└── tasks/
```

Rules：
- `current/` 只放已经 compile 后的最终态 Snapshot，也就是强语义 Spec。
- `drafts/open/` 放正在调研的新功能 Draft，允许 narrative、analysis、test notes 和候选 Patch，但不参与默认决策。
- `drafts/accepted/`、`drafts/rejected/`、`drafts/superseded/` 是历史 Draft 归档，默认不注入 context。
- `guides/` 只放弱语义 Guide，用于 debug、onboarding、理解背景，默认不作为决策输入。
- `log/` 只放 Spec Patch 历史，不作为 LLM 推理输入。
- `archive/` 只放旧 Snapshot 或 deprecated Spec，不作为当前事实。
- 文件名不带版本号；版本写进 Spec Object 的 `version` 字段。
- 一个文件只放一个 Spec Object 或一个 Guide Object。

可直接复制模板到 `omni-coding/specs/`：`assets/templates/specs/`。

## Spec Object

最小单位是 Spec Object，不是大文档。推荐三类：

- `capability`: 定义系统能做什么，例如 `capability.file.write`、`capability.network.request`。
- `system`: 定义运行环境和全局约束，例如 `system.workspace`、`system.runtime`。
- `contract`: 定义接口、Schema、RPC、事件和跨模块协议，例如 `contract.api.files`。

基础字段：

```yaml
id: capability.file.write
kind: capability
version: 1
status: current
summary: 写入 workspace 内的文件内容

input:
  path: string
  content: string

output:
  written: boolean

constraints:
  max_size: 1MB
  scope: workspace-only

invariants:
  - must_be_atomic
  - must_be_idempotent

semantics:
  must_be_atomic: write is all-or-nothing
  must_be_idempotent: same input produces same final file content

relations:
  depends_on:
    - system.workspace
  extends: []
  constrains: []
```

Writing rules：
- `id` 必须全局唯一，格式推荐 `<kind>.<domain>.<name>`。
- `kind` 必须匹配 `id` 前缀。
- `version` 只表示当前 Snapshot 的对象版本，不出现在文件名里。
- `summary` 只能是一句短 label，不承载行为规则。
- `constraints` 写硬约束，`invariants` 写永远必须成立的性质。
- `semantics` 是可选弱解释层，但 key 必须结构化，并优先绑定已有 invariant 或 constraint。
- `relations` 用来做 Graph Cut，只写真实依赖，不要写“相关但不依赖”。
- `current/` 禁止 `description`、`notes`、`tips`、`rationale`、`examples` 这类 narrative 字段。

## Spec vs Guide

核心心智模型：

```text
Spec  = constraints and facts; must be correct
Guide = explanation and examples; useful but not authoritative
```

Spec 不是“没有描述”，而是把描述压缩成可执行结构。不能结构化、不能验证、不会影响行为的内容，放进 Guide，不要放进 `current/`。

判断口诀：
- 能约束就结构化。
- 能执行就进 Spec。
- 能解释就放 Guide。

字段判断：
1. 能不能结构化？能就放 Spec，不能就放 Guide。
2. 是否影响行为？是就必须结构化。
3. 是否可验证？能就放 Spec，不能就放 Guide。

允许的 Spec 表达：
- `constraints.max_latency: 200ms`
- `constraints.max_size: 1MB`
- `constraints.auth_required: true`
- `invariants: [no_path_escape, atomic_write]`
- `semantics.atomic_write: write must be all-or-nothing`

禁止的 Spec 表达：
- `description: Must be safe, fast, and reliable`
- `constraints: [must be safe, should be fast]`
- `notes: [建议使用 streaming]`

Guide Object 推荐格式：

```yaml
id: guide.capability.file.write
kind: guide
target: capability.file.write
summary: 写文件到 workspace
notes:
  - Use atomic write to avoid partial output.
examples:
  - input:
      path: a.txt
      content: hi
```

Guide rules：
- Guide 必须有 `target` 指向一个 current Spec Object。
- Guide 可以有 `notes`、`examples`、`rationale`，但不得覆盖 Spec 的 constraints 或 invariants。
- Guide 默认不注入 LLM context；只有 debug、onboarding、理解背景时按需加载。
- 如果 Guide 和 Spec 冲突，以 Spec 为准，并生成 Patch 修正其中一个。

## Draft Workspace

Draft 是新功能落入正式 Spec 前的协作工作区。它解决“还没调研清楚，但需要和用户一起收敛”的阶段：需求澄清、方案比较、测试计划、验证证据和候选 Spec Patch。

Draft 不是 Spec truth：
- `drafts/open/` 允许 narrative、research、rationale、test notes、evidence 和多个候选 Patch。
- Draft 中的判断默认不能作为实现依据，除非用户明确要求围绕该 Draft 工作。
- Draft 不得直接改 `current/`；只允许通过 `candidate-patches/` 生成正式 Patch。
- `draft accept` / promote 必须先 dry-run 所有 candidate patches，再统一 compile 到 `current/`。
- 如果 Draft、Guide、Archive 或 Log 和 `current/` 冲突，以 `current/` 为准。

推荐 Draft workspace：

```text
omni-coding/specs/drafts/open/draft.feature.search-v2/
├── draft.yaml
├── research.md
├── analysis.md
├── test-plan.md
├── evidence.md
└── candidate-patches/
    ├── README.md
    ├── 0001.update.contract.search.patch.yaml
    └── 0002.update.capability.search.patch.yaml
```

Draft Object 推荐格式：

```yaml
id: draft.feature.search-v2
kind: draft
status: researching
title: Search V2
problem: 当前搜索能力不支持多字段过滤
goals:
  - support_multi_field_filtering
non_goals:
  - replace_existing_indexer
questions:
  - 是否需要兼容旧 query syntax？
related_specs:
  - capability.search.query
  - contract.api.search
candidate_changes:
  patches:
    - candidate-patches/0001.update.contract.search.patch.yaml
    - candidate-patches/0002.update.capability.search.patch.yaml
validation:
  test_plan: test-plan.md
  evidence: evidence.md
  commands:
    - npm test -- search
```

Draft statuses：
- `researching`: 正在调研和澄清，不要求已有 candidate patches。
- `proposed`: 已有候选 Patch，但还没有完成验证。
- `validated`: candidate patches dry-run 通过，测试计划和证据可供用户确认。
- `accepted`: 用户确认后，candidate patches 已 compile 到 `current/`。
- `rejected`: 用户确认不采纳，保留调研记录。
- `superseded`: 被另一个 Draft 取代。

Draft rules：
- 新功能、需求不清、需要测试开发或用户调研时，默认先创建 Draft，不要直接写 Spec Patch。
- 小型、明确、无争议的 Spec 修正可以直接 Patch，不必强制 Draft。
- Draft 的 `related_specs` 必须引用现有 `current/` Spec；如果缺失，先说明需要创建或拆分哪些 Spec。
- Candidate Patch 必须仍遵守正式 Patch 规则：`create` 提供完整 `spec`，`update` 只写 `changes`，`deprecate` 归档旧对象。
- Draft 被 accept 前，不要声称 Spec 已更新；只能说“Draft 已创建/已验证/可提交确认”。

## Design Principles References

当任务涉及 Spec 边界、Capability 拆分、Contract 兼容、权限 scope、Graph 关系或 abstraction 选择时，按需读取 `references/principles/` 中的英文 Principle 文件。不要一次性全部加载；只加载能影响当前 design decision 的 Principle。

仓库根目录的 `principles.md` 是中文长文参考，适合用户明确要求系统性学习或完整 review 设计原则时读取；日常 Spec 判断仍优先使用本 skill 下拆分后的 principle references，避免把大文档整体注入 context。

可用 references：
- `references/principles/single-responsibility.md`
- `references/principles/open-closed.md`
- `references/principles/liskov-substitution.md`
- `references/principles/interface-segregation.md`
- `references/principles/dependency-inversion.md`
- `references/principles/composite-reuse.md`
- `references/principles/law-of-demeter.md`
- `references/principles/occams-razor.md`
- `references/principles/kiss.md`
- `references/principles/yagni.md`
- `references/principles/dry.md`
- `references/principles/separation-of-concerns.md`
- `references/principles/least-privilege.md`

使用方式：
- 先用 Principle 判断 Spec Object 的边界是否合理。
- 再决定是 split、compose、extend、deprecate，还是只加一个 constraint。
- Principle 只提供判断标准，不替代 Current Spec Snapshot。
- 如果 Principle 和 current Spec 冲突，先生成 Spec Patch，不要直接按 Principle 改代码。

## 默认 Workflow

### 0. 判断是否需要 Draft

先判断用户请求属于哪类：
- 新功能、需求不清、需要用户调研、需要方案比较、需要测试开发或存在多种演进路线：先走 Draft Workflow。
- 明确的小型 Spec 修正、已知 bug 对应的约束修正、用户明确要求直接更新 Spec：可以直接生成 Spec Patch。
- 用户只是要解释、review 或一次性建议：不要落地 Draft 或 Spec，除非用户确认。

Draft Workflow：
1. `draft start`: 创建 `drafts/open/<draft-id>/`，记录 problem、goals、questions、related_specs。
2. `draft research`: 读取相关 `current/` Spec、必要 Guide、代码和测试，写入 `research.md` 和 `analysis.md`。
3. `draft propose`: 在 `candidate-patches/` 中生成一个或多个正式 Spec Patch，并写入 `draft.yaml` 的 `candidate_changes.patches`。
4. `draft validate`: dry-run 所有 candidate patches，检查是否能 compile、relations 是否完整、是否存在 narrative pollution。
5. `draft test`: 把测试计划写入 `test-plan.md`，把执行结果和证据写入 `evidence.md`。
6. 用户确认后 `draft accept`: 事务性应用 candidate patches，写回 `current/`、`log/`、`archive/`，并移动到 `drafts/accepted/`。
7. 用户不采纳时 `draft reject`；被其他 Draft 替代时 `draft supersede`。

Draft 阶段的输出必须明确状态：`researching`、`proposed`、`validated`、`accepted`、`rejected` 或 `superseded`。

### 1. 先读取 Current Snapshot

开始任何 Spec 相关任务时：
1. 先检查 `omni-coding/specs/current/`。
2. 按用户请求定位相关 `spec_id`。
3. 加载目标 Spec Object 和它的 `depends_on`、`extends`、`constrains`。
4. 默认不加载 `omni-coding/specs/guides/`、`omni-coding/specs/log/` 或 `omni-coding/specs/archive/`。
5. 只有用户要求 debug、解释、onboarding、历史分析时，才按需加载 Guide、Log 或 Archive。

如果 repo 还没有 `omni-coding/specs/current/`，先建议初始化最小 Spec System，不要直接写散乱文档。

### 2. 分析影响范围

对 User Request 或 Code Change 做 impact analysis：
- 哪些 `capability` 被改变。
- 哪些 `system` 约束被改变。
- 哪些 `contract` 需要同步。
- 哪些 relations 需要新增、删除或调整。

只修改受影响的 Spec Object。不要顺手重写无关 Spec。

### 3. 生成 Spec Patch

默认生成 Patch，而不是重写整个 Spec：

```yaml
op: update
target: capability.file.write
expected_version: 1
reason: 支持 append 模式并放宽单文件写入限制

changes:
  set:
    constraints.max_size: 5MB
    input.mode: "'overwrite' | 'append'"
  add:
    invariants:
      - must_support_append
```

Patch 规则：
- `create` 才允许提供完整 `spec`。
- `update` 只允许写 `changes`，不要粘贴整个 Spec。
- `deprecate` 表示从 `current/` 移除，并让旧对象进入 `archive/`。
- 如果 `expected_version` 不匹配，必须停下来处理冲突，不要强行覆盖。

### 4. Compile 成 Current State

Patch 必须经过 Spec Compiler：
1. validate Patch。
2. apply 到内存中的 current Spec。
3. merge 并检查冲突。
4. 校验 Spec 没有 narrative pollution。
5. 写回 `omni-coding/specs/current/` 的最终态 Snapshot。
6. 把 Patch 写入 `omni-coding/specs/log/`。
7. 把旧 Snapshot 写入 `omni-coding/specs/archive/`。

本 skill 提供轻量脚本：`scripts/spec_store.py`。

常用命令：

```bash
python3 scripts/spec_store.py init ./omni-coding/specs
python3 scripts/spec_store.py validate ./omni-coding/specs
python3 scripts/spec_store.py read ./omni-coding/specs capability.file.write --with-deps
python3 scripts/spec_store.py read ./omni-coding/specs capability.file.write --with-deps --with-guide
python3 scripts/spec_store.py patch ./omni-coding/specs ./patch.yaml
python3 scripts/spec_store.py draft start ./omni-coding/specs draft.feature.search-v2 --title "Search V2"
python3 scripts/spec_store.py draft research ./omni-coding/specs draft.feature.search-v2 --question "Need legacy syntax?" --note "User needs multi-field filters."
python3 scripts/spec_store.py draft propose ./omni-coding/specs draft.feature.search-v2 candidate-patches/0001.update.contract.search.patch.yaml --note "Use backward-compatible query extension."
python3 scripts/spec_store.py draft test ./omni-coding/specs draft.feature.search-v2 --command "npm test -- search" --evidence "Search tests passed."
python3 scripts/spec_store.py draft validate ./omni-coding/specs draft.feature.search-v2 --require-patches
python3 scripts/spec_store.py draft accept ./omni-coding/specs draft.feature.search-v2 --dry-run
python3 scripts/spec_store.py draft accept ./omni-coding/specs draft.feature.search-v2
python3 scripts/spec_store.py draft reject ./omni-coding/specs draft.feature.search-v2 --reason "out of scope"
```

如果脚本不在目标 repo，可以从本 skill 复制进去，或直接用同等流程手动执行；但原则不能变：LLM 只读 Snapshot，系统只通过 Patch 写入。

## Context Injection

给 LLM 注入 Spec 时使用 Graph Cut：

```text
Query → spec_id → current/spec_id → relations → minimal current subset
```

强制规则：
- 默认只注入 `current/` 的相关子集。
- 不要 `load all specs`，除非项目极小且用户明确要求。
- 不要默认注入 `drafts/`、`guides/`、`archive/` 或 `log/`。
- 只有用户正在处理某个 Draft，或明确要求查看 Draft 时，才加载对应 `drafts/open/<draft-id>/`；不要加载所有 Draft。
- `guides/` 只能辅助理解，不能覆盖 Current Spec 的决策权。
- 不允许基于 archived Spec 做当前设计或代码决策。
- 如果 Draft、Guide、历史信息和 current 冲突，以 current 为准，并说明非 current 内容仅供分析。

## Conflict 和 Consistency

更新前后至少检查：
- `id` 是否唯一。
- `kind` 是否和 `id` 前缀一致。
- relation 指向的 Spec 是否存在于 `current/`。
- Guide 的 `target` 是否存在于 `current/`。
- `current/` 是否含有 narrative 字段或无法验证的 pseudo-constraints。
- 新 constraints 是否和依赖 Spec 的 invariants 冲突。
- Contract 变化是否需要对应 Capability 或 System Spec 同步。
- Draft 的 candidate patches 是否能作为 Patch Set 一次性 dry-run 通过。
- Draft accept 是否已经获得用户确认，是否有测试计划和验证证据。

如果发现冲突：
- 不要 silence merge。
- 不要猜一个“看起来合理”的版本覆盖。
- 输出冲突对象、冲突字段、可选解决方案，让用户确认或生成明确 Patch。

## 什么时候不要落地 Spec

不要机械创建 Spec System：
- 用户只是要一次性解释或建议。
- 改动是非常局部的一次性修复，且项目没有长期 AI Coding 需求。
- 已有规范系统明确不是以 Spec 为 source of truth，且用户没有要求迁移。

即使不落地文件，也要沿用本 skill 的判断：唯一真相、结构化对象、Snapshot 优先、Patch 更新。

## 完成后的输出

完成 Spec 相关操作后，至少说明：
- 读取了哪些 current Spec Object。
- 是否创建、验证、接受、拒绝或 supersede 了哪些 Draft。
- Draft 仍未 accepted 时，明确说明它还不是 current truth。
- 生成或应用了哪些 Patch。
- 更新了哪些 `omni-coding/specs/current/` 文件。
- 是否写入了 `omni-coding/specs/log/` 和 `omni-coding/specs/archive/`。
- 是否新增或更新了 `omni-coding/specs/guides/`，以及这些 Guide 是否只用于解释。
- 是否存在未解决 conflict 或需要用户确认的 evolution decision。

如果没有完成 compile 和 validate，不要把结果描述成“Spec 已更新”。

## Completion Hook

After a Spec update is complete and validated, check whether the downstream planning skill is available:
- If `writing-plan` is installed or otherwise available in the current environment, ask the user whether they want to create an implementation Plan from the updated Spec.
- If `writing-plan` is not installed or not available, tell the user they can use `writing-plan` to create the implementation Plan for the updated Spec.

This hook is advisory only. Do not automatically create a Plan unless the user confirms.
