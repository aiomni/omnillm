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
omni-coding/tasks/
├── README.md
├── index.md
├── inbox.md
├── projects/
└── tasks/
```

## Status Model

- `todo`: defined but not started.
- `doing`: actively in progress and visible in `index.md`.
- `blocked`: cannot proceed because of a dependency or blocker.
- `done`: acceptance, review, and synchronization are complete.

## Priority Model

- `P0`: critical-path work that blocks the main delivery path.
- `P1`: important work that completes a capability loop or validation confidence.
- `P2`: later-stage follow-up or non-critical polish.

## Task Card Rules

Each task card must include background, goal, executable steps, acceptance criteria, dependencies, execution log, dynamic adjustments, review, and notes.

Do not mark a task `done` unless:
- All execution steps are complete or explicitly made unnecessary.
- Step-level and task-level acceptance criteria are satisfied.
- Dynamic adjustments and execution log are current.
- Review is complete.
- `index.md` and the project page are synchronized.
