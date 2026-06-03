import argparse
import json
from pathlib import Path


def load_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write_json(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    input_data = load_json(args.input)
    images = input_data.get("inputs", {}).get("images", {}).get("items", [])
    blocked_names = [
        str(item).strip().lower()
        for item in input_data.get("params", {}).get("blockedNames", [])
        if str(item).strip()
    ]

    matches = []
    for image in images:
        name = str(image.get("name", "")).lower()
        hit_rules = [word for word in blocked_names if word in name]
        if hit_rules:
            matches.append({
                "name": image.get("name", ""),
                "path": image.get("path", ""),
                "relativePath": image.get("relativePath", ""),
                "hitRules": hit_rules,
            })

    report_lines = [
        "### {{MODULE_NAME}}",
        "",
        f"- 处理图片：{len(images)}",
        f"- 命中图片：{len(matches)}",
        f"- 结论：{'需要人工复审' if matches else '通过'}",
    ]
    for match in matches[:20]:
        report_lines.append(f"- `{match['name']}` 命中：{', '.join(match['hitRules'])}")

    output = {
        "status": "completed",
        "verdict": "review" if matches else "pass",
        "message": f"命中 {len(matches)} 张图片。" if matches else "未发现风险图片。",
        "processedFiles": len(images),
        "matchedFiles": len(matches),
        "artifactCount": 0,
        "matches": matches,
        "reportSection": "\n".join(report_lines),
    }
    write_json(args.output, output)


if __name__ == "__main__":
    main()

