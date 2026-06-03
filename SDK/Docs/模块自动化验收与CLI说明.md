# 模块自动化验收与 CLI 说明

本文档说明模块开发完成后，怎样用本地试跑、客户端运行和 `ugc-audit.exe run` 做交付前验收。

## 验收目标

模块交付前至少确认四件事：

1. `module.json` 能被解析，入口文件存在。
2. 用示例 `input.json` 可以跑出合法 `output.json`。
3. 放进客户端流程后，步骤目录里有 `input.json`、`output.json`、`result.md`。
4. 报告页能看到模块写出的 `reportSection`。

如果模块会生成图片、表格或其他产物，还要确认这些文件写在 `stepArtifactDir` 或 `artifactDir` 下，并能在报告中打开。

## 本地试跑模块

推荐每个模块都提供：

```text
examples/
  input.demo.json
  output.demo.json
```

本地试跑命令：

```powershell
python main.py --input examples/input.demo.json --output debug/output.smoke.json
```

检查 `debug/output.smoke.json`：

- 是合法 JSON。
- 包含 `status`、`verdict`、`message`、`reportSection`。
- `processedFiles` 和 `matchedFiles` 与示例输入一致。
- 如果声明了 `dataOutputs`，对应的 `outputs.<handle>` 已写出。

## 使用 Skill 工具验收

项目内提供模块开发 Skill：

```text
SDK/Skill/ugcaudit-module-developer/
```

常用脚本：

```powershell
python SDK/Skill/ugcaudit-module-developer/scripts/validate_module.py D:/UGCAuditModels/my-module
python SDK/Skill/ugcaudit-module-developer/scripts/run_module_smoke.py D:/UGCAuditModels/my-module
```

`validate_module.py` 检查模块配置、入口文件、图标、参数和输出口。

`run_module_smoke.py` 使用模块目录里的 `examples/input.demo.json` 试跑入口脚本，并检查生成的 `output.json`。

## 客户端运行验收

在客户端里验收时，按这个顺序检查：

1. 在“模块管理”中导入模块文件夹。
2. 把模块拖到流程画布。
3. 连接顺序线和需要的数据线。
4. 运行一次代表性任务。
5. 打开本次运行的步骤目录。

步骤目录中重点看：

| 文件或目录 | 检查内容 |
| --- | --- |
| `input.json` | 是否收到了正确的文件、参数、数据输入和模型路径。 |
| `output.json` | 模块是否写出了合法结果。 |
| `stdout.log` | 是否有普通运行日志。 |
| `stderr.log` | 是否有异常、依赖缺失或路径错误。 |
| `result.md` | 报告片段是否与 `reportSection` 一致。 |
| `artifacts/` | 模块生成的产物是否在这里。 |

## CLI 验收

`ugc-audit.exe run` 用来验收完整审核方案，不是只验收单个模块。使用前要先在客户端保存好包含该模块的 `.ugcaudit` 方案，并确认模块已经导入。

命令格式：

```powershell
ugc-audit.exe run --scheme "D:\AuditSchemes\image.ugcaudit" --input "D:\UGCProject" --task-name "模块验收" --output "D:\AuditRuns\module-smoke"
```

参数说明：

| 参数 | 说明 |
| --- | --- |
| `--scheme` | 审核方案文件路径。 |
| `--input` | 待审核文件夹。CLI 当前只支持文件夹输入。 |
| `--task-name` | 本次任务名称，可选。 |
| `--output` | 输出目录，可选。建议验收时显式指定。 |
| `--note` | 本次任务说明，可选。 |

## CLI 输出检查

指定 `--output` 后，输出目录里应有：

| 文件 | 说明 |
| --- | --- |
| `run.json` | 完整运行记录。 |
| `report.md` | Markdown 报告。 |
| `cli-result.json` | 给自动化流水线读取的简要结果。 |

退出码含义：

| 退出码 | 含义 |
| --- | --- |
| `0` | 审核完成，结论通过。 |
| `2` | 审核完成，但需要复审或不通过。 |
| `1` | 运行失败。 |

## 验收通过标准

模块可以交付前，应满足：

- 本地试跑能生成合法 `output.json`。
- 客户端导入成功。
- 真实流程运行后，步骤目录里有 `input.json`、`output.json`、`result.md`。
- 报告页能显示模块报告。
- 长任务能持续显示进度，取消后能尽快停止。
- CLI 验收时能生成 `run.json`、`report.md`、`cli-result.json`。

