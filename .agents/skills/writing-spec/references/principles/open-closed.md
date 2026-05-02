# Open/Closed Principle (OCP)

A Spec System should be open for extension but closed for unsafe rewrites of existing truth.

## Spec-layer rule

Prefer extending the Spec Graph with a new Spec Object, relation, mode, or versioned contract field over rewriting stable behavior.

## Decision criteria

- Add a new capability when behavior is genuinely new.
- Add a new contract field only when backward compatibility and defaults are explicit.
- Use `Patch` to evolve the current `Snapshot`; do not replace the whole Spec Object unless creating it for the first time.
- Keep stable invariants stable. If an invariant changes, treat it as a compatibility decision.

## Spec smells

- A request for one new mode causes a full rewrite of an existing Spec.
- A contract change silently breaks existing callers.
- Extension points are hidden in prose instead of represented as structured fields.

## Good Spec move

Represent extension points explicitly, for example `input.mode`, `capabilities`, `supported_formats`, or a separate `contract` Spec connected by `relations`.
