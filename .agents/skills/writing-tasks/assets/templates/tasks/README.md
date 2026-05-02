# Tasks Guide

This directory lives at `omni-coding/tasks/` and turns engineering goals into executable, trackable, and reviewable tasks. It is not a journal. It should continuously answer:
- What should be done?
- Why should it happen now?
- What proves it is done?
- What does it depend on?
- What does it unlock?
- Where is the current work tracked?

## Directory Structure

```text
omni-coding/
├── specs/                   # Current Spec System
├── plans/                   # Strategy-level Plans
└── tasks/
    ├── README.md            # Task-system guide
    ├── index.md             # Dashboard and current task view
    ├── inbox.md             # Untriaged capture
    ├── tasks/               # Individual task cards
    ├── projects/            # Project-level coordination
    └── templates/           # Reusable task templates
```

Recommended reading order:
- Start with `index.md` to understand priority and critical path.
- Read `projects/<project>.md` for project constraints, milestones, and final acceptance.
- Open the relevant `tasks/task-xxx.md` inside `omni-coding/tasks/` before implementation.
- Put temporary ideas, risks, and untriaged follow-ups in `inbox.md` first.

## Task-System Principles

- `Frontmatter` is machine-readable: status, priority, dependencies, and project ownership.
- `Markdown Body` is human-readable: background, goal, steps, acceptance, logs, and review.
- Every task card must be executable; avoid slogans and broad intentions.
- Every task card must answer four questions:
  - Why now?
  - What counts as done?
  - What does it depend on?
  - What does it unlock?
- Tasks serve the final project goal. If the task definition, dependency chain, or implementation path becomes inaccurate, update the task system immediately.

## Project Constraints

Use this section to record hard constraints that affect task execution, such as:
- Backward compatibility requirements.
- Whether breaking refactors are allowed.
- File-size or module-boundary limits.
- Build artifact synchronization requirements.
- Team-level technology preferences or restrictions.

If there are no relevant constraints, keep this section short.

## Status Model

- `todo`: The task is defined but not started.
- `doing`: The task is actively being executed and should appear in the active dashboard.
- `blocked`: The task cannot continue because a prerequisite or external dependency is missing.
- `done`: The task satisfies all acceptance, review, and synchronization requirements.

Status transition rules:
- Move `todo -> doing` only when actual execution starts.
- Move `doing -> done` only after acceptance criteria are satisfied.
- Move `doing -> blocked` only after documenting the blocker and recovery condition.
- Move `blocked -> doing` only after documenting how the blocker was resolved.

## Priority Model

- `P0`: Critical-path work; the main delivery path is blocked without it.
- `P1`: Important work that completes a loop, restores a capability, or improves delivery confidence.
- `P2`: Later-stage work or non-critical follow-up.

If priority conflicts with dependency order, dependency order wins. Do not use priority to skip prerequisites.

## Frontmatter Fields

```yaml
id: task-001
title: Replace with an executable task title
status: doing
priority: P0
tags: [architecture]
project: your-project-slug
due: YYYY-MM-DD
parent: null
depends_on: [task-000]
blocks: [task-002]
```

Field rules:
- `id`: Globally unique task ID, usually `task-xxx`.
- `title`: Must describe executable work, not vague improvement.
- `status`: Use `todo | doing | blocked | done` unless the project defines another status model.
- `priority`: Use `P0 | P1 | P2` unless the project defines another priority model.
- `tags`: Classify by area, such as `runtime`, `api`, `docs`, or `tooling`.
- `project`: Project slug.
- `due`: Target date, not a promise. Update it when it becomes stale.
- `parent`: Parent task ID or `null`.
- `depends_on`: Tasks that must complete before this task can start or finish.
- `blocks`: Tasks directly unlocked by this task.

## Task Card Body

Each task card should include:
- `Background`
- `Goal`
- `Execution Steps`
- `Acceptance Criteria`
- `Dynamic Adjustments`
- `Execution Log`
- `Review`
- `Notes`

Section rules:
- `Background`: Explain the current gap and why the task exists.
- `Goal`: State the result, not the process.
- `Execution Steps`: Use checkable process steps.
- `Acceptance Criteria`: Separate step-level criteria from final task-level criteria.
- `Dynamic Adjustments`: Record invalid assumptions, better approaches, risks, and effects on downstream tasks.
- `Execution Log`: Record dated progress, blockers, decisions, and results.
- `Review`: Confirm whether the task is truly done and whether downstream tasks need adjustment.
- `Notes`: Store assumptions, related files, pitfalls, and acceptance details.

## Writing Execution Steps

Good steps should:
- Follow dependency order.
- Produce observable output.
- Be reviewable and testable.

Avoid vague steps:
- [ ] Research it
- [ ] Improve the architecture
- [ ] Handle edge cases

Prefer output-producing steps:
- [ ] Define the API data structure and validation rules
- [ ] Add a minimal reproduction test for invalid input and make it pass
- [ ] Migrate the legacy entry point to the new SDK and remove the raw call

## Writing Acceptance Criteria

Step-level acceptance criteria:
- Map to specific execution steps.
- Must be observable, verifiable, and reproducible.
- Prefer concrete results over subjective descriptions.

Task-level acceptance criteria:
- Answer when the whole task may become `done`.
- Protect downstream work, not just the current commit.
- Exclude partial completion, such as code without docs, tests, or state synchronization when those are required.

Recommended style:
- Step 1 acceptance: The new interface parses required fields and separates core fields from display fields.
- Task completion acceptance: Callers can complete the expected business flow through the new interface without relying on the legacy implementation.

## Execution Log Rules

Update the log whenever actual work starts, a key step completes, or a blocker appears.

Prefer factual entries:
- What changed.
- What was discovered.
- What decision was made.
- What comes next.

Avoid low-information entries such as `continued development` or `fixed some issues`.

Recommended format:

```markdown
## YYYY-MM-DD
- Completed the interface definition and validation rules.
- Found that the legacy call chain still depends on the old field; next step is to remove that dependency.
```

## Dynamic Adjustment Rules

Record a dynamic adjustment when any of these happen:
- The current implementation path has a clear problem, hidden risk, or irreversible downside.
- A safer, cheaper, or more architecture-aligned path appears.
- An upstream assumption fails and changes this task or downstream tasks.
- Continuing the original steps mechanically would cause rework or preserve the wrong design.

Each adjustment should state:
- What was discovered.
- Why it matters.
- Which tasks, dependencies, acceptance criteria, or milestones are affected.
- Recommended action: continue, split a task, change dependencies, change priority, change acceptance, or pause.

If the adjustment changes the critical path, final acceptance, or multiple downstream tasks, update `index.md` and the relevant project page.

## Review Rules

Before marking a task `done`, review:
- Were all execution steps completed, or are skipped steps proven unnecessary?
- Are step-level acceptance criteria satisfied?
- Are task-level acceptance criteria satisfied?
- Were important deviations and risks recorded in `Dynamic Adjustments`?
- Do downstream tasks, dependencies, priority, or acceptance criteria need adjustment?
- Were `index.md`, the project page, and affected task cards synchronized if needed?

A task is not done if it leaves a known gap that immediately blocks downstream work.

## Inbox Rules

Use `inbox.md` for:
- Untriaged follow-ups.
- Risks that may or may not become tasks.
- Questions without clear ownership.
- Ideas that need later classification.

During triage:
- Move items into an existing task when they are just sub-steps or notes.
- Create a new task when the item has independent goal, acceptance, and dependency boundaries.
- Delete or archive items that are no longer relevant.

## New Task Criteria

Create a new task when:
- The work has an independent goal, acceptance criteria, and dependency boundary.
- The current task is too large to execute under one stable goal.
- A risk has become real work that requires tracking.

Do not create a new task when:
- The item is only a sub-step inside the current task.
- The item is a local correction with no new dependency chain.
- The item is only execution-log information.

## Dependency Writing Rules

Use `depends_on` for prerequisites without which the current task cannot start or complete.

Use `blocks` for downstream tasks directly unlocked by the current task.

Do not overuse dependencies:
- If something is only related but not blocking, do not put it in `depends_on`.
- If something may be affected but does not have to wait, do not put it in `blocks`.

## Final Acceptance Relationship

This task system exists to satisfy project-level final acceptance, not to close cards for its own sake. Keep final acceptance visible in the project page and `index.md`, and use it to guide priority decisions.

If a task looks complete but does not contribute to final acceptance, reconsider its priority or scope.
