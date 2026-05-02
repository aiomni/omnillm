# Occam's Razor

When multiple Spec designs explain the same requirement, choose the simplest one that preserves correctness.

## Spec-layer rule

Do not add abstractions, lifecycle states, version layers, or graph edges unless they solve a current Spec problem.

## Decision criteria

- Prefer the smallest Spec Object that fully states the current truth.
- Prefer one explicit invariant over several vague rules.
- Prefer direct constraints over clever meta-models.
- Add machinery only when the project has repeated evidence that it needs it.

## Spec smells

- A Spec introduces plugin, provider, or lifecycle concepts before any real variation exists.
- A simple constraint becomes a generic policy engine.
- The Spec Graph has nodes that no current capability, system rule, or contract depends on.

## Good Spec move

Collapse speculative abstractions and keep the current Snapshot easy for LLMs to read and apply.
