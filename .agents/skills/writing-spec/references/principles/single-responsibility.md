# Single Responsibility Principle (SRP)

A Spec Object should have one reason to change. Its boundary should describe one capability, one system rule set, or one contract surface.

## Spec-layer rule

Use SRP before code exists. If one Spec mixes two independent reasons for change, split it before implementation.

## Decision criteria

- One `capability` should model one user-visible or system-visible action.
- One `system` Spec should model one environment concern, such as workspace, runtime, sandbox, or persistence.
- One `contract` Spec should model one communication surface or schema family.
- If two fields are owned by different change drivers, they probably belong in separate Spec Objects.

## Spec smells

- `capability.file.write` also defines network retry, auth, UI behavior, or unrelated runtime policy.
- One Spec has multiple unrelated `constraints` groups that evolve independently.
- A small change requires touching many unrelated parts of the same Spec Object.

## Good Spec move

Split mixed concerns and connect them with `relations.depends_on` or `relations.constrains` instead of making one large Spec.
