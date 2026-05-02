---
id: task-prompt-cache-007
title: Add cache-aware pricing
status: done
priority: P1
tags: [pricing, budget, prompt-cache]
project: prompt-cache
due: null
parent: null
depends_on:
  - task-prompt-cache-001
  - task-prompt-cache-002
blocks:
  - task-prompt-cache-008
---

# Background
Gateway budget settlement currently uses uncached prompt and completion token rates. Prompt cache discounts or write premiums must only be applied after provider usage telemetry is available and provider-specific rates are known.

# Goal
- Preserve conservative uncached pre-request cost estimates.
- Compute actual cost using cache telemetry when provider-specific cache rates are available.
- Fall back safely to existing uncached pricing when cache rates are unavailable.

# Execution Steps
- [x] Extend pricing data structures to represent cache read and cache write rates without breaking existing model prefixes.
- [x] Update `pricing::actual` to use prompt cache telemetry only when rates and usage fields are present.
- [x] Ensure `pricing::estimate` continues to avoid pre-response cache discounts.
- [x] Add tests for OpenAI cached input tokens, Claude cache read tokens, Claude cache creation tokens, mixed cached/uncached prompt tokens, and unknown rate fallback.
- [x] Verify Gateway budget settlement behavior still refunds/settles correctly for success, error, cancellation, and partial stream paths.

# Acceptance Criteria
## Step-Level Acceptance Criteria
- Estimate acceptance: estimated cost remains conservative and does not assume cache hits.
- Actual acceptance: actual cost accounts for provider-reported cache read/write usage when rates are configured.
- Fallback acceptance: unknown model or missing cache rate uses existing uncached pricing behavior.
- Gateway acceptance: budget settlement invariants from `capability.budget.cost` remain valid.

## Task Completion Acceptance Criteria
- Cache-aware pricing is covered by focused unit tests.
- No pricing discount is applied without provider usage telemetry.
- Docs task can state exactly how cost accounting behaves.

# Dynamic Adjustments
- Current discovery: none yet.
- Downstream impact: if provider pricing is too volatile for hardcoded rates, create a follow-up task for provider pricing registry instead of blocking telemetry support.
- Recommended action: prioritize correctness and fallback behavior over exhaustive model-rate coverage.

# Execution Log
## 2026-05-02
- Created the task card from `plan.prompt_cache.implementation`.
- Implemented and verified: Actual cost settlement is cache-aware only when provider telemetry and known cache rates are present; estimates remain conservative.
- Validation: `cargo fmt` and `cargo test` pass.

# Review
- Review status: completed.
- Conclusion: Accepted; task acceptance criteria are satisfied by implementation in `src/pricing.rs`, `src/gateway.rs`.
- Adjustments to downstream tasks, `index.md`, or project page: synchronized; all prompt-cache tasks are now marked done.

# Notes
- Source specs: `contract.prompt_cache.policy`, `capability.budget.cost`.
- Primary files: `src/pricing.rs`, `src/gateway.rs`, tests touching budget settlement.
