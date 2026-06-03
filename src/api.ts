import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  AppSettings,
  AuditAsset,
  AuditScheme,
  FlowDefinition,
  JsonValue,
  ModuleInfo,
  RunRecord,
  RunPerformanceSummary,
  RunStartResponse,
  RunSummary,
  RuntimeStatus,
  SavedAuditScheme,
  SchemeListItem,
  StepRun,
  ValidationResult,
} from "./types";

const FLOW_KEY = "ugc-audit.flow";
const RUNS_KEY = "ugc-audit.runs";
const SCHEME_LIBRARY_KEY = "ugc-audit.schemeLibrary";
const APP_SETTINGS_KEY = "ugc-audit.appSettings";
const PREVIEW_DEPENDENCY_ROOT = "preview/依赖";

const hasTauri = () => Boolean(window.__TAURI_INTERNALS__);
const START_NODE_ID = "flow_start";
const OUTPUT_NODE_ID = "flow_output";
const START_MODULE_ID = "system.start";
const OUTPUT_MODULE_ID = "system.output";
const DATA_ALL_IMAGES_MODULE_ID = "system.data.images.all";
const DATA_ALL_TEXTS_MODULE_ID = "system.data.texts.all";
const DATA_ARTIFACT_IMAGES_MODULE_ID = "system.data.images.artifacts";
const DATA_ARTIFACT_TEXTS_MODULE_ID = "system.data.texts.artifacts";
const DATA_RELATIVE_IMAGES_MODULE_ID = "system.data.images.relative";
const DATA_RELATIVE_TEXTS_MODULE_ID = "system.data.texts.relative";
const DATA_AUDIT_FOLDER_MODULE_ID = "system.data.folder.audit";
const DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID = "system.data.folder.audit.relative";
const DATA_ARTIFACT_FOLDER_MODULE_ID = "system.data.folder.artifact";
const DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID = "system.data.folder.artifact.relative";
const DATA_MERGE_IMAGES_MODULE_ID = "system.data.images.merge";
const DATA_MERGE_TEXTS_MODULE_ID = "system.data.texts.merge";
const ANNOTATION_MODULE_ID = "system.annotation.comment";
const CANVAS_GROUP_MODULE_ID = "system.canvas.group";
const CANVAS_NOTE_MODULE_ID = "system.canvas.note";
const EDGE_TYPE_SEQUENCE = "sequence";
const EDGE_TYPE_DATA = "data";
const HANDLE_SEQUENCE_IN = "sequence-in";
const HANDLE_SEQUENCE_OUT = "sequence-out";
const HANDLE_IMAGES = "images";
const HANDLE_TEXTS = "texts";
const HANDLE_FOLDER = "folder";
const HANDLE_IMAGES_A = "images-a";
const HANDLE_IMAGES_B = "images-b";
const HANDLE_TEXTS_A = "texts-a";
const HANDLE_TEXTS_B = "texts-b";
const DATA_TYPE_IMAGES = "imageCollection";
const DATA_TYPE_TEXTS = "textCollection";
const DATA_TYPE_FOLDER = "folder";

const legacyModuleIds: Record<string, string> = {
  "builtin.paddleocr": "preset.custom.paddleocr",
  "builtin.shieldgemma2": "preset.custom.shieldgemma2",
  "builtin.qwen3guard": "preset.custom.qwen3guard",
};

function systemLaunch(notes: string): ModuleInfo["launch"] {
  return { launchType: "system", command: null, url: null, method: null, args: [], notes };
}

function systemModule(
  id: string,
  name: string,
  kind: string,
  summary: string,
  icon: string,
  parameters: ModuleInfo["parameters"] = [],
): ModuleInfo {
  return {
    id,
    name,
    kind,
    summary,
    modelLabel: "无需模型",
    icon,
    builtIn: true,
    source: "system",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("系统内置节点，不启动外部模块。"),
    parameters,
    dataOutputs: [],
  };
}

const moduleSpecs: ModuleInfo[] = [
  systemModule(START_MODULE_ID, "开始", "flow_start", "流程入口，接收本次审核素材", "play-circle"),
  systemModule(OUTPUT_MODULE_ID, "输出结果", "flow_output", "汇总所有步骤并生成 Markdown 报告", "file-output"),
  systemModule(DATA_ALL_IMAGES_MODULE_ID, "待测项目中所有图片", "data_all_images", "提供本次待测项目里的全部图片集合", "database"),
  systemModule(DATA_ALL_TEXTS_MODULE_ID, "待测项目中所有文本", "data_all_texts", "提供本次待测项目里的全部文本集合", "file-text"),
  systemModule(DATA_ARTIFACT_IMAGES_MODULE_ID, "产物文件夹中所有图片", "data_artifact_images", "提供本次审核产物文件夹里的全部图片集合", "hard-drive"),
  systemModule(DATA_ARTIFACT_TEXTS_MODULE_ID, "产物文件夹中所有文本", "data_artifact_texts", "提供本次审核产物文件夹里的全部文本集合", "hard-drive"),
  systemModule(DATA_RELATIVE_IMAGES_MODULE_ID, "待测项目相对路径下所有图片", "data_relative_images", "提供指定相对路径下的图片集合", "folder-open", [
    {
      key: "relativePath",
      name: "相对路径",
      description: "只收集待测项目中这个相对路径下的图片。",
      parameterType: "string",
      defaultValue: "images",
      required: true,
      options: [],
    },
  ]),
  systemModule(DATA_RELATIVE_TEXTS_MODULE_ID, "待测项目相对路径下所有文本", "data_relative_texts", "提供指定相对路径下的文本集合", "folder-open", [
    {
      key: "relativePath",
      name: "相对路径",
      description: "只收集待测项目中这个相对路径下的文本。",
      parameterType: "string",
      defaultValue: "texts",
      required: true,
      options: [],
    },
  ]),
  systemModule(DATA_AUDIT_FOLDER_MODULE_ID, "待审核文件夹", "data_audit_folder", "提供本次待审核文件夹", "folder-open"),
  systemModule(DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID, "待审核文件夹下相对路径文件夹", "data_audit_relative_folder", "提供待审核文件夹下指定相对路径的文件夹", "folder-open", [
    {
      key: "relativePath",
      name: "相对路径",
      description: "定位待审核文件夹下的这个相对路径文件夹。",
      parameterType: "string",
      defaultValue: "Assets",
      required: true,
      options: [],
    },
  ]),
  systemModule(DATA_ARTIFACT_FOLDER_MODULE_ID, "产物文件夹", "data_artifact_folder", "提供本次审核产物文件夹", "hard-drive"),
  systemModule(DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID, "待产物文件夹下相对路径文件夹", "data_artifact_relative_folder", "提供产物文件夹下指定相对路径的文件夹", "hard-drive", [
    {
      key: "relativePath",
      name: "相对路径",
      description: "定位产物文件夹下的这个相对路径文件夹。",
      parameterType: "string",
      defaultValue: "outputs",
      required: true,
      options: [],
    },
  ]),
  systemModule(DATA_MERGE_IMAGES_MODULE_ID, "将两个图片集合合并", "data_merge_images", "把两个图片集合合并并去重", "database"),
  systemModule(DATA_MERGE_TEXTS_MODULE_ID, "将两个文本集合合并", "data_merge_texts", "把两个文本集合合并并去重", "database"),
  systemModule(ANNOTATION_MODULE_ID, "注释框", "annotation_comment", "画布注释和分组框", "file-text"),
  systemModule(CANVAS_GROUP_MODULE_ID, "分组", "canvas_group", "创建带名称的画布分组框", "group"),
  systemModule(CANVAS_NOTE_MODULE_ID, "注释", "canvas_note", "创建独立注释便签", "sticky-note"),
  {
    id: "preset.custom.paddleocr",
    name: "图片文字识别",
    kind: "image_ocr",
    summary: "面向 PaddleOCR 本地运行入口",
    modelLabel: "PaddleOCR 模型目录",
    icon: "scan-text",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("浏览器预览模式不启动外部模块。"),
    parameters: [],
  },
  {
    id: "preset.custom.qwen3guard",
    name: "文本合规检测",
    kind: "text_safety",
    summary: "面向 Qwen3Guard 本地入口",
    modelLabel: "Qwen3Guard 模型目录",
    icon: "shield-alert",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("浏览器预览模式不启动外部模块。"),
    parameters: [],
  },
  {
    id: "preset.custom.shieldgemma2",
    name: "图片合规检测",
    kind: "image_safety",
    summary: "面向 ShieldGemma 2 本地入口",
    modelLabel: "ShieldGemma 2 模型目录",
    icon: "shield-alert",
    builtIn: true,
    source: "preset",
    definitionDir: "浏览器预览模式",
    modelPath: null,
    modelConfigured: true,
    launch: systemLaunch("浏览器预览模式不启动外部模块。"),
    parameters: [
      {
        key: "customPolicies",
        name: "自定义策略",
        description: "默认固定检测 sexual、violence、dangerous；这里额外添加策略名称和策略描述。",
        parameterType: "policyList",
        defaultValue: [],
        required: false,
        options: [],
      },
      {
        key: "threshold",
        name: "风险阈值",
        description: "高于该分值时进入人工复审。",
        parameterType: "number",
        defaultValue: 0.7,
        required: false,
        options: [],
      },
    ],
  },
];

export function defaultConfigForModule(module: ModuleInfo): Record<string, JsonValue> {
  return Object.fromEntries(
    module.parameters.map((parameter) => [parameter.key, parameter.defaultValue]),
  ) as Record<string, JsonValue>;
}

function sequenceEdge(id: string, from: string, to: string) {
  return {
    id,
    from,
    to,
    edgeType: EDGE_TYPE_SEQUENCE as const,
    fromHandle: HANDLE_SEQUENCE_OUT,
    toHandle: HANDLE_SEQUENCE_IN,
  };
}

function dataEdge(id: string, from: string, fromHandle: string, to: string, toHandle: string) {
  return {
    id,
    from,
    to,
    edgeType: EDGE_TYPE_DATA as const,
    fromHandle,
    toHandle,
  };
}

function normalizeEdge(edge: FlowDefinition["edges"][number]): FlowDefinition["edges"][number] {
  const edgeType = edge.edgeType === EDGE_TYPE_DATA ? EDGE_TYPE_DATA : EDGE_TYPE_SEQUENCE;
  return {
    ...edge,
    edgeType,
    fromHandle:
      edge.fromHandle || (edgeType === EDGE_TYPE_SEQUENCE ? HANDLE_SEQUENCE_OUT : null),
    toHandle: edge.toHandle || (edgeType === EDGE_TYPE_SEQUENCE ? HANDLE_SEQUENCE_IN : null),
  };
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
    edges: flow.edges.map(normalizeEdge),
  };
  const ensured = ensureSystemNodes(normalized);
  return isMinimalSystemFlow(ensured)
    ? defaultFlowDefinition()
    : upgradeLegacyDefaultDataFlow(ensured);
}

export function defaultFlowDefinition(): FlowDefinition {
  return {
    id: "flow.default.image-audit",
    name: "图片 UGC 默认审核",
    version: 1,
    nodes: [
      {
        id: START_NODE_ID,
        moduleId: START_MODULE_ID,
        label: "开始",
        position: { x: 120, y: 260 },
        config: {},
      },
      {
        id: "all_images",
        moduleId: DATA_ALL_IMAGES_MODULE_ID,
        label: "待测项目中所有图片",
        position: { x: 360, y: 60 },
        config: {},
      },
      {
        id: "image_ocr",
        moduleId: "preset.custom.paddleocr",
        label: "图片文字识别",
        position: { x: 520, y: 160 },
        config: {},
      },
      {
        id: "image_safety",
        moduleId: "preset.custom.shieldgemma2",
        label: "图片合规检测",
        position: { x: 520, y: 360 },
        config: {},
      },
      {
        id: "text_safety",
        moduleId: "preset.custom.qwen3guard",
        label: "文本合规检测",
        position: { x: 820, y: 160 },
        config: {},
      },
      {
        id: OUTPUT_NODE_ID,
        moduleId: OUTPUT_MODULE_ID,
        label: "输出结果",
        position: { x: 1120, y: 260 },
        config: {},
      },
    ],
    edges: [
      sequenceEdge("edge_flow_start_image_ocr", START_NODE_ID, "image_ocr"),
      sequenceEdge("edge_flow_start_image_safety", START_NODE_ID, "image_safety"),
      sequenceEdge("edge_seq_image_ocr_text_safety", "image_ocr", "text_safety"),
      sequenceEdge("edge_image_safety_output", "image_safety", OUTPUT_NODE_ID),
      sequenceEdge("edge_text_safety_output", "text_safety", OUTPUT_NODE_ID),
      dataEdge("edge_all_images_image_ocr", "all_images", HANDLE_IMAGES, "image_ocr", HANDLE_IMAGES),
      dataEdge("edge_all_images_image_safety", "all_images", HANDLE_IMAGES, "image_safety", HANDLE_IMAGES),
      dataEdge("edge_data_image_ocr_text_safety", "image_ocr", HANDLE_TEXTS, "text_safety", HANDLE_TEXTS),
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
    edges.push(sequenceEdge("edge_flow_start_output", startId, outputId));
  }

  return { ...flow, nodes, edges };
}

function isMinimalSystemFlow(flow: FlowDefinition) {
  if (flow.nodes.length !== 2) return false;
  const start = flow.nodes.find((node) => node.moduleId === START_MODULE_ID);
  const output = flow.nodes.find((node) => node.moduleId === OUTPUT_MODULE_ID);
  return Boolean(start && output && flow.edges.some((edge) => edge.from === start.id && edge.to === output.id));
}

function firstNodeIdByModule(flow: FlowDefinition, moduleId: string) {
  return flow.nodes.find((node) => node.moduleId === moduleId)?.id ?? null;
}

function uniqueNodeId(flow: FlowDefinition, preferred: string) {
  if (!flow.nodes.some((node) => node.id === preferred)) return preferred;
  let index = 2;
  while (flow.nodes.some((node) => node.id === `${preferred}_${index}`)) index += 1;
  return `${preferred}_${index}`;
}

function upgradeLegacyDefaultDataFlow(flow: FlowDefinition): FlowDefinition {
  if (flow.id !== "flow.default.image-audit" || flow.edges.some((edge) => edge.edgeType === EDGE_TYPE_DATA)) {
    return flow;
  }
  const ocrId = firstNodeIdByModule(flow, "preset.custom.paddleocr");
  const imageSafetyId = firstNodeIdByModule(flow, "preset.custom.shieldgemma2");
  const textSafetyId = firstNodeIdByModule(flow, "preset.custom.qwen3guard");
  if (!ocrId || !imageSafetyId || !textSafetyId) return flow;
  const imageSourceId = uniqueNodeId(flow, "all_images");
  return {
    ...flow,
    nodes: [
      ...flow.nodes,
      {
        id: imageSourceId,
        moduleId: DATA_ALL_IMAGES_MODULE_ID,
        label: "待测项目中所有图片",
        position: { x: 360, y: 60 },
        config: {},
      },
    ],
    edges: [
      ...flow.edges,
      dataEdge("edge_all_images_image_ocr", imageSourceId, HANDLE_IMAGES, ocrId, HANDLE_IMAGES),
      dataEdge("edge_all_images_image_safety", imageSourceId, HANDLE_IMAGES, imageSafetyId, HANDLE_IMAGES),
      dataEdge("edge_data_image_ocr_text_safety", ocrId, HANDLE_TEXTS, textSafetyId, HANDLE_TEXTS),
    ],
  };
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

function schemeFileName(name: string) {
  const stem = (name.trim() || "审核方案")
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_")
    .replace(/\s+/g, " ")
    .replace(/[. ]+$/g, "")
    .slice(0, 80);
  return `${stem || "审核方案"}.ugcaudit`;
}

function readBrowserSchemeLibrary(): SchemeListItem[] {
  return readJson<SchemeListItem[]>(SCHEME_LIBRARY_KEY, []);
}

function writeBrowserSchemeLibrary(items: SchemeListItem[]) {
  writeJson(SCHEME_LIBRARY_KEY, items);
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

function asObject(value: JsonValue | undefined): Record<string, JsonValue> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return value as Record<string, JsonValue>;
}

function moduleSpec(moduleId: string) {
  return moduleSpecs.find((module) => module.id === moduleId);
}

function isPureDataKind(kind: string) {
  return [
    "data_all_images",
    "data_all_texts",
    "data_artifact_images",
    "data_artifact_texts",
    "data_relative_images",
    "data_relative_texts",
    "data_audit_folder",
    "data_audit_relative_folder",
    "data_artifact_folder",
    "data_artifact_relative_folder",
    "data_merge_images",
    "data_merge_texts",
  ].includes(kind);
}

function isAnnotationKind(kind: string) {
  return kind === "annotation_comment";
}

function isCanvasToolKind(kind: string) {
  return kind === "canvas_group" || kind === "canvas_note" || isAnnotationKind(kind);
}

function isPassiveCanvasKind(kind: string) {
  return isPureDataKind(kind) || isCanvasToolKind(kind);
}

function hasSequenceInput(module: ModuleInfo) {
  return module.kind !== "flow_start" && !isPassiveCanvasKind(module.kind);
}

function hasSequenceOutput(module: ModuleInfo) {
  return module.kind !== "flow_output" && !isPassiveCanvasKind(module.kind);
}

function dataInputType(module: ModuleInfo, handle: string | null | undefined) {
  if ((module.kind === "image_ocr" || module.kind === "image_safety") && handle === HANDLE_IMAGES) return DATA_TYPE_IMAGES;
  if (module.kind === "text_safety" && handle === HANDLE_TEXTS) return DATA_TYPE_TEXTS;
  if (module.kind === "data_merge_images" && (handle === HANDLE_IMAGES_A || handle === HANDLE_IMAGES_B)) return DATA_TYPE_IMAGES;
  if (module.kind === "data_merge_texts" && (handle === HANDLE_TEXTS_A || handle === HANDLE_TEXTS_B)) return DATA_TYPE_TEXTS;
  if (module.kind === "folder_processor" && handle === HANDLE_FOLDER) return DATA_TYPE_FOLDER;
  return null;
}

function dataOutputType(module: ModuleInfo, handle: string | null | undefined) {
  const declaredOutput = module.dataOutputs?.find((output) => output.handle === handle);
  if (declaredOutput) return declaredOutput.dataType;
  if (module.kind === "image_ocr" && handle === HANDLE_TEXTS) return DATA_TYPE_TEXTS;
  if (["data_all_images", "data_artifact_images", "data_relative_images", "data_merge_images"].includes(module.kind) && handle === HANDLE_IMAGES) return DATA_TYPE_IMAGES;
  if (["data_all_texts", "data_artifact_texts", "data_relative_texts", "data_merge_texts"].includes(module.kind) && handle === HANDLE_TEXTS) return DATA_TYPE_TEXTS;
  if (["data_audit_folder", "data_audit_relative_folder", "data_artifact_folder", "data_artifact_relative_folder"].includes(module.kind) && handle === HANDLE_FOLDER) return DATA_TYPE_FOLDER;
  return null;
}

function requiredDataInputs(module: ModuleInfo) {
  if (module.kind === "image_ocr" || module.kind === "image_safety") return [HANDLE_IMAGES];
  if (module.kind === "text_safety") return [HANDLE_TEXTS];
  if (module.kind === "data_merge_images") return [HANDLE_IMAGES_A, HANDLE_IMAGES_B];
  if (module.kind === "data_merge_texts") return [HANDLE_TEXTS_A, HANDLE_TEXTS_B];
  if (module.kind === "folder_processor") return [HANDLE_FOLDER];
  return [];
}

function validateFlowLocal(flow: FlowDefinition): ValidationResult {
  const messages: string[] = [];
  const nodeIds = new Set(flow.nodes.map((node) => node.id));
  const nodesById = new Map(flow.nodes.map((node) => [node.id, node]));
  const dataTargets = new Set<string>();

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
    const sourceNode = nodesById.get(edge.from);
    const targetNode = nodesById.get(edge.to);
    const sourceModule = sourceNode ? moduleSpec(sourceNode.moduleId) : null;
    const targetModule = targetNode ? moduleSpec(targetNode.moduleId) : null;
    if (!sourceModule || !targetModule || !sourceNode || !targetNode) continue;
    if (edge.edgeType === EDGE_TYPE_DATA) {
      const outputType = dataOutputType(sourceModule, edge.fromHandle);
      const inputType = dataInputType(targetModule, edge.toHandle);
      if (!outputType) messages.push(`连线 ${edge.id} 的输出口不是有效数据口。`);
      if (!inputType) messages.push(`连线 ${edge.id} 的输入口不是有效数据口。`);
      if (outputType && inputType && outputType !== inputType) messages.push(`连线 ${edge.id} 的数据类型不匹配。`);
      const targetKey = `${edge.to}:${edge.toHandle ?? ""}`;
      if (dataTargets.has(targetKey)) messages.push(`步骤 ${targetNode.label} 的同一个数据口被连接了多次。`);
      dataTargets.add(targetKey);
    } else {
      if (edge.fromHandle !== HANDLE_SEQUENCE_OUT || edge.toHandle !== HANDLE_SEQUENCE_IN) {
        messages.push(`连线 ${edge.id} 的顺序口不正确。`);
      }
      if (!hasSequenceOutput(sourceModule)) messages.push(`步骤 ${sourceNode.label} 没有顺序输出口。`);
      if (!hasSequenceInput(targetModule)) messages.push(`步骤 ${targetNode.label} 没有顺序输入口。`);
    }
  }

  const checkedDataDependencies = new Set<string>();
  for (const node of flow.nodes) {
    const module = moduleSpec(node.moduleId);
    if (!module || isPassiveCanvasKind(module.kind)) continue;
    for (const consumerId of executableDataConsumers(flow, node.id)) {
      if (consumerId === node.id) continue;
      const consumerNode = nodesById.get(consumerId);
      if (!consumerNode) continue;
      const key = `${node.id}:${consumerId}`;
      if (checkedDataDependencies.has(key)) continue;
      checkedDataDependencies.add(key);
      if (!hasPath(flow, node.id, consumerId)) {
        messages.push(`数据来源 ${node.label} 必须先通过顺序线连接到 ${consumerNode.label}。`);
      }
    }
  }

  const startId = startNodes[0]?.id;
  const outputId = outputNodes[0]?.id;
  if (startId && flow.edges.some((edge) => edge.edgeType !== EDGE_TYPE_DATA && edge.to === startId)) {
    messages.push("开始节点不能有输入连线。");
  }
  if (outputId && flow.edges.some((edge) => edge.edgeType !== EDGE_TYPE_DATA && edge.from === outputId)) {
    messages.push("输出结果节点不能有输出连线。");
  }
  if (startId && outputId && !hasPath(flow, startId, outputId)) {
    messages.push("开始节点必须能连到输出结果节点。");
  }

  for (const node of flow.nodes) {
    const module = moduleSpec(node.moduleId);
    if (!module) continue;
    const config = asObject(node.config);
    if ((module.kind === "data_relative_images" || module.kind === "data_relative_texts" || module.kind === "data_audit_relative_folder" || module.kind === "data_artifact_relative_folder") && !String(config.relativePath ?? "").trim()) {
      messages.push(`步骤 ${node.label} 需要填写相对路径。`);
    }
    for (const handle of requiredDataInputs(module)) {
      if (!flow.edges.some((edge) => edge.edgeType === EDGE_TYPE_DATA && edge.to === node.id && edge.toHandle === handle)) {
        messages.push(`步骤 ${node.label} 缺少必需的数据输入：${handle}。`);
      }
    }
  }

  return { valid: messages.length === 0, messages };
}

function executableDataConsumers(flow: FlowDefinition, sourceId: string) {
  const nodesById = new Map(flow.nodes.map((node) => [node.id, node]));
  const dataOutgoing = new Map<string, FlowDefinition["edges"]>();
  for (const edge of flow.edges.filter((item) => item.edgeType === EDGE_TYPE_DATA)) {
    dataOutgoing.set(edge.from, [...(dataOutgoing.get(edge.from) ?? []), edge]);
  }

  const consumers = new Set<string>();
  const seen = new Set<string>();
  const queue = [...(dataOutgoing.get(sourceId) ?? []).map((edge) => edge.to)];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current || seen.has(current)) continue;
    seen.add(current);
    const node = nodesById.get(current);
    const module = node ? moduleSpec(node.moduleId) : null;
    if (!module) continue;
    if (isPureDataKind(module.kind)) {
      queue.push(...(dataOutgoing.get(current) ?? []).map((edge) => edge.to));
    } else {
      consumers.add(current);
    }
  }
  return consumers;
}

function hasPath(flow: FlowDefinition, from: string, to: string) {
  const outgoing = new Map<string, string[]>();
  for (const edge of flow.edges.filter((item) => item.edgeType !== EDGE_TYPE_DATA)) {
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

const PERFORMANCE_SAMPLING_NOTE =
  "浏览器预览模式使用模拟性能数据；桌面端会按模块进程及其子进程采样。";

function formatDurationMs(durationMs: number) {
  return durationMs < 1000 ? `${Math.round(durationMs)} ms` : `${(durationMs / 1000).toFixed(2)} s`;
}

function formatBytes(bytes: number) {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  return `${Math.round(bytes)} B`;
}

function formatPercent(value: number) {
  return `${Math.max(0, value).toFixed(1)}%`;
}

function formatCpuTimeMs(value: number) {
  return value < 1000 ? `${Math.round(Math.max(0, value))} ms` : `${(value / 1000).toFixed(2)} s`;
}

function leaderForStep(step: StepRun, value: number) {
  return {
    stepId: step.stepId,
    label: step.label,
    moduleName: step.moduleName,
    value,
  };
}

function buildPerformanceSummary(steps: StepRun[]): RunPerformanceSummary {
  const measuredSteps = steps.filter((step) => step.performance);
  const totalCpuTimeMs = measuredSteps.reduce((total, step) => total + Math.max(0, step.performance?.cpuTimeMs ?? 0), 0);
  const totalDurationMs = measuredSteps.reduce((total, step) => total + (step.performance?.durationMs ?? 0), 0);
  const totalArtifactBytes = measuredSteps.reduce((total, step) => total + (step.performance?.artifactBytes ?? 0), 0);

  for (const step of measuredSteps) {
    if (step.performance) {
      step.performance.cpuSharePercent = totalCpuTimeMs > 0 ? (step.performance.cpuTimeMs / totalCpuTimeMs) * 100 : 0;
    }
  }

  const cpuLeaderStep = totalCpuTimeMs > 0
    ? measuredSteps.reduce<StepRun | null>((leader, step) => {
      if (!leader) return step;
      return (step.performance?.cpuTimeMs ?? 0) > (leader.performance?.cpuTimeMs ?? 0) ? step : leader;
    }, null)
    : null;
  const durationLeaderStep = measuredSteps.reduce<StepRun | null>((leader, step) => {
    if (!leader) return step;
    return (step.performance?.durationMs ?? 0) > (leader.performance?.durationMs ?? 0) ? step : leader;
  }, null);
  const memoryLeaderStep = measuredSteps.reduce<StepRun | null>((leader, step) => {
    if (!leader) return step;
    return (step.performance?.peakMemoryBytes ?? 0) > (leader.performance?.peakMemoryBytes ?? 0) ? step : leader;
  }, null);

  return {
    totalDurationMs,
    totalCpuTimeMs,
    totalArtifactBytes,
    measuredSteps: measuredSteps.length,
    gpuAvailable: measuredSteps.some((step) => Boolean(step.performance?.gpuAvailable)),
    gpuSampled: measuredSteps.some((step) => (step.performance?.peakGpuMemoryBytes ?? 0) > 0),
    cpuLeader: cpuLeaderStep?.performance ? leaderForStep(cpuLeaderStep, cpuLeaderStep.performance.cpuTimeMs) : null,
    durationLeader: durationLeaderStep?.performance ? leaderForStep(durationLeaderStep, durationLeaderStep.performance.durationMs) : null,
    memoryLeader: memoryLeaderStep?.performance && memoryLeaderStep.performance.peakMemoryBytes > 0
      ? leaderForStep(memoryLeaderStep, memoryLeaderStep.performance.peakMemoryBytes)
      : null,
    samplingNote: PERFORMANCE_SAMPLING_NOTE,
  };
}

function mockStepPerformance(index: number, moduleSource: ModuleInfo["source"]) {
  if (moduleSource === "system") return null;
  const durationMs = 1200 + index * 730;
  const cpuTimeMs = 680 + index * 410;
  return {
    startTime: Date.now(),
    endTime: Date.now() + durationMs,
    durationMs,
    sampleCount: 2 + index,
    cpuTimeMs,
    cpuSharePercent: 0,
    averageCpuPercent: 8 + index * 3,
    peakCpuPercent: 16 + index * 4,
    peakMemoryBytes: (180 + index * 96) * 1024 * 1024,
    artifactBytes: index * 36 * 1024,
    gpuAvailable: false,
    gpuSampleCount: 0,
    peakGpuMemoryBytes: null,
    samplingNote: PERFORMANCE_SAMPLING_NOTE,
  };
}

function performanceReport(run: RunRecord) {
  const measuredSteps = run.steps.filter((step) => step.performance);
  const summary = run.performanceSummary;
  if (!summary || measuredSteps.length === 0) {
    return "## 性能开销\n\n未采集到外部模块性能数据。\n\n";
  }
  const rows = measuredSteps.map((step) => {
    const performance = step.performance;
    if (!performance) return "";
    const gpuText = performance.gpuAvailable
      ? formatBytes(performance.peakGpuMemoryBytes ?? 0)
      : "未采集";
    return `| ${step.label} | ${step.moduleName} | ${formatDurationMs(performance.durationMs)} | ${formatPercent(performance.cpuSharePercent)} | ${formatCpuTimeMs(performance.cpuTimeMs)} | ${formatPercent(performance.averageCpuPercent)} | ${formatBytes(performance.peakMemoryBytes)} | ${formatBytes(performance.artifactBytes)} | ${gpuText} |`;
  }).filter(Boolean).join("\n");

  const leader = summary.cpuLeader ?? summary.durationLeader;
  return `## 性能开销

- 已采集模块：${summary.measuredSteps} 个
- 模块总耗时：${formatDurationMs(summary.totalDurationMs)}
- CPU 估算总量：${formatCpuTimeMs(summary.totalCpuTimeMs)}
- 最大开销模块：${leader ? `${leader.label}` : "暂无"}
- 最耗时模块：${summary.durationLeader ? `${summary.durationLeader.label}（${formatDurationMs(summary.durationLeader.value)}）` : "暂无"}
- 峰值内存最高：${summary.memoryLeader ? `${summary.memoryLeader.label}（${formatBytes(summary.memoryLeader.value)}）` : "暂无"}
- NVIDIA GPU：${summary.gpuAvailable ? "已尝试采集" : "未采集"}
- 采样说明：${summary.samplingNote}

| 步骤 | 模块 | 耗时 | CPU 占本次运行 | CPU 估算 | 平均 CPU | 峰值内存 | 产物大小 | NVIDIA GPU |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows}
`;
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

${performanceReport(run)}

## 模块结论

${run.steps
  .map((step) => step.reportSection)
  .join("\n")}

## 本地文件

- 运行目录：${run.runDir}
- 产物目录：${run.artifactDir ?? ""}
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

export async function getAppSettings(): Promise<AppSettings> {
  if (hasTauri()) return invoke<AppSettings>("get_app_settings");
  const settings = readJson<AppSettings>(APP_SETTINGS_KEY, {
    artifactRoot: "preview/审核产物",
    dependencyRoot: PREVIEW_DEPENDENCY_ROOT,
  });
  return {
    artifactRoot: settings.artifactRoot || "preview/审核产物",
    dependencyRoot: settings.dependencyRoot || PREVIEW_DEPENDENCY_ROOT,
  };
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  if (hasTauri()) return invoke<AppSettings>("save_app_settings", { settings });
  writeJson(APP_SETTINGS_KEY, settings);
  return settings;
}

export async function selectArtifactRootDirectory(): Promise<string | null> {
  if (!hasTauri()) return "preview/审核产物";
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择审核产物默认生成路径",
  });
  if (!selected) return null;
  return Array.isArray(selected) ? selected[0] ?? null : selected;
}

export async function selectDependencyRootDirectory(): Promise<string | null> {
  if (!hasTauri()) return PREVIEW_DEPENDENCY_ROOT;
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择依赖存放默认路径",
  });
  if (!selected) return null;
  return Array.isArray(selected) ? selected[0] ?? null : selected;
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

export function schemeFromFlow(flow: FlowDefinition, name?: string): AuditScheme {
  const normalized = normalizeFlow(flow);
  const schemeName = (name ?? normalized.name ?? "未命名审核方案").trim() || "未命名审核方案";
  return {
    schemaVersion: 1,
    kind: "ugcAuditScheme",
    id: `scheme_${Date.now()}`,
    name: schemeName,
    flow: {
      ...normalized,
      name: schemeName,
    },
  };
}

export async function selectSchemePath(): Promise<string | null> {
  if (!hasTauri()) return "preview/default.ugcaudit";
  const selected = await open({
    multiple: false,
    title: "加载审核方案",
    filters: [
      {
        name: "UGCAudit 审核方案",
        extensions: ["ugcaudit"],
      },
    ],
  });
  if (!selected) return null;
  return Array.isArray(selected) ? selected[0] ?? null : selected;
}

export async function selectSchemeSavePath(defaultPath?: string | null): Promise<string | null> {
  if (!hasTauri()) return defaultPath || "preview/default.ugcaudit";
  return save({
    title: "保存审核方案",
    defaultPath: defaultPath || "审核方案.ugcaudit",
    filters: [
      {
        name: "UGCAudit 审核方案",
        extensions: ["ugcaudit"],
      },
    ],
  });
}

export async function getSchemeLibraryDir(): Promise<string> {
  if (hasTauri()) return invoke<string>("get_scheme_library_dir");
  return "preview/schemes";
}

export async function listSchemeFiles(): Promise<SchemeListItem[]> {
  if (hasTauri()) return invoke<SchemeListItem[]>("list_scheme_files");
  return readBrowserSchemeLibrary();
}

export async function deleteSchemeFile(path: string): Promise<SchemeListItem[]> {
  if (hasTauri()) return invoke<SchemeListItem[]>("delete_scheme_file", { path });
  const existing = readBrowserSchemeLibrary().filter((item) => item.path !== path);
  localStorage.removeItem(`ugc-audit.scheme.${path}`);
  writeBrowserSchemeLibrary(existing);
  return existing;
}

export async function loadSchemeFile(path: string): Promise<AuditScheme> {
  if (hasTauri()) return invoke<AuditScheme>("load_scheme_file", { path });
  const raw = localStorage.getItem(`ugc-audit.scheme.${path}`);
  if (!raw) return schemeFromFlow(readJson(FLOW_KEY, defaultFlow), "浏览器预览方案");
  const scheme = JSON.parse(raw) as AuditScheme;
  return {
    ...scheme,
    flow: normalizeFlow(scheme.flow),
  };
}

export async function saveSchemeFile(path: string, scheme: AuditScheme): Promise<AuditScheme> {
  const normalized = {
    ...scheme,
    schemaVersion: 1,
    kind: "ugcAuditScheme" as const,
    flow: normalizeFlow({
      ...scheme.flow,
      name: scheme.name,
    }),
  };
  if (hasTauri()) return invoke<AuditScheme>("save_scheme_file", { path, scheme: normalized });
  const validation = validateFlowLocal(normalized.flow);
  if (!validation.valid) throw new Error(validation.messages.join(" "));
  writeJson(`ugc-audit.scheme.${path}`, normalized);
  const existing = readBrowserSchemeLibrary();
  if (existing.some((item) => item.path === path)) {
    writeBrowserSchemeLibrary(
      existing.map((item) =>
        item.path === path
          ? { ...item, id: normalized.id, name: normalized.name, modifiedAt: Math.floor(Date.now() / 1000) }
          : item,
      ),
    );
  }
  writeJson(FLOW_KEY, normalized.flow);
  return normalized;
}

export async function saveSchemeToLibrary(scheme: AuditScheme): Promise<SavedAuditScheme> {
  const normalized = {
    ...scheme,
    schemaVersion: 1,
    kind: "ugcAuditScheme" as const,
    flow: normalizeFlow({
      ...scheme.flow,
      name: scheme.name,
    }),
  };
  if (hasTauri()) return invoke<SavedAuditScheme>("save_scheme_to_library", { scheme: normalized });

  const validation = validateFlowLocal(normalized.flow);
  if (!validation.valid) throw new Error(validation.messages.join(" "));
  const existing = readBrowserSchemeLibrary();
  const baseName = schemeFileName(normalized.name);
  let path = `preview/schemes/${baseName}`;
  let index = 2;
  while (existing.some((item) => item.path === path)) {
    path = `preview/schemes/${baseName.replace(/\.ugcaudit$/i, `-${index}.ugcaudit`)}`;
    index += 1;
  }
  writeJson(`ugc-audit.scheme.${path}`, normalized);
  writeBrowserSchemeLibrary([
    {
      id: normalized.id,
      name: normalized.name,
      path,
      modifiedAt: Math.floor(Date.now() / 1000),
    },
    ...existing,
  ]);
  writeJson(FLOW_KEY, normalized.flow);
  return { path, scheme: normalized };
}

export async function validateFlow(flow: FlowDefinition): Promise<ValidationResult> {
  if (hasTauri()) return invoke<ValidationResult>("validate_flow", { flow: normalizeFlow(flow) });
  return validateFlowLocal(normalizeFlow(flow));
}

export async function saveModelPath(moduleId: string, path: string): Promise<ModuleInfo[]> {
  if (hasTauri()) return invoke<ModuleInfo[]>("save_model_path", { moduleId, path });
  return moduleSpecs;
}

export async function importModuleFolder(): Promise<ModuleInfo[] | null> {
  if (hasTauri()) {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择模块文件夹",
    });
    if (!selected) return null;
    const folderPath = Array.isArray(selected) ? selected[0] : selected;
    if (!folderPath) return null;
    return invoke<ModuleInfo[]>("import_module_folder", { folderPath });
  }

  throw new Error("浏览器预览模式不能导入本地模块");
}

export async function removeModule(moduleId: string): Promise<ModuleInfo[]> {
  if (hasTauri()) return invoke<ModuleInfo[]>("remove_module", { moduleId });
  throw new Error("浏览器预览模式不能移除本地模块");
}

export async function openModuleDefinitionFolder(moduleId: string): Promise<void> {
  if (hasTauri()) {
    await invoke("open_module_definition_folder", { moduleId });
    return;
  }
  throw new Error("浏览器预览模式不能打开本地文件夹");
}

export async function revealReportTarget(path: string, runId?: string | null): Promise<void> {
  if (hasTauri()) {
    await invoke("reveal_report_target", { path, runId: runId ?? null });
    return;
  }
  throw new Error("浏览器预览模式不能定位本地文件");
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  if (hasTauri()) return invoke<RuntimeStatus>("get_runtime_status");
  const dependencyRoot = readJson<AppSettings>(APP_SETTINGS_KEY, {
    artifactRoot: "preview/审核产物",
    dependencyRoot: PREVIEW_DEPENDENCY_ROOT,
  }).dependencyRoot || PREVIEW_DEPENDENCY_ROOT;
  return {
    runtimeRoot: "浏览器预览模式",
    runtimeSource: "preview",
    dependencyRoot,
    pythonDir: "浏览器预览模式",
    pythonPath: "浏览器预览模式",
    pythonInstalled: false,
    pythonVersion: null,
    dependencies: [
      "torch",
      "transformers",
      "pillow",
      "accelerate",
    ].map((id) => ({
      id: id as RuntimeStatus["dependencies"][number]["id"],
      name:
        {
          torch: "Torch",
          transformers: "Transformers",
          pillow: "Pillow",
          accelerate: "Accelerate",
        }[id] ?? id,
      installed: false,
      version: null,
      folder: `${dependencyRoot}\\${id}`,
      sitePackages: `${dependencyRoot}\\${id}\\site-packages`,
    })),
  };
}

export async function installRuntimeDependency(dependencyId: string): Promise<RuntimeStatus> {
  if (hasTauri()) return invoke<RuntimeStatus>("install_runtime_dependency", { dependencyId });
  throw new Error("浏览器预览模式不能安装依赖");
}

export async function openRuntimeDependencyFolder(dependencyId: string): Promise<void> {
  if (hasTauri()) {
    await invoke("open_runtime_dependency_folder", { dependencyId });
    return;
  }
  throw new Error("浏览器预览模式不能打开本地文件夹");
}

export async function openRuntimePythonFolder(): Promise<void> {
  if (hasTauri()) {
    await invoke("open_runtime_python_folder");
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
  const taskName = inputNote.match(/任务名称[:：]\s*([^\n]+)/)?.[1]?.trim() || normalized.name || "审核任务";
  const artifactRoot = readJson<AppSettings>(APP_SETTINGS_KEY, {
    artifactRoot: "preview/审核产物",
    dependencyRoot: PREVIEW_DEPENDENCY_ROOT,
  }).artifactRoot;
  const artifactDir = `${artifactRoot}/${taskName}-${runId}`;
  const executableNodes = normalized.nodes.filter((node) => {
    const module = modules.find((item) => item.id === node.moduleId) ?? modules[0];
    return !isPassiveCanvasKind(module.kind);
  });
  const steps: StepRun[] = executableNodes.map((node, index) => {
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
        performance: mockStepPerformance(index, module.source),
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
      performance: mockStepPerformance(index, module.source),
      reportSection: `### ${node.label}

- 模块：${module.name}
- 模块来源：自定义模块
- 结论：需要人工复审
- 状态：${mockStatusLabel(status)}
- 说明：${message}
`,
    };
  });
  const performanceSummary = buildPerformanceSummary(steps);
  const run: RunRecord = {
    id: runId,
    flowId: normalized.id,
    flowName: normalized.name,
    createdAt: Math.floor(Date.now() / 1000),
    status: "completed",
    verdict: "review",
    taskName,
    inputNote: inputNote || "未填写输入说明",
    assets,
    dataRoot: "浏览器预览模式",
    runDir: `local-preview/runs/${runId}`,
    resourceRoot,
    artifactRoot,
    artifactDir,
    reportPath: `${artifactDir}/report.md`,
    performanceSummary,
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

export async function startRunLive(
  flow: FlowDefinition,
  inputNote: string,
  assets: AuditAsset[],
): Promise<RunStartResponse> {
  const normalized = normalizeFlow(flow);
  if (hasTauri()) return invoke<RunStartResponse>("start_run_live", { flow: normalized, inputNote, assets });
  const run = await startRun(normalized, inputNote, assets);
  return { runId: run.id };
}

export async function cancelRun(runId: string): Promise<void> {
  if (hasTauri()) return invoke<void>("cancel_run", { runId });
}

export async function listRuns(): Promise<RunSummary[]> {
  if (hasTauri()) return invoke<RunSummary[]>("list_runs");
  return readJson(RUNS_KEY, []);
}

export async function readRunRecord(runId: string): Promise<RunRecord> {
  if (hasTauri()) return invoke<RunRecord>("read_run_record", { runId });
  const runs = readJson<RunSummary[]>(RUNS_KEY, []);
  const summary = runs.find((item) => item.id === runId);
  if (!summary) throw new Error("没有找到运行记录");
  return {
    id: summary.id,
    flowId: "flow.default.image-audit",
    flowName: summary.flowName,
    createdAt: summary.createdAt,
    status: summary.status,
    verdict: summary.verdict,
    inputNote: "",
    assets: [],
    dataRoot: "浏览器预览模式",
    runDir: `local-preview/runs/${summary.id}`,
    resourceRoot: `local-preview/runs/${summary.id}/resources`,
    artifactRoot: "preview/审核产物",
    artifactDir: `preview/审核产物/${summary.id}`,
    reportPath: summary.reportPath,
    performanceSummary: null,
    steps: [],
  };
}

export async function readRunReport(runId: string): Promise<string> {
  if (hasTauri()) return invoke<string>("read_run_report", { runId });
  return readJson(`ugc-audit.report.${runId}`, "");
}

export async function deleteRun(runId: string): Promise<RunSummary[]> {
  if (hasTauri()) return invoke<RunSummary[]>("delete_run", { runId });
  const runs = readJson<RunSummary[]>(RUNS_KEY, []).filter((run) => run.id !== runId);
  writeJson(RUNS_KEY, runs);
  localStorage.removeItem(`ugc-audit.report.${runId}`);
  return runs;
}

export async function deleteAllRuns(): Promise<RunSummary[]> {
  if (hasTauri()) return invoke<RunSummary[]>("delete_all_runs");
  for (const run of readJson<RunSummary[]>(RUNS_KEY, [])) {
    localStorage.removeItem(`ugc-audit.report.${run.id}`);
  }
  writeJson(RUNS_KEY, []);
  return [];
}
