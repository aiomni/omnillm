# KISS

Keep the Spec simple, explicit, and easy for an LLM to apply consistently.

## Spec-layer rule

A good Spec should be readable without reconstructing hidden intent. Prefer plain structured fields over clever prose or indirection.

## Decision criteria

- Use predictable fields: `id`, `kind`, `version`, `status`, `summary`, `constraints`, `invariants`, `relations`.
- Keep each Spec Object small enough for targeted context injection.
- Prefer direct YAML structures over custom mini-languages.
- Choose boring names that match the domain.

## Spec smells

- The LLM must infer rules from narrative paragraphs.
- Constraints are encoded in examples instead of fields.
- A Spec needs a long explanation before it can be used.

## Good Spec move

Rewrite narrative into structured constraints and invariants, then keep rationale outside `current/`.
