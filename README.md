# UGCAudit

UGCAudit 是一个 Tauri 桌面端 UGC 审核链原型。

当前版本已经包含：

- 可视化审核流编辑
- 内置模块入口：PaddleOCR、ShieldGemma 2、Qwen3Guard
- 本地模型路径配置
- 流程保存和校验
- 一次审核运行记录
- Markdown 报告生成

当前版本不会自动下载任何模型。没有配置本地模型目录时，运行结果会显示“未配置模型”。

## 开发

```powershell
npm install
npm run dev
```

## 构建

```powershell
npm run build
```

生成结果：

```text
src-tauri\target\release\ugc-audit.exe
```

## 免安装包

```powershell
npm run portable
```

生成结果：

```text
dist-portable\UGCAudit
dist-portable\UGCAudit-portable.zip
```
