# Interface Segregation Principle (ISP)

Consumers should not depend on Spec fields, capability methods, or contract operations they do not use.

## Spec-layer rule

Design small, focused `contract` and `capability` Specs. Avoid fat interfaces that force every consumer to understand every mode.

## Decision criteria

- Split read/write/admin/control-plane contracts when their consumers differ.
- Keep optional fields from becoming hidden mandatory knowledge.
- Do not overload one endpoint with unrelated operations just to avoid adding another contract.
- Let `relations` connect small Specs instead of merging them into one large surface.

## Spec smells

- A client must know write-only fields to perform a read operation.
- A single `contract.api` Spec contains unrelated file, network, auth, and billing operations.
- Most consumers use only a tiny subset of a large schema.

## Good Spec move

Split contract surfaces by consumer need and make composition explicit with `depends_on` or `constrains`.
