# Dependency Inversion Principle (DIP)

High-level Specs should depend on abstractions, not low-level implementation details.

## Spec-layer rule

A `capability` or `contract` Spec should describe behavior and guarantees. Implementation details belong in lower-level Specs only when they are real constraints.

## Decision criteria

- Depend on capability IDs, not concrete file paths, SDK classes, or vendor names.
- Keep provider-specific constraints behind adapter or system Specs.
- Put stable behavior in `capability` Specs and volatile mechanics in implementation-facing Specs.
- If a low-level detail is exposed to users, promote it to an explicit contract field.

## Spec smells

- A public contract requires knowledge of one internal class or storage engine.
- A high-level capability depends directly on a specific vendor API without an abstraction.
- Changing an implementation detail requires changing multiple high-level Specs.

## Good Spec move

Introduce an abstract capability or contract, then connect concrete implementations through relations or implementation notes outside the current Spec truth.
