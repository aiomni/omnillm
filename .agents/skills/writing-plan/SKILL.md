---
name: writing-plan
description: Use when asked to create, review, or refine an implementation strategy that bridges Specs and executable Tasks, including technical approach, sequencing, orchestration, dependencies, milestones, migration paths, rollout strategy, tradeoffs, risk mitigation, validation strategy, or converting a Spec into a concrete execution path before task decomposition. Also use when deciding how work should be staged without rewriting the Spec or prematurely creating atomic task cards.
---

# Writing Plan

Convert `Spec` into an execution strategy without prematurely creating atomic `Task` cards. A Plan answers: how will we satisfy the Spec, what path will we take, what depends on what, how will we validate the path, and when should work be handed off to the task layer?

Core boundary:
- `Spec` defines what the system is and what constraints must hold. It is the source of truth.
- `Plan` defines how to satisfy the Spec. It is strategy plus orchestration.
- `Task` defines what to execute now. It is the smallest schedulable execution unit.

## Bundled Assets

This skill includes reusable Plan templates under `assets/templates/plans/`:
- `README.md` for project-level planning guidance.
- `plan.md` for a single strategy-level Plan.

Use these assets when the target repository has no existing planning convention and the Plan should be persisted. Copy only the needed templates, replace placeholders immediately, and adapt naming to the target repository. Do not force a new planning layout when the repository already has one.

Default persisted output location for new repositories: `omni-coding/plans/`. Keep Plan artifacts under this shared `omni-coding/` workspace so they sit next to `omni-coding/specs/` and `omni-coding/tasks/`.

## References

Read `references/plan-patterns.md` when a Plan needs a known strategy shape, such as compatibility-first migration, strangler replacement, vertical slice delivery, risk-first spike, or rollout with guardrails. Do not load it for simple Plans where the default workflow is enough.

## Language Policy

- If the user's request is in English, respond in English and write specs, plans, task cards, reviews, and other generated artifacts in English.
- If the user's request is in Chinese or another language, respond in that same language and write specs, plans, task cards, reviews, and other generated artifacts in that language.
- If the user explicitly requests a language, or the surrounding context specifies a language, use that language instead of inferring the language from the request.

## When To Use

Use this skill when:
- The user asks for an implementation strategy derived from a Spec, requirement, or design constraint.
- The user asks for a technical approach, migration path, rollout plan, or staged delivery path.
- The user asks to reason about ordering, dependencies, orchestration, or the critical path.
- The user asks to analyze tradeoffs, risks, validation strategy, or rollback strategy.
- The user needs a strategy before work is decomposed into executable task cards.

Do not use this skill when:
- The work is to define system truth, interfaces, inputs, outputs, invariants, or permission boundaries; use `writing-spec`.
- The work is already about task cards, execution logs, dashboards, status sync, acceptance, or review; use `writing-tasks`.
- The work is a small one-off change that does not need a durable strategy; provide a short action sequence instead.

## Plan Boundary

A Plan may contain:
- Goal and non-goals.
- Source Spec IDs.
- Assumptions and open questions.
- Technical approach and tradeoffs.
- Phases, ordering, dependencies, and orchestration.
- Risks, mitigations, validation, and rollback strategy.
- Candidate Tasks that can be created later.

A Plan must not contain:
- New system truth or hidden hard constraints.
- Interfaces, schemas, permissions, or invariants that have not been synchronized into the Spec layer.
- Atomic execution logs.
- Fully schedulable task cards.
- Background material that does not affect the strategy.

If a Plan reveals that a Spec is missing, stale, or contradictory, propose a Spec Patch. Do not hide missing truth inside the Plan.

## Default Workflow

### 1. Align With Spec And Goal

Before writing the Plan, answer:
1. Which Specs, requirements, or constraints does this Plan serve?
2. What final acceptance signal must the strategy satisfy?
3. Which constraints are hard requirements, and which are implementation choices?
4. Which unresolved questions could change the route?

If the target repository has `omni-coding/specs/current/`, cite the relevant current Spec Objects. Do not read `omni-coding/specs/log/` or `omni-coding/specs/archive/` unless the user asks for historical analysis.

### 2. Stay At Strategy Level

Plan phases should describe coarse delivery paths, such as:
- compatibility layer
- data migration
- read-path rollout
- cleanup and deprecation

Do not write phases as atomic implementation steps, such as:
- edit line 12 in `foo.ts`
- add one helper function
- run one test command

If a step has independent inputs, outputs, acceptance criteria, retry boundaries, and scheduling value, list it under `task_candidates` for the task layer.

### 3. Make Dependencies Explicit

Separate dependency types:
- `hard_dependencies`: prerequisites that block progress if missing.
- `soft_dependencies`: useful inputs that improve quality or efficiency but do not block progress.
- `phase_dependencies`: ordering relationships between phases.
- `external_dependencies`: people, systems, permissions, release windows, or third-party services.

Only write real dependencies. Do not label loosely related items as dependencies.

### 4. Capture Tradeoffs And Risks

For each important technical choice, state:
- What was chosen.
- What was rejected.
- Why the chosen route fits the current constraints.
- How to adjust if an assumption fails.

Every risk must have a mitigation, validation method, rollback path, or explicit owner. Do not write risk entries that only say a risk exists.

### 5. Emit Task Candidates, Not Task Cards

A Plan may end with `task_candidates` for later use by `writing-tasks`:
- Each candidate has a clear output.
- Each candidate can be independently accepted.
- Each candidate maps to a phase.
- The Plan does not track task status, execution logs, or reviews.

## Recommended Plan Object

```yaml
id: plan.auth.session-migration
kind: plan
status: draft
source_specs:
  - capability.auth.session
  - contract.api.auth
goal: Migrate to the new session contract without breaking the existing login flow.
non_goals:
  - Redesigning the authorization model.
  - Changing third-party OAuth providers.
assumptions:
  - Legacy session tokens remain valid during migration.
strategy:
  summary: Add compatibility first, enable dual writes, roll out reads gradually, then remove the legacy path.
  approach:
    - Add a new session adapter.
    - Enable dual writes.
    - Add a read-path rollout switch.
    - Remove legacy entry points after validation.
phases:
  - id: phase-1
    name: compatibility layer
    output: New and legacy sessions can coexist.
    depends_on: []
  - id: phase-2
    name: gradual read migration
    output: Read traffic can switch to the new session path by rollout control.
    depends_on:
      - phase-1
dependencies:
  hard:
    - The new session schema exists in contract.api.auth.
  soft:
    - Login-path observability is available before rollout.
risks:
  - risk: Legacy token state may diverge from new session state.
    mitigation: Add consistency-check logs during the dual-write period.
validation:
  - Unit tests cover adapter behavior.
  - Integration tests cover login, refresh, and logout.
  - Rollout monitoring tracks session mismatch rate.
task_candidates:
  - Define the session adapter interface.
  - Implement the dual-write path.
  - Add the read-path rollout switch.
```

## Persistence Guidance

If the target project already has a planning convention, follow it. Do not force a new directory layout.

If the target project has no convention and the Plan must be persisted, use the templates in `assets/templates/plans/` and prefer the shared `omni-coding/` structure:

```text
omni-coding/
├── specs/
├── plans/
│   ├── README.md
│   ├── current/
│   │   └── plan.<domain>.<name>.md
│   └── archive/
└── tasks/
```

Rules:
- `omni-coding/plans/current/` contains active strategy.
- `omni-coding/plans/archive/` contains obsolete strategy and is not a current execution source by default.
- File names should not carry version numbers; status and version belong inside the Plan object.
- Plans may reference `omni-coding/specs/current/` and `omni-coding/tasks/`, but should not duplicate their contents.

## Final Response Checklist

After Plan work, report:
- Which Specs, requirements, or goals the Plan serves.
- The chosen technical path and phase order.
- Key dependencies, risks, and validation methods.
- What should be synchronized back to Spec.
- What can be handed off as Task candidates.

## Completion Hook

After an implementation Plan is complete, check whether the downstream task skill is available:
- If `writing-tasks` is installed or otherwise available in the current environment, ask the user whether they want to turn the Plan into executable task cards.
- If `writing-tasks` is not installed or not available, tell the user they can use `writing-tasks` to turn the Plan into trackable and reviewable execution tasks.

This hook is advisory only. Do not automatically create task cards unless the user confirms.
