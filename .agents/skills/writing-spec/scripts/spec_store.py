#!/usr/bin/env python3
"""Tiny Spec Store helper for Snapshot + Patch + Guide workflows."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import re
import shutil
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None

KINDS = {"capability", "system", "contract"}
RELATION_KEYS = ("depends_on", "extends", "constrains")
CHANGE_OPERATOR_KEYS = {"set", "add", "remove"}
DRAFT_BUCKETS = ("open", "accepted", "rejected", "superseded")
DRAFT_OPEN_STATUSES = {"researching", "proposed", "validated"}
DRAFT_TERMINAL_STATUSES = {"accepted", "rejected", "superseded"}
DRAFT_WORKFLOW_STATUSES = DRAFT_OPEN_STATUSES | DRAFT_TERMINAL_STATUSES
BAD_VERSION_NAME = re.compile(r"(^|[_.-])(v\d+|final|latest|new)([_.-]|$)", re.IGNORECASE)
FORBIDDEN_CURRENT_KEYS = {
    "description",
    "notes",
    "tips",
    "rationale",
    "examples",
    "guide",
    "guides",
    "doc",
    "docs",
}


class SpecError(Exception):
    pass


@dataclass
class PlannedPatch:
    patch: dict[str, Any]
    op: str
    target: str
    written_path: Path | None = None
    old_spec: dict[str, Any] | None = None
    old_spec_path: Path | None = None
    deleted_guides: list[tuple[dict[str, Any], Path]] = field(default_factory=list)


@dataclass
class PatchResult:
    op: str
    target: str
    current_path: Path | None = None
    archive_path: Path | None = None
    log_path: Path | None = None
    guide_archive_paths: list[Path] = field(default_factory=list)


def require_yaml() -> None:
    if yaml is None:
        raise SpecError("PyYAML is required. Install with: python3 -m pip install pyyaml")


def load_yaml(path: Path) -> Any:
    require_yaml()
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle) or {}


def dump_yaml(path: Path, data: Any) -> None:
    require_yaml()
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        yaml.safe_dump(data, handle, allow_unicode=True, sort_keys=False)


def ensure_layout(root: Path) -> None:
    for base in ("current", "guides"):
        for kind in sorted(KINDS):
            (root / base / kind).mkdir(parents=True, exist_ok=True)
    for relative in ("log", "archive"):
        (root / relative).mkdir(parents=True, exist_ok=True)
    for bucket in DRAFT_BUCKETS:
        (root / "drafts" / bucket).mkdir(parents=True, exist_ok=True)


def object_path(root: Path, base: str, object_id: str) -> Path:
    parts = object_id.split(".")
    kind = parts[0]
    leaf = ".".join(parts[1:]) or object_id
    return root / base / kind / f"{leaf}.yaml"


def spec_path(root: Path, spec_id: str) -> Path:
    return object_path(root, "current", spec_id)


def guide_path(root: Path, target_id: str) -> Path:
    return object_path(root, "guides", target_id)


def draft_slug(draft_id: str) -> str:
    return sanitize(draft_id)


def draft_dir(root: Path, draft_id: str, bucket: str = "open") -> Path:
    if bucket not in DRAFT_BUCKETS:
        raise SpecError(f"draft bucket must be one of {list(DRAFT_BUCKETS)}")
    return root / "drafts" / bucket / draft_slug(draft_id)


def draft_file(root: Path, draft_id: str, bucket: str = "open") -> Path:
    return draft_dir(root, draft_id, bucket) / "draft.yaml"


def sanitize(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "-", value).strip("-") or "spec"


def next_log_path(root: Path, op: str, target: str) -> Path:
    log_dir = root / "log"
    log_dir.mkdir(parents=True, exist_ok=True)
    current = 0
    for path in log_dir.glob("*.patch.yaml"):
        match = re.match(r"^(\d+)", path.name)
        if match:
            current = max(current, int(match.group(1)))
    return log_dir / f"{current + 1:04d}.{sanitize(op)}.{sanitize(target)}.patch.yaml"


def archive_snapshot(root: Path, spec: dict[str, Any], source_path: Path) -> Path:
    stamp = dt.datetime.now().strftime("%Y-%m-%dT%H%M%S%f")
    current_root = root / "current"
    try:
        relative = source_path.relative_to(current_root)
    except ValueError:
        relative = source_path.name
    archived = copy.deepcopy(spec)
    archived["status"] = "archived"
    destination = root / "archive" / stamp / relative
    dump_yaml(destination, archived)
    return destination


def archive_guide(root: Path, guide: dict[str, Any], source_path: Path) -> Path:
    stamp = dt.datetime.now().strftime("%Y-%m-%dT%H%M%S%f")
    guides_root = root / "guides"
    try:
        relative = source_path.relative_to(guides_root)
    except ValueError:
        relative = source_path.name
    archived = copy.deepcopy(guide)
    archived["status"] = "archived"
    destination = root / "archive" / stamp / "guides" / relative
    dump_yaml(destination, archived)
    return destination


def load_current(root: Path) -> tuple[dict[str, dict[str, Any]], dict[str, Path], list[str]]:
    current_root = root / "current"
    specs: dict[str, dict[str, Any]] = {}
    paths: dict[str, Path] = {}
    errors: list[str] = []
    if not current_root.exists():
        return specs, paths, [f"missing current directory: {current_root}"]

    for path in sorted(current_root.rglob("*.yaml")):
        try:
            data = load_yaml(path)
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{path}: cannot parse YAML: {exc}")
            continue
        if not isinstance(data, dict):
            errors.append(f"{path}: spec must be a YAML object")
            continue
        spec_id = data.get("id")
        if not isinstance(spec_id, str) or not spec_id:
            errors.append(f"{path}: missing string id")
            continue
        if spec_id in specs:
            errors.append(f"{path}: duplicate id {spec_id}; first seen at {paths[spec_id]}")
            continue
        specs[spec_id] = data
        paths[spec_id] = path
    return specs, paths, errors


def load_guides(root: Path) -> tuple[dict[str, list[dict[str, Any]]], dict[str, list[Path]], list[str]]:
    guides_root = root / "guides"
    guides: dict[str, list[dict[str, Any]]] = {}
    paths: dict[str, list[Path]] = {}
    errors: list[str] = []
    if not guides_root.exists():
        return guides, paths, []

    for path in sorted(guides_root.rglob("*.yaml")):
        try:
            data = load_yaml(path)
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{path}: cannot parse YAML: {exc}")
            continue
        if not isinstance(data, dict):
            errors.append(f"{path}: guide must be a YAML object")
            continue
        target = data.get("target")
        if not isinstance(target, str) or not target:
            errors.append(f"{path}: missing string target")
            continue
        guides.setdefault(target, []).append(data)
        paths.setdefault(target, []).append(path)
    return guides, paths, errors


def validate_current_shape(spec: dict[str, Any], path: Path) -> list[str]:
    errors: list[str] = []
    forbidden = sorted(FORBIDDEN_CURRENT_KEYS.intersection(spec.keys()))
    if forbidden:
        errors.append(f"{path}: current Spec must not contain narrative fields: {forbidden}")

    summary = spec.get("summary")
    if summary is not None:
        if not isinstance(summary, str):
            errors.append(f"{path}: summary must be a short string label")
        elif "\n" in summary or len(summary) > 160:
            errors.append(f"{path}: summary must be one short label, not narrative documentation")

    constraints = spec.get("constraints")
    if constraints is not None and not isinstance(constraints, dict):
        errors.append(f"{path}: constraints must be an object, not narrative or list text")

    invariants = spec.get("invariants")
    if invariants is not None:
        if not isinstance(invariants, list):
            errors.append(f"{path}: invariants must be a list")
        else:
            for invariant in invariants:
                if not isinstance(invariant, str) or not invariant:
                    errors.append(f"{path}: invariant values must be non-empty strings")

    semantics = spec.get("semantics")
    if semantics is not None:
        if not isinstance(semantics, dict):
            errors.append(f"{path}: semantics must be an object keyed by structured terms")
        else:
            for key, value in semantics.items():
                if not isinstance(key, str) or not key:
                    errors.append(f"{path}: semantics keys must be non-empty strings")
                if not isinstance(value, str) or not value:
                    errors.append(f"{path}: semantics.{key} must be a non-empty string explanation")
    return errors


def validate_specs(specs: dict[str, dict[str, Any]], paths: dict[str, Path], root: Path) -> list[str]:
    errors: list[str] = []
    current_root = root / "current"
    for spec_id, spec in sorted(specs.items()):
        path = paths.get(spec_id, spec_path(root, spec_id))
        kind = spec.get("kind")
        version = spec.get("version")
        status = spec.get("status")

        errors.extend(validate_current_shape(spec, path))
        if not isinstance(kind, str) or kind not in KINDS:
            errors.append(f"{path}: kind must be one of {sorted(KINDS)}")
        if isinstance(kind, str) and not spec_id.startswith(f"{kind}."):
            errors.append(f"{path}: id must start with kind prefix '{kind}.'")
        if not isinstance(version, int) or version < 1:
            errors.append(f"{path}: version must be a positive integer")
        if status != "current":
            errors.append(f"{path}: current spec status must be 'current'")
        if BAD_VERSION_NAME.search(path.stem):
            errors.append(f"{path}: file name must not encode versions like v2/final/latest")

        try:
            relative = path.relative_to(current_root)
            if relative.parts and relative.parts[0] in KINDS and kind in KINDS:
                if relative.parts[0] != kind:
                    errors.append(f"{path}: directory kind '{relative.parts[0]}' does not match spec kind '{kind}'")
        except ValueError:
            pass

        relations = spec.get("relations") or {}
        if not isinstance(relations, dict):
            errors.append(f"{path}: relations must be an object")
            continue
        for key in RELATION_KEYS:
            values = relations.get(key, [])
            if values is None:
                continue
            if not isinstance(values, list):
                errors.append(f"{path}: relations.{key} must be a list")
                continue
            for related_id in values:
                if not isinstance(related_id, str):
                    errors.append(f"{path}: relations.{key} values must be strings")
                elif related_id not in specs:
                    errors.append(f"{path}: relations.{key} references missing spec {related_id}")
    return errors


def validate_guides(
    guides: dict[str, list[dict[str, Any]]],
    guide_paths: dict[str, list[Path]],
    specs: dict[str, dict[str, Any]],
    root: Path,
) -> list[str]:
    errors: list[str] = []
    guides_root = root / "guides"
    for target, guide_list in sorted(guides.items()):
        if len(guide_list) > 1:
            duplicate_paths = guide_paths.get(target, [])
            first_path = duplicate_paths[0] if duplicate_paths else guide_path(root, target)
            other_paths = ", ".join(str(path) for path in duplicate_paths[1:]) or "unknown duplicate path"
            errors.append(f"{first_path}: duplicate Guide target {target}; also seen at {other_paths}")
        for index, guide in enumerate(guide_list):
            path = guide_paths.get(target, [guide_path(root, target)])[index]
            guide_id = guide.get("id")
            kind = guide.get("kind")
            expected_id = f"guide.{target}"
            if guide_id != expected_id:
                errors.append(f"{path}: guide id must be '{expected_id}'")
            if kind != "guide":
                errors.append(f"{path}: guide kind must be 'guide'")
            if target not in specs:
                errors.append(f"{path}: guide target is missing from current Snapshot: {target}")
            try:
                relative = path.relative_to(guides_root)
                target_kind = target.split(".", 1)[0]
                if relative.parts and relative.parts[0] in KINDS and relative.parts[0] != target_kind:
                    errors.append(f"{path}: guide directory kind must match target kind '{target_kind}'")
            except ValueError:
                pass
            for forbidden_key in ("constraints", "invariants"):
                if forbidden_key in guide:
                    errors.append(f"{path}: Guide must not override Spec {forbidden_key}")
    return errors


def split_path(path: str) -> list[str]:
    parts = [part for part in path.split(".") if part]
    if not parts:
        raise SpecError("empty change path")
    return parts


def parent_for(data: dict[str, Any], dotted: str, create: bool = False) -> tuple[dict[str, Any], str]:
    parts = split_path(dotted)
    node: Any = data
    for part in parts[:-1]:
        if not isinstance(node, dict):
            raise SpecError(f"cannot traverse non-object path at {part} in {dotted}")
        if part not in node:
            if create:
                node[part] = {}
            else:
                raise SpecError(f"path does not exist: {dotted}")
        node = node[part]
    if not isinstance(node, dict):
        raise SpecError(f"parent is not an object: {dotted}")
    return node, parts[-1]


def set_dotted(data: dict[str, Any], dotted: str, value: Any) -> None:
    parent, key = parent_for(data, dotted, create=True)
    parent[key] = copy.deepcopy(value)


def get_dotted(data: dict[str, Any], dotted: str) -> Any:
    parent, key = parent_for(data, dotted, create=False)
    if key not in parent:
        raise SpecError(f"path does not exist: {dotted}")
    return parent[key]


def add_dotted(data: dict[str, Any], dotted: str, value: Any) -> None:
    parent, key = parent_for(data, dotted, create=True)
    if key not in parent:
        parent[key] = copy.deepcopy(value)
        return

    current = parent[key]
    if isinstance(current, list):
        values = value if isinstance(value, list) else [value]
        for item in values:
            if item not in current:
                current.append(copy.deepcopy(item))
        return
    if isinstance(current, dict) and isinstance(value, dict):
        for item_key, item_value in value.items():
            if item_key in current:
                raise SpecError(f"add would overwrite existing key: {dotted}.{item_key}")
            current[item_key] = copy.deepcopy(item_value)
        return
    raise SpecError(f"add requires a list or object target: {dotted}")


def delete_dotted(data: dict[str, Any], dotted: str) -> None:
    parent, key = parent_for(data, dotted, create=False)
    if key not in parent:
        raise SpecError(f"path does not exist: {dotted}")
    del parent[key]


def remove_dotted(data: dict[str, Any], dotted: str, value: Any = None) -> None:
    if value is None or value is True:
        delete_dotted(data, dotted)
        return
    current = get_dotted(data, dotted)
    if isinstance(current, list):
        values = value if isinstance(value, list) else [value]
        parent, key = parent_for(data, dotted, create=False)
        parent[key] = [item for item in current if item not in values]
        return
    raise SpecError(f"remove with value requires a list target: {dotted}")


def flatten_changes(values: dict[str, Any], prefix: str = "") -> list[tuple[str, Any]]:
    result: list[tuple[str, Any]] = []
    for key, value in values.items():
        dotted = f"{prefix}.{key}" if prefix else str(key)
        if isinstance(value, dict) and value:
            result.extend(flatten_changes(value, dotted))
        else:
            result.append((dotted, value))
    return result


def apply_changes(spec: dict[str, Any], changes: dict[str, Any]) -> None:
    if not isinstance(changes, dict) or not changes:
        raise SpecError("update patch requires non-empty changes object")

    if any(key in changes for key in CHANGE_OPERATOR_KEYS):
        direct_keys = sorted(str(key) for key in changes if key not in CHANGE_OPERATOR_KEYS)
        if direct_keys:
            raise SpecError(
                "changes cannot mix operator keys (set/add/remove) with direct paths: "
                + ", ".join(direct_keys)
            )

        set_changes = changes.get("set") or {}
        add_changes = changes.get("add") or {}
        if not isinstance(set_changes, dict):
            raise SpecError("changes.set must be an object")
        if not isinstance(add_changes, dict):
            raise SpecError("changes.add must be an object")

        for dotted, value in flatten_changes(set_changes):
            set_dotted(spec, dotted, value)
        for dotted, value in flatten_changes(add_changes):
            add_dotted(spec, dotted, value)
        remove_changes = changes.get("remove") or []
        if isinstance(remove_changes, list):
            for dotted in remove_changes:
                remove_dotted(spec, str(dotted))
        elif isinstance(remove_changes, dict):
            for dotted, value in flatten_changes(remove_changes):
                remove_dotted(spec, dotted, value)
        else:
            raise SpecError("changes.remove must be a list or object")
        return

    for dotted, value in flatten_changes(changes):
        set_dotted(spec, dotted, value)


def check_expected_version(patch: dict[str, Any], spec: dict[str, Any]) -> None:
    expected = patch.get("expected_version")
    if expected is None:
        return
    current = spec.get("version")
    if expected != current:
        raise SpecError(f"version conflict for {spec.get('id')}: expected {expected}, current {current}")


def write_patch_log(root: Path, patch: dict[str, Any]) -> Path:
    logged = copy.deepcopy(patch)
    logged["applied_at"] = dt.datetime.now().isoformat(timespec="seconds")
    log_path = next_log_path(root, str(patch.get("op", "patch")), str(patch.get("target", "unknown")))
    dump_yaml(log_path, logged)
    return log_path


def now_iso() -> str:
    return dt.datetime.now().isoformat(timespec="seconds")


def safe_relative(path: Path, root: Path) -> str:
    try:
        return str(path.resolve().relative_to(root.resolve()))
    except ValueError:
        return str(path)


def find_draft(root: Path, draft_id: str, bucket: str | None = None) -> tuple[str, Path]:
    buckets = (bucket,) if bucket else DRAFT_BUCKETS
    matches: list[tuple[str, Path]] = []
    for item in buckets:
        if item not in DRAFT_BUCKETS:
            raise SpecError(f"draft bucket must be one of {list(DRAFT_BUCKETS)}")
        path = draft_file(root, draft_id, item)
        if path.exists():
            matches.append((item, path))
    if not matches:
        scope = bucket or "any bucket"
        raise SpecError(f"draft not found in {scope}: {draft_id}")
    if len(matches) > 1:
        locations = ", ".join(f"{item}:{path}" for item, path in matches)
        raise SpecError(f"draft id is ambiguous across buckets: {locations}")
    return matches[0]


def load_draft(root: Path, draft_id: str, bucket: str | None = "open") -> tuple[str, Path, dict[str, Any]]:
    found_bucket, path = find_draft(root, draft_id, bucket)
    draft = load_yaml(path)
    if not isinstance(draft, dict):
        raise SpecError(f"{path}: draft must be a YAML object")
    return found_bucket, path, draft


def validate_draft_shape(root: Path, bucket: str, path: Path, draft: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    draft_id = draft.get("id")
    kind = draft.get("kind")
    status = draft.get("status")
    if not isinstance(draft_id, str) or not draft_id:
        errors.append(f"{path}: draft id must be a non-empty string")
    if kind != "draft":
        errors.append(f"{path}: draft kind must be 'draft'")
    if status not in DRAFT_WORKFLOW_STATUSES:
        errors.append(f"{path}: draft status must be one of {sorted(DRAFT_WORKFLOW_STATUSES)}")
    elif bucket == "open" and status not in DRAFT_OPEN_STATUSES:
        errors.append(f"{path}: open drafts must use one of {sorted(DRAFT_OPEN_STATUSES)}")
    elif bucket in DRAFT_TERMINAL_STATUSES and status != bucket:
        errors.append(f"{path}: {bucket} drafts must have status '{bucket}'")

    title = draft.get("title")
    if title is not None and not isinstance(title, str):
        errors.append(f"{path}: draft title must be a string")
    for key in ("goals", "non_goals", "questions", "related_specs"):
        values = draft.get(key, [])
        if values is None:
            continue
        if not isinstance(values, list):
            errors.append(f"{path}: draft {key} must be a list")
            continue
        for value in values:
            if not isinstance(value, str):
                errors.append(f"{path}: draft {key} values must be strings")

    candidate_changes = draft.get("candidate_changes", {})
    if candidate_changes is None:
        candidate_changes = {}
    if not isinstance(candidate_changes, dict):
        errors.append(f"{path}: candidate_changes must be an object")
    else:
        patches = candidate_changes.get("patches", [])
        if patches is None:
            patches = []
        if not isinstance(patches, list):
            errors.append(f"{path}: candidate_changes.patches must be a list")
        else:
            for patch_ref in patches:
                if not isinstance(patch_ref, str) or not patch_ref:
                    errors.append(f"{path}: candidate_changes.patches values must be non-empty strings")

    related_specs = draft.get("related_specs", []) or []
    if isinstance(related_specs, list) and related_specs:
        specs, spec_paths, load_errors = load_current(root)
        current_errors = load_errors + validate_specs(specs, spec_paths, root)
        if current_errors:
            errors.extend(current_errors)
        else:
            for spec_id in related_specs:
                if isinstance(spec_id, str) and spec_id not in specs:
                    errors.append(f"{path}: related_specs references missing current Spec {spec_id}")
    return errors


def candidate_patch_refs(draft: dict[str, Any]) -> list[str]:
    candidate_changes = draft.get("candidate_changes") or {}
    if not isinstance(candidate_changes, dict):
        return []
    patches = candidate_changes.get("patches") or []
    return [patch_ref for patch_ref in patches if isinstance(patch_ref, str) and patch_ref]


def resolve_draft_path(draft_root: Path, reference: str) -> Path:
    relative = Path(reference)
    if relative.is_absolute():
        raise SpecError(f"draft candidate patch path must be relative: {reference}")
    resolved = (draft_root / relative).resolve()
    draft_resolved = draft_root.resolve()
    try:
        resolved.relative_to(draft_resolved)
    except ValueError as exc:
        raise SpecError(f"draft candidate patch path escapes draft directory: {reference}") from exc
    return resolved


def load_draft_patches(draft_root: Path, draft: dict[str, Any]) -> list[dict[str, Any]]:
    patches: list[dict[str, Any]] = []
    for patch_ref in candidate_patch_refs(draft):
        patch_path = resolve_draft_path(draft_root, patch_ref)
        if not patch_path.exists():
            raise SpecError(f"draft candidate patch not found: {patch_path}")
        patches.append(load_patch_file(patch_path))
    return patches


def write_text_if_missing(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_text(content, encoding="utf-8")


def append_note(path: Path, heading: str, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    prefix = "\n" if path.exists() and path.read_text(encoding="utf-8").strip() else ""
    with path.open("a", encoding="utf-8") as handle:
        handle.write(f"{prefix}## {heading}\n\n{text.strip()}\n")


def list_field(draft: dict[str, Any], key: str) -> list[Any]:
    values = draft.setdefault(key, [])
    if not isinstance(values, list):
        raise SpecError(f"draft {key} must be a list")
    return values


def add_unique(values: list[Any], additions: list[str] | None) -> None:
    for value in additions or []:
        if value not in values:
            values.append(value)


def move_draft(root: Path, draft_id: str, source_bucket: str, target_bucket: str, draft: dict[str, Any]) -> Path:
    source_dir = draft_dir(root, draft_id, source_bucket)
    destination_dir = draft_dir(root, draft_id, target_bucket)
    if destination_dir.exists():
        raise SpecError(f"target draft directory already exists: {destination_dir}")
    dump_yaml(source_dir / "draft.yaml", draft)
    destination_dir.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(source_dir), str(destination_dir))
    return destination_dir


def validate_open_drafts(root: Path) -> tuple[int, list[str]]:
    open_root = root / "drafts" / "open"
    if not open_root.exists():
        return 0, []
    errors: list[str] = []
    count = 0
    for path in sorted(open_root.glob("*/draft.yaml")):
        count += 1
        try:
            draft = load_yaml(path)
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{path}: cannot parse YAML: {exc}")
            continue
        if not isinstance(draft, dict):
            errors.append(f"{path}: draft must be a YAML object")
            continue
        errors.extend(validate_draft_shape(root, "open", path, draft))
        if draft.get("status") in {"proposed", "validated"} and candidate_patch_refs(draft):
            try:
                plan_patch_set(root, load_draft_patches(path.parent, draft))
            except SpecError as exc:
                errors.append(f"{path}: candidate patches do not dry-run cleanly: {exc}")
    return count, errors


def example_specs() -> dict[str, dict[str, Any]]:
    return {
        "system.workspace": {
            "id": "system.workspace",
            "kind": "system",
            "version": 1,
            "status": "current",
            "summary": "定义 workspace 边界和文件访问规则",
            "constraints": {"scope": "workspace-only"},
            "invariants": ["writes_must_not_escape_workspace"],
            "semantics": {
                "writes_must_not_escape_workspace": "writes must stay under the configured workspace root",
            },
            "relations": {"depends_on": [], "extends": [], "constrains": []},
        },
        "capability.file.write": {
            "id": "capability.file.write",
            "kind": "capability",
            "version": 1,
            "status": "current",
            "summary": "写入 workspace 内的文件内容",
            "input": {"path": "string", "content": "string"},
            "output": {"written": "boolean"},
            "constraints": {"max_size": "1MB", "scope": "workspace-only"},
            "invariants": ["must_be_atomic", "must_be_idempotent"],
            "semantics": {
                "must_be_atomic": "write is all-or-nothing",
                "must_be_idempotent": "same input produces same final file content",
            },
            "relations": {"depends_on": ["system.workspace"], "extends": [], "constrains": []},
        },
    }


def example_guides() -> dict[str, dict[str, Any]]:
    return {
        "capability.file.write": {
            "id": "guide.capability.file.write",
            "kind": "guide",
            "target": "capability.file.write",
            "summary": "写文件到 workspace",
            "notes": ["Use atomic write to avoid partial output."],
            "examples": [{"input": {"path": "a.txt", "content": "hi"}}],
        }
    }


def command_init(args: argparse.Namespace) -> int:
    root = Path(args.root)
    ensure_layout(root)
    readme = root / "README.md"
    if not readme.exists():
        readme.write_text(
            "# Spec System\n\n"
            "- `current/` 是 LLM 默认唯一可读的 Current State Snapshot。\n"
            "- `drafts/` 是非权威 Draft Workspace，用于调研、方案、测试证据和候选 Patch。\n"
            "- `guides/` 是弱语义解释层，默认不进决策 context。\n"
            "- `log/` 是 Patch Log，默认不进 context。\n"
            "- `archive/` 是历史 Snapshot，默认不参与当前决策。\n",
            encoding="utf-8",
        )
    if args.with_examples:
        for spec_id, spec in example_specs().items():
            path = spec_path(root, spec_id)
            if not path.exists():
                dump_yaml(path, spec)
        for target, guide in example_guides().items():
            path = guide_path(root, target)
            if not path.exists():
                dump_yaml(path, guide)
    print(f"initialized Spec System at {root}")
    return 0


def command_validate(args: argparse.Namespace) -> int:
    root = Path(args.root)
    specs, spec_paths, load_errors = load_current(root)
    guides, guide_paths, guide_errors = load_guides(root)
    draft_count, draft_errors = validate_open_drafts(root)
    errors = load_errors + guide_errors
    errors.extend(validate_specs(specs, spec_paths, root))
    errors.extend(validate_guides(guides, guide_paths, specs, root))
    errors.extend(draft_errors)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    guide_count = sum(len(items) for items in guides.values())
    print(
        f"OK: {len(specs)} current Spec Object(s), "
        f"{guide_count} Guide Object(s), and {draft_count} open Draft(s) validated"
    )
    return 0


def collect_with_deps(specs: dict[str, dict[str, Any]], start: str) -> list[dict[str, Any]]:
    seen: set[str] = set()
    ordered: list[dict[str, Any]] = []
    stack = [start]
    while stack:
        spec_id = stack.pop(0)
        if spec_id in seen:
            continue
        if spec_id not in specs:
            raise SpecError(f"missing spec: {spec_id}")
        seen.add(spec_id)
        spec = specs[spec_id]
        ordered.append(spec)
        relations = spec.get("relations") or {}
        for key in RELATION_KEYS:
            for related_id in relations.get(key, []) or []:
                if related_id not in seen:
                    stack.append(related_id)
    return ordered


def command_read(args: argparse.Namespace) -> int:
    root = Path(args.root)
    specs, spec_paths, load_errors = load_current(root)
    guides, guide_paths, guide_errors = load_guides(root)
    errors = load_errors + guide_errors
    errors.extend(validate_specs(specs, spec_paths, root))
    errors.extend(validate_guides(guides, guide_paths, specs, root))
    if errors:
        raise SpecError("Spec System is invalid; run validate for details")
    if args.spec_id not in specs:
        raise SpecError(f"spec not found in current Snapshot: {args.spec_id}")
    selected = collect_with_deps(specs, args.spec_id) if args.with_deps else [specs[args.spec_id]]
    output: dict[str, Any] = {"specs": selected}
    if args.with_guide:
        selected_ids = [spec["id"] for spec in selected]
        selected_guides: list[dict[str, Any]] = []
        for spec_id in selected_ids:
            selected_guides.extend(guides.get(spec_id, []))
        output["guides"] = selected_guides
    print(yaml.safe_dump(output, allow_unicode=True, sort_keys=False).strip())
    return 0


def load_patch_file(path: Path) -> dict[str, Any]:
    patch = load_yaml(path)
    if not isinstance(patch, dict):
        raise SpecError(f"{path}: patch must be a YAML object")
    return patch


def patch_operation(patch: dict[str, Any]) -> tuple[str, str]:
    op = patch.get("op")
    target = patch.get("target")
    if op not in {"create", "update", "deprecate"}:
        raise SpecError("patch op must be create, update, or deprecate")
    if not isinstance(target, str) or not target:
        raise SpecError("patch target must be a non-empty string")
    return op, target


def stage_patch(
    root: Path,
    patch: dict[str, Any],
    specs: dict[str, dict[str, Any]],
    spec_paths: dict[str, Path],
    guides: dict[str, list[dict[str, Any]]],
    guide_paths: dict[str, list[Path]],
) -> PlannedPatch:
    op, target = patch_operation(patch)

    if op == "create":
        if target in specs:
            raise SpecError(f"spec already exists: {target}")
        spec = patch.get("spec")
        if not isinstance(spec, dict):
            raise SpecError("create patch requires spec object")
        if spec.get("id") != target:
            raise SpecError("create patch spec.id must equal target")
        created = copy.deepcopy(spec)
        created.setdefault("version", 1)
        created.setdefault("status", "current")
        specs[target] = created
        spec_paths[target] = spec_path(root, target)
        return PlannedPatch(patch=patch, op=op, target=target, written_path=spec_paths[target])

    if op == "update":
        if target not in specs:
            raise SpecError(f"spec not found: {target}")
        old_spec = copy.deepcopy(specs[target])
        old_path = spec_paths[target]
        check_expected_version(patch, old_spec)
        updated = copy.deepcopy(old_spec)
        apply_changes(updated, patch.get("changes") or {})
        old_version = old_spec.get("version")
        if not isinstance(old_version, int):
            raise SpecError(f"current version is invalid for {target}")
        updated["id"] = target
        updated["kind"] = old_spec.get("kind")
        updated["status"] = "current"
        updated["version"] = old_version + 1
        specs[target] = updated
        return PlannedPatch(
            patch=patch,
            op=op,
            target=target,
            written_path=old_path,
            old_spec=old_spec,
            old_spec_path=old_path,
        )

    if target not in specs:
        raise SpecError(f"spec not found: {target}")
    old_spec = copy.deepcopy(specs[target])
    old_path = spec_paths[target]
    check_expected_version(patch, old_spec)
    deleted_guides = list(zip(guides.get(target, []), guide_paths.get(target, []), strict=False))
    del specs[target]
    del spec_paths[target]
    guides.pop(target, None)
    guide_paths.pop(target, None)
    return PlannedPatch(
        patch=patch,
        op=op,
        target=target,
        written_path=old_path,
        old_spec=old_spec,
        old_spec_path=old_path,
        deleted_guides=deleted_guides,
    )


def plan_patch_set(root: Path, patches: list[dict[str, Any]]) -> tuple[list[PlannedPatch], dict[str, dict[str, Any]]]:
    if not patches:
        raise SpecError("Patch Set requires at least one patch")

    specs, spec_paths, load_errors = load_current(root)
    guides, guide_paths, guide_errors = load_guides(root)
    if load_errors or guide_errors:
        raise SpecError("; ".join(load_errors + guide_errors))

    staged_specs = copy.deepcopy(specs)
    staged_paths = dict(spec_paths)
    staged_guides = copy.deepcopy(guides)
    staged_guide_paths = {target: list(paths) for target, paths in guide_paths.items()}
    planned: list[PlannedPatch] = []
    seen_targets: set[str] = set()
    for patch in patches:
        _, target = patch_operation(patch)
        if target in seen_targets:
            raise SpecError(f"Patch Set must touch each target at most once; duplicate target: {target}")
        seen_targets.add(target)
        planned.append(stage_patch(root, patch, staged_specs, staged_paths, staged_guides, staged_guide_paths))

    validation_errors = validate_specs(staged_specs, staged_paths, root)
    validation_errors.extend(validate_guides(staged_guides, staged_guide_paths, staged_specs, root))
    if validation_errors:
        raise SpecError("validation failed after patch:\n" + "\n".join(validation_errors))
    return planned, staged_specs


def apply_planned_patch_set(
    root: Path,
    planned: list[PlannedPatch],
    staged_specs: dict[str, dict[str, Any]],
) -> list[PatchResult]:
    results: list[PatchResult] = []
    for item in planned:
        archive_path: Path | None = None
        guide_archive_paths: list[Path] = []

        if item.op == "create":
            if item.written_path is None:
                raise SpecError(f"create patch missing write path: {item.target}")
            dump_yaml(item.written_path, staged_specs[item.target])
        elif item.op == "update":
            if item.old_spec is None or item.old_spec_path is None:
                raise SpecError(f"update patch missing archive source: {item.target}")
            archive_path = archive_snapshot(root, item.old_spec, item.old_spec_path)
            dump_yaml(item.old_spec_path, staged_specs[item.target])
        else:
            if item.old_spec is None or item.old_spec_path is None:
                raise SpecError(f"deprecate patch missing archive source: {item.target}")
            archive_path = archive_snapshot(root, item.old_spec, item.old_spec_path)
            item.old_spec_path.unlink()
            for guide, path in item.deleted_guides:
                guide_archive_paths.append(archive_guide(root, guide, path))
                path.unlink()

        log_path = write_patch_log(root, item.patch)
        results.append(
            PatchResult(
                op=item.op,
                target=item.target,
                current_path=item.written_path,
                archive_path=archive_path,
                log_path=log_path,
                guide_archive_paths=guide_archive_paths,
            )
        )
    return results


def print_patch_results(results: list[PatchResult]) -> None:
    for result in results:
        print(f"applied {result.op} patch to {result.target}")
        if result.current_path:
            print(f"current: {result.current_path}")
        if result.archive_path:
            print(f"archive: {result.archive_path}")
        for guide_archive in result.guide_archive_paths:
            print(f"guide archive: {guide_archive}")
        if result.log_path:
            print(f"log: {result.log_path}")


def command_patch(args: argparse.Namespace) -> int:
    root = Path(args.root)
    ensure_layout(root)
    patch = load_patch_file(Path(args.patch_file))
    planned, staged_specs = plan_patch_set(root, [patch])
    results = apply_planned_patch_set(root, planned, staged_specs)
    print_patch_results(results)
    return 0


def command_draft_start(args: argparse.Namespace) -> int:
    root = Path(args.root)
    ensure_layout(root)
    path = draft_file(root, args.draft_id, "open")
    if path.exists():
        raise SpecError(f"draft already exists: {path}")
    for bucket in DRAFT_TERMINAL_STATUSES:
        if draft_file(root, args.draft_id, bucket).exists():
            raise SpecError(f"draft already exists in {bucket}: {args.draft_id}")

    draft_root = path.parent
    title = args.title or args.draft_id
    now = now_iso()
    draft = {
        "id": args.draft_id,
        "kind": "draft",
        "status": "researching",
        "title": title,
        "problem": args.problem or "",
        "goals": args.goal or [],
        "non_goals": [],
        "questions": [],
        "related_specs": args.related_spec or [],
        "candidate_changes": {"patches": []},
        "validation": {
            "test_plan": "test-plan.md",
            "evidence": "evidence.md",
            "commands": [],
        },
        "created_at": now,
        "updated_at": now,
    }
    dump_yaml(path, draft)
    (draft_root / "candidate-patches").mkdir(parents=True, exist_ok=True)
    write_text_if_missing(
        draft_root / "research.md",
        f"# Research: {title}\n\nCapture user interviews, code findings, constraints, and open questions here.\n",
    )
    write_text_if_missing(
        draft_root / "analysis.md",
        f"# Analysis: {title}\n\nCompare options, risks, compatibility concerns, and affected Specs here.\n",
    )
    write_text_if_missing(
        draft_root / "test-plan.md",
        f"# Test Plan: {title}\n\nList validation scenarios and commands before accepting the draft.\n",
    )
    write_text_if_missing(
        draft_root / "evidence.md",
        f"# Evidence: {title}\n\nRecord test results, links, logs, screenshots, and review notes here.\n",
    )
    write_text_if_missing(
        draft_root / "candidate-patches" / "README.md",
        "# Candidate Patches\n\nPlace YAML Spec Patch files here and list them in `draft.yaml` under `candidate_changes.patches`.\n",
    )
    print(f"created draft: {path}")
    print(f"workspace: {draft_root}")
    return 0


def command_draft_list(args: argparse.Namespace) -> int:
    root = Path(args.root)
    buckets = DRAFT_BUCKETS if args.all else ("open",)
    rows: list[tuple[str, str, str, str]] = []
    for bucket in buckets:
        bucket_root = root / "drafts" / bucket
        if not bucket_root.exists():
            continue
        for path in sorted(bucket_root.glob("*/draft.yaml")):
            draft = load_yaml(path)
            if not isinstance(draft, dict):
                continue
            rows.append((bucket, str(draft.get("id", path.parent.name)), str(draft.get("status", "")), str(draft.get("title", ""))))
    if not rows:
        print("no drafts found")
        return 0
    for bucket, draft_id, status, title in rows:
        print(f"{bucket}\t{status}\t{draft_id}\t{title}")
    return 0


def command_draft_research(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft_root = path.parent
    add_unique(list_field(draft, "questions"), args.question)
    add_unique(list_field(draft, "related_specs"), args.related_spec)
    if args.problem:
        draft["problem"] = args.problem
    if args.note:
        append_note(draft_root / "research.md", now_iso(), args.note)
    draft["status"] = "researching"
    draft["updated_at"] = now_iso()
    dump_yaml(path, draft)
    print(f"updated draft research: {path}")
    if args.note:
        print(f"research: {draft_root / 'research.md'}")
    return 0


def command_draft_propose(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft_root = path.parent
    for patch_ref in args.patch_ref or []:
        patch_path = resolve_draft_path(draft_root, patch_ref)
        if not patch_path.exists():
            raise SpecError(f"candidate patch file does not exist: {patch_path}")
        load_patch_file(patch_path)
    if args.note:
        append_note(draft_root / "analysis.md", now_iso(), args.note)
    candidate_changes = draft.setdefault("candidate_changes", {})
    if not isinstance(candidate_changes, dict):
        raise SpecError("candidate_changes must be an object")
    patches = candidate_changes.setdefault("patches", [])
    if not isinstance(patches, list):
        raise SpecError("candidate_changes.patches must be a list")
    add_unique(patches, args.patch_ref)
    draft["status"] = "proposed" if patches else "researching"
    draft["updated_at"] = now_iso()
    dump_yaml(path, draft)
    print(f"updated draft proposal: {path}")
    if args.note:
        print(f"analysis: {draft_root / 'analysis.md'}")
    return 0


def command_draft_show(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, args.bucket)
    errors = validate_draft_shape(root, bucket, path, draft)
    output = {"bucket": bucket, "path": str(path), "draft": draft}
    if errors:
        output["errors"] = errors
    print(yaml.safe_dump(output, allow_unicode=True, sort_keys=False).strip())
    return 1 if errors else 0


def command_draft_validate(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, args.bucket)
    draft_root = path.parent
    errors = validate_draft_shape(root, bucket, path, draft)
    patches: list[dict[str, Any]] = []
    if not errors:
        try:
            patches = load_draft_patches(draft_root, draft)
            if args.require_patches and not patches:
                errors.append(f"{path}: draft must list at least one candidate patch")
            if patches:
                plan_patch_set(root, patches)
        except SpecError as exc:
            errors.append(str(exc))
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"OK: draft {args.draft_id} validated with {len(patches)} candidate patch(es)")
    if bucket == "open" and draft.get("status") != "validated" and patches:
        draft["status"] = "validated"
        draft["updated_at"] = now_iso()
        dump_yaml(path, draft)
        print(f"status: validated ({path})")
    return 0


def command_draft_test(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft_root = path.parent
    validation = draft.setdefault("validation", {})
    if not isinstance(validation, dict):
        raise SpecError("draft validation must be an object")
    commands = validation.setdefault("commands", [])
    if not isinstance(commands, list):
        raise SpecError("draft validation.commands must be a list")
    add_unique(commands, args.command)
    if args.plan:
        append_note(draft_root / str(validation.get("test_plan", "test-plan.md")), now_iso(), args.plan)
    if args.evidence:
        append_note(draft_root / str(validation.get("evidence", "evidence.md")), now_iso(), args.evidence)
    draft["updated_at"] = now_iso()
    dump_yaml(path, draft)
    print(f"updated draft validation notes: {path}")
    return 0


def command_draft_accept(args: argparse.Namespace) -> int:
    root = Path(args.root)
    ensure_layout(root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    draft_root = path.parent
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    patches = load_draft_patches(draft_root, draft)
    if not patches:
        raise SpecError("accepted draft must list at least one candidate patch")
    planned, staged_specs = plan_patch_set(root, patches)
    if args.dry_run:
        print(f"OK: draft {args.draft_id} can be accepted with {len(patches)} candidate patch(es)")
        return 0

    results = apply_planned_patch_set(root, planned, staged_specs)
    draft["status"] = "accepted"
    draft["accepted_at"] = now_iso()
    draft["updated_at"] = draft["accepted_at"]
    draft["applied_patches"] = [
        {
            "target": result.target,
            "op": result.op,
            "log": safe_relative(result.log_path, root) if result.log_path else None,
            "current": safe_relative(result.current_path, root) if result.current_path else None,
            "archive": safe_relative(result.archive_path, root) if result.archive_path else None,
        }
        for result in results
    ]
    accepted_dir = move_draft(root, args.draft_id, "open", "accepted", draft)
    print_patch_results(results)
    print(f"draft accepted: {accepted_dir / 'draft.yaml'}")
    return 0


def command_draft_reject(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft["status"] = "rejected"
    draft["rejected_at"] = now_iso()
    draft["updated_at"] = draft["rejected_at"]
    if args.reason:
        draft["rejection_reason"] = args.reason
    rejected_dir = move_draft(root, args.draft_id, "open", "rejected", draft)
    print(f"draft rejected: {rejected_dir / 'draft.yaml'}")
    return 0


def command_draft_supersede(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft["status"] = "superseded"
    draft["superseded_at"] = now_iso()
    draft["updated_at"] = draft["superseded_at"]
    if args.by:
        draft["superseded_by"] = args.by
    if args.reason:
        draft["supersede_reason"] = args.reason
    superseded_dir = move_draft(root, args.draft_id, "open", "superseded", draft)
    print(f"draft superseded: {superseded_dir / 'draft.yaml'}")
    return 0


def command_draft_add_patch(args: argparse.Namespace) -> int:
    root = Path(args.root)
    bucket, path, draft = load_draft(root, args.draft_id, "open")
    errors = validate_draft_shape(root, bucket, path, draft)
    if errors:
        raise SpecError("draft validation failed:\n" + "\n".join(errors))
    draft_root = path.parent
    patch_path = resolve_draft_path(draft_root, args.patch_ref)
    if not patch_path.exists():
        raise SpecError(f"candidate patch file does not exist: {patch_path}")
    load_patch_file(patch_path)
    candidate_changes = draft.setdefault("candidate_changes", {})
    if not isinstance(candidate_changes, dict):
        raise SpecError("candidate_changes must be an object")
    patches = candidate_changes.setdefault("patches", [])
    if not isinstance(patches, list):
        raise SpecError("candidate_changes.patches must be a list")
    if args.patch_ref not in patches:
        patches.append(args.patch_ref)
    draft["status"] = "proposed"
    draft["updated_at"] = now_iso()
    dump_yaml(path, draft)
    print(f"added candidate patch: {args.patch_ref}")
    print(f"draft: {path}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Maintain a Snapshot + Draft + Patch + Guide Spec System")
    subparsers = parser.add_subparsers(dest="command", required=True)

    init_parser = subparsers.add_parser("init", help="create omni-coding/specs/current, guides, drafts, log, and archive")
    init_parser.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    init_parser.add_argument("--with-examples", action="store_true", help="write minimal example Spec and Guide Objects")
    init_parser.set_defaults(func=command_init)

    validate_parser = subparsers.add_parser("validate", help="validate current Snapshot and Guides")
    validate_parser.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    validate_parser.set_defaults(func=command_validate)

    read_parser = subparsers.add_parser("read", help="read a current Spec Object")
    read_parser.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    read_parser.add_argument("spec_id", help="Spec id to read from current Snapshot")
    read_parser.add_argument("--with-deps", action="store_true", help="include related current Specs")
    read_parser.add_argument("--with-guide", action="store_true", help="include weak Guide Object(s) for selected Specs")
    read_parser.set_defaults(func=command_read)

    patch_parser = subparsers.add_parser("patch", help="apply a Spec Patch and compile current Snapshot")
    patch_parser.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    patch_parser.add_argument("patch_file", help="YAML patch file")
    patch_parser.set_defaults(func=command_patch)

    draft_parser = subparsers.add_parser("draft", help="manage non-authoritative feature Draft workspaces")
    draft_subparsers = draft_parser.add_subparsers(dest="draft_command", required=True)

    draft_start = draft_subparsers.add_parser("start", help="create an open feature Draft workspace")
    draft_start.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_start.add_argument("draft_id", help="Draft id, for example draft.feature.search-v2")
    draft_start.add_argument("--title", help="Human-readable Draft title")
    draft_start.add_argument("--problem", help="Problem statement for the Draft")
    draft_start.add_argument("--goal", action="append", help="Goal to add; may be repeated")
    draft_start.add_argument("--related-spec", action="append", help="Current Spec id related to this Draft; may be repeated")
    draft_start.set_defaults(func=command_draft_start)

    draft_list = draft_subparsers.add_parser("list", help="list Draft workspaces")
    draft_list.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_list.add_argument("--all", action="store_true", help="include accepted, rejected, and superseded Drafts")
    draft_list.set_defaults(func=command_draft_list)

    draft_research = draft_subparsers.add_parser("research", help="record research notes and open questions for an open Draft")
    draft_research.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_research.add_argument("draft_id", help="Draft id")
    draft_research.add_argument("--problem", help="Replace the Draft problem statement")
    draft_research.add_argument("--question", action="append", help="Question to add; may be repeated")
    draft_research.add_argument("--related-spec", action="append", help="Related current Spec id to add; may be repeated")
    draft_research.add_argument("--note", help="Research note to append to research.md")
    draft_research.set_defaults(func=command_draft_research)

    draft_propose = draft_subparsers.add_parser("propose", help="record proposal analysis and candidate patch references")
    draft_propose.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_propose.add_argument("draft_id", help="Draft id")
    draft_propose.add_argument("patch_ref", nargs="*", help="Patch path relative to the Draft workspace")
    draft_propose.add_argument("--note", help="Proposal analysis note to append to analysis.md")
    draft_propose.set_defaults(func=command_draft_propose)

    draft_show = draft_subparsers.add_parser("show", help="print Draft metadata")
    draft_show.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_show.add_argument("draft_id", help="Draft id")
    draft_show.add_argument("--bucket", choices=DRAFT_BUCKETS, help="Draft bucket; defaults to searching all buckets")
    draft_show.set_defaults(func=command_draft_show)

    draft_add_patch = draft_subparsers.add_parser("add-patch", help="add a candidate patch reference to an open Draft")
    draft_add_patch.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_add_patch.add_argument("draft_id", help="Draft id")
    draft_add_patch.add_argument("patch_ref", help="Patch path relative to the Draft workspace")
    draft_add_patch.set_defaults(func=command_draft_add_patch)

    draft_validate = draft_subparsers.add_parser("validate", help="validate Draft metadata and dry-run candidate patches")
    draft_validate.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_validate.add_argument("draft_id", help="Draft id")
    draft_validate.add_argument("--bucket", choices=DRAFT_BUCKETS, default="open", help="Draft bucket to validate")
    draft_validate.add_argument("--require-patches", action="store_true", help="fail if the Draft has no candidate patches")
    draft_validate.set_defaults(func=command_draft_validate)

    draft_test = draft_subparsers.add_parser("test", help="record test plan, commands, and evidence for an open Draft")
    draft_test.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_test.add_argument("draft_id", help="Draft id")
    draft_test.add_argument("--command", action="append", help="Validation command to record; may be repeated")
    draft_test.add_argument("--plan", help="Test-plan note to append to test-plan.md")
    draft_test.add_argument("--evidence", help="Evidence note to append to evidence.md")
    draft_test.set_defaults(func=command_draft_test)

    draft_accept = draft_subparsers.add_parser("accept", help="promote an open Draft by applying candidate patches")
    draft_accept.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_accept.add_argument("draft_id", help="Draft id")
    draft_accept.add_argument("--dry-run", action="store_true", help="validate promotion without writing current/log/archive")
    draft_accept.set_defaults(func=command_draft_accept)

    draft_reject = draft_subparsers.add_parser("reject", help="move an open Draft to rejected without changing current")
    draft_reject.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_reject.add_argument("draft_id", help="Draft id")
    draft_reject.add_argument("--reason", help="Reason for rejecting the Draft")
    draft_reject.set_defaults(func=command_draft_reject)

    draft_supersede = draft_subparsers.add_parser("supersede", help="move an open Draft to superseded without changing current")
    draft_supersede.add_argument("root", help="Spec root directory, usually ./omni-coding/specs")
    draft_supersede.add_argument("draft_id", help="Draft id")
    draft_supersede.add_argument("--by", help="Replacement Draft id")
    draft_supersede.add_argument("--reason", help="Reason for superseding the Draft")
    draft_supersede.set_defaults(func=command_draft_supersede)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.func(args)
    except SpecError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
