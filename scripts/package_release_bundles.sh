#!/usr/bin/env bash

set -euo pipefail

root_dir="$(
  CDPATH= cd -- "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null
  pwd
)"
dist_dir="${DIST_DIR:-$root_dir/dist}"

crate_name="$(sed -n 's/^name *= *"\([^"]*\)".*/\1/p' "$root_dir/Cargo.toml" | head -n 1)"
crate_version="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$root_dir/Cargo.toml" | head -n 1)"

if [[ -z "$crate_name" || -z "$crate_version" ]]; then
  echo "failed to read crate name/version from Cargo.toml" >&2
  exit 1
fi

library_zip_name="${crate_name}-rust-library-v${crate_version}.zip"
skill_zip_name="${crate_name}-claude-skill-v${crate_version}.zip"
checksums_name="SHA256SUMS.txt"

rm -rf "$dist_dir"
mkdir -p "$dist_dir"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "Packaging Rust library bundle..."
(
  cd "$root_dir"
  cargo package --allow-dirty --no-verify >/dev/null
)

crate_archive="$root_dir/target/package/${crate_name}-${crate_version}.crate"
if [[ ! -f "$crate_archive" ]]; then
  echo "expected crate archive not found: $crate_archive" >&2
  exit 1
fi

library_extract_dir="$tmp_dir/library-extract"
mkdir -p "$library_extract_dir"
tar -xzf "$crate_archive" -C "$library_extract_dir"

library_contents_dir="$(find "$library_extract_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$library_contents_dir" ]]; then
  echo "failed to extract packaged crate contents" >&2
  exit 1
fi

library_stage_dir="$tmp_dir/${crate_name}-rust-library-v${crate_version}"
mv "$library_contents_dir" "$library_stage_dir"

(
  cd "$tmp_dir"
  zip -Xrq "$dist_dir/$library_zip_name" "$(basename "$library_stage_dir")"
)

echo "Packaging Claude Skill bundle..."
skill_stage_dir="$tmp_dir/skill-upload"
mkdir -p "$skill_stage_dir"
cp -R "$root_dir/skill/." "$skill_stage_dir/"

for required_path in SKILL.md references assets; do
  if [[ ! -e "$skill_stage_dir/$required_path" ]]; then
    echo "skill bundle is missing required path: $required_path" >&2
    exit 1
  fi
done

(
  cd "$skill_stage_dir"
  zip -Xrq "$dist_dir/$skill_zip_name" SKILL.md references assets
)

echo "Writing checksums..."
(
  cd "$dist_dir"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$library_zip_name" "$skill_zip_name" > "$checksums_name"
  else
    shasum -a 256 "$library_zip_name" "$skill_zip_name" > "$checksums_name"
  fi
)

echo "Created bundles:"
ls -lh "$dist_dir"
