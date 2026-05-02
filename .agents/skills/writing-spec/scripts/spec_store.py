#!/usr/bin/env python3
"""Tiny Spec Store helper for Snapshot + Patch + Guide workflows."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import re
import sys
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None

KINDS = {"capability", "system", "contract"}
RELATION_KEYS = ("depends_on", "extends", "constrains")
CHANGE_OPERATOR_KEYS = {"set", "add", "remove"}
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


def object_path(root: Path, base: str, object_id: str) -> Path:
    parts = object_id.split(".")
    kind = parts[0]
    leaf = ".".join(parts[1:]) or object_id
    return root / base / kind / f"{leaf}.yaml"


def spec_path(root: Path, spec_id: str) -> Path:
    return object_path(root, "current", spec_id)


def guide_path(root: Path, target_id: str) -> Path:
    return object_path(root, "guides", target_id)


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
    errors = load_errors + guide_errors
    errors.extend(validate_specs(specs, spec_paths, root))
    errors.extend(validate_guides(guides, guide_paths, specs, root))
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    guide_count = sum(len(items) for items in guides.values())
    print(f"OK: {len(specs)} current Spec Object(s) and {guide_count} Guide Object(s) validated")
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


def planned_guides_without(
    guides: dict[str, list[dict[str, Any]]],
    guide_paths: dict[str, list[Path]],
    target: str,
) -> tuple[dict[str, list[dict[str, Any]]], dict[str, list[Path]]]:
    planned_guides = {key: list(value) for key, value in guides.items() if key != target}
    planned_paths = {key: list(value) for key, value in guide_paths.items() if key != target}
    return planned_guides, planned_paths


def command_patch(args: argparse.Namespace) -> int:
    root = Path(args.root)
    ensure_layout(root)
    patch_path = Path(args.patch_file)
    patch = load_yaml(patch_path)
    if not isinstance(patch, dict):
        raise SpecError("patch must be a YAML object")
    op = patch.get("op")
    target = patch.get("target")
    if op not in {"create", "update", "deprecate"}:
        raise SpecError("patch op must be create, update, or deprecate")
    if not isinstance(target, str) or not target:
        raise SpecError("patch target must be a non-empty string")

    specs, spec_paths, load_errors = load_current(root)
    guides, guide_paths, guide_errors = load_guides(root)
    if load_errors or guide_errors:
        raise SpecError("; ".join(load_errors + guide_errors))

    new_specs = copy.deepcopy(specs)
    new_paths = dict(spec_paths)
    archived_path: Path | None = None
    written_path: Path | None = None
    archived_guides: list[Path] = []

    if op == "create":
        if target in specs:
            raise SpecError(f"spec already exists: {target}")
        spec = patch.get("spec")
        if not isinstance(spec, dict):
            raise SpecError("create patch requires spec object")
        if spec.get("id") != target:
            raise SpecError("create patch spec.id must equal target")
        spec.setdefault("version", 1)
        spec.setdefault("status", "current")
        new_specs[target] = copy.deepcopy(spec)
        new_paths[target] = spec_path(root, target)
        written_path = new_paths[target]
        planned_guides = guides
        planned_guide_paths = guide_paths

    elif op == "update":
        if target not in specs:
            raise SpecError(f"spec not found: {target}")
        old_spec = specs[target]
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
        new_specs[target] = updated
        written_path = spec_paths[target]
        planned_guides = guides
        planned_guide_paths = guide_paths

    else:
        if target not in specs:
            raise SpecError(f"spec not found: {target}")
        check_expected_version(patch, specs[target])
        del new_specs[target]
        written_path = spec_paths[target]
        planned_guides, planned_guide_paths = planned_guides_without(guides, guide_paths, target)

    validation_errors = validate_specs(new_specs, new_paths, root)
    validation_errors.extend(validate_guides(planned_guides, planned_guide_paths, new_specs, root))
    if validation_errors:
        raise SpecError("validation failed after patch:\n" + "\n".join(validation_errors))

    if op == "update":
        archived_path = archive_snapshot(root, specs[target], spec_paths[target])
        dump_yaml(spec_paths[target], new_specs[target])
    elif op == "create":
        dump_yaml(written_path, new_specs[target])
    else:
        archived_path = archive_snapshot(root, specs[target], spec_paths[target])
        spec_paths[target].unlink()
        for guide, path in zip(guides.get(target, []), guide_paths.get(target, []), strict=False):
            archived_guides.append(archive_guide(root, guide, path))
            path.unlink()

    log_path = write_patch_log(root, patch)
    print(f"applied {op} patch to {target}")
    if written_path:
        print(f"current: {written_path}")
    if archived_path:
        print(f"archive: {archived_path}")
    for archived_guide in archived_guides:
        print(f"guide archive: {archived_guide}")
    print(f"log: {log_path}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Maintain a Snapshot + Patch + Guide Spec System")
    subparsers = parser.add_subparsers(dest="command", required=True)

    init_parser = subparsers.add_parser("init", help="create omni-coding/specs/current, guides, log, and archive")
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
