# UGCAudit CLI 使用说明

UGCAudit 可以用同一个 `ugc-audit.exe` 以无窗口方式运行，适合接入自动审核流水线。

## 命令格式

```powershell
ugc-audit.exe run --scheme "D:\AuditSchemes\image.ugcaudit" --input "D:\UGCProject" --task-name "每日图片审核" --output "D:\AuditRuns\run-001"
```

## 参数

- `run`：进入无窗口审核模式。
- `--scheme`：审核方案文件路径，默认扩展名为 `.ugcaudit`。
- `--input`：要审核的文件夹路径。第一版 CLI 只支持文件夹，不支持单文件。
- `--task-name`：可选，本次任务名称。不传时使用方案名称。
- `--output`：可选，本次产物目录。传入后直接把 `run.json`、`report.md` 和 `cli-result.json` 写入该目录；不传时使用客户端设置里的默认产物路径，并自动创建 `任务名称-任务ID` 文件夹。
- `--note`：可选，本次任务说明。

## 方案文件位置

客户端里新建的方案默认保存在程序根目录下的 `Schemes` 文件夹，例如：

```text
UGCAudit\Schemes\图片默认审核.ugcaudit
```

CLI 可以直接使用这个目录里的方案文件，也可以使用手动另存到其他位置的 `.ugcaudit` 文件。

## 退出码

- `0`：审核完成，结论通过。
- `2`：审核完成，但结论需要复审或不通过。
- `1`：运行失败，例如参数错误、方案错误、文件夹不存在或模块执行失败。

## 输出文件

本次产物目录内会生成：

- `run.json`：完整运行结果。
- `report.md`：Markdown 审核报告。
- `cli-result.json`：给流水线读取的简要结果，包含退出码、结论、运行编号、产物目录和报告路径。

## 流水线示例

```powershell
.\ugc-audit.exe run --scheme "D:\AuditSchemes\image.ugcaudit" --input "D:\UGCProject" --task-name "每日图片审核" --output "D:\AuditRuns\latest"
if ($LASTEXITCODE -eq 0) {
  Write-Host "审核通过"
} elseif ($LASTEXITCODE -eq 2) {
  Write-Host "需要人工复审"
  exit 2
} else {
  Write-Host "审核运行失败"
  exit 1
}
```

## 常见错误

- `缺少参数 --scheme`：没有传入审核方案。
- `缺少参数 --input`：没有传入待审文件夹。
- `CLI 只支持审核文件夹`：`--input` 不是文件夹或路径不存在。
- `不是 UGCAudit 审核方案文件`：方案文件格式不正确。
- `步骤 ... 缺少必需的数据输入`：方案中的流程连线不完整。
