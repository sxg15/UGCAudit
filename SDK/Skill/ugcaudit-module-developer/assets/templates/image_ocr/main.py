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

    results = []
    for image in images:
        name = image.get("name", "")
        results.append({
            "path": image.get("path", ""),
            "name": name,
            "relativePath": image.get("relativePath", ""),
            "fullText": f"这里是 {name} 的示例 OCR 文本",
        })

    output = {
        "status": "completed",
        "verdict": "pass",
        "message": f"识别 {len(results)} 张图片。",
        "processedFiles": len(images),
        "matchedFiles": 0,
        "artifactCount": 0,
        "results": results,
        "reportSection": f"### {{MODULE_NAME}}\n\n已识别 {len(results)} 张图片。",
    }
    write_json(args.output, output)


if __name__ == "__main__":
    main()

