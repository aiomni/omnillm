---
id: plan.<domain>.<name>
kind: plan
status: draft
source_specs: []
version: 1
---

# Plan: [Name]

## Goal
State the target outcome this Plan must achieve.

## Non-Goals
- Out-of-scope item 1
- Out-of-scope item 2

## Source Specs And Requirements
- `spec.id.or.requirement`: why it matters for this Plan

## Assumptions
- Assumption 1
- Assumption 2

## Open Questions
- Question 1 and why it could change the route

## Strategy Summary
Describe the chosen route in one short paragraph.

## Technical Approach
- Step or approach component 1
- Step or approach component 2
- Step or approach component 3

## Tradeoffs
| Decision | Chosen Route | Rejected Alternative | Rationale | Reversal Signal |
| --- | --- | --- | --- | --- |
| Decision 1 | Chosen option | Alternative option | Why this fits current constraints | Signal that would change the decision |

## Phases
### Phase 1: [Name]
- Output: concrete phase-level output
- Depends on: none
- Unlocks: next phase or task candidates
- Validation: how to prove this phase is ready

### Phase 2: [Name]
- Output: concrete phase-level output
- Depends on: Phase 1
- Unlocks: next phase or task candidates
- Validation: how to prove this phase is ready

## Dependencies
### Hard Dependencies
- Dependency that blocks progress if missing

### Soft Dependencies
- Dependency that improves quality or speed but does not block progress

### External Dependencies
- Person, permission, system, release window, or third-party dependency

## Risks And Mitigations
| Risk | Impact | Mitigation | Validation Or Rollback |
| --- | --- | --- | --- |
| Risk 1 | Impact description | Mitigation action | Validation or rollback path |

## Validation Strategy
- Unit-level validation
- Integration-level validation
- Rollout or monitoring validation

## Rollback Strategy
- How to safely reverse or pause the chosen route if assumptions fail

## Spec Patch Needs
- None yet. Add missing, stale, or conflicting Spec truth here instead of hiding it in the Plan.

## Task Candidates
- Candidate task 1: expected output
- Candidate task 2: expected output
- Candidate task 3: expected output
