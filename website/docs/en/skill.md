---
title: Skill Guide
description: Install the OmniLLM Skill in Claude Code, Codex, or OpenCode with the Vercel Labs skills installer, and use it for OmniLLM-aware coding and debugging.
label: skill guide
release: v0.1.0
updated: Apr 2026
summary: Vercel Labs skills CLI commands for GitHub-based installation and verification across agent runtimes.
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

## Install With Vercel Labs Skills

These instructions use the [Vercel Labs `skills` installer](https://github.com/vercel-labs/skills).

The skill is declared as `omnillm`. When you install with `--skill omnillm`,
the installer creates the correct target directory name automatically.

Agent runtimes only require:

- `SKILL.md`
- `references/`
- `assets/`

The installer may also add `README.md` next to the skill files and a
project-level `skills-lock.json`.

The commands below install directly from GitHub, so you do not need to clone
the repository first.

The commands below use `--copy` so the installed skill stays self-contained in
the target agent directory.

## Claude Code

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent claude-code --copy
```

Add `-g` for a user-level install.

## Codex

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent codex --copy
```

Add `-g` for a user-level install.

## OpenCode

```sh
npx skills add https://github.com/aiomni/omnillm --skill omnillm --agent opencode --copy
```

Add `-g` for a user-level install.

## Verify Installation

Use the installer to confirm that the skill is present for the agent you care
about:

```sh
npx skills ls -a codex --json
```

Replace `codex` with `claude-code` or `opencode` as needed.

Then start a new session in your chosen agent and ask for something
OmniLLM-specific, for example:

- scaffold a `GatewayBuilder` flow with `ProviderEndpoint` and `KeyConfig`
- explain when `Gateway` is correct versus `transcode_*`
- debug `NoAvailableKey`, `BudgetExceeded`, or `Protocol(...)`
- emit an `ApiRequest` into a provider wire format

If the skill does not appear immediately, restart the session and rerun
`npx skills ls -a <agent>`.
