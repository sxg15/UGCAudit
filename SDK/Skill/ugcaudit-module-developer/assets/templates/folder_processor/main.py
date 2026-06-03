import argparse
import json
from pathlib import Path


def load_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write_json(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


def write_progress(input_data, progress, message, processed=None, total=None):
    path = str(input_data.get("progressPath", "")).strip()
    if not path:
        return
    item = {"progress": progress, "message": message}
    if processed is not None:
        item["processed"] = processed
    if total is not None:
        item["total"] = total
    progress_path = Path(path)
    progress_path.parent.mkdir(parents=True, exist_ok=True)
    with progress_path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(item, ensure_ascii=False) + "\n")


def is_cancelled(input_data):
    path = str(input_data.get("cancelPath", "")).strip()
    return bool(path) and Path(path).exists()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    input_data = load_json(args.input)
    folder_info = input_data.get("inputs", {}).get("folder", {})
    folder_path = Path(str(folder_info.get("path", "")).strip() or ".").resolve()
    max_files = int(input_data.get("params", {}).get("maxFiles", 100))

    if not folder_path.is_dir():
        output = {
            "status": "error",
            "verdict": "error",
            "message": f"文件夹不存在：{folder_path}",
            "processedFiles": 0,
            "matchedFiles": 0,
            "artifactCount": 0,
            "reportSection": f"### {{MODULE_NAME}}\n\n文件夹不存在：{folder_path}",
        }
        write_json(args.output, output)
        return

    files = [path for path in folder_path.rglob("*") if path.is_file()]
    files = files[:max_files]
    write_progress(input_data, 0, "开始扫描文件夹", 0, len(files))

    lines = []
    for index, path in enumerate(files, start=1):
        if is_cancelled(input_data):
            output = {
                "status": "cancelled",
                "verdict": "review",
                "message": "任务已中断。",
                "processedFiles": index - 1,
                "matchedFiles": 0,
                "artifactCount": 0,
                "reportSection": "### {{MODULE_NAME}}\n\n用户中断了任务。",
            }
            write_json(args.output, output)
            return
        try:
            relative = path.relative_to(folder_path)
        except ValueError:
            relative = path
        lines.append(str(relative))
        write_progress(input_data, index / max(len(files), 1), f"已扫描 {index} 个文件", index, len(files))

    artifact_dir = Path(input_data.get("stepArtifactDir", "./debug/artifacts"))
    artifact_dir.mkdir(parents=True, exist_ok=True)
    list_path = artifact_dir / "file_list.txt"
    list_path.write_text("\n".join(lines), encoding="utf-8")

    text_item = {
        "sourceType": "file",
        "path": str(list_path),
        "name": list_path.name,
        "relativePath": list_path.name,
        "text": "\n".join(lines),
    }
    output = {
        "status": "completed",
        "verdict": "pass",
        "message": f"扫描 {len(files)} 个文件。",
        "processedFiles": len(files),
        "matchedFiles": 0,
        "artifactCount": 1,
        "outputs": {
            "texts": {
                "dataType": "textCollection",
                "items": [text_item],
            }
        },
        "reportSection": f"### {{MODULE_NAME}}\n\n已扫描 {len(files)} 个文件。\n\n清单文件：`{list_path}`",
    }
    write_progress(input_data, 1, "扫描完成", len(files), len(files))
    write_json(args.output, output)


if __name__ == "__main__":
    main()

