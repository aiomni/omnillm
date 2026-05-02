# Liskov Substitution Principle (LSP)

A subtype, extension, adapter, or replacement must preserve the guarantees promised by the base Spec.

## Spec-layer rule

If a Spec Object `extends` another Spec, it must keep the parent invariants and constraints unless the parent explicitly allows variation.

## Decision criteria

- Child Specs may add stricter guarantees, but should not weaken inherited invariants.
- Replacements must accept the same valid inputs unless the contract declares a migration.
- A new implementation behind the same contract must preserve observable behavior.
- If behavior changes incompatibly, create a new contract or deprecate the old one through the Spec Evolution flow.

## Spec smells

- `relations.extends` is used while inherited constraints are ignored.
- A new backend or provider returns different error shapes under the same contract.
- A supposedly compatible capability rejects inputs that the original Spec allowed.

## Good Spec move

Write inherited invariants explicitly and validate extension Specs against them before marking them `current`.
