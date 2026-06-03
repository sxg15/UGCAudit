# Flow Connections

Use this reference when choosing module kind or wiring a module into a scheme.

## Connection types

- Sequence lines decide when steps run.
- Data lines decide which image, text, or folder values a module receives.

A runnable module usually needs both sequence and data lines.

## Module kinds

| kind | Input | Default output |
| --- | --- | --- |
| `text_safety` | Text collection | None |
| `image_safety` | Image collection | None |
| `image_ocr` | Image collection | Text collection |
| `folder_processor` | Folder | None unless `dataOutputs` is declared |

## Typical flows

Text audit:

```text
开始 -> 文本审核 -> 输出结果
待测项目中所有文本 --文本数据--> 文本审核
```

Image audit:

```text
开始 -> 图片审核 -> 输出结果
待测项目中所有图片 --图片数据--> 图片审核
```

OCR then text audit:

```text
开始 -> OCR -> 文本审核 -> 输出结果
待测项目中所有图片 --图片数据--> OCR
OCR --文本数据--> 文本审核
```

Folder processor:

```text
开始 -> 文件夹处理 -> 输出结果
待审核文件夹 --文件夹数据--> 文件夹处理
```

## Common failures

- Processing count is 0: the data line is missing or the module reads the wrong input handle.
- Text module receives no OCR text: OCR output lacks `fullText`.
- Output result runs too early: sequence dependencies are incomplete.
- Custom output port is invisible: `dataOutputs` is missing or has an unknown `dataType`.

