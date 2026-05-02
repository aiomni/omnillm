# Composite/Aggregate Reuse Principle

Prefer composition over inheritance when building larger behavior from smaller Spec Objects.

## Spec-layer rule

Compose capabilities through `relations.depends_on` and `relations.constrains` before introducing deep `extends` chains.

## Decision criteria

- Use composition when a Spec uses another Spec without becoming a subtype of it.
- Use `extends` only when substitutability is intended and LSP can be preserved.
- Keep reusable constraints as small System Specs or Contract Specs.
- Prefer a shallow Spec Graph over inheritance-like hierarchy.

## Spec smells

- Deep `extends` chains make it unclear which invariant wins.
- A child Spec inherits many fields it does not need.
- Reuse is achieved by copy-pasting constraints across Specs.

## Good Spec move

Extract the reusable rule into its own Spec Object and reference it from multiple Specs.
