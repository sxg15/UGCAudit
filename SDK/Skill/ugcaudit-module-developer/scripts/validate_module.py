#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
from typing import Any


KNOWN_KINDS = {"text_safety", "image_safety", "image_ocr", "folder_processor"}
PARAMETER_TYPES = {
    "string",
    "number",
    "boolean",
    "select",
    "multiSelect",
    "stringList",
    "policyList",
    "path",
    "textarea",
}
DATA_TYPES = {"imageCollection", "textCollection", "folder"}
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp", ".svg", ".ico"}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def looks_like_image_path(value: str) -> bool:
    return Path(value).suffix.lower() in IMAGE_EXTENSIONS


def resolve_module_path(module_dir: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else module_dir / path


def validate(module_dir: Path) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    module_json = module_dir / "module.json"
    if not module_dir.is_dir():
        return [f"Module directory does not exist: {module_dir}"], warnings
    if not module_json.is_file():
        return [f"Missing module.json: {module_json}"], warnings

    try:
        module = load_json(module_json)
    except Exception as exc:
        return [f"module.json is not valid JSON: {exc}"], warnings

    for field in ("id", "name", "kind", "icon"):
        if not str(module.get(field, "")).strip():
            errors.append(f"Missing required field: {field}")

    kind = str(module.get("kind", "")).strip()
    if kind and kind not in KNOWN_KINDS:
        warnings.append(f"Unknown kind '{kind}'. The client may not show data ports.")

    icon = str(module.get("icon", "")).strip()
    if icon and looks_like_image_path(icon):
        icon_path = resolve_module_path(module_dir, icon)
        if not icon_path.is_file():
            errors.append(f"Icon file does not exist: {icon_path}")

    launch = module.get("launch")
    if not isinstance(launch, dict):
        errors.append("Missing launch object.")
    else:
        command = str(launch.get("command", "")).strip()
        if not command:
            errors.append("Missing launch.command.")
        else:
            command_path = resolve_module_path(module_dir, command)
            if not command_path.is_file():
                errors.append(f"launch.command file does not exist: {command_path}")
        args = launch.get("args", [])
        if not isinstance(args, list):
            errors.append("launch.args must be an array.")
        else:
            joined_args = " ".join(str(item) for item in args)
            if "{inputJson}" not in joined_args:
                warnings.append("launch.args does not contain {inputJson}; the client will append it.")
            if "{outputJson}" not in joined_args:
                warnings.append("launch.args does not contain {outputJson}; the client will append it.")

    parameters = module.get("parameters", [])
    if parameters is None:
        parameters = []
    if not isinstance(parameters, list):
        errors.append("parameters must be an array.")
    else:
        seen_keys: set[str] = set()
        for index, parameter in enumerate(parameters):
            if not isinstance(parameter, dict):
                errors.append(f"parameters[{index}] must be an object.")
                continue
            key = str(parameter.get("key", "")).strip()
            parameter_type = str(parameter.get("parameterType", "")).strip()
            if not key:
                errors.append(f"parameters[{index}] missing key.")
            elif key in seen_keys:
                errors.append(f"Duplicate parameter key: {key}")
            seen_keys.add(key)
            if parameter_type not in PARAMETER_TYPES:
                errors.append(f"Parameter '{key}' has unsupported parameterType: {parameter_type}")
            if parameter_type in {"select", "multiSelect"}:
                options = parameter.get("options", [])
                if not isinstance(options, list) or not options:
                    errors.append(f"Parameter '{key}' requires non-empty options.")

    outputs = module.get("dataOutputs", [])
    if outputs is None:
        outputs = []
    if not isinstance(outputs, list):
        errors.append("dataOutputs must be an array.")
    else:
        seen_handles: set[str] = set()
        for index, output in enumerate(outputs):
            if not isinstance(output, dict):
                errors.append(f"dataOutputs[{index}] must be an object.")
                continue
            handle = str(output.get("handle", "")).strip()
            data_type = str(output.get("dataType", "")).strip()
            if not handle:
                errors.append(f"dataOutputs[{index}] missing handle.")
            elif handle in seen_handles:
                errors.append(f"Duplicate data output handle: {handle}")
            seen_handles.add(handle)
            if data_type not in DATA_TYPES:
                errors.append(f"Data output '{handle}' has unsupported dataType: {data_type}")

    if not (module_dir / "examples" / "input.demo.json").is_file():
        warnings.append("Missing examples/input.demo.json for smoke testing.")
    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate a UGCAudit module folder.")
    parser.add_argument("module_dir")
    args = parser.parse_args()

    errors, warnings = validate(Path(args.module_dir).resolve())
    for warning in warnings:
        print(f"WARNING: {warning}")
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        print("Validation failed.")
        return 1
    print("Validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

