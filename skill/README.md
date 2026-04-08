# OmniLLM Skill

This directory contains the source bundle for the OmniLLM agent skill. It
teaches coding agents how OmniLLM's runtime gateway, protocol transcoding
layer, typed multi-endpoint API, and replay sanitization helpers actually
work.

Important: install this skill under a directory named `omnillm`. The repo
folder is named `skill/` for packaging convenience, but the skill's declared
name is `omnillm`.

## What To Copy

The installed skill directory only needs:

- `SKILL.md`
- `references/`
- `assets/`

This `README.md` is for humans. Agent runtimes do not require it.

## Claude Code

Project-local:

- `.claude/skills/omnillm/`

Global:

- `~/.claude/skills/omnillm/`

From the repository root:

```sh
DEST=.claude/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

Use `~/.claude/skills/omnillm` as `DEST` if you want the skill available in
every project.

## Codex

Repository-local:

- `.agents/skills/omnillm/`

Global:

- `~/.agents/skills/omnillm/`

From the repository root:

```sh
DEST=.agents/skills/omnillm
mkdir -p "$DEST"
cp -R skill/SKILL.md skill/references skill/assets "$DEST"/
```

If you install under the repository root, Codex can discover the skill from
subdirectories in the same repo.

## OpenCode

Preferred project-local:

- `.opencode/skills/omnillm/`

Preferred global:

- `~/.config/opencode/skills/omnillm/`

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

Start a new agent session and ask it to do something OmniLLM-specific, for
example:

- build a `GatewayBuilder` flow
- explain when `Gateway` is the right surface versus `transcode_*`
- debug `NoAvailableKey` or `BudgetExceeded`
- emit an `ApiRequest` as a provider wire format

If the skill does not appear immediately, restart the agent session and check
that the install directory is named `omnillm`.
