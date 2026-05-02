# Plans Guide

This directory stores strategy-level Plans under `omni-coding/plans/` that bridge current Specs and executable Tasks. A Plan should explain how the system will satisfy the Spec, not redefine the Spec and not track task execution.

## Directory Structure

```text
omni-coding/
├── specs/                   # Current Spec System
├── plans/
│   ├── README.md            # Planning guide
│   ├── current/             # Active Plans
│   └── archive/             # Obsolete Plans, not active by default
└── tasks/                   # Executable task system
```

Use the smallest structure that fits the project. If the project already has a planning convention, follow it instead of forcing this layout.

## Plan Principles

- A Plan is strategy plus orchestration.
- A Plan should cite source Specs or requirements.
- A Plan may contain assumptions, tradeoffs, phases, risks, and validation strategy.
- A Plan should emit Task candidates, not maintain task status or execution logs.
- A Plan must not introduce new system truth that belongs in the Spec layer.

## Recommended Workflow

1. Identify the source Specs, requirements, or constraints.
2. Define goal and non-goals.
3. List assumptions and open questions.
4. Choose the technical path and document tradeoffs.
5. Split the path into strategy-level phases.
6. Make hard, soft, phase, and external dependencies explicit.
7. Add risks, mitigations, validation, and rollback strategy.
8. Emit Task candidates for the task layer.

## Done Criteria For A Plan

A Plan is ready for task decomposition when:
- The goal and non-goals are clear.
- Source Specs or requirements are identified.
- Phase order and dependencies are explicit.
- Major tradeoffs are documented.
- Key risks have mitigations or validation methods.
- Missing or conflicting Spec truth is called out as a Spec Patch need.
- Task candidates are clear enough to turn into schedulable Tasks.
