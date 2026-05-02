# Least Privilege

A Spec should grant only the minimum authority required for the capability to work.

## Spec-layer rule

Permissions, scopes, side effects, and resource access should be constrained in the Spec before code is written.

## Decision criteria

- Every capability should declare its scope and allowed side effects.
- File access should state path boundaries, write modes, and size limits.
- Network access should state allowed targets, methods, auth model, and timeout rules.
- Process execution should state sandbox, environment, working directory, and forbidden operations.

## Spec smells

- A capability can access the whole filesystem when it only needs workspace files.
- A contract accepts arbitrary commands, URLs, or paths without constraints.
- A system Spec says permissions are decided by implementation.

## Good Spec move

Encode minimum permissions in `constraints` and make any privilege expansion an explicit Patch with a reason.
