---
name: writing-tasks
description: Use when asked to create, update, execute, review, or maintain executable engineering Tasks, task cards, task systems, dashboards, inbox capture, execution logs, status tracking, acceptance criteria, dependencies, blockers, handoff notes, or convert a Plan into schedulable and retryable task units. Also use when syncing real implementation progress into task files, marking tasks done or blocked, or deciding whether follow-up work should become a tracked task.
---

# Writing Tasks

Convert a `Plan` or clear engineering goal into executable, trackable, and reviewable `Task` units. A Task answers what should be done now, what proves it is done, what it depends on, and what it unlocks.

Core boundary:
- `Spec` defines what the system is and what constraints must hold. It is the source of truth.
- `Plan` defines how to satisfy the Spec. It is strategy plus orchestration.
- `Task` defines what to execute now. It is the smallest schedulable execution unit.

If the target repository already has a task system, follow its conventions first. Do not overwrite an existing structure without checking how it works.

## Bundled Assets

This skill includes reusable task-system templates under `assets/templates/tasks/`:
- `README.md` for task-system rules and operating guidance.
- `index.md` for active task tracking.
- `inbox.md` for untriaged follow-ups and risks.
- `projects/project.md` for project-level task coordination.
- `templates/task.md` for individual task cards.

Use these assets when the target repository has no existing task-system convention and durable task tracking is useful. Copy only the needed templates into `omni-coding/tasks/`, replace placeholders immediately, and adapt names and sections to the target repository. Do not create large empty task trees. Keep task artifacts under the shared `omni-coding/` workspace so they sit next to `omni-coding/specs/` and `omni-coding/plans/`.

## Language Policy

- If the user's request is in English, respond in English and write specs, plans, task cards, reviews, and other generated artifacts in English.
- If the user's request is in Chinese or another language, respond in that same language and write specs, plans, task cards, reviews, and other generated artifacts in that language.
- If the user explicitly requests a language, or the surrounding context specifies a language, use that language instead of inferring the language from the request.

## Code Writing Boundary

- Do not write, modify, patch, or generate production code, test code, scripts, or other source files unless the user explicitly asks for direct code implementation or code edits.
- When the user asks only for Task work, produce task cards, execution steps, acceptance criteria, status synchronization, review notes, or handoff context without changing source code.
- It is acceptable to reference code paths, describe expected edits, or include clearly labeled illustrative pseudocode when needed for task clarity, but do not apply code changes without explicit user confirmation.
- If implementation would be useful but the user did not ask for coding, ask for confirmation before writing code.

## When To Use

Use this skill when:
- The user asks to turn a Plan, milestone, bug, follow-up, or clear goal into schedulable task cards.
- The user asks to create or maintain task cards, dashboards, project pages, inbox capture, or execution logs.
- The user asks to add task dependencies, priority, acceptance criteria, status, blocker notes, or review notes.
- Implementation progress must be synchronized into task files instead of remaining only in chat.
- A follow-up must be classified as a formal task, an inbox item, or a note inside an existing task.

Do not use this skill when:
- The work is to define system truth, interfaces, inputs, outputs, invariants, or permission boundaries; use `writing-spec`.
- The work is to define technical approach, migration path, staged orchestration, rollout, or tradeoffs; use `writing-plan`.
- The work is a small one-file change with no durable dependency or tracking value; do not create a full task system mechanically.

## Task Boundary

A valid Task must be:
- Atomic enough to execute without changing the goal mid-flight.
- Schedulable by one owner or one agent.
- Retryable if execution fails.
- Reviewable against explicit acceptance criteria.
- Connected to real dependencies and downstream work when those relationships exist.

A Task may contain:
- Background and source context.
- Goal and non-goals.
- Execution steps.
- Step-level acceptance criteria.
- Task-level acceptance criteria.
- Dependencies, blockers, and downstream unlocks.
- Execution log.
- Dynamic adjustments found during execution.
- Review notes.

A Task must not contain:
- New Spec truth that has not been synchronized to the Spec layer.
- Strategy-level tradeoff analysis that belongs in a Plan.
- Vague actions with no output.
- Broad epics that cannot be independently accepted.

## Default Workflow

### 1. Define The Task Boundary First

Before creating or updating a task, answer:
1. What Plan, Spec, bug, requirement, or follow-up produced this Task?
2. Is it atomic enough to schedule, execute, retry, and accept independently?
3. What final acceptance signal proves the Task is done?
4. Does the repository already have a task system, and is it current?
5. Does this work deserve durable task tracking, or is a short execution checklist enough?

If the user only needs one-off guidance, provide a short execution checklist instead of creating task files.

### 2. Create A Minimal Task System Only When Needed

Persist a task system when at least one condition applies:
- The work spans multiple steps or files.
- The work has a real dependency chain, milestone, or critical path.
- Progress must be synchronized across multiple sessions.
- Multiple people or agents may need handoff context.

If no task system exists and persistence is useful, prefer the shared `omni-coding/` structure:

```text
omni-coding/
├── specs/
├── plans/
└── tasks/
    ├── README.md
    ├── index.md
    ├── inbox.md
    ├── projects/
    └── tasks/
```

Create only the files that are needed for the current work. Do not generate large empty task trees.

### 3. Write Task Cards With Executable Outputs

Each task card must answer:
- Why this task should happen now.
- What result counts as done.
- What it depends on.
- What it directly unlocks.

Recommended task card sections:
- `Background`
- `Goal`
- `Execution Steps`
- `Acceptance Criteria`
- `Dynamic Adjustments`
- `Execution Log`
- `Review`
- `Notes`

Writing rules:
- `Goal` describes the result, not the process.
- `Execution Steps` must be observable, checkable, and reviewable.
- `Acceptance Criteria` should separate step-level criteria from final task-level criteria.
- Dependencies must be hard dependencies, not loose associations.
- Downstream unlocks should only include work directly unblocked by this Task.

Avoid vague steps such as:
- Research it.
- See if this can be done.
- Improve the whole architecture.
- Handle edge cases.

Rewrite vague steps into output-producing actions, such as:
- Define the API data structure and validation rules.
- Add a minimal reproduction test and make it pass.
- Migrate the legacy entry point to the new SDK and remove the raw call.

### 4. Keep Execution State Synchronized

During execution, update at least three kinds of information:

1. `Execution Log`
   Record facts: what changed, what was discovered, what decision was made, and what comes next.

2. `Dynamic Adjustments`
   Record invalid assumptions, hidden risks, better approaches, rework risk, or dependency changes.

3. `Dashboard / Project Page / Index`
   Update status, critical path, milestone, and acceptance changes when they occur.

Do not let chat history become the only source of execution truth.

### 5. Review Before Marking Done

A Task can be marked `done` only when all apply:
- Execution steps are complete, or incomplete steps are proven unnecessary.
- Step-level acceptance criteria are satisfied.
- Task-level acceptance criteria are satisfied.
- Important deviations, new approaches, and risks are recorded in `Dynamic Adjustments`.
- `Review` is complete.
- Index, dashboard, project page, and affected task cards are synchronized as needed.
- No remaining gap immediately blocks downstream work.

`Review` must answer:
- Whether the Task can truly be marked `done`.
- Whether follow-up tasks, dependencies, priority, or acceptance criteria must change.
- What changed and why, if an adjustment was needed.
- If no adjustment is needed, explicitly state that no adjustment is needed.

## Status And Priority

Recommended statuses:
- `todo`
- `doing`
- `blocked`
- `done`

Recommended priorities:
- `P0`
- `P1`
- `P2`

Default semantics:
- `todo`: defined but not started.
- `doing`: actively in progress and visible in the active dashboard or index.
- `blocked`: cannot proceed because of an external dependency or prerequisite.
- `done`: all acceptance, review, and synchronization requirements are satisfied.

If the target project already has a status model, use it while preserving the acceptance and synchronization principles in this skill.

## Split Task, Inbox, Or Note

Create a formal task when:
- It has an independent goal, acceptance criteria, and dependency boundary.
- The current task is too large to execute under one stable goal.
- A risk has become work that requires formal tracking.

Put an item in the inbox when:
- It is a temporary risk, idea, or question.
- Ownership is unclear.
- It is not yet clear whether it deserves a formal task.

Keep it as a note when:
- It is only a sub-step of the current task.
- It is a local correction with no new dependency chain.
- It is only execution-log information.

## Final Response Checklist

After task-system work, report:
- Which task files changed.
- Whether any statuses changed.
- Whether dependencies, critical path, or acceptance criteria changed.
- Whether the index, project page, dashboard, or inbox was synchronized.

If these synchronization steps are not complete, do not describe the work as fully done.
