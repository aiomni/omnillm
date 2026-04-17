---
name: omnillm-release
description: |
  Release workflow for the OmniLLM repository.
  Use this skill when the user asks to bump the OmniLLM version, update the
  bundled skill or website docs for a release, publish the crate to crates.io,
  commit and push release changes, create or verify the GitHub tag and release,
  or confirm the Deploy Docs and Package Bundles workflows.
  Trigger on requests mentioning OmniLLM release, publish, crates.io, tag,
  GitHub release, docs deploy, release bundles, version bump, or updating
  `Cargo.toml`, `skill/`, and `website/docs/` together.
---

# OmniLLM Release

This skill is for the `aiomni/omnillm` repository only. It captures the
release process that updates repo docs, publishes the crate, and verifies the
GitHub automation already wired into this repo.

## When To Use It

Use this skill when the user wants any of the following:

- cut a new OmniLLM release
- bump `Cargo.toml` and sync version references
- update the bundled OmniLLM skill and release-facing website docs
- publish to crates.io
- push `main`, tag `vX.Y.Z`, and verify GitHub release assets
- confirm GitHub Pages docs deployment for the repo site

Do not use this skill for normal feature work that does not end in a release.

## Repo-Specific Facts

- The crate version lives in `Cargo.toml`.
- `Cargo.lock` is ignored in this repo. Do not expect it to appear in `git status`.
- `website/doc_build/` is ignored. Rebuild the site for validation, but do not
  expect those files to be committed.
- Pushing `main` triggers `.github/workflows/gh-pages.yml` and publishes docs.
- Pushing a `v*` tag triggers `.github/workflows/package-bundles.yml`, which
  uploads release bundles and may create or update the GitHub release.
- The repo already has a bundled public skill under `skill/`. Release work may
  also require updating that skill and the website copy of the same guidance.

## Preconditions

Before publishing anything:

1. Confirm the working branch and remote state.
2. Check `git status -sb`.
3. Check `git rev-list --left-right --count origin/main...HEAD`.
4. If the branch is not `main`, or `origin/main` is ahead, stop and fix that
   first.
5. If the tree contains unrelated local changes, do not publish over them.

## Release Workflow

Follow these steps in order.

### 1. Decide the target version

- Read the current version from `Cargo.toml`.
- Check recent tags with `git tag --sort=-version:refname | sed -n '1,20p'`.
- Use the requested semver version or infer the next patch version if the user
  clearly asked for a normal release without specifying one.

### 2. Update version references

At minimum, update:

- `Cargo.toml`
- `skill/SKILL.md` metadata version when the bundled skill changed or when you
  want the skill metadata to match the crate release
- `website/docs/en/*.md` and `website/docs/zh/*.md` frontmatter `release:`
  fields that track the current release
- `website/theme/components/doc-chrome.tsx` if the default release chip is
  hard-coded
- any inline `version = "..."` examples in website docs that intentionally show
  the current crate release

Then search for stale literals and resolve the real ones:

```sh
rg -n 'v0\.[0-9]+\.[0-9]+|0\.[0-9]+\.[0-9]+' Cargo.toml README.md skill src tests website/docs website/theme .github scripts -g '!target'
```

Do not replace unrelated dependency versions.

### 3. Update release-facing docs and skill text

If the release includes behavior changes, update the docs that users will rely
on right away:

- `skill/SKILL.md`
- `skill/README.md`
- `website/docs/en/usage.md`
- `website/docs/zh/usage.md`
- `website/docs/en/skill.md`
- `website/docs/zh/skill.md`

Keep the website wording aligned with the bundled skill when both describe the
same OmniLLM workflow.

### 4. Validate before commit

Run:

```sh
cargo test
```

```sh
cd website && npm run site:build
```

If you want a packaging check before commit while the tree is still dirty, run:

```sh
cargo publish --dry-run --allow-dirty
```

If the tree is already clean and committed, drop `--allow-dirty`.

### 5. Review the change set

Run:

```sh
git diff --stat
```

```sh
git status --short
```

Confirm that the diff contains only intended release edits.

### 6. Commit and push `main`

Use a direct release commit message:

```sh
git add <updated-files>
git commit -m "Release vX.Y.Z"
git push origin main
```

Do not amend or rewrite unrelated history unless the user explicitly asks.

### 7. Publish the crate

From a clean tree on `main`:

```sh
cargo publish
```

Wait for crates.io confirmation. If it fails, fix the packaging issue before
tagging the release.

### 8. Tag the release

```sh
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

### 9. Verify GitHub release automation

Check recent workflow runs:

```sh
gh run list --limit 10 --json databaseId,workflowName,headBranch,headSha,status,conclusion,event,displayTitle,url
```

Check the release:

```sh
gh release view vX.Y.Z --json tagName,name,isDraft,isPrerelease,url,publishedAt,assets
```

Expected repo behavior:

- `Deploy Docs` should run after pushing `main`
- `Package Bundles` should run after pushing the tag
- release assets should include the Rust library zip, the skill zip, and
  `SHA256SUMS.txt`

If the release does not already exist after the tag push, create it manually:

```sh
gh release create vX.Y.Z --title "vX.Y.Z" --generate-notes
```

If GitHub reports that the tag already has a release, switch to inspection
instead of retrying creation.

### 10. Verify final public endpoints

Useful checks:

```sh
gh api repos/aiomni/omnillm/pages --jq '.html_url'
```

- confirm the GitHub Pages URL
- confirm the release URL
- confirm the crate is published on crates.io

## Output Expectations

When executing this workflow:

- be explicit about the target version
- say which files were updated for version sync versus behavior docs
- report validation status for `cargo test`, website build, and publish
- report the commit SHA, pushed tag, release URL, and Pages URL
- call out any automation that already created the release so you do not do the
  same step twice

## Guardrails

- Never publish from a tree with unrelated dirty changes.
- Never assume the GitHub release needs manual creation; check first.
- Never assume the docs site needs manual deployment; this repo already deploys
  from GitHub Actions.
- Do not edit ignored artifacts just to make `git status` look busy.
- If the user asks for a release but credentials for GitHub or crates.io are
  missing, stop at the first blocked step and report exactly what succeeded.
