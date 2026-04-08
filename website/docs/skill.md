---
title: Skill Guide
description: Install the OmniLLM Skill in Claude Code, Codex, OpenCode, or Claude, and use it for OmniLLM-aware coding and debugging.
label: skill guide
release: v0.1.0
updated: Apr 2026
summary: Installation paths, copy commands, zip packaging, and verification across agent runtimes.
---

# Skill Guide

OmniLLM ships with a first-party agent skill in the repository's
[`skill/` directory](https://github.com/aiomni/omnillm/tree/main/skill). The
skill teaches agents the crate's real boundaries:

- runtime generation through `Gateway`
- protocol parsing, emission, and transcoding through `parse_*`, `emit_*`, and `transcode_*`
- typed multi-endpoint conversion through `ApiRequest`, `ApiResponse`, and `WireFormat`
- replay fixture sanitization through `ReplayFixture` and `sanitize_*`

If you only need the Rust crate, go back to [Usage Guide](./usage.md). This
page is specifically about installing the OmniLLM Skill into coding agents.

## What To Install

Install the skill under a directory named `omnillm`. The source lives in the
repository's `skill/` folder, but the skill's declared name is `omnillm`, and
agents that validate skill names expect the installed directory to match.

The installed skill directory only needs:

- `SKILL.md`
- `references/`
- `assets/`

This repo also includes `skill/README.md` for humans, but agent runtimes do
not require it.

## Claude Code

Claude Code supports both project-local and personal skills:

- project-local: `.claude/skills/omnillm/`
- global: `~/.claude/skills/omnillm/`

From the repository root:

```sh
DEST=.claude/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

Use `~/.claude/skills/omnillm` as `DEST` if you want the skill available in
every project.

## Codex

For Codex, install the skill in an `.agents/skills/` directory:

- repository-local: `.agents/skills/omnillm/`
- global: `~/.agents/skills/omnillm/`

From the repository root:

```sh
DEST=.agents/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

Installing at the repository root makes the skill available from subdirectories
in the same repo.

## OpenCode

OpenCode supports its own skill directory plus Claude-compatible and
agent-compatible locations.

Recommended locations:

- project-local: `.opencode/skills/omnillm/`
- global: `~/.config/opencode/skills/omnillm/`

Compatible alternatives:

- `.claude/skills/omnillm/`
- `~/.claude/skills/omnillm/`
- `.agents/skills/omnillm/`
- `~/.agents/skills/omnillm/`

From the repository root:

```sh
DEST=.opencode/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

## Claude

If you want Claude's uploaded-skill flow instead of local agent directories,
build a zip whose root contains `SKILL.md`, `references/`, and `assets/`:

```sh
cd skill
zip -r ../omnillm-claude-skill.zip SKILL.md references assets
```

Then upload the zip in Claude -> Settings -> Capabilities -> Skills -> Upload.

## Verify Installation

Start a new session in your chosen agent and ask for something OmniLLM-specific,
for example:

- scaffold a `GatewayBuilder` flow with `ProviderEndpoint` and `KeyConfig`
- explain when `Gateway` is correct versus `transcode_*`
- debug `NoAvailableKey`, `BudgetExceeded`, or `Protocol(...)`
- emit an `ApiRequest` into a provider wire format

If the skill does not appear immediately, restart the session and verify that
the install directory is named `omnillm`.
