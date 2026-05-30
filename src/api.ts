import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AuditAsset,
  FlowDefinition,
  JsonValue,
  ModuleInfo,
  ModuleParameter,
  RunRecord,
  RunSummary,
  ValidationResult,
} from "./types";

const FLOW_KEY = "ugc-audit.flow";
const RUNS_KEY = "ugc-audit.runs";

const hasTauri = () => Boolean(window.__TAURI_INTERNALS__);
const START_NODE_ID = "flow_start";
const OUTPUT_NODE_ID = "flow_output";
const START_MODULE_ID = "system.start";
const OUTPUT_MODULE_ID = "system.output";

const legacyModuleIds: Record<string, string> = {
  "builtin.paddleocr": "preset.custom.paddleocr",
  "builtin.shieldgemma2": "preset.custom.shieldgemma2",
  "builtin.qwen3guard": "preset.custom.qwen3guard",
};

function option(label: string, value: string) {
  return { label, value };
}

function param(
  key: string,
  name: string,
  description: string,
  parameterType: ModuleParameter["parameterType"],
  defaultValue: JsonValue,
  required = false,
  options: ReturnType<typeof option>[] = [],
): ModuleParameter {
  return { key, name, description, parameterType, defaultValue, required, options };
}

function systemLaunch(notes: string): ModuleInfo["launch"] {
  return { launchType: "system", command: null, url: null, method: null, args: [], notes };
}

function exeLaunch(command: string, notes: string): ModuleInfo["launch"] {
  return {
    launchType: "exe",
    command,
    url: null,
    method: null,
    args: ["--resource-root", "{resourceRoot}", "--params", "{paramsJson}"],
    notes,
  };
}

function httpLaunch(url: string, notes: string): ModuleInfo["launch"] {
  return { launchType: "http", command: null, url, method: "POST", args: [], notes };
}

const moduleSpecs: ModuleInfo[] = [
  {
    id: START_MODULE_ID,
    name: "开始",
    kind: "flow_start",
    summary: "流程入口，接收本次审核素材",
    modelLabel: "无需模型",
    icon: "play-circle",
    builtIn: true,
    source: "system",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("流程入口，不启动外部模块。"),
    parameters: [],
  },
  {
    id: OUTPUT_MODULE_ID,
    name: "输出结果",
    kind: "flow_output",
    summary: "汇总所有步骤并生成 Markdown 报告",
    modelLabel: "无需模型",
    icon: "file-output",
    builtIn: true,
    source: "system",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("报告汇总节点，不启动外部模块。"),
    parameters: [],
  },
  {
    id: "preset.custom.paddleocr",
    name: "图片文字识别",
    kind: "image_ocr",
    summary: "预置自定义模块，面向 PaddleOCR 本地运行入口",
    modelLabel: "PaddleOCR 本地目录",
    icon: "scan-text",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: false,
    launch: exeLaunch(
      "paddleocr-module.exe",
      "客户端启动本地可执行文件，只传入待审核资源根目录和用户参数 JSON。",
    ),
    parameters: [
      param("modelPath", "PaddleOCR 本地目录", "PaddleOCR 模型或运行环境所在目录。", "path", "", true),
      param("profile", "识别模式", "mobile 速度更快，server 更适合高精度。", "select", "mobile", true, [
        option("mobile", "mobile"),
        option("server", "server"),
      ]),
      param("language", "识别语言", "传给模块的语言代码。", "select", "ch", true, [
        option("中文", "ch"),
        option("英文", "en"),
        option("多语言", "multi"),
      ]),
      param("minConfidence", "最低置信度", "低于该值的文字会被标记为低可信。", "number", 0.5),
      param("drawBoxes", "输出标注图", "是否要求模块输出 OCR 标注图片。", "boolean", true),
    ],
  },
  {
    id: "preset.custom.shieldgemma2",
    name: "图片合规检测",
    kind: "image_safety",
    summary: "预置自定义模块，面向 ShieldGemma 2 本地入口",
    modelLabel: "ShieldGemma 2 模型目录",
    icon: "shield-alert",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: false,
    launch: exeLaunch(
      "shieldgemma2-module.exe",
      "客户端启动本地可执行文件，只传入待审核资源根目录和用户参数 JSON。",
    ),
    parameters: [
      param("modelPath", "ShieldGemma 2 模型目录", "ShieldGemma 2 本地模型目录。", "path", "", true),
      param("policies", "检测策略", "模块需要检测的图片风险类别。", "multiSelect", [
        "sexual",
        "violence_gore",
        "dangerous",
      ], true, [
        option("色情", "sexual"),
        option("暴力/血腥", "violence_gore"),
        option("危险内容", "dangerous"),
      ]),
      param("threshold", "风险阈值", "高于该分值时进入人工复审。", "number", 0.7),
      param("policyPrompt", "策略说明", "传给模型的策略文本，可按业务调整。", "textarea", "检查图片是否包含色情、暴力血腥或危险内容。"),
    ],
  },
  {
    id: "preset.custom.qwen3guard",
    name: "文本合规检测",
    kind: "text_safety",
    summary: "预置自定义模块，面向 Qwen3Guard 本地入口",
    modelLabel: "Qwen3Guard 模型目录",
    icon: "file-check",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: false,
    launch: httpLaunch(
      "http://127.0.0.1:8787/audit/text",
      "客户端以 HTTP POST 调用本地服务，请求体只包含 resourceRoot 和 params。",
    ),
    parameters: [
      param("modelPath", "Qwen3Guard 模型目录", "Qwen3Guard 本地模型目录。", "path", "", true),
      param("textPattern", "文本文件匹配", "模块在资源根目录内读取的文本文件匹配规则。", "string", "**/*.{txt,md,json}", true),
      param("modelSize", "模型尺寸", "传给模块的模型尺寸标记。", "select", "0.6b", true, [
        option("0.6B", "0.6b"),
        option("4B", "4b"),
        option("8B", "8b"),
      ]),
      param("categories", "风险类别", "文本模块要关注的风险类别。", "multiSelect", [
        "sexual",
        "violence",
        "illegal",
        "privacy",
      ], false, [
        option("色情", "sexual"),
        option("暴力", "violence"),
        option("违法", "illegal"),
        option("隐私", "privacy"),
        option("自伤", "self_harm"),
      ]),
      param("rejectUnsafe", "Unsafe 直接拒绝", "模型返回 Unsafe 时是否直接判为不通过。", "boolean", true),
    ],
  },
];

export function defaultConfigForModule(module: ModuleInfo): Record<string, JsonValue> {
  return Object.fromEntries(
    module.parameters.map((parameter) => [parameter.key, parameter.defaultValue]),
  ) as Record<string, JsonValue>;
}

function normalizeFlow(flow: FlowDefinition): FlowDefinition {
  const normalized = {
    ...flow,
    nodes: flow.nodes.map((node) => {
      const moduleId = legacyModuleIds[node.moduleId] ?? node.moduleId;
      const module = moduleSpecs.find((item) => item.id === moduleId);
      const defaults = module ? defaultConfigForModule(module) : {};
      const config = {
        ...defaults,
        ...(typeof node.config === "object" && node.config && !Array.isArray(node.config)
          ? node.config
          : {}),
      };
      if (moduleId === "preset.custom.qwen3guard") {
        delete config.input;
      }
      return {
        ...node,
        moduleId,
        config,
      };
    }),
  };
  return ensureSystemNodes(normalized);
}

function defaultFlowDefinition(): FlowDefinition {
  return {
    id: "flow.default.image-audit",
    name: "图片 UGC 默认审核",
    version: 1,
    nodes: [
      {
        id: START_NODE_ID,
        moduleId: START_MODULE_ID,
        label: "开始",
        position: { x: 120, y: 220 },
        config: {},
      },
      {
        id: OUTPUT_NODE_ID,
        moduleId: OUTPUT_MODULE_ID,
        label: "输出结果",
        position: { x: 520, y: 220 },
        config: {},
      },
    ],
    edges: [
      {
        id: "edge_flow_start_output",
        from: START_NODE_ID,
        to: OUTPUT_NODE_ID,
      },
    ],
  };
}

function hasSystemNode(flow: FlowDefinition, moduleId: string) {
  return flow.nodes.some((node) => node.moduleId === moduleId);
}

function ensureSystemNodes(flow: FlowDefinition): FlowDefinition {
  const hasStart = hasSystemNode(flow, START_MODULE_ID);
  const hasOutput = hasSystemNode(flow, OUTPUT_MODULE_ID);

  if (!hasStart && !hasOutput) {
    return defaultFlowDefinition();
  }

  const nodes = [...flow.nodes];
  const edges = [...flow.edges];

  if (!hasStart) {
    nodes.unshift({
      id: START_NODE_ID,
      moduleId: START_MODULE_ID,
      label: "开始",
      position: { x: 120, y: 220 },
      config: {},
    });
  }

  if (!hasOutput) {
    nodes.push({
      id: OUTPUT_NODE_ID,
      moduleId: OUTPUT_MODULE_ID,
      label: "输出结果",
      position: { x: 520, y: 220 },
      config: {},
    });
  }

  const startId = nodes.find((node) => node.moduleId === START_MODULE_ID)?.id ?? START_NODE_ID;
  const outputId = nodes.find((node) => node.moduleId === OUTPUT_MODULE_ID)?.id ?? OUTPUT_NODE_ID;
  if (nodes.length === 2 && !edges.some((edge) => edge.from === startId && edge.to === outputId)) {
    edges.push({ id: "edge_flow_start_output", from: startId, to: outputId });
  }

  return { ...flow, nodes, edges };
}

const defaultFlow: FlowDefinition = defaultFlowDefinition();

function readJson<T>(key: string, fallback: T): T {
  const raw = localStorage.getItem(key);
  if (!raw) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeJson<T>(key: string, value: T) {
  localStorage.setItem(key, JSON.stringify(value));
}

function assetName(path: string) {
  const normalized = path.replace(/\\/g, "/");
  return normalized.split("/").filter(Boolean).pop() ?? path;
}

function assetExtension(path: string) {
  const name = assetName(path);
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : "";
}

function toAsset(path: string, kind: AuditAsset["kind"]): AuditAsset {
  return {
    id: `${kind}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    kind,
    path,
    name: assetName(path),
    extension: kind === "file" ? assetExtension(path) : "",
  };
}

function normalizeDialogSelection(selection: string | string[] | null, kind: AuditAsset["kind"]) {
  if (!selection) return [];
  return (Array.isArray(selection) ? selection : [selection]).map((path) => toAsset(path, kind));
}

function validateFlowLocal(flow: FlowDefinition): ValidationResult {
  const messages: string[] = [];
  const nodeIds = new Set(flow.nodes.map((node) => node.id));

  if (flow.nodes.length === 0) {
    messages.push("流程至少需要一个步骤。");
  }

  const startNodes = flow.nodes.filter((node) => node.moduleId === START_MODULE_ID);
  const outputNodes = flow.nodes.filter((node) => node.moduleId === OUTPUT_MODULE_ID);
  if (startNodes.length !== 1) {
    messages.push("流程必须有且只能有一个开始节点。");
  }
  if (outputNodes.length !== 1) {
    messages.push("流程必须有且只能有一个输出结果节点。");
  }

  for (const node of flow.nodes) {
    if (!node.id.trim()) messages.push("存在没有 ID 的步骤。");
    if (!moduleSpecs.some((module) => module.id === node.moduleId)) {
      messages.push(`步骤 ${node.label} 使用了未知模块。`);
    }
  }

  for (const edge of flow.edges) {
    if (!nodeIds.has(edge.from)) messages.push(`连线 ${edge.id} 的起点不存在。`);
    if (!nodeIds.has(edge.to)) messages.push(`连线 ${edge.id} 的终点不存在。`);
    if (edge.from === edge.to) messages.push(`连线 ${edge.id} 指向了同一个步骤。`);
  }

  const startId = startNodes[0]?.id;
  const outputId = outputNodes[0]?.id;
  if (startId && flow.edges.some((edge) => edge.to === startId)) {
    messages.push("开始节点不能有输入连线。");
  }
  if (outputId && flow.edges.some((edge) => edge.from === outputId)) {
    messages.push("输出结果节点不能有输出连线。");
  }
  if (startId && outputId && !hasPath(flow, startId, outputId)) {
    messages.push("开始节点必须能连到输出结果节点。");
  }

  return { valid: messages.length === 0, messages };
}

function hasPath(flow: FlowDefinition, from: string, to: string) {
  const outgoing = new Map<string, string[]>();
  for (const edge of flow.edges) {
    outgoing.set(edge.from, [...(outgoing.get(edge.from) ?? []), edge.to]);
  }
  const seen = new Set<string>();
  const queue = [from];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current || seen.has(current)) continue;
    if (current === to) return true;
    seen.add(current);
    queue.push(...(outgoing.get(current) ?? []));
  }
  return false;
}

function moduleConfigured(config: JsonValue) {
  if (!config || typeof config !== "object" || Array.isArray(config)) return false;
  return typeof config.modelPath === "string" && config.modelPath.trim().length > 0;
}

function mockStatusLabel(status: string) {
  if (status === "ready") return "本地入口已配置";
  if (status === "system") return "系统节点";
  return "未配置模型";
}

function mockReport(run: RunRecord) {
  const rows = run.steps
    .map(
      (step) =>
        `| ${step.label} | ${step.moduleName} | ${mockStatusLabel(step.status)} | ${step.verdict === "pass" ? "通过" : "需要人工复审"} | ${step.message.replace(/\n/g, " ")} |`,
    )
    .join("\n");

  return `# UGC 审核报告

## 总结

- 最终结论：需要人工复审
- 运行编号：${run.id}
- 流程：${run.flowName}
- 输入：${run.inputNote || "未填写输入说明"}
- 素材数量：${run.assets.length}
- 资源根目录：${run.resourceRoot}
- 模型下载：本次运行未触发任何模型下载。

## 输入素材

${
  run.assets.length
    ? `| 类型 | 名称 | 路径 |
| --- | --- | --- |
${run.assets
  .map((asset) => `| ${asset.kind === "directory" ? "文件夹" : "文件"} | ${asset.name} | ${asset.path} |`)
  .join("\n")}`
    : `- 未选择本地素材。
- 说明：${run.inputNote || "未填写输入说明"}`
}

## 流程结果

| 步骤 | 模块 | 状态 | 结论 | 说明 |
| --- | --- | --- | --- | --- |
${rows}

## 模块结论

${run.steps
  .map((step) => step.reportSection)
  .join("\n")}

## 本地文件

- 运行目录：${run.runDir}
- 资源根目录：${run.resourceRoot}
- 报告文件：${run.reportPath}
`;
}

export async function listModules(): Promise<ModuleInfo[]> {
  if (hasTauri()) return invoke<ModuleInfo[]>("list_modules");
  return moduleSpecs;
}

export async function getDataRoot(): Promise<string> {
  if (hasTauri()) return invoke<string>("get_data_root");
  return "浏览器预览模式";
}

export async function loadFlow(): Promise<FlowDefinition> {
  if (hasTauri()) return invoke<FlowDefinition>("load_flow");
  return normalizeFlow(readJson(FLOW_KEY, defaultFlow));
}

export async function saveFlow(flow: FlowDefinition): Promise<FlowDefinition> {
  if (hasTauri()) return invoke<FlowDefinition>("save_flow", { flow: normalizeFlow(flow) });
  const normalized = normalizeFlow(flow);
  const validation = validateFlowLocal(normalized);
  if (!validation.valid) throw new Error(validation.messages.join(" "));
  writeJson(FLOW_KEY, normalized);
  return normalized;
}

export async function validateFlow(flow: FlowDefinition): Promise<ValidationResult> {
  if (hasTauri()) return invoke<ValidationResult>("validate_flow", { flow: normalizeFlow(flow) });
  return validateFlowLocal(normalizeFlow(flow));
}

export async function saveModelPath(moduleId: string, path: string): Promise<ModuleInfo[]> {
  if (hasTauri()) return invoke<ModuleInfo[]>("save_model_path", { moduleId, path });
  return moduleSpecs;
}

export async function openModuleDefinitionFolder(moduleId: string): Promise<void> {
  if (hasTauri()) {
    await invoke("open_module_definition_folder", { moduleId });
    return;
  }
  throw new Error("浏览器预览模式不能打开本地文件夹");
}

export async function selectAssetFiles(): Promise<AuditAsset[]> {
  if (hasTauri()) {
    const selected = await open({
      multiple: true,
      title: "选择 UGC 素材文件",
      filters: [
        {
          name: "UGC 素材",
          extensions: [
            "png",
            "jpg",
            "jpeg",
            "webp",
            "bmp",
            "gif",
            "txt",
            "md",
            "json",
          ],
        },
      ],
    });
    return normalizeDialogSelection(selected, "file");
  }

  return [toAsset("preview/sample-image.png", "file")];
}

export async function selectAssetDirectory(): Promise<AuditAsset[]> {
  if (hasTauri()) {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择 UGC 素材文件夹",
    });
    return normalizeDialogSelection(selected, "directory");
  }

  return [toAsset("preview/sample-folder", "directory")];
}

export async function startRun(
  flow: FlowDefinition,
  inputNote: string,
  assets: AuditAsset[],
): Promise<RunRecord> {
  const normalized = normalizeFlow(flow);
  if (hasTauri()) return invoke<RunRecord>("start_run", { flow: normalized, inputNote, assets });

  const modules = moduleSpecs;
  const runId = `run_${Date.now()}`;
  const resourceRoot = `local-preview/runs/${runId}/resources`;
  const steps = normalized.nodes.map((node, index) => {
    const module = modules.find((item) => item.id === node.moduleId) ?? modules[0];
    if (module.source === "system") {
      const message =
        module.id === START_MODULE_ID
          ? "流程开始，已接收本次审核素材。"
          : "流程结束，审核结果将汇总到 Markdown 报告。";
      return {
        stepId: node.id,
        moduleId: node.moduleId,
        moduleName: module.name,
        label: node.label,
        status: "system",
        verdict: "pass",
        message,
        executionGroup: index,
        reportSection: `### ${node.label}

- 模块：${module.name}
- 模块来源：流程系统节点
- 结论：通过
- 状态：系统节点
- 说明：${message}
`,
      };
    }
    const status = moduleConfigured(node.config) ? "ready" : "needs_model";
    const modelPath =
      node.config && typeof node.config === "object" && !Array.isArray(node.config)
        ? String(node.config.modelPath ?? "")
        : "";
    const message =
      status === "ready"
        ? `已收到模块参数，本地入口：${modelPath}。首版只完成入口检查，尚未执行真实识别。`
        : `${module.modelLabel} 未配置，本轮没有执行真实识别，也没有下载模型。`;

    return {
      stepId: node.id,
      moduleId: node.moduleId,
      moduleName: module.name,
      label: node.label,
      status,
      verdict: "review",
      message,
      executionGroup: index,
      reportSection: `### ${node.label}

- 模块：${module.name}
- 模块来源：预置自定义模块
- 结论：需要人工复审
- 状态：${mockStatusLabel(status)}
- 说明：${message}
`,
    };
  });
  const run: RunRecord = {
    id: runId,
    flowId: normalized.id,
    flowName: normalized.name,
    createdAt: Math.floor(Date.now() / 1000),
    status: "completed",
    verdict: "review",
    inputNote: inputNote || "未填写输入说明",
    assets,
    dataRoot: "浏览器预览模式",
    runDir: `local-preview/runs/${runId}`,
    resourceRoot,
    reportPath: `local-preview/runs/${runId}/report.md`,
    steps,
  };

  const report = mockReport(run);
  writeJson(`ugc-audit.report.${runId}`, report);
  const runs = readJson<RunSummary[]>(RUNS_KEY, []);
  writeJson(RUNS_KEY, [
    {
      id: run.id,
      flowName: run.flowName,
      createdAt: run.createdAt,
      status: run.status,
      verdict: run.verdict,
      reportPath: run.reportPath,
    },
    ...runs,
  ]);
  return run;
}

export async function listRuns(): Promise<RunSummary[]> {
  if (hasTauri()) return invoke<RunSummary[]>("list_runs");
  return readJson(RUNS_KEY, []);
}

export async function readRunReport(runId: string): Promise<string> {
  if (hasTauri()) return invoke<string>("read_run_report", { runId });
  return readJson(`ugc-audit.report.${runId}`, "");
}
