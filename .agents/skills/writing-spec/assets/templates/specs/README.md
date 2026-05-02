# Spec System

本目录位于 `omni-coding/specs/`，把项目规则维护成唯一有效的 Current State Snapshot。

## Hard Rules

- `current/` 是 LLM 默认唯一可读的 Spec source of truth。
- `drafts/` 是非权威的新功能 Draft Workspace，用于调研、方案、测试计划、验证证据和候选 Patch。
- `guides/` 是弱语义解释层，默认不进 LLM decision context。
- `log/` 是 Patch Log，默认不进 LLM context。
- `archive/` 是历史 Snapshot，默认不参与当前决策。
- LLM 不 replay Patch Log，只读取 compile 后的 Snapshot。
- Draft 不直接改变 `current/`；只有用户确认 accept 后，candidate patches 才能经过 Spec Compiler 写入 `current/`。
- 一个文件只放一个 Spec Object 或 Guide Object，文件名不带版本号。

## Spec vs Guide

```text
Spec  = constraints and facts; must be correct
Guide = explanation and examples; useful but not authoritative
```

- 能约束就结构化。
- 能执行就进 Spec。
- 能解释就放 Guide。

`current/` 中禁止 `description`、`notes`、`tips`、`rationale`、`examples` 这类 narrative 字段。必要解释用 `semantics` 的结构化 key，或放入 `guides/`。

## Workflow

1. 如果是新功能、需求调研、方案比较或需要测试开发，先在 `drafts/open/` 创建 Draft。
2. 在 Draft 中写 `research.md`、`analysis.md`、`test-plan.md`、`evidence.md` 和 `candidate-patches/`。
3. 用户确认 accept 前，Draft 不是 current truth。
4. 明确的小型 Spec 修正可以直接读取 `current/` 的相关 Spec Object 并生成 Spec Patch。
5. 使用 Spec Compiler apply + merge + validate。
6. 写回 `current/`，记录 `log/`，归档旧 Snapshot 到 `archive/`。
7. 只有处理 Draft、debug、onboarding、理解背景时，才按需读取 `drafts/` 或 `guides/`。

## Draft Object

```yaml
id: draft.feature.example
kind: draft
status: researching
title: Example Feature
problem: 待调研的问题
goals: []
non_goals: []
questions: []
related_specs: []
candidate_changes:
  patches: []
validation:
  test_plan: test-plan.md
  evidence: evidence.md
  commands: []
```

Draft statuses: `researching`, `proposed`, `validated`, `accepted`, `rejected`, `superseded`.
