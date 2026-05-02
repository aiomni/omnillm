# Plan Patterns

Use this reference when a Plan needs a stronger strategy pattern than a simple phase list.

## Compatibility-First Migration

Use when changing behavior while old consumers still exist.

Shape:
1. Add a compatibility layer.
2. Support old and new paths together.
3. Add observability for divergence.
4. Move traffic gradually.
5. Remove the old path after validation.

Good for:
- Contract migrations.
- Storage migrations.
- Auth/session changes.
- SDK or API replacement.

Watch for:
- Hidden old-path dependencies.
- Divergent writes.
- Metrics that only cover the new path.

## Strangler Fig Replacement

Use when replacing a large subsystem incrementally.

Shape:
1. Put a routing or adapter boundary around the old system.
2. Move one capability at a time.
3. Keep a rollback path for each moved capability.
4. Delete old code only after traffic and ownership are fully moved.

Good for:
- Legacy module replacement.
- Service extraction.
- Large UI or state-management rewrites.

Watch for:
- Two systems sharing mutable state.
- Duplicate business rules.
- Cleanup never becoming a scheduled task.

## Vertical Slice Delivery

Use when value depends on completing a thin end-to-end path before broad coverage.

Shape:
1. Choose one representative use case.
2. Implement the smallest full path across layers.
3. Validate the integration boundary.
4. Generalize after the slice proves the design.

Good for:
- New product capabilities.
- Uncertain integration design.
- Early risk reduction.

Watch for:
- Mistaking a demo slice for production readiness.
- Leaving cross-cutting concerns undefined.
- Expanding scope before the first slice is accepted.

## Risk-First Spike

Use when uncertainty is high enough that direct implementation may waste effort.

Shape:
1. Define the question the spike must answer.
2. Set a time or scope limit.
3. Produce a decision artifact, not production code by default.
4. Convert the result into a Plan update, Spec Patch, or Task candidate.

Good for:
- Unknown third-party behavior.
- Performance feasibility.
- Hard-to-reverse architecture choices.

Watch for:
- Open-ended research.
- Spike code silently becoming production code.
- No decision after the spike.

## Rollout With Guardrails

Use when deployment risk matters as much as implementation correctness.

Shape:
1. Add feature flag or rollout control.
2. Define eligibility and blast radius.
3. Add monitoring and alert thresholds.
4. Ramp gradually.
5. Define rollback triggers.

Good for:
- User-facing behavior changes.
- Data-path changes.
- Performance-sensitive changes.

Watch for:
- Flags without owners or removal tasks.
- Metrics that do not match user impact.
- Rollback that has never been tested.
