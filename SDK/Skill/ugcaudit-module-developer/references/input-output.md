# Input and Output Contract

Use this reference when implementing `main.py` or reviewing module results.

## Input

The client writes `input.json` before launching the module. Important fields:

- `inputs`: data-line values such as `images`, `texts`, or `folder`.
- `files`: narrowed file list for file-based modules.
- `params`: node parameter values.
- `modelPath`: final model directory.
- `previous`: raw upstream module outputs.
- `artifactDir`: run-level artifact directory.
- `stepArtifactDir`: current-step artifact directory.
- `progressPath`: optional JSONL progress path for live runs.
- `cancelPath`: optional cancellation flag path for live runs.

Prefer `inputs` for normal data consumption. Use `previous` only when raw upstream output is needed.

## Output

Always write a legal `output.json`:

```json
{
  "status": "completed",
  "verdict": "pass",
  "message": "模块运行完成。",
  "processedFiles": 0,
  "matchedFiles": 0,
  "artifactCount": 0,
  "reportSection": "### 模块结果\n\n模块运行完成。"
}
```

Recommended `status` values:

- `completed`
- `skipped`
- `cancelled`
- `error`

Recommended `verdict` values:

- `pass`
- `review`
- `reject`
- `error`

## Downstream outputs

If `module.json` declares `dataOutputs`, write matching `outputs.<handle>`.

Example:

```json
{
  "outputs": {
    "images": {
      "dataType": "imageCollection",
      "items": []
    }
  }
}
```

OCR modules can emit text through either `results[].fullText` or `outputs.fullText`.

## Progress and cancellation

For long tasks:

- Append JSON lines to `progressPath`.
- Check `cancelPath` in loops.
- On cancellation, write `status: "cancelled"` and a clear `reportSection`.
- Do not delete already useful artifacts.

