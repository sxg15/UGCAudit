# Module Configuration

Use this reference when writing or reviewing `module.json`.

## Required fields

- `id`: stable unique id, preferably `custom.company.module-name`.
- `name`: visible module name in the client.
- `kind`: one of `text_safety`, `image_safety`, `image_ocr`, `folder_processor`.
- `icon`: built-in icon name or image file path.
- `launch.command`: entry file or executable.

## Launch

Recommended Python launch:

```json
{
  "launch": {
    "launchType": "python",
    "command": "main.py",
    "args": ["--input", "{inputJson}", "--output", "{outputJson}"],
    "notes": "使用客户端 Python 运行。"
  }
}
```

Important details:

- `launchType` is primarily a UI label.
- A `.py` command is run through the client Python.
- Relative commands and icons resolve from the module folder.
- The module process current directory is the module folder.
- Non-Python commands must be directly executable.

## Parameters

Supported `parameterType` values:

- `string`
- `number`
- `boolean`
- `select`
- `multiSelect`
- `stringList`
- `policyList`
- `path`
- `textarea`

For `select` and `multiSelect`, provide `options` with `label` and `value`.

## Model path

If the module needs a model:

- Put portable default files under `Model/` when possible.
- Add a `modelPath` parameter.
- Read the final value from `input.json.modelPath`.
- Never hard-code a developer-machine model path.

## Data outputs

Declare extra downstream outputs with `dataOutputs`:

```json
{
  "dataOutputs": [
    {"handle": "images", "name": "图片集合", "dataType": "imageCollection"}
  ]
}
```

Supported data types:

- `imageCollection`
- `textCollection`
- `folder`

