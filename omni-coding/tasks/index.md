# Tasks Dashboard

Last updated: 2026-05-02

Guide: see `omni-coding/tasks/README.md`
Project: `omni-coding/tasks/projects/prompt-cache.md`
Plan: `omni-coding/plans/current/plan.prompt-cache.md`

## Doing
- No active tasks.

## Todo
- No todo tasks.

## Blocked
- No blocked tasks.

## Done
- `task-prompt-cache-001`: Define prompt cache canonical types
- `task-prompt-cache-002`: Parse prompt cache usage telemetry
- `task-prompt-cache-003`: Emit OpenAI prompt cache fields
- `task-prompt-cache-004`: Emit Claude cache_control breakpoints
- `task-prompt-cache-005`: Apply prompt cache bridge semantics
- `task-prompt-cache-006`: Add prompt prefix builder
- `task-prompt-cache-007`: Add cache-aware pricing
- `task-prompt-cache-008`: Update prompt cache documentation

## Critical Path
- Completed: canonical model, telemetry, provider-native emission, bridge semantics, prefix builder, cache-aware pricing, and documentation.

## Milestones
- Milestone 1: canonical model and telemetry — done.
- Milestone 2: provider-native emission — done.
- Milestone 3: safe cross-provider behavior — done.
- Milestone 4: cost and release readiness — done.

## Final Acceptance
- Prompt cache policy can be represented with BestEffort and Required semantics.
- OpenAI and Claude provider-native prompt cache fields are emitted and usage telemetry is parsed.
- Unsupported providers or unsupported policy shapes produce either lossy ConversionReport metadata or explicit errors according to policy.
- Budget actual cost can account for provider-reported cache usage when provider-specific rates are available.
- README, website docs, and bundled skill reference describe current support and limitations.
- Validation completed with `cargo fmt` and `cargo test`.
