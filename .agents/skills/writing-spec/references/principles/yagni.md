# YAGNI

Do not put future-only requirements into the current Spec truth.

## Spec-layer rule

`current/` describes what the system is now, not what it might become later. Future ideas belong in tasks, proposals, or backlog notes, not in current Spec Objects.

## Decision criteria

- Add a Spec field only when current behavior or accepted near-term work requires it.
- Do not model unsupported modes as if they are already available.
- Keep placeholder capabilities out of `current/`.
- Use `Patch` later when the requirement becomes real.

## Spec smells

- `supported_modes` lists modes the system cannot actually run.
- A contract includes fields reserved for hypothetical clients.
- The Spec Graph contains roadmap nodes marked as `current`.

## Good Spec move

Remove future-only fields from `current/` and track them outside the source of truth until they become real requirements.
