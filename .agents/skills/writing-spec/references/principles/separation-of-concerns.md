# Separation of Concerns (SoC)

Separate different reasons, audiences, and layers of system knowledge into different Spec Objects.

## Spec-layer rule

Do not mix capability behavior, runtime environment rules, interface contracts, implementation guides, and design rationale in one Spec.

## Decision criteria

- `capability` describes what the system can do.
- `system` describes environment constraints and runtime rules.
- `contract` describes how components communicate.
- Guides, tips, rationale, and migration notes stay outside `current/` unless they are hard constraints.

## Spec smells

- A capability Spec contains API examples, UI advice, runtime setup, and implementation tips together.
- A contract Spec explains why a design was chosen instead of stating the current interface truth.
- One change request forces updates across unrelated concern areas.

## Good Spec move

Move each concern into the right Spec kind and connect them through `relations` only where the dependency is real.
