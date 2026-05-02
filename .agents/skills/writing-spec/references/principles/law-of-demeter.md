# Law of Demeter

A Spec Object should know only about its direct collaborators.

## Spec-layer rule

Keep relations local. A Spec should depend on direct requirements, not on the internal dependencies of its dependencies.

## Decision criteria

- Use `relations.depends_on` only for direct dependencies.
- Do not duplicate transitive dependencies just to make the graph look complete.
- Hide internal composition behind the Spec that owns it.
- Keep context injection to the Graph Cut, not the whole transitive universe unless needed.

## Spec smells

- A capability lists every internal subsystem as a direct dependency.
- A contract encodes details from several layers below it.
- One Spec must change whenever a dependency changes its own dependencies.

## Good Spec move

Depend on the nearest stable abstraction and let Graph traversal load direct neighbors only as needed.
