---
id: task-prompt-cache-008
title: Update prompt cache documentation
status: done
priority: P1
tags: [docs, skill, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
  - task-prompt-cache-002
  - task-prompt-cache-003
  - task-prompt-cache-004
  - task-prompt-cache-005
  - task-prompt-cache-006
  - task-prompt-cache-007
blocks: []
---

# Background
Prompt cache support changes public capability semantics. README, website docs, and bundled skill references must describe actual typed support, provider-specific behavior, limitations, and telemetry-driven verification.

# Goal
- Document how to use prompt cache policy for OpenAI and Claude.
- Document BestEffort versus Required semantics and how to observe cache hits.
- Update bundled skill/reference docs so AI coding guidance matches the implementation.

# Execution Steps
- [x] Update README with prompt cache support status, minimal examples, and limitations.
- [x] Update website English and Chinese usage/architecture docs with provider-specific behavior.
- [x] Update `skill/` reference material with task-relevant API examples.
- [x] Add examples for direct policy use and Prefix Builder use if task 006 lands.
- [x] Document cache telemetry fields and pricing behavior from tasks 002 and 007.
- [x] Document unsupported provider behavior, BestEffort loss reports, and Required errors from task 005.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- README acceptance: users can identify whether OpenAI and Claude prompt cache are typed-supported and how to enable them.
- Website acceptance: English and Chinese docs explain provider differences without claiming guaranteed cache hits.
- Skill acceptance: bundled skill references expose the correct public API and caveats.
- Pricing acceptance: docs state that estimate-time pricing does not assume cache hits.

## Task Completion Acceptance Criteria
- Docs match implemented API names, test-proven behavior, and current Spec constraints.
- No docs claim support for provider behavior that remains unsupported or BestEffort-only.
- Task dashboard and project page are synchronized before marking this task done.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: if any implementation task ships a reduced scope, docs must explicitly reflect the reduced scope instead of preserving aspirational Plan language.
- Recommended action: use test fixtures and public reexports as the source for examples.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: User-facing docs and skill references now describe typed prompt cache support, provider differences, telemetry, bridge behavior, and pricing caveats.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `README.md`, `website/docs/en/usage.md`, `website/docs/zh/usage.md`, `skill/references/api-reference.md`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `contract.public_api.surface`.
- Primary files: `README.md`, `website/docs/en/*`, `website/docs/zh/*`, `skill/*`.
