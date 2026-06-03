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
    texts = input_data.get("inputs", {}).get("texts", {}).get("items", [])
    risk_words = [
        str(item).strip()
        for item in input_data.get("params", {}).get("riskWords", [])
        if str(item).strip()
    ]

    matches = []
    for item in texts:
        text = str(item.get("text", ""))
        hit_words = [word for word in risk_words if word in text]
        if hit_words:
            matches.append({
                "name": item.get("name", ""),
                "path": item.get("path", ""),
                "relativePath": item.get("relativePath", ""),
                "hitWords": hit_words,
            })

    report_lines = [
        "### {{MODULE_NAME}}",
        "",
        f"- 处理文本：{len(texts)}",
        f"- 命中文本：{len(matches)}",
        f"- 结论：{'需要人工复审' if matches else '通过'}",
    ]
    for match in matches[:20]:
        report_lines.append(f"- `{match['name']}` 命中：{', '.join(match['hitWords'])}")

    output = {
        "status": "completed",
        "verdict": "review" if matches else "pass",
        "message": f"命中 {len(matches)} 条文本。" if matches else "未发现风险文本。",
        "processedFiles": len(texts),
        "matchedFiles": len(matches),
        "artifactCount": 0,
        "matches": matches,
        "reportSection": "\n".join(report_lines),
    }
    write_json(args.output, output)


if __name__ == "__main__":
    main()

