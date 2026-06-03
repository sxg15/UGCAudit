# Acceptance Checklist

Use this before declaring a module complete.

## Configuration

- `module.json` is valid JSON.
- `id`, `name`, `kind`, `icon`, and `launch.command` are present.
- Entry file exists.
- Icon file exists when `icon` is an image path.
- Parameters have supported types.
- Select parameters have options.
- `dataOutputs` use supported data types.

## Runtime

- `examples/input.demo.json` exists.
- Local smoke test writes `output.json`.
- `output.json` has `status`, `verdict`, `message`, and `reportSection`.
- `processedFiles` and `matchedFiles` are plausible.
- Declared outputs are present under `outputs`.
- Generated artifacts are written under `stepArtifactDir` or `artifactDir`.

## Report

- `reportSection` starts at `###`.
- The report explains what was processed and what needs review.
- Local image paths are absolute.
- Long raw output is folded or summarized.

## Long tasks

- Progress is written during long loops.
- Cancellation is checked during long loops.
- Cancellation still writes legal `output.json`.

## Delivery

- No `.venv/`, `__pycache__/`, `.git/`, logs, run history, caches, or private user data.
- Dependencies and model requirements are documented.
- The module was validated with `validate_module.py`.
- The module was smoke-tested with `run_module_smoke.py`.

