#!/usr/bin/env python3
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


def resolve_module_path(module_dir: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else module_dir / path


def replace_arg(arg: str, input_path: Path, output_path: Path, resource_root: Path, params_json: str) -> str:
    return (
        arg.replace("{inputJson}", str(input_path))
        .replace("{outputJson}", str(output_path))
        .replace("{resourceRoot}", str(resource_root))
        .replace("{paramsJson}", params_json)
    )


def build_command(module_dir: Path, module: dict[str, Any], input_path: Path, output_path: Path, resource_root: Path) -> list[str]:
    launch = module.get("launch") or {}
    command = str(launch.get("command", "")).strip()
    if not command:
        raise RuntimeError("launch.command is empty.")
    command_path = resolve_module_path(module_dir, command)
    if not command_path.is_file():
        raise RuntimeError(f"launch.command file does not exist: {command_path}")

    prefix = [str(command_path)]
    if command_path.suffix.lower() == ".py":
        prefix = [sys.executable, str(command_path)]

    args = launch.get("args") or ["--input", "{inputJson}", "--output", "{outputJson}"]
    if not isinstance(args, list):
        raise RuntimeError("launch.args must be an array.")
    if not any("{inputJson}" in str(item) for item in args):
        args.extend(["--input", "{inputJson}"])
    if not any("{outputJson}" in str(item) for item in args):
        args.extend(["--output", "{outputJson}"])

    params_json = "{}"
    try:
        input_data = load_json(input_path)
        params_json = json.dumps(input_data.get("params", {}), ensure_ascii=False)
    except Exception:
        pass
    return prefix + [replace_arg(str(item), input_path, output_path, resource_root, params_json) for item in args]


def validate_output(module: dict[str, Any], output_path: Path) -> list[str]:
    problems: list[str] = []
    if not output_path.is_file():
        return [f"Output file was not created: {output_path}"]
    try:
        output = load_json(output_path)
    except Exception as exc:
        return [f"Output file is not valid JSON: {exc}"]

    for field in ("status", "verdict", "message", "reportSection"):
        if field not in output or str(output.get(field, "")).strip() == "":
            problems.append(f"Output missing non-empty field: {field}")

    data_outputs = module.get("dataOutputs") or []
    output_values = output.get("outputs") or {}
    for item in data_outputs:
        if not isinstance(item, dict):
            continue
        handle = str(item.get("handle", "")).strip()
        if handle and handle not in output_values:
            problems.append(f"Declared data output missing from output.json: outputs.{handle}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description="Run a UGCAudit module with examples/input.demo.json.")
    parser.add_argument("module_dir")
    parser.add_argument("--input")
    parser.add_argument("--output")
    parser.add_argument("--work-dir")
    args = parser.parse_args()

    module_dir = Path(args.module_dir).resolve()
    module = load_json(module_dir / "module.json")
    input_path = Path(args.input).resolve() if args.input else module_dir / "examples" / "input.demo.json"
    work_dir = Path(args.work_dir).resolve() if args.work_dir else module_dir / ".ugcaudit-smoke"
    output_path = Path(args.output).resolve() if args.output else work_dir / "output.json"
    resource_root = work_dir / "resources"
    work_dir.mkdir(parents=True, exist_ok=True)
    resource_root.mkdir(parents=True, exist_ok=True)

    command = build_command(module_dir, module, input_path, output_path, resource_root)
    env = os.environ.copy()
    env["UGCAUDIT_RUN_ID"] = "smoke_run"
    env["UGCAUDIT_TASK_NAME"] = "smoke"
    env["UGCAUDIT_RESOURCE_ROOT"] = str(resource_root)
    env["UGCAUDIT_ARTIFACT_DIR"] = str(work_dir / "artifacts")
    env["UGCAUDIT_STEP_ARTIFACT_DIR"] = str(work_dir / "artifacts")
    env["UGCAUDIT_PROGRESS_FILE"] = str(work_dir / "progress.jsonl")
    env["UGCAUDIT_CANCEL_FILE"] = str(work_dir / "cancel.flag")
    env["UGCAUDIT_MODULE_DIR"] = str(module_dir)

    completed = subprocess.run(command, cwd=module_dir, env=env, text=True, capture_output=True)
    (work_dir / "stdout.log").write_text(completed.stdout, encoding="utf-8")
    (work_dir / "stderr.log").write_text(completed.stderr, encoding="utf-8")

    problems = validate_output(module, output_path)
    if completed.returncode != 0:
        problems.append(f"Process exited with code {completed.returncode}")

    if problems:
        for problem in problems:
            print(f"ERROR: {problem}")
        print(f"stdout: {work_dir / 'stdout.log'}")
        print(f"stderr: {work_dir / 'stderr.log'}")
        return 1

    print(f"Smoke test passed: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
