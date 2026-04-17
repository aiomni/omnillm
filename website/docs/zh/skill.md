---
title: 技能指南
description: 通过 Vercel Labs skills 安装器把 OmniLLM Skill 安装到 Claude Code、Codex 或 OpenCode 中，并将其用于 OmniLLM 相关编码与调试。
label: 技能指南
release: v0.1.4
updated: 2026 年 4 月
summary: 通过 GitHub 源安装 OmniLLM Skill 的 Vercel Labs skills CLI 命令，以及不同 agent 的验证方法。
---

# 技能指南

OmniLLM 在仓库的
[`skill/` 目录](https://github.com/aiomni/omnillm/tree/main/skill)
中提供了一份官方智能体技能。它会把这个库的真实边界教给各类智能体：

- 通过 `Gateway`、`ProviderEndpoint` 与 `EndpointProtocol` 进行运行时生成调用
- 通过 `parse_*`、`emit_*` 与 `transcode_*` 完成协议解析、输出和转码
- 通过 `ApiRequest`、`ApiResponse` 与 `WireFormat` 完成类型化多端点转换
- 通过 `ReplayFixture` 与 `sanitize_*` 完成回放夹具脱敏

如果你只需要 Rust 库，请返回 [使用指南](./usage.md)。这一页只讨论如何把 OmniLLM Skill 安装到编码智能体中。

## 使用 Vercel Labs Skills 安装

下面的命令统一使用 [Vercel Labs `skills` 安装器](https://github.com/vercel-labs/skills)。

这个技能声明名是 `omnillm`。只要在命令里传 `--skill omnillm`，
安装器就会自动创建正确的目标目录名。

智能体运行时实际只需要：

- `SKILL.md`
- `references/`
- `assets/`

安装器还可能额外写入 `README.md`，并在项目根目录生成
`skills-lock.json`。

下面的命令统一直接从 GitHub 安装，所以不需要先 clone 这个仓库。

下面的命令统一带上 `--copy`，这样安装后的 skill 会保持为目标 agent
目录中的一份独立副本。

## Claude Code

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent claude-code --copy
```

如果你希望安装到用户级位置，请追加 `-g`。

## Codex

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent codex --copy
```

如果你希望安装到用户级位置，请追加 `-g`。

## OpenCode

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent opencode --copy
```

如果你希望安装到用户级位置，请追加 `-g`。

## 验证安装

先用安装器确认某个 agent 已经能看到这个技能：

```sh
npx skills ls -a codex --json
```

把 `codex` 替换成 `claude-code` 或 `opencode` 即可。

然后在你选择的智能体中开启一个新会话，并提出一个 OmniLLM 相关的问题，例如：

- 用 `ProviderEndpoint` 和 `KeyConfig` 搭一个 `GatewayBuilder` 流程
- 给某个要求 `messages[].content[]` 的 OpenAI 兼容包装层配置 `EndpointProtocol::*_compat` 运行时端点
- 排查某个 OpenAI Chat compat 流里 `delta.role` 和首段 `delta.content` 落在同一个 SSE frame 时的首段正文丢失问题
- 用 `LlmRequest.vendor_extensions` 透传 `enable_thinking` 这类包装层特有的 OpenAI 顶层字段
- 解释什么时候应该使用 `Gateway`，什么时候应该直接用 `transcode_*`
- 排查 `NoAvailableKey`、`BudgetExceeded` 或 `Protocol(...)`
- 把一个 `ApiRequest` 输出成 provider 的传输格式

如果技能没有立即出现，请重启会话，并重新执行
`npx skills ls -a <agent>`。
