# Report Section

Use this reference when writing `reportSection`.

## Basic rules

- Write only the current module's report fragment.
- Start with a third-level heading such as `### 图片审核`.
- Put the verdict and counts near the top.
- Use Markdown tables for repeated file results.
- Use absolute local paths for generated images and local file links.

## Good shape

```markdown
### 图片审核

**结论：需要人工复审**

本轮处理 12 张图片，命中 2 张。

| 文件 | 原因 | 结论 |
| --- | --- | --- |
| image_001.png | 疑似联系方式 | 复审 |
```

## Local images

If the module generates an image, write it into `stepArtifactDir` and reference the absolute path:

```markdown
![标注图](D:/AuditRuns/run_001/steps/image_safety/artifacts/marked.png)
```

## Local file reveal links

Use the client reveal scheme when the report should locate a file in the system file explorer:

```markdown
[查看文件](ugcaudit://reveal?path=D%3A%5CUGC%5Cimages%5Cimage_001.png)
```

## Mermaid

Mermaid diagrams are allowed in `mermaid` code blocks. Keep them small and use them for counts, distribution, or process summaries.

Do not write scripts, buttons, forms, unsafe links, or page navigation logic in reports.

