# DRY

Every system rule should have one authoritative representation in the Spec System.

## Spec-layer rule

Avoid repeating the same constraint, invariant, schema, or capability rule across multiple current Specs. Extract shared truth and reference it.

## Decision criteria

- If two Specs repeat the same constraint, decide which Spec owns it.
- Shared environment rules usually belong in `system` Specs.
- Shared request/response shapes usually belong in `contract` Specs.
- Capabilities should reference shared rules through `relations`, not copy them.

## Spec smells

- The same max size, timeout, permission, or schema appears in many files.
- Two Specs describe the same concept with slightly different wording.
- Updating one rule requires hunting across the whole `current/` tree.

## Good Spec move

Extract duplicated truth into one Spec Object and replace copies with explicit relations.
