# OmniLLM Skill

This directory contains the source bundle for the OmniLLM agent skill. It
teaches coding agents how OmniLLM's runtime gateway, protocol transcoding
layer, typed multi-endpoint API, and replay sanitization helpers actually
work.

Install this skill with the [Vercel Labs `skills` installer](https://github.com/vercel-labs/skills).

## What Gets Installed

The skill is declared as `omnillm`. When you install with `--skill omnillm`,
the installer creates the correct target directory name automatically.

Agent runtimes only require:

- `SKILL.md`
- `references/`
- `assets/`

The installer may also place `README.md` alongside those files and create a
project-level `skills-lock.json`.

The commands below use `--copy` so the installed skill stays self-contained in
the target agent directory.

## Choose A Source

From the repository root, you can confirm that the local checkout exposes the
skill:

```sh
npx skills add . --list
```

If you are installing from GitHub instead of a local checkout, replace `.`
with `https://github.com/aiomni/omnillm`.

## Claude Code

From the repository root:

```sh
npx skills add . --skill omnillm --agent claude-code --copy
```

Add `-g` if you want a user-level install.

## Codex

From the repository root:

```sh
npx skills add . --skill omnillm --agent codex --copy
```

Add `-g` if you want a user-level install.

## OpenCode

From the repository root:

```sh
npx skills add . --skill omnillm --agent opencode --copy
```

Add `-g` if you want a user-level install.

## Verify Installation

Use the installer to verify that the skill is present for a given agent:

```sh
npx skills ls -a codex --json
```

Replace `codex` with `claude-code` or `opencode` as needed.

Start a new agent session and ask it to do something OmniLLM-specific, for
example:

- build a `GatewayBuilder` flow
- explain when `Gateway` is the right surface versus `transcode_*`
- debug `NoAvailableKey` or `BudgetExceeded`
- emit an `ApiRequest` as a provider wire format

If the skill does not appear immediately, restart the agent session and rerun
`npx skills ls -a <agent>`.
