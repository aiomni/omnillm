# Spec System

本目录位于 `omni-coding/specs/`，把项目规则维护成唯一有效的 Current State Snapshot。

## Hard Rules

- `current/` 是 LLM 默认唯一可读的 Spec source of truth。
- `guides/` 是弱语义解释层，默认不进 LLM decision context。
- `log/` 是 Patch Log，默认不进 LLM context。
- `archive/` 是历史 Snapshot，默认不参与当前决策。
- LLM 不 replay Patch Log，只读取 compile 后的 Snapshot。
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

1. 读取 `current/` 的相关 Spec Object。
2. 生成 Spec Patch。
3. 使用 Spec Compiler apply + merge + validate。
4. 写回 `current/`，记录 `log/`，归档旧 Snapshot 到 `archive/`。
5. 只有 debug、onboarding、理解背景时，才按需读取 `guides/`。
