#!/usr/bin/env python3
import argparse
import shutil
from pathlib import Path


KINDS = ("text_safety", "image_safety", "image_ocr", "folder_processor")


def skill_root() -> Path:
    return Path(__file__).resolve().parents[1]


def default_folder_name(module_id: str) -> str:
    return module_id.strip().replace("/", "-").replace("\\", "-")


def replace_tokens(path: Path, replacements: dict[str, str]) -> None:
    if path.is_dir():
        return
    if path.suffix.lower() not in {".json", ".py", ".md", ".txt"}:
        return
    text = path.read_text(encoding="utf-8")
    for key, value in replacements.items():
        text = text.replace(key, value)
    path.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Scaffold a UGCAudit module from a template.")
    parser.add_argument("--kind", choices=KINDS, required=True)
    parser.add_argument("--module-id", required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--folder-name")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    template = skill_root() / "assets" / "templates" / args.kind
    if not template.is_dir():
        raise SystemExit(f"Template not found: {template}")

    output_dir = Path(args.output_dir).resolve()
    folder_name = args.folder_name or default_folder_name(args.module_id)
    target = output_dir / folder_name
    if target.exists():
        if not args.force:
            raise SystemExit(f"Target already exists: {target}")
        shutil.rmtree(target)

    output_dir.mkdir(parents=True, exist_ok=True)
    shutil.copytree(template, target)
    replacements = {
        "{{MODULE_ID}}": args.module_id,
        "{{MODULE_NAME}}": args.name,
        "{{MODULE_KIND}}": args.kind,
    }
    for item in target.rglob("*"):
        replace_tokens(item, replacements)

    print(f"Created module: {target}")
    print(f"Validate: python {skill_root() / 'scripts' / 'validate_module.py'} {target}")
    print(f"Smoke test: python {skill_root() / 'scripts' / 'run_module_smoke.py'} {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

