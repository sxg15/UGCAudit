---
name: ugcaudit-module-developer
description: Create, modify, validate, smoke-test, and package UGCAudit custom audit modules. Use when Codex needs to build or review modules with module.json, main.py, input.json/output.json, reportSection, dataOutputs, progressPath/cancelPath, or UGCAudit SDK Docs.
---

# UGCAudit Module Developer

## Workflow

1. Determine the module kind from the user's goal:
   - `text_safety`: consumes text collections and returns a risk verdict.
   - `image_safety`: consumes image collections and returns a risk verdict.
   - `image_ocr`: consumes image collections and emits OCR text for downstream text modules.
   - `folder_processor`: consumes a folder path and may emit image/text/folder collections.
2. Read only the references needed for the task:
   - `references/module-config.md` for `module.json`, launch, parameters, icons, model paths.
   - `references/input-output.md` for `input.json`, `output.json`, outputs, progress, cancellation.
   - `references/flow-connections.md` for sequence/data edges and system data nodes.
   - `references/report-section.md` for Markdown, local images, links, and Mermaid.
   - `references/acceptance-checklist.md` before handing off a module.
3. For a new module, scaffold from `assets/templates/<kind>` with `scripts/scaffold_module.py`.
4. Keep the module contract stable: one module folder, root `module.json`, explicit entry command, explicit `--input` and `--output`, output a legal `output.json`.
5. Validate with `scripts/validate_module.py`.
6. Smoke-test with `scripts/run_module_smoke.py` and inspect `output.json` before reporting completion.

## Commands

Create a module:

```powershell
python SDK/Skill/ugcaudit-module-developer/scripts/scaffold_module.py --kind text_safety --module-id custom.demo.text-risk --name "示例文本审核" --output-dir D:/UGCAuditModels
```

Validate a module:

```powershell
python SDK/Skill/ugcaudit-module-developer/scripts/validate_module.py D:/UGCAuditModels/custom.demo.text-risk
```

Smoke-test a module:

```powershell
python SDK/Skill/ugcaudit-module-developer/scripts/run_module_smoke.py D:/UGCAuditModels/custom.demo.text-risk
```

## Rules

- Do not invent new module kinds unless the UGCAudit client code supports their ports.
- Do not write developer-machine absolute paths into module code or `module.json`.
- Read runtime paths from `input.json`: `inputs`, `params`, `modelPath`, `artifactDir`, `stepArtifactDir`.
- Prefer writing generated artifacts into `stepArtifactDir`.
- Write `reportSection` as a module-local report fragment starting at `###`.
- For long tasks, append JSON lines to `progressPath` and check `cancelPath`.
- If expected failure occurs, still write `output.json` with `status: "error"` and a clear `message`.

## Completion Standard

A module is not complete until:

- `module.json` validates.
- Local smoke test produces legal `output.json`.
- Declared `dataOutputs` have matching `outputs.<handle>` in the result.
- `reportSection` is visible and useful.
- Long-running work shows progress and handles cancellation.
- Delivery excludes caches, virtual environments, logs, run history, and private data.

