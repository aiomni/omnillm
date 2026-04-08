---
title: 技能指南
description: 在 Claude Code、Codex、OpenCode 或 Claude 中安装 OmniLLM Skill，并将其用于 OmniLLM 相关编码与调试。
label: 技能指南
release: v0.1.0
updated: 2026 年 4 月
summary: 安装路径、复制命令、zip 压缩包打包方式，以及在不同 agent 运行环境中的验证方法。
---

# 技能指南

OmniLLM 在仓库的
[`skill/` 目录](https://github.com/aiomni/omnillm/tree/main/skill)
中提供了一份官方智能体技能。它会把这个库的真实边界教给各类智能体：

- 通过 `Gateway` 进行运行时生成调用
- 通过 `parse_*`、`emit_*` 与 `transcode_*` 完成协议解析、输出和转码
- 通过 `ApiRequest`、`ApiResponse` 与 `WireFormat` 完成类型化多端点转换
- 通过 `ReplayFixture` 与 `sanitize_*` 完成回放夹具脱敏

如果你只需要 Rust 库，请返回 [使用指南](./usage.md)。这一页只讨论如何把 OmniLLM Skill 安装到编码智能体中。

## 需要安装什么

请把这个技能安装到名为 `omnillm` 的目录下。源码位于仓库中的 `skill/` 目录，但 skill 声明名是 `omnillm`，那些会校验 skill 名称的智能体也会要求安装目录与之保持一致。

安装后的技能目录只需要包含：

- `SKILL.md`
- `references/`
- `assets/`

这个仓库还包含 `skill/README.md` 供人类阅读，但智能体运行环境并不需要它。

## Claude Code

Claude Code 同时支持项目级和个人级技能目录：

- 项目级：`.claude/skills/omnillm/`
- 全局级：`~/.claude/skills/omnillm/`

在仓库根目录执行：

```sh
DEST=.claude/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

如果你希望这个技能在所有项目中可用，请把 `DEST` 改成 `~/.claude/skills/omnillm`。

## Codex

对于 Codex，请把这个技能安装到 `.agents/skills/` 目录中：

- 仓库级：`.agents/skills/omnillm/`
- 全局级：`~/.agents/skills/omnillm/`

在仓库根目录执行：

```sh
DEST=.agents/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

如果把这个技能安装在仓库根目录，它也会对同一仓库下的子目录生效。

## OpenCode

OpenCode 既支持自己的技能目录，也兼容 Claude 风格和 `.agents` 风格的安装位置。

推荐位置：

- 项目级：`.opencode/skills/omnillm/`
- 全局级：`~/.config/opencode/skills/omnillm/`

兼容的替代位置：

- `.claude/skills/omnillm/`
- `~/.claude/skills/omnillm/`
- `.agents/skills/omnillm/`
- `~/.agents/skills/omnillm/`

在仓库根目录执行：

```sh
DEST=.opencode/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

## Claude

如果你想使用 Claude 的上传式技能流程，而不是本地智能体目录，可以构建一个 zip 压缩包，要求压缩包根目录直接包含 `SKILL.md`、`references/` 和 `assets/`：

```sh
cd skill
zip -r ../omnillm-claude-skill.zip SKILL.md references assets
```

然后在 Claude 中依次进入 `Settings -> Capabilities -> Skills -> Upload` 上传这个 zip 压缩包。

## 验证安装

在你选择的智能体中开启一个新会话，然后提出一个 OmniLLM 相关的问题，例如：

- 用 `ProviderEndpoint` 和 `KeyConfig` 搭一个 `GatewayBuilder` 流程
- 解释什么时候应该使用 `Gateway`，什么时候应该直接用 `transcode_*`
- 排查 `NoAvailableKey`、`BudgetExceeded` 或 `Protocol(...)`
- 把一个 `ApiRequest` 输出成 provider 的传输格式

如果技能没有立即出现，请重启会话，并确认安装目录名称就是 `omnillm`。
