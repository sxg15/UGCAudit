import {
  convertFileSrc,
} from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  addEdge,
  Background,
  BaseEdge,
  Connection,
  Controls,
  Edge,
  EdgeProps,
  FinalConnectionState,
  Handle,
  MiniMap,
  Node,
  NodeChange,
  NodeResizer,
  NodeProps,
  Position as HandlePosition,
  ReactFlow,
  ReactFlowProvider,
  SelectionMode,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "@xyflow/react";
import {
  Activity,
  AlertTriangle,
  ArrowUp,
  Bell,
  CheckCircle2,
  Cpu,
  Database,
  Download,
  Eye,
  FileCheck2,
  FilePlus2,
  FileText,
  FolderInput,
  FolderOpen,
  GripVertical,
  Group,
  HardDrive,
  History,
  Pencil,
  Play,
  RefreshCw,
  Redo2,
  Save,
  ScanText,
  Settings,
  ShieldAlert,
  SlidersHorizontal,
  StickyNote,
  TableProperties,
  Trash2,
  Timer,
  Undo2,
  XCircle,
} from "lucide-react";
import mermaid from "mermaid";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import type { Components, UrlTransform } from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import type { Options as RehypeSanitizeOptions } from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import {
  Children,
  DragEvent,
  isValidElement,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  deleteAllRuns,
  deleteRun,
  deleteSchemeFile,
  defaultConfigForModule,
  getAppSettings,
  getDataRoot,
  getSchemeLibraryDir,
  getRuntimeStatus,
  importModuleFolder,
  installRuntimeDependency,
  listSchemeFiles,
  listModules,
  listRuns,
  loadFlow,
  openModuleDefinitionFolder,
  openRuntimeDependencyFolder,
  openRuntimePythonFolder,
  cancelRun,
  defaultFlowDefinition,
  readRunReport,
  readRunRecord,
  removeModule,
  revealReportTarget,
  saveFlow,
  saveAppSettings,
  saveSchemeFile,
  saveSchemeToLibrary,
  selectAssetDirectory,
  selectArtifactRootDirectory,
  selectDependencyRootDirectory,
  selectSchemePath,
  selectSchemeSavePath,
  loadSchemeFile,
  schemeFromFlow,
  startRun,
  startRunLive,
  validateFlow,
} from "./api";
import type {
  AppSettings,
  AuditAsset,
  AuditScheme,
  FlowDefinition,
  JsonValue,
  ModuleInfo,
  ModuleParameter,
  RunRecord,
  RunProgressEvent,
  RunSummary,
  RuntimeDependencyStatus,
  RuntimeLogLine,
  RuntimeStatus,
  SchemeListItem,
  ValidationResult,
} from "./types";

type AppTab = "flow" | "results" | "modules" | "settings";
type SettingsTab = "runtime" | "app";
type LibraryTab = "modules" | "data" | "tools";
type CanvasToolType = "group" | "note";
type JsonObject = Record<string, JsonValue>;
type EdgeKind = "sequence" | "data";
type PortKind = "sequence" | "data";
type PortDirection = "in" | "out";
type DataPortType = "imageCollection" | "textCollection" | "folder";
const CURRENT_SCHEME_VALUE = "__current_scheme__";

type PortDefinition = {
  id: string;
  label: string;
  kind: PortKind;
  direction: PortDirection;
  dataType?: DataPortType;
};

type AuditNodeData = Record<string, unknown> & {
  label: string;
  moduleId: string;
  moduleName: string;
  moduleKind: string;
  moduleIcon: string;
  moduleIconPath?: string | null;
  moduleIconDataUrl?: string | null;
  dataOutputs: ModuleInfo["dataOutputs"];
  source: string;
  config: JsonObject;
  requiresModelPath: boolean;
  runStatus?: string;
  runProgress?: number;
  runMessage?: string;
  runProcessed?: number | null;
  runTotal?: number | null;
};

type AuditNode = Node<AuditNodeData>;

type ModuleDragPreview = {
  name: string;
  icon: string;
  iconPath?: string | null;
  iconDataUrl?: string | null;
  x: number;
  y: number;
  overCanvas: boolean;
};

type PendingConnectionOption = {
  id: string;
  group: "existing" | "new";
  label: string;
  detail: string;
  nodeId?: string;
  module?: ModuleInfo;
  port: PortDefinition;
  edgeType: EdgeKind;
};

type PendingConnectionMenu = {
  x: number;
  y: number;
  flowPosition: { x: number; y: number };
  anchorNodeId: string;
  anchorHandleId: string;
  anchorPort: PortDefinition;
  options: PendingConnectionOption[];
};

type RunNodeState = {
  status: string;
  progress: number;
  message: string;
  processed?: number | null;
  total?: number | null;
};

type QueueTaskStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

type QueueTask = {
  id: string;
  name: string;
  note: string;
  flow: FlowDefinition;
  assets: AuditAsset[];
  status: QueueTaskStatus;
  createdAt: number;
  updatedAt: number;
  startedAt?: number | null;
  finishedAt?: number | null;
  runId?: string | null;
  verdict?: string | null;
  reportPath?: string | null;
  message?: string | null;
  nodeStates?: Record<string, RunNodeState>;
};

type CompletionToast = {
  id: string;
  taskId: string;
  name: string;
  status: QueueTaskStatus;
  runId?: string | null;
  verdict?: string | null;
  message: string;
};

type CanvasSnapshot = {
  nodes: AuditNode[];
  edges: Edge[];
  selectedNodeId: string | null;
};

type CanvasHistoryEntry = {
  id: string;
  label: string;
  timestamp: number;
  snapshot: CanvasSnapshot;
};

type CanvasToolItem = {
  type: CanvasToolType;
  name: string;
  summary: string;
  icon: "group" | "sticky-note";
};

type DataNodeGroup = {
  id: string;
  title: string;
  items: ModuleInfo[];
};

const nodeTypes = {
  audit: AuditNodeCard,
  canvasGroup: CanvasToolNodeCard,
  canvasNote: CanvasToolNodeCard,
};

const edgeTypes = {
  curved: CurvedFlowEdge,
};

const TASK_QUEUE_STORAGE_KEY = "ugc-audit.taskQueue.v1";
const ENTRY_UNREAD_RUNS_STORAGE_KEY = "ugc-audit.entryUnreadRuns.v1";
const HISTORY_UNREAD_RUNS_STORAGE_KEY = "ugc-audit.historyUnreadRuns.v1";

const canvasToolItems: CanvasToolItem[] = [
  {
    type: "group",
    name: "分组",
    summary: "创建带名称的画布分组框",
    icon: "group",
  },
  {
    type: "note",
    name: "注释",
    summary: "创建独立注释便签",
    icon: "sticky-note",
  },
];

const EDGE_TYPE_SEQUENCE = "sequence";
const EDGE_TYPE_DATA = "data";
const ANNOTATION_KIND = "annotation_comment";
const CANVAS_GROUP_MODULE_ID = "system.canvas.group";
const CANVAS_NOTE_MODULE_ID = "system.canvas.note";
const CANVAS_GROUP_KIND = "canvas_group";
const CANVAS_NOTE_KIND = "canvas_note";
const CANVAS_GROUP_DEFAULT_WIDTH = 420;
const CANVAS_GROUP_DEFAULT_HEIGHT = 220;
const CANVAS_NOTE_DEFAULT_WIDTH = 260;
const CANVAS_NOTE_DEFAULT_HEIGHT = 160;
const CANVAS_HISTORY_LIMIT = 100;
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

const markdownSanitizeSchema: RehypeSanitizeOptions = {
  ...defaultSchema,
  attributes: {
    ...defaultSchema.attributes,
    "*": [
      ...(defaultSchema.attributes?.["*"] ?? []),
      "className",
      "style",
    ],
    a: [
      ...(defaultSchema.attributes?.a ?? []),
      "title",
      "target",
      "rel",
    ],
    div: [
      ...(defaultSchema.attributes?.div ?? []),
      "className",
      "style",
      "title",
    ],
    span: [
      ...(defaultSchema.attributes?.span ?? []),
      "className",
      "style",
      "title",
    ],
    img: [
      ...(defaultSchema.attributes?.img ?? []),
      "alt",
      "title",
      "width",
      "height",
    ],
    details: [
      ...(defaultSchema.attributes?.details ?? []),
      "className",
      "open",
    ],
    summary: [
      ...(defaultSchema.attributes?.summary ?? []),
      "className",
      "title",
    ],
    figure: [
      "className",
      "style",
      "title",
    ],
    figcaption: [
      "className",
      "style",
      "title",
    ],
    mark: [
      "className",
      "style",
      "title",
    ],
    small: [
      "className",
      "style",
      "title",
    ],
  },
  protocols: {
    ...defaultSchema.protocols,
    href: [
      ...new Set([
        ...(defaultSchema.protocols?.href ?? []),
        "http",
        "https",
        "mailto",
        "tel",
        "file",
        "ugcaudit",
      ]),
    ],
    src: [],
  },
  strip: [
    ...(defaultSchema.strip ?? []),
    "style",
    "iframe",
    "object",
    "embed",
    "form",
    "textarea",
    "select",
    "button",
  ],
  tagNames: [
    ...new Set([
      ...(defaultSchema.tagNames ?? []),
      "article",
      "aside",
      "figure",
      "figcaption",
      "mark",
      "small",
    ]),
  ],
};

mermaid.initialize({
  startOnLoad: false,
  securityLevel: "strict",
  theme: "base",
  themeVariables: {
    background: "#ffffff",
    primaryColor: "#e9f3f0",
    primaryTextColor: "#172026",
    primaryBorderColor: "#87a59c",
    lineColor: "#62706d",
    secondaryColor: "#f6f7f3",
    tertiaryColor: "#f7f4ed",
    fontFamily: "Inter, Segoe UI, Arial, sans-serif",
  },
  flowchart: {
    htmlLabels: false,
  },
});

function iconFor(name: string) {
  if (name === "play-circle") return Play;
  if (name === "file-output") return FileText;
  if (name === "scan-text") return ScanText;
  if (name === "shield-alert") return ShieldAlert;
  if (name === "database") return Database;
  if (name === "folder-open") return FolderOpen;
  if (name === "file-text") return FileText;
  if (name === "hard-drive") return HardDrive;
  if (name === "group") return Group;
  if (name === "sticky-note") return StickyNote;
  return FileCheck2;
}

function iconImageSrc(iconPath?: string | null) {
  if (!iconPath) return null;
  if (/^(data:|https?:|asset:|file:|blob:)/i.test(iconPath)) return iconPath;
  return window.__TAURI_INTERNALS__ ? convertFileSrc(iconPath) : iconPath;
}

function ModuleIcon({
  icon,
  iconPath,
  iconDataUrl,
  size = 18,
}: {
  icon: string;
  iconPath?: string | null;
  iconDataUrl?: string | null;
  size?: number;
}) {
  const imageSrc = iconDataUrl ?? iconImageSrc(iconPath);
  if (imageSrc) {
    return (
      <img
        className="module-icon-image"
        src={imageSrc}
        alt=""
        aria-hidden="true"
        style={{ width: size, height: size }}
      />
    );
  }

  const Icon = iconFor(icon);
  return <Icon size={size} />;
}

function PythonMark() {
  return (
    <span className="python-mark" aria-hidden="true">
      Py
    </span>
  );
}

function verdictText(verdict: string) {
  if (verdict === "pass") return "通过";
  if (verdict === "reject") return "不通过";
  if (verdict === "error") return "失败";
  return "复审";
}

function statusText(status: string) {
  if (status === "pending") return "等待中";
  if (status === "running") return "运行中";
  if (status === "system") return "系统节点";
  if (status === "ready") return "已配置";
  if (status === "completed") return "已完成";
  if (status === "skipped") return "已跳过";
  if (status === "error") return "失败";
  if (status === "failed") return "失败";
  if (status === "cancelled") return "已中断";
  if (status === "invalid_model_path") return "路径不可用";
  if (status === "needs_model") return "未配置";
  return "已记录";
}

function queueTaskStatusText(status: QueueTaskStatus) {
  if (status === "pending") return "等待中";
  if (status === "running") return "运行中";
  if (status === "completed") return "已完成";
  if (status === "failed") return "失败";
  if (status === "cancelled") return "已中断";
  return "等待中";
}

function isTerminalQueueStatus(status: QueueTaskStatus) {
  return status === "completed" || status === "failed" || status === "cancelled";
}

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values.filter(Boolean)));
}

function addUniqueString(values: string[], value: string) {
  return uniqueStrings([...values, value]);
}

function removeString(values: string[], value: string) {
  return values.filter((item) => item !== value);
}

function readStoredJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function readStoredStringList(key: string) {
  const values = readStoredJson<string[]>(key, []);
  return uniqueStrings(Array.isArray(values) ? values.map(String) : []);
}

function isQueueTaskStatus(value: unknown): value is QueueTaskStatus {
  return value === "pending" || value === "running" || value === "completed" || value === "failed" || value === "cancelled";
}

function orderQueueTasks(tasks: QueueTask[]) {
  const running = tasks.filter((task) => task.status === "running");
  const pending = tasks.filter((task) => task.status === "pending");
  const finished = tasks.filter((task) => task.status !== "running" && task.status !== "pending");
  return [...running, ...pending, ...finished];
}

function replacePendingTaskOrder(tasks: QueueTask[], pending: QueueTask[]) {
  const running = tasks.filter((task) => task.status === "running");
  const finished = tasks.filter((task) => task.status !== "running" && task.status !== "pending");
  return [...running, ...pending, ...finished];
}

function normalizeStoredQueueTask(value: Partial<QueueTask>, index: number): QueueTask | null {
  if (!value || typeof value !== "object") return null;
  if (!value.flow || !Array.isArray(value.assets)) return null;
  const createdAt = typeof value.createdAt === "number" ? value.createdAt : nowSeconds() + index;
  const status = isQueueTaskStatus(value.status) ? value.status : "pending";
  const normalizedStatus = status === "running" ? "cancelled" : status;
  return {
    id: String(value.id || `task_restored_${createdAt}_${index}`),
    name: String(value.name || value.flow.name || "审核任务"),
    note: String(value.note || ""),
    flow: value.flow,
    assets: value.assets,
    status: normalizedStatus,
    createdAt,
    updatedAt: typeof value.updatedAt === "number" ? value.updatedAt : createdAt,
    startedAt: value.startedAt ?? null,
    finishedAt:
      normalizedStatus === "cancelled" && status === "running"
        ? nowSeconds()
        : value.finishedAt ?? null,
    runId: value.runId ?? null,
    verdict: value.verdict ?? null,
    reportPath: value.reportPath ?? null,
    message:
      normalizedStatus === "cancelled" && status === "running"
        ? "客户端上次关闭时任务仍在运行，已标记为中断。"
        : value.message ?? null,
    nodeStates: value.nodeStates ?? {},
  };
}

function readStoredTaskQueue() {
  const raw = readStoredJson<Partial<QueueTask>[]>(TASK_QUEUE_STORAGE_KEY, []);
  return orderQueueTasks(
    (Array.isArray(raw) ? raw : [])
      .map(normalizeStoredQueueTask)
      .filter((task): task is QueueTask => Boolean(task)),
  );
}

function formatDate(seconds: number) {
  return new Date(seconds * 1000).toLocaleString("zh-CN", { hour12: false });
}

function formatLogTime(seconds: number) {
  return new Date(seconds * 1000).toLocaleTimeString("zh-CN", { hour12: false });
}

function formatDurationMs(durationMs?: number | null) {
  const value = Math.max(0, durationMs ?? 0);
  return value < 1000 ? `${Math.round(value)} ms` : `${(value / 1000).toFixed(2)} s`;
}

function formatBytes(bytes?: number | null) {
  const value = Math.max(0, bytes ?? 0);
  if (value >= 1024 * 1024 * 1024) return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(2)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(2)} KB`;
  return `${Math.round(value)} B`;
}

function formatPercent(value?: number | null) {
  return `${Math.max(0, value ?? 0).toFixed(1)}%`;
}

function formatCpuTimeMs(value?: number | null) {
  const ms = Math.max(0, value ?? 0);
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(2)} s`;
}

function performanceGpuText(step: RunRecord["steps"][number]) {
  const performance = step.performance;
  if (!performance?.gpuAvailable) return "未采集";
  return formatBytes(performance.peakGpuMemoryBytes ?? 0);
}

function runtimeSourceText(source: string) {
  if (source === "program") return "程序同级目录";
  if (source === "data") return "用户数据目录";
  if (source === "override") return "环境变量指定";
  if (source === "preview") return "预览模式";
  return source;
}

function dependencyStateText(dependency: RuntimeDependencyStatus) {
  if (dependency.installed && dependency.version) return `已安装 ${dependency.version}`;
  if (dependency.installed) return "已安装";
  return "未安装";
}

function asObject(value: JsonValue | undefined): JsonObject {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return value as JsonObject;
}

function moduleById(modules: ModuleInfo[], moduleId: string) {
  return modules.find((module) => module.id === moduleId);
}

function moduleSourceLabel(source: ModuleInfo["source"] | string) {
  if (source === "system") return "内置节点";
  if (source === "custom") return "自定义模块";
  return "预置模块";
}

function isFixedSystemKind(kind: string) {
  return kind === "flow_start" || kind === "flow_output";
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
  return kind === ANNOTATION_KIND;
}

function isCanvasGroupKind(kind: string) {
  return kind === CANVAS_GROUP_KIND;
}

function isCanvasNoteKind(kind: string) {
  return kind === CANVAS_NOTE_KIND || isAnnotationKind(kind);
}

function isCanvasToolKind(kind: string) {
  return isCanvasGroupKind(kind) || isCanvasNoteKind(kind);
}

function isPassiveCanvasKind(kind: string) {
  return isPureDataKind(kind) || isCanvasToolKind(kind);
}

function portsForKind(kind: string, dataOutputs: ModuleInfo["dataOutputs"] = []): PortDefinition[] {
  const ports: PortDefinition[] = [];
  if (kind !== "flow_start" && !isPassiveCanvasKind(kind)) {
    ports.push({ id: HANDLE_SEQUENCE_IN, label: "顺序入", kind: "sequence", direction: "in" });
  }
  if (kind !== "flow_output" && !isPassiveCanvasKind(kind)) {
    ports.push({ id: HANDLE_SEQUENCE_OUT, label: "顺序出", kind: "sequence", direction: "out" });
  }

  if (kind === "image_ocr" || kind === "image_safety") {
    ports.push({ id: HANDLE_IMAGES, label: "图片", kind: "data", direction: "in", dataType: DATA_TYPE_IMAGES });
  }
  if (kind === "text_safety") {
    ports.push({ id: HANDLE_TEXTS, label: "文本", kind: "data", direction: "in", dataType: DATA_TYPE_TEXTS });
  }
  if (kind === "folder_processor") {
    ports.push({ id: HANDLE_FOLDER, label: "文件夹", kind: "data", direction: "in", dataType: DATA_TYPE_FOLDER });
  }
  if (kind === "image_ocr") {
    ports.push({ id: HANDLE_TEXTS, label: "文本", kind: "data", direction: "out", dataType: DATA_TYPE_TEXTS });
  }
  if (["data_all_images", "data_artifact_images", "data_relative_images", "data_merge_images"].includes(kind)) {
    ports.push({ id: HANDLE_IMAGES, label: "图片", kind: "data", direction: "out", dataType: DATA_TYPE_IMAGES });
  }
  if (["data_all_texts", "data_artifact_texts", "data_relative_texts", "data_merge_texts"].includes(kind)) {
    ports.push({ id: HANDLE_TEXTS, label: "文本", kind: "data", direction: "out", dataType: DATA_TYPE_TEXTS });
  }
  if (["data_audit_folder", "data_audit_relative_folder", "data_artifact_folder", "data_artifact_relative_folder"].includes(kind)) {
    ports.push({ id: HANDLE_FOLDER, label: "文件夹", kind: "data", direction: "out", dataType: DATA_TYPE_FOLDER });
  }
  for (const output of dataOutputs ?? []) {
    if (!ports.some((port) => port.direction === "out" && port.id === output.handle)) {
      ports.push({
        id: output.handle,
        label: output.name,
        kind: "data",
        direction: "out",
        dataType: output.dataType,
      });
    }
  }
  if (kind === "data_merge_images") {
    ports.push({ id: HANDLE_IMAGES_A, label: "图片 A", kind: "data", direction: "in", dataType: DATA_TYPE_IMAGES });
    ports.push({ id: HANDLE_IMAGES_B, label: "图片 B", kind: "data", direction: "in", dataType: DATA_TYPE_IMAGES });
  }
  if (kind === "data_merge_texts") {
    ports.push({ id: HANDLE_TEXTS_A, label: "文本 A", kind: "data", direction: "in", dataType: DATA_TYPE_TEXTS });
    ports.push({ id: HANDLE_TEXTS_B, label: "文本 B", kind: "data", direction: "in", dataType: DATA_TYPE_TEXTS });
  }
  return ports;
}

function portForHandle(
  kind: string,
  direction: PortDirection,
  handle?: string | null,
  dataOutputs: ModuleInfo["dataOutputs"] = [],
) {
  return portsForKind(kind, dataOutputs).find((port) => port.direction === direction && port.id === handle);
}

function edgeKindForPorts(sourcePort?: PortDefinition, targetPort?: PortDefinition): EdgeKind | null {
  if (!sourcePort || !targetPort) return null;
  if (sourcePort.kind !== targetPort.kind) return null;
  if (sourcePort.kind === "data" && sourcePort.dataType !== targetPort.dataType) return null;
  return sourcePort.kind;
}

function portTypeLabel(port: PortDefinition) {
  if (port.kind === "sequence") {
    return port.direction === "in" ? "开始" : "结束";
  }
  if (port.dataType === DATA_TYPE_IMAGES) return "图片集合";
  if (port.dataType === DATA_TYPE_TEXTS) return "文本集合";
  if (port.dataType === DATA_TYPE_FOLDER) return "文件夹";
  return port.label;
}

function portColorClass(port: PortDefinition) {
  if (port.kind === "sequence") return "sequence";
  if (port.dataType === DATA_TYPE_IMAGES) return "images";
  if (port.dataType === DATA_TYPE_TEXTS) return "texts";
  if (port.dataType === DATA_TYPE_FOLDER) return "folder";
  return "data";
}

function edgeTypeClass(edgeType: EdgeKind) {
  return edgeType === EDGE_TYPE_DATA ? "flow-edge--data" : "flow-edge--sequence";
}

function runStatusLabel(status?: string) {
  if (status === "running") return "运行中";
  if (status === "completed" || status === "system") return "已完成";
  if (status === "skipped") return "已跳过";
  if (status === "cancelled") return "已中断";
  if (status === "error" || status === "failed") return "失败";
  if (status === "needs_model" || status === "invalid_model_path" || status === "ready") return "已记录";
  if (status === "pending") return "等待中";
  return "";
}

function stringListValue(value: JsonValue): string[] {
  if (Array.isArray(value)) {
    return value.map((item) => String(item));
  }
  const text = String(value ?? "").trim();
  return text ? [text] : [];
}

type PolicyListItem = {
  name: string;
  description: string;
};

function policyListValue(value: JsonValue): PolicyListItem[] {
  if (!Array.isArray(value)) return [];
  return value.map((item) => {
    if (!item || typeof item !== "object" || Array.isArray(item)) {
      return { name: String(item ?? ""), description: "" };
    }
    const record = item as Record<string, JsonValue>;
    return {
      name: String(record.name ?? ""),
      description: String(record.description ?? ""),
    };
  });
}

function flowEdgeForConnection(connection: Connection, edgeType: EdgeKind): Edge {
  return {
    ...connection,
    id: `edge_${edgeType}_${connection.source}_${connection.sourceHandle}_${connection.target}_${connection.targetHandle}_${Date.now()}`,
    type: "curved",
    className: edgeTypeClass(edgeType),
    data: { edgeType },
  };
}

function isDataInputOccupied(edges: Edge[], nodeId: string, handleId: string) {
  return edges.some(
    (edge) =>
      ((edge.data?.edgeType as EdgeKind | undefined) ?? EDGE_TYPE_SEQUENCE) === EDGE_TYPE_DATA &&
      edge.target === nodeId &&
      edge.targetHandle === handleId,
  );
}

function hasSameConnection(edges: Edge[], connection: Connection) {
  return edges.some(
    (edge) =>
      edge.source === connection.source &&
      edge.target === connection.target &&
      edge.sourceHandle === connection.sourceHandle &&
      edge.targetHandle === connection.targetHandle,
  );
}

function eventClientPoint(event: MouseEvent | TouchEvent) {
  if ("changedTouches" in event) {
    const touch = event.changedTouches[0];
    return touch ? { x: touch.clientX, y: touch.clientY } : null;
  }
  return { x: event.clientX, y: event.clientY };
}

function CurvedFlowEdge({
  id,
  data,
  sourceX,
  sourceY,
  targetX,
  targetY,
  markerEnd,
  style,
}: EdgeProps) {
  const deltaX = targetX - sourceX;
  const halfDistance = Math.abs(deltaX) * 0.5;
  const horizontalDirection = deltaX >= 0 ? 1 : -1;
  const horizontalReach = Math.min(220, Math.max(36, halfDistance), halfDistance);
  const path = [
    `M ${sourceX},${sourceY}`,
    `C ${sourceX + horizontalReach * horizontalDirection},${sourceY}`,
    `${targetX - horizontalReach * horizontalDirection},${targetY}`,
    `${targetX},${targetY}`,
  ].join(" ");
  const active = Boolean((data as { active?: boolean } | undefined)?.active);

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        style={style}
        interactionWidth={18}
      />
      {active ? (
        <circle className="flow-edge-run-dot" r="4">
          <animateMotion dur="1.15s" repeatCount="indefinite" path={path} />
        </circle>
      ) : null}
    </>
  );
}

function launchTypeLabel(launchType: string) {
  if (launchType === "python") return "Python";
  if (launchType === "http") return "HTTP";
  if (launchType === "exe") return "EXE 传参";
  if (launchType === "system") return "系统内置";
  return "手动接入";
}

function launchDetail(module: ModuleInfo) {
  const launch = module.launch;
  if (launch.launchType === "http") {
    return `${launch.method ?? "POST"} ${launch.url ?? "未配置 URL"}`;
  }
  if (launch.launchType === "exe") {
    const args = launch.args.length > 0 ? ` ${launch.args.join(" ")}` : "";
    return `${launch.command ?? "未配置可执行文件"}${args}`;
  }
  if (launch.launchType === "python") {
    const args = launch.args.length > 0 ? ` ${launch.args.join(" ")}` : "";
    return `${launch.command ?? "未配置脚本"}${args}`;
  }
  return launch.notes || "无需启动外部模块";
}

function fallbackModule(moduleId: string): ModuleInfo {
  return {
    id: moduleId,
    name: "未知模块",
    kind: "unknown",
    summary: "模块未注册",
    modelLabel: "本地运行目录",
    icon: "file-check",
    builtIn: false,
    source: "custom",
    definitionDir: "",
    modelPath: null,
    modelConfigured: false,
    launch: {
      launchType: "manual",
      command: null,
      url: null,
      method: null,
      args: [],
      notes: "模块未注册，暂无启动方式。",
    },
    parameters: [],
    dataOutputs: [],
  };
}

function configForNode(module: ModuleInfo, config: JsonValue): JsonObject {
  return {
    ...defaultConfigForModule(module),
    ...asObject(config),
  };
}

function dimensionNumber(value: unknown, fallback: number) {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string") {
    const parsed = Number.parseFloat(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return fallback;
}

function annotationTone(value: JsonValue | undefined) {
  const tone = String(value ?? "blue");
  return ["blue", "green", "yellow", "red", "gray"].includes(tone) ? tone : "blue";
}

function canvasToolDefaults(kind: string) {
  if (isCanvasGroupKind(kind)) {
    return {
      title: "分组",
      width: CANVAS_GROUP_DEFAULT_WIDTH,
      height: CANVAS_GROUP_DEFAULT_HEIGHT,
      minWidth: 220,
      minHeight: 140,
    };
  }
  return {
    title: "注释",
    width: CANVAS_NOTE_DEFAULT_WIDTH,
    height: CANVAS_NOTE_DEFAULT_HEIGHT,
    minWidth: 200,
    minHeight: 130,
  };
}

function canvasToolConfig(config: JsonObject, kind: string, fallbackLabel?: string) {
  const defaults = canvasToolDefaults(kind);
  const title = String(config.title ?? fallbackLabel ?? defaults.title).trim() || defaults.title;
  const text = String(config.text ?? "");
  const color = annotationTone(config.color);
  const width = Math.max(defaults.minWidth, dimensionNumber(config.width, defaults.width));
  const height = Math.max(defaults.minHeight, dimensionNumber(config.height, defaults.height));
  return {
    ...config,
    title,
    text,
    color,
    width,
    height,
  };
}

function nodeDimension(node: AuditNode, key: "width" | "height", fallback: number) {
  const nodeWithMeasured = node as AuditNode & { measured?: { width?: number; height?: number } };
  return dimensionNumber(
    node[key] ??
      nodeWithMeasured.measured?.[key] ??
      (node.style as Record<string, unknown> | undefined)?.[key] ??
      node.data.config?.[key],
    fallback,
  );
}

function toReactNodes(flow: FlowDefinition, modules: ModuleInfo[]): AuditNode[] {
  return flow.nodes.map((node) => {
    const module = moduleById(modules, node.moduleId) ?? fallbackModule(node.moduleId);
    const config = configForNode(module, node.config);
    if (isCanvasToolKind(module.kind)) {
      const nextConfig = canvasToolConfig(config, module.kind, node.label);
      const isGroup = isCanvasGroupKind(module.kind);
      return {
        id: node.id,
        type: isGroup ? "canvasGroup" : "canvasNote",
        position: node.position,
        deletable: true,
        zIndex: isGroup ? 0 : 8,
        style: {
          width: dimensionNumber(nextConfig.width, canvasToolDefaults(module.kind).width),
          height: dimensionNumber(nextConfig.height, canvasToolDefaults(module.kind).height),
        },
        data: {
          label: String(nextConfig.title),
          moduleId: node.moduleId,
          moduleName: module.name,
          moduleKind: module.kind,
          moduleIcon: module.icon,
          moduleIconPath: module.iconPath ?? null,
          moduleIconDataUrl: module.iconDataUrl ?? null,
          dataOutputs: module.dataOutputs ?? [],
          source: module.source,
          config: nextConfig,
          requiresModelPath: false,
        },
      };
    }
    return {
      id: node.id,
      type: "audit",
      position: node.position,
      deletable: !isFixedSystemKind(module.kind),
      zIndex: 10,
      data: {
        label: node.label,
        moduleId: node.moduleId,
        moduleName: module.name,
        moduleKind: module.kind,
        moduleIcon: module.icon,
        moduleIconPath: module.iconPath ?? null,
        moduleIconDataUrl: module.iconDataUrl ?? null,
        dataOutputs: module.dataOutputs ?? [],
        source: module.source,
        config,
        requiresModelPath: module.parameters.some((parameter) => parameter.key === "modelPath"),
      },
    };
  });
}

function initialRunNodeStates(flow: FlowDefinition, modules: ModuleInfo[]): Record<string, RunNodeState> {
  return Object.fromEntries(
    toReactNodes(flow, modules)
      .filter((node) => !isPassiveCanvasKind(String(node.data.moduleKind)))
      .map((node) => [
        node.id,
        {
          status: "pending",
          progress: 0,
          message: "",
        },
      ]),
  );
}

function nodeStatesFromRun(run: RunRecord): Record<string, RunNodeState> {
  return Object.fromEntries(
    run.steps.map((step) => [
      step.stepId,
      {
        status: step.status,
        progress: step.progress ?? 1,
        message: step.message,
        processed: step.processedFiles ?? null,
        total: null,
      },
    ]),
  );
}

function toReactEdges(flow: FlowDefinition): Edge[] {
  return flow.edges.map((edge) => ({
    id: edge.id,
    source: edge.from,
    target: edge.to,
    sourceHandle: edge.fromHandle ?? (edge.edgeType === EDGE_TYPE_DATA ? null : HANDLE_SEQUENCE_OUT),
    targetHandle: edge.toHandle ?? (edge.edgeType === EDGE_TYPE_DATA ? null : HANDLE_SEQUENCE_IN),
    type: "curved",
    className: edgeTypeClass(edge.edgeType ?? EDGE_TYPE_SEQUENCE),
    data: { edgeType: edge.edgeType ?? EDGE_TYPE_SEQUENCE },
  }));
}

function buildFlow(nodes: AuditNode[], edges: Edge[]): FlowDefinition {
  return {
    id: "flow.default.image-audit",
    name: "图片 UGC 默认审核",
    version: 1,
    nodes: nodes.map((node) => {
      const moduleKind = String(node.data.moduleKind);
      const config = isCanvasToolKind(moduleKind)
        ? canvasToolConfig(
            {
              ...node.data.config,
              width: nodeDimension(node, "width", canvasToolDefaults(moduleKind).width),
              height: nodeDimension(node, "height", canvasToolDefaults(moduleKind).height),
            },
            moduleKind,
            String(node.data.label),
          )
        : node.data.config;
      return {
        id: node.id,
        moduleId: String(node.data.moduleId),
        label: isCanvasToolKind(moduleKind) ? String(config.title ?? node.data.label) : String(node.data.label),
        position: node.position,
        config,
      };
    }),
    edges: edges.map((edge) => ({
      id: edge.id,
      from: edge.source,
      to: edge.target,
      edgeType: ((edge.data?.edgeType as EdgeKind | undefined) ?? EDGE_TYPE_SEQUENCE),
      fromHandle: edge.sourceHandle ?? null,
      toHandle: edge.targetHandle ?? null,
    })),
  };
}

function cloneJsonObject(value: JsonObject): JsonObject {
  return JSON.parse(JSON.stringify(value)) as JsonObject;
}

function cloneAuditNode(node: AuditNode): AuditNode {
  return {
    ...node,
    position: { ...node.position },
    style: node.style ? { ...node.style } : undefined,
    data: {
      ...node.data,
      config: cloneJsonObject(node.data.config ?? {}),
    },
  };
}

function cloneAuditEdge(edge: Edge): Edge {
  return {
    ...edge,
    data: edge.data ? { ...edge.data } : undefined,
  };
}

function cloneCanvasSnapshot(snapshot: CanvasSnapshot): CanvasSnapshot {
  return {
    nodes: snapshot.nodes.map(cloneAuditNode),
    edges: snapshot.edges.map(cloneAuditEdge),
    selectedNodeId: snapshot.selectedNodeId,
  };
}

function canvasSnapshot(nodes: AuditNode[], edges: Edge[], selectedNodeId: string | null): CanvasSnapshot {
  return {
    nodes: nodes.map(cloneAuditNode),
    edges: edges.map(cloneAuditEdge),
    selectedNodeId,
  };
}

function canvasSnapshotKey(snapshot: CanvasSnapshot) {
  return JSON.stringify(buildFlow(snapshot.nodes, snapshot.edges));
}

function historyTimeLabel(timestamp: number) {
  const date = new Date(timestamp);
  return date.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

const FLOW_LAYOUT_ORIGIN_X = 120;
const FLOW_LAYOUT_ORIGIN_Y = 120;
const FLOW_LAYOUT_COLUMN_GAP = 300;
const FLOW_LAYOUT_ROW_GAP = 150;

function compareAuditNodes(left: AuditNode, right: AuditNode) {
  return (
    left.position.y - right.position.y ||
    left.position.x - right.position.x ||
    String(left.data.label).localeCompare(String(right.data.label)) ||
    left.id.localeCompare(right.id)
  );
}

function layoutAuditNodes(nodes: AuditNode[], edges: Edge[]): AuditNode[] {
  if (nodes.length < 2) return nodes;

  const layoutNodes = nodes.filter((node) => !isCanvasToolKind(String(node.data.moduleKind)));
  if (layoutNodes.length < 2) return nodes;

  const byId = new Map(layoutNodes.map((node) => [node.id, node]));
  const outgoing = new Map<string, string[]>();
  const indegree = new Map(layoutNodes.map((node) => [node.id, 0]));
  const layerById = new Map(layoutNodes.map((node) => [node.id, 0]));
  const validEdges = edges.filter(
    (edge) =>
      ((edge.data?.edgeType as EdgeKind | undefined) ?? EDGE_TYPE_SEQUENCE) === EDGE_TYPE_SEQUENCE &&
      byId.has(edge.source) &&
      byId.has(edge.target) &&
      edge.source !== edge.target,
  );

  for (const edge of validEdges) {
    outgoing.set(edge.source, [...(outgoing.get(edge.source) ?? []), edge.target]);
    indegree.set(edge.target, (indegree.get(edge.target) ?? 0) + 1);
  }

  const sortIds = (ids: string[]) =>
    ids.sort((left, right) => compareAuditNodes(byId.get(left)!, byId.get(right)!));
  const ready = sortIds(
    layoutNodes
      .filter((node) => (indegree.get(node.id) ?? 0) === 0)
      .map((node) => node.id),
  );
  const visited = new Set<string>();

  while (ready.length > 0) {
    const id = ready.shift()!;
    if (visited.has(id)) continue;
    visited.add(id);

    for (const child of outgoing.get(id) ?? []) {
      layerById.set(child, Math.max(layerById.get(child) ?? 0, (layerById.get(id) ?? 0) + 1));
      indegree.set(child, Math.max(0, (indegree.get(child) ?? 0) - 1));
      if ((indegree.get(child) ?? 0) === 0) {
        ready.push(child);
      }
    }
    sortIds(ready);
  }

  const remaining = sortIds(layoutNodes.filter((node) => !visited.has(node.id)).map((node) => node.id));
  for (const id of remaining) {
    visited.add(id);
    const incomingLayers = validEdges
      .filter((edge) => edge.target === id)
      .map((edge) => (layerById.get(edge.source) ?? 0) + 1);
    layerById.set(id, Math.max(layerById.get(id) ?? 0, 0, ...incomingLayers));
  }

  const groups = new Map<number, AuditNode[]>();
  for (const node of layoutNodes) {
    const layer = layerById.get(node.id) ?? 0;
    groups.set(layer, [...(groups.get(layer) ?? []), node]);
  }

  const positions = new Map<string, { x: number; y: number }>();
  for (const layer of Array.from(groups.keys()).sort((left, right) => left - right)) {
    const group = groups.get(layer)!.sort(compareAuditNodes);
    group.forEach((node, index) => {
      positions.set(node.id, {
        x: FLOW_LAYOUT_ORIGIN_X + layer * FLOW_LAYOUT_COLUMN_GAP,
        y: FLOW_LAYOUT_ORIGIN_Y + index * FLOW_LAYOUT_ROW_GAP,
      });
    });
  }

  return nodes.map((node) => ({
    ...node,
    position: positions.get(node.id) ?? node.position,
  }));
}

function modelConfigured(config: JsonObject) {
  return typeof config.modelPath === "string" && config.modelPath.trim().length > 0;
}

function isFixedSystemNode(node: AuditNode | null | undefined) {
  return Boolean(node && isFixedSystemKind(String(node.data.moduleKind)));
}

function reportAssetSrc(src?: string | null) {
  if (!src) return "";
  const trimmed = src.trim();
  if (!trimmed || /^javascript:/i.test(trimmed)) return "";
  if (/^(data:|https?:|asset:|file:|blob:)/i.test(trimmed)) return trimmed;
  if (/^[a-zA-Z]:[\\/]/.test(trimmed) || /^\\\\/.test(trimmed)) {
    return window.__TAURI_INTERNALS__ ? convertFileSrc(trimmed) : trimmed;
  }
  return trimmed;
}

function isLocalReportPath(url: string) {
  return /^[a-zA-Z]:[\\/]/.test(url) || /^\\\\/.test(url);
}

function reportRevealUrl(path: string) {
  return `ugcaudit://reveal?path=${encodeURIComponent(path)}`;
}

function reportRevealPath(href?: string | null) {
  if (!href) return null;
  const trimmed = href.trim();
  if (!trimmed) return null;
  if (isLocalReportPath(trimmed)) return trimmed;
  try {
    const url = new URL(trimmed);
    if (url.protocol !== "ugcaudit:" || url.hostname !== "reveal") return null;
    return url.searchParams.get("path");
  } catch {
    return null;
  }
}

const reportUrlTransform: UrlTransform = (url, key, node) => {
  if (key === "src" && node.tagName === "img") {
    return reportAssetSrc(url);
  }
  if (key === "href" && node.tagName === "a") {
    const trimmed = url.trim();
    if (/^ugcaudit:\/\/reveal/i.test(trimmed) && reportRevealPath(trimmed)) return trimmed;
    if (isLocalReportPath(trimmed)) return reportRevealUrl(trimmed);
  }
  return defaultUrlTransform(url);
};

function MermaidDiagram({ chart }: { chart: string }) {
  const renderId = useRef(`mermaid-${Math.random().toString(36).slice(2)}`);
  const [svg, setSvg] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    const source = chart.trim();

    setSvg("");
    setError("");

    if (!source) {
      setError("图示内容为空。");
      return;
    }

    const render = async () => {
      try {
        const result = await mermaid.render(
          `${renderId.current}-${Date.now().toString(36)}`,
          source,
        );
        if (!cancelled) {
          setSvg(result.svg);
        }
      } catch (renderError) {
        if (!cancelled) {
          setError(renderError instanceof Error ? renderError.message : String(renderError));
        }
      }
    };

    void render();

    return () => {
      cancelled = true;
    };
  }, [chart]);

  if (error) {
    return (
      <div className="mermaid-diagram mermaid-diagram--error">
        <strong>图示无法渲染</strong>
        <pre>{chart}</pre>
      </div>
    );
  }

  if (!svg) {
    return <div className="mermaid-diagram mermaid-diagram--loading">图示渲染中...</div>;
  }

  return (
    <div
      className="mermaid-diagram"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

function markdownCodeText(children: ReactNode) {
  return Children.toArray(children)
    .map((child) => {
      if (typeof child === "string" || typeof child === "number") return String(child);
      return "";
    })
    .join("")
    .replace(/\n$/, "");
}

function markdownComponentsForRun(runId?: string | null): Components {
  return {
  a({ node: _node, href, children, ...props }) {
    const revealPath = reportRevealPath(href);
    if (revealPath) {
      return (
        <a
          href={href}
          {...props}
          onClick={(event) => {
            event.preventDefault();
            void revealReportTarget(revealPath, runId).catch((error) => {
              window.alert(error instanceof Error ? error.message : String(error));
            });
          }}
        >
          {children}
        </a>
      );
    }
    return (
      <a href={href} rel="noreferrer" target="_blank" {...props}>
        {children}
      </a>
    );
  },
  img({ node: _node, src, alt, ...props }) {
    return (
      <img
        {...props}
        alt={alt ?? ""}
        loading="lazy"
        src={reportAssetSrc(src)}
      />
    );
  },
  pre({ node: _node, children, ...props }) {
    const child = Children.toArray(children)[0];
    if (isValidElement(child)) {
      const codeProps = child.props as { className?: string; children?: ReactNode };
      const language = codeProps.className?.match(/language-([^\s]+)/)?.[1]?.toLowerCase();
      if (language === "mermaid") {
        return <MermaidDiagram chart={markdownCodeText(codeProps.children)} />;
      }
    }
    return <pre {...props}>{children}</pre>;
  },
  table({ node: _node, ...props }) {
    return (
      <div className="markdown-table-wrap">
        <table {...props} />
      </div>
    );
  },
  };
}

function MarkdownViewer({ markdown, runId }: { markdown: string; runId?: string | null }) {
  if (!markdown.trim()) {
    return <div className="markdown-empty">暂无报告</div>;
  }

  return (
    <article className="markdown-viewer">
      <ReactMarkdown
        components={markdownComponentsForRun(runId)}
        rehypePlugins={[
          rehypeRaw,
          [rehypeSanitize, markdownSanitizeSchema],
        ]}
        remarkPlugins={[remarkGfm]}
        urlTransform={reportUrlTransform}
      >
        {markdown}
      </ReactMarkdown>
    </article>
  );
}

function AuditNodeCard({ data, selected }: NodeProps<AuditNode>) {
  const isSystem = data.source === "system";
  const ports = portsForKind(data.moduleKind, data.dataOutputs);
  const inputPorts = ports.filter((port) => port.direction === "in");
  const outputPorts = ports.filter((port) => port.direction === "out");
  const runStatus = typeof data.runStatus === "string" ? data.runStatus : "";
  const runProgress = typeof data.runProgress === "number" ? Math.max(0, Math.min(1, data.runProgress)) : 0;
  const runMessage = typeof data.runMessage === "string" ? data.runMessage : "";
  const hasRunState = Boolean(runStatus);

  const renderPortRow = (port: PortDefinition) => {
    const isInput = port.direction === "in";
    const rowSide = isInput ? "input" : "output";
    const handle = (
      <Handle
        className={`node-handle node-handle--${portColorClass(port)}`}
        id={port.id}
        position={isInput ? HandlePosition.Left : HandlePosition.Right}
        type={isInput ? "target" : "source"}
      />
    );

    return (
      <div className={`node-port-row node-port-row--${rowSide}`} key={`${rowSide}-${port.id}`}>
        {isInput ? handle : null}
        <span className={`node-port node-port--${portColorClass(port)}`}>
          {portTypeLabel(port)}
        </span>
        {isInput ? null : handle}
      </div>
    );
  };

  return (
    <div className={`audit-node ${isSystem ? "audit-node--system" : ""} ${hasRunState ? `audit-node--run audit-node--${runStatus}` : ""} ${selected ? "is-selected" : ""}`}>
      <div className="audit-node__header">
        <span className="audit-node__icon" aria-hidden="true">
          <ModuleIcon
            icon={data.moduleIcon}
            iconPath={data.moduleIconPath}
            iconDataUrl={data.moduleIconDataUrl}
            size={18}
          />
        </span>
        <span className="audit-node__title">{data.label}</span>
      </div>
      {hasRunState ? (
        <div className="audit-node__run">
          <div className="audit-node__run-line">
            <span>{runStatusLabel(runStatus)}</span>
            <strong>{Math.round(runProgress * 100)}%</strong>
          </div>
          <div className="audit-node__progress" aria-hidden="true">
            <span style={{ width: `${runProgress * 100}%` }} />
          </div>
          {runMessage ? <small>{runMessage}</small> : null}
        </div>
      ) : null}
      {inputPorts.length > 0 || outputPorts.length > 0 ? (
        <div className="node-port-columns" aria-hidden="true">
          <div className="node-port-column node-port-column--input">
            {inputPorts.map(renderPortRow)}
          </div>
          <div className="node-port-column node-port-column--output">
            {outputPorts.map(renderPortRow)}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function CanvasToolNodeCard({ data, selected }: NodeProps<AuditNode>) {
  const kind = String(data.moduleKind);
  const isGroup = isCanvasGroupKind(kind);
  const config = canvasToolConfig(data.config ?? {}, kind, String(data.label));
  const tone = annotationTone(config.color);
  const title = String(config.title ?? data.label);
  const text = String(config.text ?? "").trim();
  const defaults = canvasToolDefaults(kind);

  return (
    <div className={`canvas-tool-node canvas-tool-node--${isGroup ? "group" : "note"} canvas-tool-node--${tone} ${selected ? "is-selected" : ""}`}>
      <NodeResizer
        isVisible={Boolean(selected)}
        minWidth={defaults.minWidth}
        minHeight={defaults.minHeight}
        lineClassName="canvas-tool-node__resize-line"
        handleClassName="canvas-tool-node__resize-handle"
      />
      {isGroup ? (
        <>
          <div className="canvas-group-title">{title}</div>
          <div className="canvas-group-frame" />
        </>
      ) : (
        <div className="canvas-note">
          <div className="canvas-note__header">
            <StickyNote size={16} />
            <strong>{title}</strong>
          </div>
          {text ? (
            <div className="canvas-note__text">{text}</div>
          ) : (
            <div className="canvas-note__placeholder">注释内容</div>
          )}
        </div>
      )}
    </div>
  );
}

function ParameterField({
  parameter,
  value,
  onChange,
}: {
  parameter: ModuleParameter;
  value: JsonValue;
  onChange: (value: JsonValue) => void;
}) {
  const id = `param-${parameter.key}`;

  if (parameter.parameterType === "boolean") {
    return (
      <label className="param-field param-field--check" htmlFor={id}>
        <input
          id={id}
          type="checkbox"
          checked={Boolean(value)}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span>
          <strong>{parameter.name}</strong>
          <small>{parameter.description}</small>
        </span>
      </label>
    );
  }

  if (parameter.parameterType === "select") {
    return (
      <label className="param-field" htmlFor={id}>
        <span>
          {parameter.name}
          {parameter.required ? <b>必填</b> : null}
        </span>
        <small>{parameter.description}</small>
        <select
          id={id}
          value={String(value ?? "")}
          onChange={(event) => onChange(event.target.value)}
        >
          {parameter.options.map((item) => (
            <option key={item.value} value={item.value}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
    );
  }

  if (parameter.parameterType === "multiSelect") {
    const selected = Array.isArray(value) ? value.map(String) : [];
    return (
      <fieldset className="param-field param-fieldset">
        <legend>
          {parameter.name}
          {parameter.required ? <b>必填</b> : null}
        </legend>
        <small>{parameter.description}</small>
        <div className="option-grid">
          {parameter.options.map((item) => (
            <label key={item.value}>
              <input
                type="checkbox"
                checked={selected.includes(item.value)}
                onChange={(event) => {
                  const next = event.target.checked
                    ? [...selected, item.value]
                    : selected.filter((current) => current !== item.value);
                  onChange(next);
                }}
              />
              <span>{item.label}</span>
            </label>
          ))}
        </div>
      </fieldset>
    );
  }

  if (parameter.parameterType === "textarea") {
    return (
      <label className="param-field" htmlFor={id}>
        <span>
          {parameter.name}
          {parameter.required ? <b>必填</b> : null}
        </span>
        <small>{parameter.description}</small>
        <textarea
          id={id}
          value={String(value ?? "")}
          onChange={(event) => onChange(event.target.value)}
          spellCheck={false}
        />
      </label>
    );
  }

  if (parameter.parameterType === "stringList") {
    const values = stringListValue(value);
    const nextValues = values.length > 0 ? values : [""];
    return (
      <fieldset className="param-field param-fieldset param-string-list">
        <legend>
          {parameter.name}
          {parameter.required ? <b>必填</b> : null}
        </legend>
        <small>{parameter.description}</small>
        <div className="param-string-list__rows">
          {nextValues.map((item, index) => (
            <div className="param-string-list__row" key={`${parameter.key}-${index}`}>
              <input
                aria-label={`${parameter.name} ${index + 1}`}
                value={item}
                onChange={(event) => {
                  const next = [...nextValues];
                  next[index] = event.target.value;
                  onChange(next);
                }}
              />
              <button
                type="button"
                className="ghost-button"
                onClick={() => {
                  const next = nextValues.filter((_, currentIndex) => currentIndex !== index);
                  onChange(next);
                }}
                disabled={nextValues.length <= 1 && !item.trim()}
              >
                删除
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          className="secondary param-string-list__add"
          onClick={() => onChange([...nextValues, ""])}
        >
          添加策略
        </button>
      </fieldset>
    );
  }

  if (parameter.parameterType === "policyList") {
    const values = policyListValue(value);
    const rows = values.length > 0 ? values : [{ name: "", description: "" }];
    return (
      <fieldset className="param-field param-fieldset param-policy-list">
        <legend>
          {parameter.name}
          {parameter.required ? <b>必填</b> : null}
        </legend>
        <small>{parameter.description}</small>
        <div className="param-policy-list__rows">
          {rows.map((item, index) => (
            <div className="param-policy-list__row" key={`${parameter.key}-${index}`}>
              <input
                aria-label={`${parameter.name} 名称 ${index + 1}`}
                value={item.name}
                placeholder="策略名称"
                onChange={(event) => {
                  const next = [...rows];
                  next[index] = { ...next[index], name: event.target.value };
                  onChange(next);
                }}
              />
              <textarea
                aria-label={`${parameter.name} 描述 ${index + 1}`}
                value={item.description}
                placeholder="策略描述"
                onChange={(event) => {
                  const next = [...rows];
                  next[index] = { ...next[index], description: event.target.value };
                  onChange(next);
                }}
                spellCheck={false}
              />
              <button
                type="button"
                className="ghost-button"
                onClick={() => onChange(rows.filter((_, currentIndex) => currentIndex !== index))}
                disabled={rows.length <= 1 && !item.name.trim() && !item.description.trim()}
              >
                删除
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          className="secondary param-policy-list__add"
          onClick={() => onChange([...rows, { name: "", description: "" }])}
        >
          添加自定义策略
        </button>
      </fieldset>
    );
  }

  return (
    <label className="param-field" htmlFor={id}>
      <span>
        {parameter.name}
        {parameter.required ? <b>必填</b> : null}
      </span>
      <small>{parameter.description}</small>
      <input
        id={id}
        type={parameter.parameterType === "number" ? "number" : "text"}
        value={String(value ?? "")}
        onChange={(event) =>
          onChange(
            parameter.parameterType === "number"
              ? Number(event.target.value)
              : event.target.value,
          )
        }
        placeholder={parameter.parameterType === "path" ? "本地路径" : undefined}
      />
    </label>
  );
}

export default function App() {
  const { fitView, screenToFlowPosition } = useReactFlow();
  const flowPanelRef = useRef<HTMLElement | null>(null);
  const pendingConnectionOpenedAtRef = useRef(0);
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [nodes, setNodes, onNodesChange] = useNodesState<AuditNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [validation, setValidation] = useState<ValidationResult>({ valid: true, messages: [] });
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [currentRun, setCurrentRun] = useState<RunRecord | null>(null);
  const [report, setReport] = useState("");
  const [dataRoot, setDataRoot] = useState("");
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  const [artifactRootDraft, setArtifactRootDraft] = useState("");
  const [dependencyRootDraft, setDependencyRootDraft] = useState("");
  const [inputNote, setInputNote] = useState("");
  const [selectedAssets, setSelectedAssets] = useState<AuditAsset[]>([]);
  const [schemeName, setSchemeName] = useState("图片 UGC 默认审核");
  const [schemePath, setSchemePath] = useState<string | null>(null);
  const [schemeDirty, setSchemeDirty] = useState(false);
  const [schemeLibraryDir, setSchemeLibraryDir] = useState<string | null>(null);
  const [schemeLibraryItems, setSchemeLibraryItems] = useState<SchemeListItem[]>([]);
  const [schemeMenuOpen, setSchemeMenuOpen] = useState(false);
  const schemeDropdownRef = useRef<HTMLDivElement | null>(null);
  const suppressSchemeDirtyRef = useRef(false);
  const [runDialogOpen, setRunDialogOpen] = useState(false);
  const [runDialogSchemePath, setRunDialogSchemePath] = useState(CURRENT_SCHEME_VALUE);
  const [runTaskName, setRunTaskName] = useState("");
  const [runTaskNote, setRunTaskNote] = useState("");
  const [runDialogAssets, setRunDialogAssets] = useState<AuditAsset[]>([]);
  const [runDialogError, setRunDialogError] = useState("");
  const [runStarting, setRunStarting] = useState(false);
  const [configText, setConfigText] = useState("{}");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [activeTab, setActiveTab] = useState<AppTab>("flow");
  const [activeLibraryTab, setActiveLibraryTab] = useState<LibraryTab>("modules");
  const [activeSettingsTab, setActiveSettingsTab] = useState<SettingsTab>("runtime");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [runtimeLogs, setRuntimeLogs] = useState<RuntimeLogLine[]>([]);
  const [runtimeBusyDependency, setRuntimeBusyDependency] = useState<string | null>(null);
  const [runtimeTerminalCollapsed, setRuntimeTerminalCollapsed] = useState(true);
  const [moduleDragPreview, setModuleDragPreview] = useState<ModuleDragPreview | null>(null);
  const [pendingConnectionMenu, setPendingConnectionMenu] = useState<PendingConnectionMenu | null>(null);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const activeRunIdRef = useRef<string | null>(null);
  const [activeQueueTaskId, setActiveQueueTaskId] = useState<string | null>(null);
  const activeQueueTaskIdRef = useRef<string | null>(null);
  const [taskQueue, setTaskQueue] = useState<QueueTask[]>(readStoredTaskQueue);
  const taskQueueRef = useRef<QueueTask[]>([]);
  const queueProcessingRef = useRef(false);
  const draggedQueueTaskIdRef = useRef<string | null>(null);
  const [selectedQueueTaskId, setSelectedQueueTaskId] = useState<string | null>(null);
  const [dragOverQueueTaskId, setDragOverQueueTaskId] = useState<string | null>(null);
  const [entryUnreadRunIds, setEntryUnreadRunIds] = useState<string[]>(() => readStoredStringList(ENTRY_UNREAD_RUNS_STORAGE_KEY));
  const [historyUnreadRunIds, setHistoryUnreadRunIds] = useState<string[]>(() => readStoredStringList(HISTORY_UNREAD_RUNS_STORAGE_KEY));
  const [completionToasts, setCompletionToasts] = useState<CompletionToast[]>([]);
  const [runInProgress, setRunInProgress] = useState(false);
  const [runCancelling, setRunCancelling] = useState(false);
  const [runOverlayOpen, setRunOverlayOpen] = useState(false);
  const runOverlayOpenRef = useRef(false);
  const activeTabRef = useRef<AppTab>("flow");
  const [runBubblePosition, setRunBubblePosition] = useState({ x: 18, y: 140 });
  const runBubbleDragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    originX: number;
    originY: number;
    moved: boolean;
  } | null>(null);
  const runBubbleSuppressClickRef = useRef(false);
  const dependencyRootHasUnsavedChanges = Boolean(
    appSettings &&
      dependencyRootDraft.trim() &&
      dependencyRootDraft.trim() !== appSettings.dependencyRoot,
  );
  const [highlightRunId, setHighlightRunId] = useState<string | null>(null);
  const [historyEntries, setHistoryEntries] = useState<CanvasHistoryEntry[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [historyOpen, setHistoryOpen] = useState(false);
  const nodesRef = useRef<AuditNode[]>([]);
  const edgesRef = useRef<Edge[]>([]);
  const selectedNodeIdRef = useRef<string | null>(null);
  const historyIndexRef = useRef(-1);
  const historyEntriesRef = useRef<CanvasHistoryEntry[]>([]);
  const historyApplyingRef = useRef(false);
  const pendingHistoryRef = useRef<{ label: string; timer: number | null } | null>(null);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId],
  );
  const selectedModule = selectedNode
    ? moduleById(modules, String(selectedNode.data.moduleId))
    : null;
  const selectedCanvasToolNode = selectedNode && isCanvasToolKind(String(selectedNode.data.moduleKind))
    ? selectedNode
    : null;
  const selectedCanvasToolConfig = selectedCanvasToolNode
    ? canvasToolConfig(
        selectedCanvasToolNode.data.config ?? {},
        String(selectedCanvasToolNode.data.moduleKind),
        String(selectedCanvasToolNode.data.label),
      )
    : null;
  const moduleLibraryItems = useMemo(
    () => modules.filter((module) => !isFixedSystemKind(module.kind) && !isPureDataKind(module.kind) && !isCanvasToolKind(module.kind)),
    [modules],
  );
  const dataNodeLibraryItems = useMemo(
    () => modules.filter((module) => isPureDataKind(module.kind)),
    [modules],
  );
  const dataNodeGroups = useMemo<DataNodeGroup[]>(() => {
    const groupForKind = (kind: string) => {
      if (kind === "data_artifact_images" || kind === "data_artifact_texts" || kind === "data_artifact_folder" || kind === "data_artifact_relative_folder") return "artifact";
      if (kind === "data_merge_images" || kind === "data_merge_texts") return "tools";
      return "project";
    };
    const titleByGroup: Record<string, string> = {
      project: "待测项目",
      artifact: "产物文件夹",
      tools: "数据处理",
    };
    const groups = new Map<string, ModuleInfo[]>();
    for (const module of dataNodeLibraryItems) {
      const groupId = groupForKind(module.kind);
      groups.set(groupId, [...(groups.get(groupId) ?? []), module]);
    }
    return ["project", "artifact", "tools"]
      .map((id) => ({
        id,
        title: titleByGroup[id],
        items: groups.get(id) ?? [],
      }))
      .filter((group) => group.items.length > 0);
  }, [dataNodeLibraryItems]);
  const activeLibraryItems = activeLibraryTab === "modules" ? moduleLibraryItems : activeLibraryTab === "data" ? dataNodeLibraryItems : [];
  const customModules = useMemo(
    () => modules.filter((module) => module.source === "custom"),
    [modules],
  );
  const historyUnreadRunIdSet = useMemo(
    () => new Set(historyUnreadRunIds),
    [historyUnreadRunIds],
  );
  const visibleQueueTasks = useMemo(
    () =>
      taskQueue.filter(
        (task) =>
          !isTerminalQueueStatus(task.status) ||
          !task.runId ||
          historyUnreadRunIdSet.has(task.runId),
      ),
    [historyUnreadRunIdSet, taskQueue],
  );
  const selectedQueueTask = useMemo(() => {
    if (visibleQueueTasks.length === 0) return null;
    return (
      visibleQueueTasks.find((task) => task.id === selectedQueueTaskId) ??
      visibleQueueTasks.find((task) => task.id === activeQueueTaskId) ??
      visibleQueueTasks[0]
    );
  }, [activeQueueTaskId, selectedQueueTaskId, visibleQueueTasks]);
  const selectedQueueTaskNodeStates: Record<string, RunNodeState> = selectedQueueTask?.status === "pending"
    ? {}
    : selectedQueueTask?.nodeStates ?? {};
  const displayRunNodes = useMemo(
    () =>
      selectedQueueTask
        ? toReactNodes(selectedQueueTask.flow, modules).map((node) => {
        if (isPassiveCanvasKind(String(node.data.moduleKind))) {
          return {
            ...node,
            draggable: false,
            selectable: true,
            deletable: false,
            data: {
              ...node.data,
              runStatus: undefined,
              runProgress: undefined,
              runMessage: undefined,
              runProcessed: undefined,
              runTotal: undefined,
            },
          };
        }
        const state = selectedQueueTaskNodeStates[node.id];
        return {
          ...node,
          draggable: false,
          selectable: true,
          deletable: false,
          data: {
            ...node.data,
            runStatus: state?.status ?? "pending",
            runProgress: state?.progress ?? 0,
            runMessage: state?.message ?? "",
            runProcessed: state?.processed ?? null,
            runTotal: state?.total ?? null,
          },
        };
      })
        : [],
    [modules, selectedQueueTask, selectedQueueTaskNodeStates],
  );
  const displayRunEdges = useMemo(
    () =>
      selectedQueueTask
        ? toReactEdges(selectedQueueTask.flow).map((edge) => {
        const sourceState = selectedQueueTaskNodeStates[edge.source];
        const targetState = selectedQueueTaskNodeStates[edge.target];
        const active =
          (selectedQueueTask.status === "running" && sourceState?.status === "running") ||
          (selectedQueueTask.status === "running" && targetState?.status === "running") ||
          (selectedQueueTask.status === "running" &&
            sourceState &&
            ["completed", "system", "skipped"].includes(sourceState.status) &&
            (!targetState || targetState.status === "pending"));
        return {
          ...edge,
          deletable: false,
          selectable: false,
          data: {
            ...edge.data,
            active,
          },
        };
      })
        : [],
    [selectedQueueTask, selectedQueueTaskNodeStates],
  );
  const entryUnreadCount = entryUnreadRunIds.length;
  const activeQueueTask = useMemo(
    () => taskQueue.find((task) => task.id === activeQueueTaskId) ?? null,
    [activeQueueTaskId, taskQueue],
  );

  useEffect(() => {
    nodesRef.current = nodes;
  }, [nodes]);

  useEffect(() => {
    edgesRef.current = edges;
  }, [edges]);

  useEffect(() => {
    selectedNodeIdRef.current = selectedNodeId;
  }, [selectedNodeId]);

  useEffect(() => {
    historyEntriesRef.current = historyEntries;
  }, [historyEntries]);

  useEffect(() => {
    historyIndexRef.current = historyIndex;
  }, [historyIndex]);

  const appendCanvasHistory = useCallback((label: string, snapshot?: CanvasSnapshot) => {
    if (historyApplyingRef.current || suppressSchemeDirtyRef.current) return;
    const nextSnapshot = cloneCanvasSnapshot(
      snapshot ?? canvasSnapshot(nodesRef.current, edgesRef.current, selectedNodeIdRef.current),
    );
    const nextKey = canvasSnapshotKey(nextSnapshot);
    const currentEntry = historyEntriesRef.current[historyIndexRef.current];
    if (currentEntry && canvasSnapshotKey(currentEntry.snapshot) === nextKey) return;

    const entry: CanvasHistoryEntry = {
      id: `history_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      label,
      timestamp: Date.now(),
      snapshot: nextSnapshot,
    };

    setHistoryEntries((current) => {
      const base = current.slice(0, historyIndexRef.current + 1);
      const limited = [...base, entry].slice(-CANVAS_HISTORY_LIMIT);
      const nextIndex = limited.length - 1;
      historyIndexRef.current = nextIndex;
      historyEntriesRef.current = limited;
      setHistoryIndex(nextIndex);
      return limited;
    });
  }, []);

  const flushCanvasHistory = useCallback(() => {
    const pending = pendingHistoryRef.current;
    if (!pending) return;
    if (pending.timer !== null) {
      window.clearTimeout(pending.timer);
    }
    pendingHistoryRef.current = null;
    appendCanvasHistory(pending.label);
  }, [appendCanvasHistory]);

  const scheduleCanvasHistory = useCallback((label: string, delay = 0) => {
    if (historyApplyingRef.current || suppressSchemeDirtyRef.current) return;
    const pending = pendingHistoryRef.current;
    if (pending?.timer !== null && pending?.timer !== undefined) {
      window.clearTimeout(pending.timer);
    }
    const timer = window.setTimeout(() => {
      flushCanvasHistory();
    }, delay);
    pendingHistoryRef.current = { label, timer };
  }, [flushCanvasHistory]);

  const resetCanvasHistory = useCallback((label: string, nextNodes: AuditNode[], nextEdges: Edge[], nextSelectedNodeId: string | null = null) => {
    const entry: CanvasHistoryEntry = {
      id: `history_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      label,
      timestamp: Date.now(),
      snapshot: canvasSnapshot(nextNodes, nextEdges, nextSelectedNodeId),
    };
    if (pendingHistoryRef.current?.timer !== null && pendingHistoryRef.current?.timer !== undefined) {
      window.clearTimeout(pendingHistoryRef.current.timer);
    }
    pendingHistoryRef.current = null;
    historyEntriesRef.current = [entry];
    historyIndexRef.current = 0;
    setHistoryEntries([entry]);
    setHistoryIndex(0);
  }, []);

  const applyCanvasHistoryIndex = useCallback((targetIndex: number) => {
    const entry = historyEntriesRef.current[targetIndex];
    if (!entry) return;
    const snapshot = cloneCanvasSnapshot(entry.snapshot);
    historyApplyingRef.current = true;
    setNodes(snapshot.nodes);
    setEdges(snapshot.edges);
    setSelectedNodeId(snapshot.selectedNodeId);
    setValidation({ valid: true, messages: [] });
    setSchemeDirty(true);
    setNotice(`已恢复到：${entry.label}`);
    historyIndexRef.current = targetIndex;
    setHistoryIndex(targetIndex);
    window.setTimeout(() => {
      historyApplyingRef.current = false;
    }, 0);
  }, [setEdges, setNodes]);

  const handleUndo = useCallback(() => {
    flushCanvasHistory();
    const targetIndex = historyIndexRef.current - 1;
    if (targetIndex < 0) {
      setNotice("没有可撤销的操作");
      return;
    }
    applyCanvasHistoryIndex(targetIndex);
  }, [applyCanvasHistoryIndex, flushCanvasHistory]);

  const handleRedo = useCallback(() => {
    flushCanvasHistory();
    const targetIndex = historyIndexRef.current + 1;
    if (targetIndex >= historyEntriesRef.current.length) {
      setNotice("没有可反撤销的操作");
      return;
    }
    applyCanvasHistoryIndex(targetIndex);
  }, [applyCanvasHistoryIndex, flushCanvasHistory]);

  const handleHistoryJump = useCallback((targetIndex: number) => {
    flushCanvasHistory();
    applyCanvasHistoryIndex(targetIndex);
    setHistoryOpen(false);
  }, [applyCanvasHistoryIndex, flushCanvasHistory]);

  const hydrate = useCallback(async () => {
    setBusy(true);
    try {
      const [nextModules, flow, nextRuns, root, nextRuntimeStatus, nextAppSettings] = await Promise.all([
        listModules(),
        loadFlow(),
        listRuns(),
        getDataRoot(),
        getRuntimeStatus(),
        getAppSettings(),
      ]);
      const nextNodes = toReactNodes(flow, nextModules);
      const nextEdges = toReactEdges(flow);
      setModules(nextModules);
      setNodes(nextNodes);
      setEdges(nextEdges);
      setSchemeName(flow.name || "未命名审核方案");
      setSchemePath(null);
      setSchemeDirty(false);
      setRuns(nextRuns);
      setDataRoot(root);
      setRuntimeStatus(nextRuntimeStatus);
      setAppSettings(nextAppSettings);
      setArtifactRootDraft(nextAppSettings.artifactRoot);
      setDependencyRootDraft(nextAppSettings.dependencyRoot);
      resetCanvasHistory("打开流程", nextNodes, nextEdges);
      setNotice("已加载");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [resetCanvasHistory, setEdges, setNodes]);

  const refreshRuntimeStatus = useCallback(async () => {
    try {
      setRuntimeStatus(await getRuntimeStatus());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const handleSelectArtifactRoot = useCallback(async () => {
    try {
      const selected = await selectArtifactRootDirectory();
      if (selected) {
        setArtifactRootDraft(selected);
      }
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const handleSaveArtifactRoot = useCallback(async () => {
    const artifactRoot = artifactRootDraft.trim();
    if (!artifactRoot) {
      setNotice("请先填写审核产物默认生成路径");
      return;
    }
    try {
      const saved = await saveAppSettings({
        artifactRoot,
        dependencyRoot: dependencyRootDraft.trim() || appSettings?.dependencyRoot || "",
      });
      setAppSettings(saved);
      setArtifactRootDraft(saved.artifactRoot);
      setDependencyRootDraft(saved.dependencyRoot);
      setNotice("审核产物路径已保存");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, [appSettings?.dependencyRoot, artifactRootDraft, dependencyRootDraft]);

  const handleSelectDependencyRoot = useCallback(async () => {
    try {
      const selected = await selectDependencyRootDirectory();
      if (selected) {
        setDependencyRootDraft(selected);
      }
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const handleSaveDependencyRoot = useCallback(async () => {
    const dependencyRoot = dependencyRootDraft.trim();
    if (!dependencyRoot) {
      setNotice("请先填写依赖存放默认路径");
      return;
    }
    try {
      const saved = await saveAppSettings({
        artifactRoot: artifactRootDraft.trim() || appSettings?.artifactRoot || "",
        dependencyRoot,
      });
      setAppSettings(saved);
      setArtifactRootDraft(saved.artifactRoot);
      setDependencyRootDraft(saved.dependencyRoot);
      setRuntimeStatus(await getRuntimeStatus());
      setNotice("依赖存放路径已保存");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, [appSettings?.artifactRoot, artifactRootDraft, dependencyRootDraft]);

  const markEntryNotificationsSeen = useCallback(() => {
    setEntryUnreadRunIds([]);
  }, []);

  const pushCompletionToast = useCallback((toast: Omit<CompletionToast, "id">) => {
    const id = `toast_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
    setCompletionToasts((current) => [{ ...toast, id }, ...current].slice(0, 4));
    window.setTimeout(() => {
      setCompletionToasts((current) => current.filter((item) => item.id !== id));
    }, 9000);
  }, []);

  const finishQueueTask = useCallback((
    taskId: string,
    status: QueueTaskStatus,
    message: string,
    run: RunRecord | null,
  ) => {
    const task = taskQueueRef.current.find((item) => item.id === taskId);
    const taskName = task?.name ?? run?.taskName ?? run?.flowName ?? "审核任务";
    const runId = run?.id ?? task?.runId ?? null;
    const now = nowSeconds();
    setTaskQueue((current) =>
      orderQueueTasks(
        current.map((item) =>
          item.id === taskId
            ? {
                ...item,
                status,
                updatedAt: now,
                finishedAt: now,
                runId,
                verdict: run?.verdict ?? item.verdict ?? null,
                reportPath: run?.reportPath ?? item.reportPath ?? null,
                message,
                nodeStates: run ? nodeStatesFromRun(run) : item.nodeStates ?? {},
              }
            : item,
        ),
      ),
    );
    if (runId) {
      if (!runOverlayOpenRef.current && activeTabRef.current !== "results") {
        setEntryUnreadRunIds((current) => addUniqueString(current, runId));
      }
      setHistoryUnreadRunIds((current) => addUniqueString(current, runId));
    }
    pushCompletionToast({
      taskId,
      name: taskName,
      status,
      runId,
      verdict: run?.verdict ?? task?.verdict ?? null,
      message,
    });
  }, [pushCompletionToast]);

  const runQueuedTask = useCallback(async (task: QueueTask) => {
    const nodeStates = initialRunNodeStates(task.flow, modules);
    const startedAt = nowSeconds();
    setSelectedQueueTaskId(task.id);
    setActiveQueueTaskId(task.id);
    activeQueueTaskIdRef.current = task.id;
    setActiveRunId(null);
    activeRunIdRef.current = null;
    setRunInProgress(true);
    setRunCancelling(false);
    setRuntimeTerminalCollapsed(false);
    setTaskQueue((current) =>
      orderQueueTasks(
        current.map((item) =>
          item.id === task.id
            ? {
                ...item,
                status: "running",
                startedAt,
                updatedAt: startedAt,
                message: "任务正在运行。",
                nodeStates,
              }
            : item,
        ),
      ),
    );

    try {
      const validationResult = await validateFlow(task.flow);
      setValidation(validationResult);
      if (!validationResult.valid) {
        throw new Error(validationResult.messages.join(" "));
      }

      if (!window.__TAURI_INTERNALS__) {
        const run = await startRun(task.flow, task.note, task.assets);
        finishQueueTask(
          task.id,
          run.status === "cancelled" ? "cancelled" : "completed",
          run.status === "cancelled" ? "任务已中断。" : "任务已完成。",
          run,
        );
        setRuns(await listRuns());
        setRunInProgress(false);
        setRunCancelling(false);
        setActiveRunId(null);
        activeRunIdRef.current = null;
        setActiveQueueTaskId(null);
        activeQueueTaskIdRef.current = null;
        return;
      }

      const started = await startRunLive(task.flow, task.note, task.assets);
      setActiveRunId(started.runId);
      activeRunIdRef.current = started.runId;
      setTaskQueue((current) =>
        current.map((item) =>
          item.id === task.id
            ? {
                ...item,
                runId: started.runId,
                updatedAt: nowSeconds(),
              }
            : item,
        ),
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      finishQueueTask(task.id, "failed", message, null);
      setRunInProgress(false);
      setRunCancelling(false);
      setActiveRunId(null);
      activeRunIdRef.current = null;
      setActiveQueueTaskId(null);
      activeQueueTaskIdRef.current = null;
      setNotice(message);
    }
  }, [finishQueueTask, modules]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    activeRunIdRef.current = activeRunId;
  }, [activeRunId]);

  useEffect(() => {
    activeQueueTaskIdRef.current = activeQueueTaskId;
  }, [activeQueueTaskId]);

  useEffect(() => {
    taskQueueRef.current = taskQueue;
    localStorage.setItem(TASK_QUEUE_STORAGE_KEY, JSON.stringify(taskQueue));
  }, [taskQueue]);

  useEffect(() => {
    localStorage.setItem(ENTRY_UNREAD_RUNS_STORAGE_KEY, JSON.stringify(entryUnreadRunIds));
  }, [entryUnreadRunIds]);

  useEffect(() => {
    localStorage.setItem(HISTORY_UNREAD_RUNS_STORAGE_KEY, JSON.stringify(historyUnreadRunIds));
  }, [historyUnreadRunIds]);

  useEffect(() => {
    runOverlayOpenRef.current = runOverlayOpen;
  }, [runOverlayOpen]);

  useEffect(() => {
    activeTabRef.current = activeTab;
  }, [activeTab]);

  useEffect(() => {
    if (selectedQueueTaskId && visibleQueueTasks.some((task) => task.id === selectedQueueTaskId)) return;
    setSelectedQueueTaskId(
      visibleQueueTasks.find((task) => task.id === activeQueueTaskId)?.id ??
        visibleQueueTasks[0]?.id ??
        null,
    );
  }, [activeQueueTaskId, selectedQueueTaskId, visibleQueueTasks]);

  useEffect(() => {
    if (modules.length === 0 || queueProcessingRef.current || runInProgress || activeQueueTaskId) return;
    const nextTask = taskQueueRef.current.find((task) => task.status === "pending");
    if (!nextTask) return;
    queueProcessingRef.current = true;
    void runQueuedTask(nextTask).finally(() => {
      queueProcessingRef.current = false;
    });
  }, [activeQueueTaskId, modules.length, runInProgress, runQueuedTask, taskQueue]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.altKey || event.metaKey || event.key.toLowerCase() !== "z") return;
      event.preventDefault();
      if (event.shiftKey) {
        handleRedo();
      } else {
        handleUndo();
      }
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [handleRedo, handleUndo]);

  useEffect(() => {
    if (!highlightRunId) return;
    const timer = window.setTimeout(() => setHighlightRunId(null), 3600);
    return () => window.clearTimeout(timer);
  }, [highlightRunId]);

  useEffect(() => {
    const preventContextMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    document.addEventListener("contextmenu", preventContextMenu);
    return () => {
      document.removeEventListener("contextmenu", preventContextMenu);
    };
  }, []);

  useEffect(() => {
    const closeSchemeMenu = (event: MouseEvent) => {
      if (!schemeDropdownRef.current?.contains(event.target as globalThis.Node)) {
        setSchemeMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", closeSchemeMenu);
    return () => document.removeEventListener("mousedown", closeSchemeMenu);
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let unlistenLog: (() => void) | null = null;
    let unlistenStatus: (() => void) | null = null;
    const runUnlisteners: Array<() => void> = [];
    const applyRunEvent = (eventName: string, payload: RunProgressEvent) => {
      const currentRunId = activeRunIdRef.current;
      if (currentRunId && payload.runId !== currentRunId) return;
      const currentTaskId = activeQueueTaskIdRef.current;

      if (eventName === "run_started") {
        setRunInProgress(true);
        setRunCancelling(false);
        setNotice("流程运行中");
        return;
      }

      if (payload.nodeId && currentTaskId) {
        setTaskQueue((current) =>
          current.map((task) => {
            if (task.id !== currentTaskId) return task;
            const nodeStates = task.nodeStates ?? {};
            const previous = nodeStates[payload.nodeId ?? ""] ?? { status: "pending", progress: 0, message: "" };
            const status =
              eventName === "step_started"
                ? "running"
                : eventName === "step_cancelled"
                  ? "cancelled"
                  : eventName === "step_failed"
                    ? "error"
                    : payload.step?.status ?? payload.status ?? previous.status;
            const progress =
              typeof payload.progress === "number"
                ? payload.progress
                : eventName === "step_completed" || eventName === "step_failed"
                  ? 1
                  : previous.progress;
            return {
              ...task,
              updatedAt: nowSeconds(),
              nodeStates: {
                ...nodeStates,
                [payload.nodeId ?? ""]: {
                  status,
                  progress,
                  message: payload.message || payload.step?.message || previous.message,
                  processed: payload.processed ?? payload.step?.processedFiles ?? previous.processed ?? null,
                  total: payload.total ?? previous.total ?? null,
                },
              },
            };
          }),
        );
      }

      if (eventName === "run_completed" || eventName === "run_cancelled" || eventName === "run_failed") {
        setRunInProgress(false);
        setRunCancelling(false);
        setActiveRunId(null);
        activeRunIdRef.current = null;
        setActiveQueueTaskId(null);
        activeQueueTaskIdRef.current = null;
        const loadRun = async () => {
          let runRecord = payload.run ?? null;
          if (!runRecord && payload.runId) {
            try {
              runRecord = await readRunRecord(payload.runId);
            } catch {
              // 运行失败时可能还没有写出完整记录。
            }
          }
          const status: QueueTaskStatus =
            eventName === "run_cancelled" || runRecord?.status === "cancelled"
              ? "cancelled"
              : eventName === "run_failed"
                ? "failed"
                : "completed";
          const message =
            status === "completed"
              ? "任务已完成。"
              : status === "cancelled"
                ? "任务已中断。"
                : payload.message || "任务失败。";
          if (currentTaskId) {
            finishQueueTask(currentTaskId, status, message, runRecord);
          }
          setNotice(message);
          setRuns(await listRuns());
        };
        void loadRun();
      }
    };

    void listen<RuntimeLogLine>("runtime_log", (event) => {
      setRuntimeLogs((current) => [...current.slice(-799), event.payload]);
    }).then((unlisten) => {
      unlistenLog = unlisten;
    });
    void listen("runtime_status_changed", () => {
      void refreshRuntimeStatus();
    }).then((unlisten) => {
      unlistenStatus = unlisten;
    });
    for (const eventName of [
      "run_started",
      "step_started",
      "step_progress",
      "step_completed",
      "step_failed",
      "step_cancelled",
      "run_completed",
      "run_failed",
      "run_cancelled",
    ]) {
      void listen<RunProgressEvent>(eventName, (event) => applyRunEvent(eventName, event.payload)).then((unlisten) => {
        runUnlisteners.push(unlisten);
      });
    }
    return () => {
      unlistenLog?.();
      unlistenStatus?.();
      runUnlisteners.forEach((unlisten) => unlisten());
    };
  }, [finishQueueTask, refreshRuntimeStatus]);

  useEffect(() => {
    setNodes((current) =>
      current.map((node) => {
        const module = moduleById(modules, String(node.data.moduleId));
        if (!module) return node;
        return {
          ...node,
          deletable: !isFixedSystemKind(module.kind),
          data: {
            ...node.data,
            moduleName: module.name,
            moduleKind: module.kind,
            moduleIcon: module.icon,
            moduleIconPath: module.iconPath ?? null,
            moduleIconDataUrl: module.iconDataUrl ?? null,
            dataOutputs: module.dataOutputs ?? [],
            source: module.source,
            config: {
              ...defaultConfigForModule(module),
              ...node.data.config,
            },
            requiresModelPath: module.parameters.some((parameter) => parameter.key === "modelPath"),
          },
        };
      }),
    );
  }, [modules, setNodes]);

  useEffect(() => {
    if (!selectedNode) {
      setConfigText("{}");
      return;
    }
    setConfigText(JSON.stringify(selectedNode.data.config ?? {}, null, 2));
  }, [selectedNode]);

  const currentFlow = useCallback(() => {
    const flow = buildFlow(nodes, edges);
    return {
      ...flow,
      name: schemeName.trim() || flow.name || "未命名审核方案",
    };
  }, [edges, nodes, schemeName]);

  const currentScheme = useCallback((): AuditScheme => {
    const flow = currentFlow();
    return schemeFromFlow(flow, schemeName);
  }, [currentFlow, schemeName]);

  const confirmDiscardUnsavedScheme = useCallback(() => {
    if (!schemeDirty) return true;
    return window.confirm("当前审核方案还没有保存，继续会丢失这些修改。");
  }, [schemeDirty]);

  const schemePathLabel = useMemo(() => {
    if (!schemePath) return "未保存";
    const normalized = schemePath.replace(/\\/g, "/");
    const name = normalized.split("/").filter(Boolean).pop();
    return name || schemePath;
  }, [schemePath]);

  const schemePickerValue = useMemo(() => {
    if (!schemePath) return "";
    const normalizedPath = schemePath.replace(/\\/g, "/").toLowerCase();
    return (
      schemeLibraryItems.find((item) => item.path.replace(/\\/g, "/").toLowerCase() === normalizedPath)?.path ?? ""
    );
  }, [schemeLibraryItems, schemePath]);

  const activeSchemeListItem = useMemo(() => {
    if (!schemePath) return null;
    const normalizedPath = schemePath.replace(/\\/g, "/").toLowerCase();
    return (
      schemeLibraryItems.find((item) => item.path.replace(/\\/g, "/").toLowerCase() === normalizedPath) ?? null
    );
  }, [schemeLibraryItems, schemePath]);

  const schemeDropdownLabel = activeSchemeListItem?.name ?? schemeName ?? "当前审核方案";

  const refreshSchemeLibrary = useCallback(async () => {
    try {
      const [dir, items] = await Promise.all([getSchemeLibraryDir(), listSchemeFiles()]);
      setSchemeLibraryDir(dir);
      setSchemeLibraryItems(items);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => {
    void refreshSchemeLibrary();
  }, [refreshSchemeLibrary]);

  const handleValidate = useCallback(async () => {
    const result = await validateFlow(currentFlow());
    setValidation(result);
    setNotice(result.valid ? "流程有效" : result.messages.join(" "));
    return result;
  }, [currentFlow]);

  const handleSaveFlow = useCallback(async () => {
    setBusy(true);
    try {
      const flow = await saveFlow(currentFlow());
      const nextNodes = toReactNodes(flow, modules);
      const nextEdges = toReactEdges(flow);
      setNodes(nextNodes);
      setEdges(nextEdges);
      setSchemeName(flow.name || schemeName);
      setValidation({ valid: true, messages: [] });
      setSchemeDirty(false);
      resetCanvasHistory("保存流程", nextNodes, nextEdges, selectedNodeIdRef.current);
      setNotice("流程已保存");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [currentFlow, modules, resetCanvasHistory, schemeName, setEdges, setNodes]);

  const applyScheme = useCallback(
    (scheme: AuditScheme, path: string | null, dirty: boolean) => {
      suppressSchemeDirtyRef.current = true;
      const nextNodes = toReactNodes(scheme.flow, modules);
      const nextEdges = toReactEdges(scheme.flow);
      setSchemeName(scheme.name || scheme.flow.name || "未命名审核方案");
      setSchemePath(path);
      setNodes(nextNodes);
      setEdges(nextEdges);
      setSelectedNodeId(null);
      setValidation({ valid: true, messages: [] });
      setSchemeDirty(dirty);
      resetCanvasHistory(path ? "加载审核方案" : "新建审核方案", nextNodes, nextEdges);
      window.setTimeout(() => {
        void fitView({ padding: 0.2, duration: 180 });
      }, 0);
      window.setTimeout(() => {
        suppressSchemeDirtyRef.current = false;
      }, 80);
    },
    [fitView, modules, resetCanvasHistory, setEdges, setNodes],
  );

  const loadSchemeFromPath = useCallback(
    async (path: string) => {
      const scheme = await loadSchemeFile(path);
      const result = await validateFlow(scheme.flow);
      if (!result.valid) {
        setValidation(result);
        setNotice(result.messages.join(" "));
        return false;
      }
      applyScheme(scheme, path, false);
      setNotice(`已加载审核方案：${scheme.name}`);
      return true;
    },
    [applyScheme],
  );

  const handleNewScheme = useCallback(async () => {
    if (!confirmDiscardUnsavedScheme()) return;
    const name = window.prompt("审核方案名称", "图片 UGC 默认审核");
    if (name === null) return;
    const flow = defaultFlowDefinition();
    const scheme = schemeFromFlow(flow, name.trim() || "图片 UGC 默认审核");
    setBusy(true);
    try {
      const saved = await saveSchemeToLibrary(scheme);
      applyScheme(saved.scheme, saved.path, false);
      await refreshSchemeLibrary();
      setNotice(`已创建审核方案：${saved.scheme.name}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [applyScheme, confirmDiscardUnsavedScheme, refreshSchemeLibrary]);

  const handleLoadScheme = useCallback(async () => {
    if (!confirmDiscardUnsavedScheme()) return;
    setBusy(true);
    try {
      const path = await selectSchemePath();
      if (!path) return;
      const loaded = await loadSchemeFromPath(path);
      if (loaded) await refreshSchemeLibrary();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [confirmDiscardUnsavedScheme, loadSchemeFromPath, refreshSchemeLibrary]);

  const handleQuickLoadScheme = useCallback(
    async (path: string) => {
      if (!path || path === schemePath) return;
      if (!confirmDiscardUnsavedScheme()) return;
      setBusy(true);
      try {
        await loadSchemeFromPath(path);
        setSchemeMenuOpen(false);
      } catch (error) {
        setNotice(error instanceof Error ? error.message : String(error));
      } finally {
        setBusy(false);
      }
    },
    [confirmDiscardUnsavedScheme, loadSchemeFromPath, schemePath],
  );

  const handleDeleteSchemeItem = useCallback(
    async (item: SchemeListItem) => {
      if (!window.confirm(`删除审核方案“${item.name}”？`)) return;
      setBusy(true);
      try {
        const nextItems = await deleteSchemeFile(item.path);
        setSchemeLibraryItems(nextItems);
        const deletingCurrent =
          schemePath?.replace(/\\/g, "/").toLowerCase() === item.path.replace(/\\/g, "/").toLowerCase();
        if (deletingCurrent) {
          setSchemePath(null);
          setSchemeDirty(true);
        }
        if (runDialogSchemePath === item.path) {
          setRunDialogSchemePath(CURRENT_SCHEME_VALUE);
        }
        setNotice(`已删除审核方案：${item.name}`);
      } catch (error) {
        setNotice(error instanceof Error ? error.message : String(error));
      } finally {
        setBusy(false);
      }
    },
    [runDialogSchemePath, schemePath],
  );

  const handleRenameSchemeItem = useCallback(
    async (item: SchemeListItem) => {
      const nextName = window.prompt("新的审核方案名称", item.name)?.trim();
      if (nextName === undefined) return;
      if (!nextName) {
        setNotice("审核方案名称不能为空");
        return;
      }
      if (nextName === item.name) return;

      setBusy(true);
      try {
        const scheme = await loadSchemeFile(item.path);
        await saveSchemeFile(item.path, {
          ...scheme,
          name: nextName,
          flow: {
            ...scheme.flow,
            name: nextName,
          },
        });
        await refreshSchemeLibrary();
        const renamingCurrent =
          schemePath?.replace(/\\/g, "/").toLowerCase() === item.path.replace(/\\/g, "/").toLowerCase();
        if (renamingCurrent) {
          setSchemeName(nextName);
        }
        setNotice(`已重命名审核方案：${nextName}`);
      } catch (error) {
        setNotice(error instanceof Error ? error.message : String(error));
      } finally {
        setBusy(false);
      }
    },
    [refreshSchemeLibrary, schemePath],
  );

  const saveCurrentSchemeToPath = useCallback(
    async (path: string) => {
      const scheme = await saveSchemeFile(path, currentScheme());
      const savedPath = /\.[^\\/]+$/.test(path) ? path : `${path}.ugcaudit`;
      applyScheme(scheme, savedPath, false);
      await refreshSchemeLibrary();
      setNotice(`已保存审核方案：${scheme.name}`);
    },
    [applyScheme, currentScheme, refreshSchemeLibrary],
  );

  const saveCurrentSchemeToLibrary = useCallback(
    async () => {
      const saved = await saveSchemeToLibrary(currentScheme());
      applyScheme(saved.scheme, saved.path, false);
      await refreshSchemeLibrary();
      setNotice(`已保存审核方案：${saved.scheme.name}`);
    },
    [applyScheme, currentScheme, refreshSchemeLibrary],
  );

  const handleSaveScheme = useCallback(async () => {
    setBusy(true);
    try {
      if (schemePath) {
        await saveCurrentSchemeToPath(schemePath);
      } else {
        await saveCurrentSchemeToLibrary();
      }
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [saveCurrentSchemeToLibrary, saveCurrentSchemeToPath, schemePath]);

  const handleSaveSchemeAs = useCallback(async () => {
    setBusy(true);
    try {
      const defaultName = `${schemeName || "审核方案"}.ugcaudit`;
      const libraryDefault = schemeLibraryDir ? `${schemeLibraryDir}\\${defaultName}` : defaultName;
      const path = await selectSchemeSavePath(schemePath ?? libraryDefault);
      if (!path) return;
      await saveCurrentSchemeToPath(path);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [saveCurrentSchemeToPath, schemeLibraryDir, schemeName, schemePath]);

  const handleAutoAlign = useCallback(() => {
    setNodes((current) => layoutAuditNodes(current, edges));
    setValidation({ valid: true, messages: [] });
    setSchemeDirty(true);
    scheduleCanvasHistory("自动对齐");
    setNotice("流程已自动对齐");
    window.setTimeout(() => {
      void fitView({ padding: 0.2, duration: 240 });
    }, 0);
  }, [edges, fitView, scheduleCanvasHistory, setNodes]);

  const addCanvasTool = useCallback((toolType: CanvasToolType, clientPoint?: { x: number; y: number }, wrapSelection = true) => {
    const kind = toolType === "group" ? CANVAS_GROUP_KIND : CANVAS_NOTE_KIND;
    const defaults = canvasToolDefaults(kind);
    const selectedContentNodes = nodesRef.current.filter((node) => node.selected);
    const paddingX = 42;
    const paddingTop = 54;
    const paddingBottom = 36;
    let position = { x: 0, y: 0 };
    let width = defaults.width;
    let height = defaults.height;

    if (toolType === "group" && wrapSelection && selectedContentNodes.length > 0) {
      const bounds = selectedContentNodes.reduce(
        (acc, node) => {
          const nodeWidth = nodeDimension(node, "width", isCanvasToolKind(String(node.data.moduleKind)) ? canvasToolDefaults(String(node.data.moduleKind)).width : 204);
          const nodeHeight = nodeDimension(node, "height", isCanvasToolKind(String(node.data.moduleKind)) ? canvasToolDefaults(String(node.data.moduleKind)).height : 96);
          return {
            minX: Math.min(acc.minX, node.position.x),
            minY: Math.min(acc.minY, node.position.y),
            maxX: Math.max(acc.maxX, node.position.x + nodeWidth),
            maxY: Math.max(acc.maxY, node.position.y + nodeHeight),
          };
        },
        { minX: Number.POSITIVE_INFINITY, minY: Number.POSITIVE_INFINITY, maxX: Number.NEGATIVE_INFINITY, maxY: Number.NEGATIVE_INFINITY },
      );
      position = {
        x: bounds.minX - paddingX,
        y: bounds.minY - paddingTop,
      };
      width = Math.max(defaults.width, bounds.maxX - bounds.minX + paddingX * 2);
      height = Math.max(defaults.height, bounds.maxY - bounds.minY + paddingTop + paddingBottom);
    } else {
      const center = clientPoint
        ? screenToFlowPosition(clientPoint)
        : (() => {
            const rect = flowPanelRef.current?.getBoundingClientRect();
            return rect
              ? screenToFlowPosition({ x: rect.left + rect.width * 0.5, y: rect.top + rect.height * 0.5 })
              : { x: 220, y: 160 };
          })();
      position = {
        x: center.x - width * 0.5,
        y: center.y - height * 0.5,
      };
    }

    const id = `${toolType}_${Date.now()}`;
    const config = canvasToolConfig({
      title: toolType === "group" ? "分组" : "注释",
      text: "",
      color: "blue",
      width,
      height,
    }, kind);
    const nextNode: AuditNode = {
      id,
      type: toolType === "group" ? "canvasGroup" : "canvasNote",
      position,
      deletable: true,
      selected: true,
      zIndex: toolType === "group" ? 0 : 8,
      style: { width, height },
      data: {
        label: String(config.title),
        moduleId: toolType === "group" ? CANVAS_GROUP_MODULE_ID : CANVAS_NOTE_MODULE_ID,
        moduleName: toolType === "group" ? "分组" : "注释",
        moduleKind: kind,
        moduleIcon: toolType === "group" ? "group" : "sticky-note",
        moduleIconPath: null,
        moduleIconDataUrl: null,
        source: "system",
        config,
        requiresModelPath: false,
      },
    };

    setNodes((current) => [
      ...current.map((node) => ({ ...node, selected: false })),
      nextNode,
    ]);
    setSelectedNodeId(id);
    setValidation({ valid: true, messages: [] });
    setSchemeDirty(true);
    scheduleCanvasHistory(toolType === "group" ? "添加分组" : "添加注释");
    setNotice(
      toolType === "group" && wrapSelection && selectedContentNodes.length > 0
        ? `已为 ${selectedContentNodes.length} 个对象添加分组`
        : toolType === "group"
          ? "已添加分组"
          : "已添加注释",
    );
  }, [scheduleCanvasHistory, screenToFlowPosition, setNodes]);

  const enqueueRunTask = useCallback((
    flow: FlowDefinition,
    note: string,
    assets: AuditAsset[],
    name: string,
  ) => {
    const createdAt = nowSeconds();
    const task: QueueTask = {
      id: `task_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      name: name.trim() || flow.name || "审核任务",
      note,
      flow,
      assets,
      status: "pending",
      createdAt,
      updatedAt: createdAt,
      runId: null,
      verdict: null,
      reportPath: null,
      message: "等待运行。",
      nodeStates: initialRunNodeStates(flow, modules),
    };
    setTaskQueue((current) => orderQueueTasks([...current, task]));
    setSelectedQueueTaskId(task.id);
    markEntryNotificationsSeen();
    setRunOverlayOpen(true);
    setNotice(`已加入队列：${task.name}`);
  }, [markEntryNotificationsSeen, modules]);

  const handleOpenRunDialog = useCallback(() => {
    setRunDialogSchemePath(schemePickerValue || CURRENT_SCHEME_VALUE);
    setRunTaskName(schemeName || "审核任务");
    setRunTaskNote("");
    setRunDialogAssets(selectedAssets);
    setRunDialogError("");
    setRunStarting(false);
    setRunDialogOpen(true);
  }, [schemeName, schemePickerValue, selectedAssets]);

  const handleCreateTaskFromRunOverlay = useCallback(() => {
    setRunOverlayOpen(false);
    handleOpenRunDialog();
  }, [handleOpenRunDialog]);

  const handleSelectRunDialogDirectory = useCallback(async () => {
    try {
      const assets = await selectAssetDirectory();
      setRunDialogAssets(assets);
      if (assets.length > 0) {
        setRunDialogError("");
        setNotice(`已选择 ${assets[0].name}`);
      } else {
        setRunDialogError("请选择要检测的文件夹");
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setRunDialogError(message);
      setNotice(message);
    }
  }, []);

  const handleConfirmRunDialog = useCallback(async () => {
    if (runStarting) return;
    setRunDialogError("");
    const assets = runDialogAssets.filter((asset) => asset.kind === "directory");
    if (assets.length === 0) {
      const message = "请先选择要检测的文件夹";
      setRunDialogError(message);
      setNotice(message);
      return;
    }

    setRunStarting(true);
    let flow: FlowDefinition;
    let resolvedSchemeName = schemeName;
    if (runDialogSchemePath && runDialogSchemePath !== CURRENT_SCHEME_VALUE) {
      try {
        const scheme = await loadSchemeFile(runDialogSchemePath);
        flow = scheme.flow;
        resolvedSchemeName = scheme.name || scheme.flow.name;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setRunDialogError(message);
        setNotice(message);
        setRunStarting(false);
        return;
      }
    } else {
      flow = currentFlow();
    }

    const result = await validateFlow(flow);
    setValidation(result);
    if (!result.valid) {
      const message = result.messages.join(" ");
      setRunDialogError(message);
      setNotice(message);
      setRunStarting(false);
      return;
    }

    const taskName = runTaskName.trim() || resolvedSchemeName || "审核任务";
    const taskNote = runTaskNote.trim();
    const note = taskNote ? `任务名称：${taskName}\n任务说明：${taskNote}` : `任务名称：${taskName}`;
    setInputNote(note);
    setSelectedAssets(assets);
    setRunDialogOpen(false);
    enqueueRunTask(flow, note, assets, taskName);
    setRunStarting(false);
  }, [currentFlow, enqueueRunTask, runDialogAssets, runDialogSchemePath, runStarting, runTaskName, runTaskNote, schemeName]);

  const handleCancelRun = useCallback(async () => {
    if (!activeRunId || runCancelling) return;
    setRunCancelling(true);
    setNotice("正在中断流程");
    try {
      await cancelRun(activeRunId);
    } catch (error) {
      setRunCancelling(false);
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, [activeRunId, runCancelling]);

  const handleOpenResultsTab = useCallback(() => {
    markEntryNotificationsSeen();
    setActiveTab("results");
  }, [markEntryNotificationsSeen]);

  const handleOpenQueueOverlay = useCallback(() => {
    markEntryNotificationsSeen();
    setRunOverlayOpen(true);
  }, [markEntryNotificationsSeen]);

  const openRunResult = useCallback(async (runId: string, flash = false) => {
    markEntryNotificationsSeen();
    setActiveTab("results");
    try {
      const run = await readRunRecord(runId);
      setCurrentRun(run);
      setReport(await readRunReport(run.id));
      setHistoryUnreadRunIds((current) => removeString(current, run.id));
      setTaskQueue((current) =>
        current.filter((task) => !(task.runId === run.id && isTerminalQueueStatus(task.status))),
      );
      if (flash) {
        setHighlightRunId("");
        window.setTimeout(() => setHighlightRunId(run.id), 0);
      }
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }, [markEntryNotificationsSeen]);

  const handleQuickViewTask = useCallback((task: QueueTask) => {
    if (!task.runId) {
      setNotice(task.message || "这个任务没有可打开的运行结果。");
      return;
    }
    setRunOverlayOpen(false);
    void openRunResult(task.runId, true);
  }, [openRunResult]);

  const handleMoveQueueTaskEarlier = useCallback((taskId: string) => {
    setTaskQueue((current) => {
      const pending = current.filter((task) => task.status === "pending");
      const index = pending.findIndex((task) => task.id === taskId);
      if (index <= 0) return current;
      const nextPending = [...pending];
      [nextPending[index - 1], nextPending[index]] = [nextPending[index], nextPending[index - 1]];
      return replacePendingTaskOrder(current, nextPending);
    });
  }, []);

  const handleDeleteQueueTask = useCallback((taskId: string) => {
    const task = taskQueueRef.current.find((item) => item.id === taskId);
    if (!task || task.status === "running") return;
    setTaskQueue((current) => current.filter((item) => item.id !== taskId));
    if (selectedQueueTaskId === taskId) {
      setSelectedQueueTaskId(null);
    }
    setNotice(`已删除队列任务：${task.name}`);
  }, [selectedQueueTaskId]);

  const handleQueueTaskDragStart = useCallback((event: DragEvent<HTMLDivElement>, task: QueueTask) => {
    if (task.status !== "pending") return;
    draggedQueueTaskIdRef.current = task.id;
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", task.id);
  }, []);

  const handleQueueTaskDragOver = useCallback((event: DragEvent<HTMLDivElement>, task: QueueTask) => {
    const draggedId = draggedQueueTaskIdRef.current;
    if (!draggedId || task.status !== "pending" || draggedId === task.id) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setDragOverQueueTaskId(task.id);
  }, []);

  const handleQueueTaskDrop = useCallback((event: DragEvent<HTMLDivElement>, task: QueueTask) => {
    event.preventDefault();
    const draggedId = draggedQueueTaskIdRef.current;
    draggedQueueTaskIdRef.current = null;
    setDragOverQueueTaskId(null);
    if (!draggedId || task.status !== "pending" || draggedId === task.id) return;
    setTaskQueue((current) => {
      const pending = current.filter((item) => item.status === "pending");
      const dragged = pending.find((item) => item.id === draggedId);
      const targetIndex = pending.findIndex((item) => item.id === task.id);
      if (!dragged || targetIndex < 0) return current;
      const nextPending = pending.filter((item) => item.id !== draggedId);
      nextPending.splice(targetIndex, 0, dragged);
      return replacePendingTaskOrder(current, nextPending);
    });
  }, []);

  const handleQueueTaskDragEnd = useCallback(() => {
    draggedQueueTaskIdRef.current = null;
    setDragOverQueueTaskId(null);
  }, []);

  const clampRunBubblePosition = useCallback((position: { x: number; y: number }) => {
    const margin = 12;
    const size = 68;
    const maxX = Math.max(margin, window.innerWidth - size - margin);
    const maxY = Math.max(margin, window.innerHeight - size - margin);
    return {
      x: Math.min(Math.max(position.x, margin), maxX),
      y: Math.min(Math.max(position.y, margin), maxY),
    };
  }, []);

  useEffect(() => {
    const handleResize = () => {
      setRunBubblePosition((position) => clampRunBubblePosition(position));
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [clampRunBubblePosition]);

  const handleRunBubblePointerDown = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      if (event.button !== 0) return;
      const pointerId = event.pointerId;
      const drag = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        originX: runBubblePosition.x,
        originY: runBubblePosition.y,
        moved: false,
      };
      runBubbleDragRef.current = drag;

      const handlePointerMove = (moveEvent: PointerEvent) => {
        if (moveEvent.pointerId !== pointerId) return;
        const deltaX = moveEvent.clientX - drag.startX;
        const deltaY = moveEvent.clientY - drag.startY;
        if (Math.hypot(deltaX, deltaY) > 3) {
          drag.moved = true;
        }
        if (!drag.moved) return;
        setRunBubblePosition(clampRunBubblePosition({
          x: drag.originX + deltaX,
          y: drag.originY + deltaY,
        }));
        moveEvent.preventDefault();
      };

      const stopDrag = (upEvent: PointerEvent) => {
        if (upEvent.pointerId !== pointerId) return;
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", stopDrag);
        window.removeEventListener("pointercancel", stopDrag);
        runBubbleDragRef.current = null;
        if (drag.moved) {
          runBubbleSuppressClickRef.current = true;
          window.setTimeout(() => {
            runBubbleSuppressClickRef.current = false;
          }, 0);
        }
      };

      window.addEventListener("pointermove", handlePointerMove, { passive: false });
      window.addEventListener("pointerup", stopDrag);
      window.addEventListener("pointercancel", stopDrag);
    },
    [clampRunBubblePosition, runBubblePosition.x, runBubblePosition.y],
  );

  const handleRunBubbleClick = useCallback((event: ReactMouseEvent<HTMLButtonElement>) => {
    if (runBubbleSuppressClickRef.current) {
      event.preventDefault();
      return;
    }
    handleOpenQueueOverlay();
  }, [handleOpenQueueOverlay]);

  const handleConnect = useCallback(
    (connection: Connection) => {
      const source = nodes.find((node) => node.id === connection.source);
      const target = nodes.find((node) => node.id === connection.target);
      if (!source || !target) return;
      const sourcePort = portForHandle(
        String(source.data.moduleKind),
        "out",
        connection.sourceHandle,
        source.data.dataOutputs,
      );
      const targetPort = portForHandle(
        String(target.data.moduleKind),
        "in",
        connection.targetHandle,
        target.data.dataOutputs,
      );
      const edgeType = edgeKindForPorts(sourcePort, targetPort);
      if (!edgeType) {
        setNotice("只能连接同类端口，图片接图片，文本接文本，顺序接顺序");
        return;
      }
      if (
        edgeType === EDGE_TYPE_DATA &&
        isDataInputOccupied(edges, connection.target, connection.targetHandle ?? "")
      ) {
        setNotice("这个数据输入口已经连接过了");
        return;
      }
      setEdges((current) =>
        addEdge(
          flowEdgeForConnection(connection, edgeType),
          current,
        ),
      );
      setSchemeDirty(true);
      setPendingConnectionMenu(null);
      scheduleCanvasHistory("连接节点");
    },
    [edges, nodes, scheduleCanvasHistory, setEdges],
  );

  const handleConnectEnd = useCallback(
    (event: MouseEvent | TouchEvent, connectionState: FinalConnectionState) => {
      if (connectionState.toHandle || connectionState.toNode) {
        setPendingConnectionMenu(null);
        return;
      }

      const point = eventClientPoint(event);
      const fromNodeId = connectionState.fromNode?.id;
      const fromHandleId = connectionState.fromHandle?.id ?? null;
      const fromHandleType = connectionState.fromHandle?.type;
      if (!point || !fromNodeId || !fromHandleId || !fromHandleType) {
        setPendingConnectionMenu(null);
        return;
      }

      const anchorNode = nodes.find((node) => node.id === fromNodeId);
      if (!anchorNode) {
        setPendingConnectionMenu(null);
        return;
      }

      const anchorDirection: PortDirection = fromHandleType === "source" ? "out" : "in";
      const candidateDirection: PortDirection = anchorDirection === "out" ? "in" : "out";
      const anchorPort = portForHandle(
        String(anchorNode.data.moduleKind),
        anchorDirection,
        fromHandleId,
        anchorNode.data.dataOutputs,
      );
      if (!anchorPort) {
        setPendingConnectionMenu(null);
        return;
      }

      const canUseOption = (candidateNodeId: string, candidatePort: PortDefinition) => {
        const sourceNodeId = anchorDirection === "out" ? fromNodeId : candidateNodeId;
        const targetNodeId = anchorDirection === "out" ? candidateNodeId : fromNodeId;
        const sourcePort = anchorDirection === "out" ? anchorPort : candidatePort;
        const targetPort = anchorDirection === "out" ? candidatePort : anchorPort;
        const edgeType = edgeKindForPorts(sourcePort, targetPort);
        if (!edgeType) return null;
        const connection: Connection = {
          source: sourceNodeId,
          target: targetNodeId,
          sourceHandle: sourcePort.id,
          targetHandle: targetPort.id,
        };
        if (edgeType === EDGE_TYPE_DATA && isDataInputOccupied(edges, targetNodeId, targetPort.id)) {
          return null;
        }
        if (hasSameConnection(edges, connection)) {
          return null;
        }
        return edgeType;
      };

      const existingOptions: PendingConnectionOption[] = nodes.flatMap((node) => {
        if (node.id === fromNodeId) return [];
        return portsForKind(String(node.data.moduleKind), node.data.dataOutputs)
          .filter((port) => port.direction === candidateDirection)
          .flatMap((port) => {
            const edgeType = canUseOption(node.id, port);
            if (!edgeType) return [];
            return [{
              id: `existing-${node.id}-${port.id}`,
              group: "existing" as const,
              label: String(node.data.label),
              detail: portTypeLabel(port),
              nodeId: node.id,
              port,
              edgeType,
            }];
          });
      });

      const newNodeOptions: PendingConnectionOption[] = modules
        .filter((module) => !isFixedSystemKind(module.kind))
        .flatMap((module) =>
          portsForKind(module.kind, module.dataOutputs)
            .filter((port) => port.direction === candidateDirection)
            .flatMap((port) => {
              const edgeType = canUseOption(`new-${module.id}`, port);
              if (!edgeType) return [];
              return [{
                id: `new-${module.id}-${port.id}`,
                group: "new" as const,
                label: module.name,
                detail: portTypeLabel(port),
                module,
                port,
                edgeType,
              }];
            }),
        );

      setPendingConnectionMenu({
        x: Math.min(point.x, window.innerWidth - 340),
        y: Math.min(point.y, window.innerHeight - 360),
        flowPosition: screenToFlowPosition({ x: point.x, y: point.y }),
        anchorNodeId: fromNodeId,
        anchorHandleId: fromHandleId,
        anchorPort,
        options: [...existingOptions, ...newNodeOptions],
      });
      pendingConnectionOpenedAtRef.current = Date.now();
    },
    [edges, modules, nodes, screenToFlowPosition],
  );

  const handlePendingConnectionOption = useCallback(
    (option: PendingConnectionOption) => {
      if (!pendingConnectionMenu) return;
      const anchorDirection = pendingConnectionMenu.anchorPort.direction;
      let candidateNodeId = option.nodeId ?? null;

      if (option.group === "new" && option.module) {
        candidateNodeId = `${option.module.kind}_${Date.now()}`;
        const module = option.module;
        const nextNode: AuditNode = {
          id: candidateNodeId,
          type: "audit",
          position: pendingConnectionMenu.flowPosition,
          deletable: !isFixedSystemKind(module.kind),
          zIndex: 10,
          data: {
            label: module.name,
            moduleId: module.id,
            moduleName: module.name,
            moduleKind: module.kind,
            moduleIcon: module.icon,
            moduleIconPath: module.iconPath ?? null,
            moduleIconDataUrl: module.iconDataUrl ?? null,
            source: module.source,
            config: defaultConfigForModule(module),
            requiresModelPath: module.parameters.some((parameter) => parameter.key === "modelPath"),
          },
        };
        setNodes((current) => [...current, nextNode]);
        setSelectedNodeId(candidateNodeId);
      }

      if (!candidateNodeId) return;

      const sourcePort = anchorDirection === "out" ? pendingConnectionMenu.anchorPort : option.port;
      const targetPort = anchorDirection === "out" ? option.port : pendingConnectionMenu.anchorPort;
      const sourceNodeId = anchorDirection === "out" ? pendingConnectionMenu.anchorNodeId : candidateNodeId;
      const targetNodeId = anchorDirection === "out" ? candidateNodeId : pendingConnectionMenu.anchorNodeId;
      const connection: Connection = {
        source: sourceNodeId,
        target: targetNodeId,
        sourceHandle: sourcePort.id,
        targetHandle: targetPort.id,
      };

      setEdges((current) => addEdge(flowEdgeForConnection(connection, option.edgeType), current));
      setPendingConnectionMenu(null);
      setValidation({ valid: true, messages: [] });
      setSchemeDirty(true);
      scheduleCanvasHistory(option.group === "new" ? `新增并连接：${option.label}` : "连接节点");
      setNotice(`已连接到 ${option.label} · ${option.detail}`);
    },
    [pendingConnectionMenu, scheduleCanvasHistory, setEdges, setNodes],
  );

  const isPointInFlowPanel = useCallback((clientX: number, clientY: number) => {
    const rect = flowPanelRef.current?.getBoundingClientRect();
    if (!rect) return false;
    return (
      clientX >= rect.left &&
      clientX <= rect.right &&
      clientY >= rect.top &&
      clientY <= rect.bottom
    );
  }, []);

  const addModuleNode = useCallback(
    (module: ModuleInfo, clientX: number, clientY: number) => {
      if (isFixedSystemKind(module.kind)) return;
      const position = screenToFlowPosition({ x: clientX, y: clientY });
      const id = `${module.kind}_${Date.now()}`;
      const nextNode: AuditNode = {
        id,
        type: "audit",
        position,
        deletable: !isFixedSystemKind(module.kind),
        zIndex: 10,
        data: {
          label: module.name,
          moduleId: module.id,
          moduleName: module.name,
          moduleKind: module.kind,
          moduleIcon: module.icon,
          moduleIconPath: module.iconPath ?? null,
          moduleIconDataUrl: module.iconDataUrl ?? null,
          dataOutputs: module.dataOutputs ?? [],
          source: module.source,
          config: defaultConfigForModule(module),
          requiresModelPath: module.parameters.some((parameter) => parameter.key === "modelPath"),
        },
      };
      setNodes((current) => [...current, nextNode]);
      setSelectedNodeId(id);
      setValidation({ valid: true, messages: [] });
      setSchemeDirty(true);
      scheduleCanvasHistory(`添加步骤：${module.name}`);
      setNotice(`已添加步骤：${module.name}`);
    },
    [scheduleCanvasHistory, screenToFlowPosition, setNodes],
  );

  const handleModulePointerDown = (
    event: ReactPointerEvent<HTMLDivElement>,
    module: ModuleInfo,
  ) => {
    if (event.button !== 0 || isFixedSystemKind(module.kind)) return;
    event.preventDefault();

    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const updatePreview = (clientX: number, clientY: number) => {
      const overCanvas = isPointInFlowPanel(clientX, clientY);
      setModuleDragPreview({
        name: module.name,
        icon: module.icon,
        iconPath: module.iconPath ?? null,
        iconDataUrl: module.iconDataUrl ?? null,
        x: clientX,
        y: clientY,
        overCanvas,
      });
    };

    const stopDrag = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerCancel);
      document.body.classList.remove("is-module-dragging");
      setModuleDragPreview(null);
    };

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const distance = Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY);
      if (distance < 4 && !dragging) return;
      dragging = true;
      document.body.classList.add("is-module-dragging");
      updatePreview(moveEvent.clientX, moveEvent.clientY);
      moveEvent.preventDefault();
    };

    const handlePointerUp = (upEvent: PointerEvent) => {
      const shouldAdd = dragging && isPointInFlowPanel(upEvent.clientX, upEvent.clientY);
      stopDrag();
      if (shouldAdd) {
        addModuleNode(module, upEvent.clientX, upEvent.clientY);
      } else if (dragging) {
        setNotice("把模块拖到画布区域再松开");
      }
    };

    const handlePointerCancel = () => {
      stopDrag();
    };

    window.addEventListener("pointermove", handlePointerMove, { passive: false });
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerCancel);
  };

  const handleToolPointerDown = (
    event: ReactPointerEvent<HTMLDivElement>,
    tool: CanvasToolItem,
  ) => {
    if (event.button !== 0) return;
    event.preventDefault();

    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const updatePreview = (clientX: number, clientY: number) => {
      const overCanvas = isPointInFlowPanel(clientX, clientY);
      setModuleDragPreview({
        name: tool.name,
        icon: tool.icon,
        iconPath: null,
        iconDataUrl: null,
        x: clientX,
        y: clientY,
        overCanvas,
      });
    };

    const stopDrag = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerCancel);
      document.body.classList.remove("is-module-dragging");
      setModuleDragPreview(null);
    };

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const distance = Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY);
      if (distance < 4 && !dragging) return;
      dragging = true;
      document.body.classList.add("is-module-dragging");
      updatePreview(moveEvent.clientX, moveEvent.clientY);
      moveEvent.preventDefault();
    };

    const handlePointerUp = (upEvent: PointerEvent) => {
      const shouldAddAtPoint = dragging && isPointInFlowPanel(upEvent.clientX, upEvent.clientY);
      stopDrag();
      if (shouldAddAtPoint) {
        addCanvasTool(tool.type, { x: upEvent.clientX, y: upEvent.clientY }, false);
      } else if (dragging) {
        setNotice("把工具拖到画布区域再松开");
      } else {
        addCanvasTool(tool.type);
      }
    };

    const handlePointerCancel = () => {
      stopDrag();
    };

    window.addEventListener("pointermove", handlePointerMove, { passive: false });
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerCancel);
  };

  const handleDrop = useCallback(
    (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const moduleId = event.dataTransfer.getData("application/ugc-module");
      const module = moduleById(modules, moduleId);
      if (!module) return;
      if (isFixedSystemKind(module.kind)) return;
      addModuleNode(module, event.clientX, event.clientY);
    },
    [addModuleNode, modules],
  );

  const handleDragOver = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  };

  const handleNodeChange = useCallback(
    (changes: NodeChange<AuditNode>[]) => {
      const filtered = changes.filter((change) => {
        if (change.type !== "remove") return true;
        const node = nodes.find((item) => item.id === change.id);
        return !isFixedSystemNode(node);
      });
      if (filtered.length !== changes.length) {
        setNotice("开始和输出结果节点不能删除");
      }
      onNodesChange(filtered);
      const meaningfulChanges = filtered.filter((change) => change.type !== "select");
      if (!suppressSchemeDirtyRef.current && meaningfulChanges.length > 0) {
        setSchemeDirty(true);
        const label = meaningfulChanges.some((change) => change.type === "remove")
          ? "删除节点"
          : meaningfulChanges.some((change) => change.type === "dimensions")
            ? "调整对象大小"
            : meaningfulChanges.some((change) => change.type === "position")
              ? "移动对象"
              : "编辑画布";
        scheduleCanvasHistory(label, label === "移动对象" || label === "调整对象大小" ? 260 : 0);
      }
    },
    [nodes, onNodesChange, scheduleCanvasHistory],
  );

  const handleEdgesChange = useCallback(
    (changes: Parameters<typeof onEdgesChange>[0]) => {
      onEdgesChange(changes);
      const meaningfulChanges = changes.filter((change) => change.type !== "select");
      if (!suppressSchemeDirtyRef.current && meaningfulChanges.length > 0) {
        setSchemeDirty(true);
        scheduleCanvasHistory(
          meaningfulChanges.some((change) => change.type === "remove") ? "删除连线" : "编辑连线",
        );
      }
    },
    [onEdgesChange, scheduleCanvasHistory],
  );

  const updateSelectedNode = (patch: Partial<AuditNodeData>, historyLabel = "编辑参数", historyDelay = 700) => {
    if (!selectedNodeId) return;
    setNodes((current) =>
      current.map((node) =>
        node.id === selectedNodeId ? { ...node, data: { ...node.data, ...patch } } : node,
      ),
    );
    setSchemeDirty(true);
    scheduleCanvasHistory(historyLabel, historyDelay);
  };

  const updateSelectedConfig = (key: string, value: JsonValue) => {
    if (!selectedNode) return;
    const nextConfig = {
      ...selectedNode.data.config,
      [key]: value,
    };
    updateSelectedNode({ config: nextConfig }, "编辑参数", 700);
    setConfigText(JSON.stringify(nextConfig, null, 2));
  };

  const handleSaveConfig = () => {
    try {
      const parsed = JSON.parse(configText) as JsonValue;
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        setNotice("步骤配置必须是 JSON 对象");
        return;
      }
      updateSelectedNode({ config: parsed as JsonObject }, "应用高级配置", 0);
      setNotice("步骤配置已更新");
    } catch {
      setNotice("步骤配置不是有效 JSON");
    }
  };

  const handleDeleteSelectedNode = () => {
    if (!selectedNodeId) return;
    const deletedNode = nodes.find((node) => node.id === selectedNodeId);
    if (isFixedSystemNode(deletedNode)) {
      setNotice("开始和输出结果节点不能删除");
      return;
    }
    setNodes((current) => current.filter((node) => node.id !== selectedNodeId));
    setEdges((current) =>
      current.filter((edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId),
    );
    setSelectedNodeId(null);
    setValidation({ valid: true, messages: [] });
    setSchemeDirty(true);
    scheduleCanvasHistory("删除节点");
    const nodeLabel = deletedNode ? String(deletedNode.data.label) : "";
    const nodeTypeLabel = deletedNode && isCanvasToolKind(String(deletedNode.data.moduleKind)) ? "画布组件" : "步骤";
    setNotice(nodeLabel ? `已删除${nodeTypeLabel}：${nodeLabel}` : `已删除${nodeTypeLabel}`);
  };

  const appendAssets = (assets: AuditAsset[]) => {
    if (assets.length === 0) return;
    setSelectedAssets((current) => {
      const byPath = new Map(current.map((asset) => [asset.path, asset]));
      for (const asset of assets) {
        byPath.set(asset.path, asset);
      }
      return Array.from(byPath.values());
    });
    setNotice(`已选择 ${assets.length} 个素材`);
  };

  const handleSelectDirectory = async () => {
    try {
      appendAssets(await selectAssetDirectory());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const removeAsset = (assetId: string) => {
    setSelectedAssets((current) => current.filter((asset) => asset.id !== assetId));
  };

  const handleSelectRun = async (run: RunSummary) => {
    await openRunResult(run.id);
  };

  const handleDeleteRun = async (run: RunSummary) => {
    if (!window.confirm(`删除这条历史记录？\n${run.flowName}`)) return;
    setBusy(true);
    try {
      const nextRuns = await deleteRun(run.id);
      setRuns(nextRuns);
      setEntryUnreadRunIds((current) => removeString(current, run.id));
      setHistoryUnreadRunIds((current) => removeString(current, run.id));
      setTaskQueue((current) => current.filter((task) => task.runId !== run.id));
      if (currentRun?.id === run.id) {
        setCurrentRun(null);
        setReport("");
      }
      setNotice("历史记录已删除");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const handleDeleteAllRuns = async () => {
    if (runs.length === 0) return;
    if (!window.confirm("删除所有历史记录？")) return;
    setBusy(true);
    try {
      const nextRuns = await deleteAllRuns();
      setRuns(nextRuns);
      setCurrentRun(null);
      setReport("");
      setEntryUnreadRunIds([]);
      setHistoryUnreadRunIds([]);
      setTaskQueue((current) => current.filter((task) => !task.runId));
      setNotice("所有历史记录已删除");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const handleImportModule = async () => {
    setBusy(true);
    try {
      const nextModules = await importModuleFolder();
      if (!nextModules) {
        setNotice("已取消导入模块");
        return;
      }
      setModules(nextModules);
      setNotice("模块已导入");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const handleOpenModuleFolder = async (module: ModuleInfo) => {
    try {
      await openModuleDefinitionFolder(module.id);
      setNotice(`已打开模块文件夹：${module.name}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const handleRemoveModule = async (module: ModuleInfo) => {
    if (module.source !== "custom") {
      setNotice("只能移除导入的自定义模块");
      return;
    }
    if (nodes.some((node) => String(node.data.moduleId) === module.id)) {
      setNotice(`模块 ${module.name} 已在流程中使用，请先删除相关步骤`);
      return;
    }
    if (!window.confirm(`移除模块“${module.name}”？`)) return;

    setBusy(true);
    try {
      setModules(await removeModule(module.id));
      setNotice(`已移除模块：${module.name}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const handleInstallRuntimeDependency = async (dependency: RuntimeDependencyStatus) => {
    if (dependencyRootHasUnsavedChanges) {
      setNotice("依赖路径已改动，请先保存后再安装");
      return;
    }
    setRuntimeBusyDependency(dependency.id);
    setRuntimeTerminalCollapsed(false);
    setRuntimeLogs((current) => [
      ...current.slice(-799),
      {
        timestamp: Math.floor(Date.now() / 1000),
        scope: dependency.id,
        stream: "info",
        line: `开始安装 ${dependency.name}`,
      },
    ]);
    try {
      setRuntimeStatus(await installRuntimeDependency(dependency.id));
      setNotice(`${dependency.name} 已安装`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setRuntimeBusyDependency(null);
    }
  };

  const handleOpenRuntimeDependencyFolder = async (dependency: RuntimeDependencyStatus) => {
    if (dependencyRootHasUnsavedChanges) {
      setNotice("依赖路径已改动，请先保存后再打开文件夹");
      return;
    }
    try {
      await openRuntimeDependencyFolder(dependency.id);
      setNotice(`已打开 ${dependency.name} 文件夹`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const handleOpenRuntimePythonFolder = async () => {
    try {
      await openRuntimePythonFolder();
      setNotice("已打开 Python 文件夹");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const validationClass = validation.valid ? "validation validation--ok" : "validation";
  const runOverlayHasActiveTask = visibleQueueTasks.length > 0 || runInProgress || Boolean(activeRunId);
  const runBubbleText = runInProgress ? "运行中" : visibleQueueTasks.length > 0 ? "队列" : "运行";
  const hasRunDialogDirectory = runDialogAssets.some((asset) => asset.kind === "directory");
  const runDialogLocked = busy || runStarting;
  const showRunFloatingButton = runOverlayHasActiveTask || activeTab !== "results";
  const performanceSummary = currentRun?.performanceSummary ?? null;
  const performanceSteps = (currentRun?.steps ?? []).filter((step) => step.performance);
  const primaryPerformanceLeader = performanceSummary?.cpuLeader ?? performanceSummary?.durationLeader ?? null;

  return (
    <main className={`app-shell ${runtimeTerminalCollapsed ? "is-terminal-collapsed" : ""}`}>
      <header className="topbar">
        <nav className="main-tabs" aria-label="主功能">
          <button
            type="button"
            className={activeTab === "flow" ? "active" : ""}
            onClick={() => setActiveTab("flow")}
            disabled={runInProgress}
          >
            <SlidersHorizontal size={16} />
            流程设计
          </button>
          <button
            type="button"
            className={activeTab === "results" ? "active" : ""}
            onClick={handleOpenResultsTab}
          >
            <TableProperties size={16} />
            运行结果
            {entryUnreadCount > 0 ? <span className="notification-badge">{entryUnreadCount}</span> : null}
          </button>
          <button
            type="button"
            className={activeTab === "modules" ? "active" : ""}
            onClick={() => setActiveTab("modules")}
          >
            <Database size={16} />
            模块管理
          </button>
          <button
            type="button"
            className={activeTab === "settings" ? "active" : ""}
            onClick={() => setActiveTab("settings")}
          >
            <Settings size={16} />
            设置
          </button>
        </nav>
      </header>

      <section className="workflow-bar" aria-label="审核方案和运行操作">
        <div className="workflow-left">
          <div className="scheme-dropdown" ref={schemeDropdownRef}>
            <button
              type="button"
              className="scheme-dropdown__trigger"
              onClick={() => setSchemeMenuOpen((open) => !open)}
              disabled={busy || runInProgress}
              title={schemePath ?? "当前未保存方案"}
            >
              <FileCheck2 size={16} />
              <span>{schemeDropdownLabel}</span>
              <small>{schemeDirty ? "未保存" : schemePath ? "已保存" : "当前"}</small>
            </button>
            {schemeMenuOpen ? (
              <div className="scheme-dropdown__menu">
                {schemeLibraryItems.length === 0 ? (
                  <div className="scheme-dropdown__empty">方案库为空</div>
                ) : null}
                {schemeLibraryItems.map((item) => {
                  const isActive =
                    schemePath?.replace(/\\/g, "/").toLowerCase() ===
                    item.path.replace(/\\/g, "/").toLowerCase();
                  return (
                    <div className={`scheme-dropdown__row ${isActive ? "is-active" : ""}`} key={item.path}>
                      <button
                        type="button"
                        className="scheme-dropdown__item"
                        onClick={() => {
                          if (isActive) {
                            setSchemeMenuOpen(false);
                            return;
                          }
                          void handleQuickLoadScheme(item.path);
                        }}
                        disabled={busy || runInProgress}
                        aria-current={isActive ? "true" : undefined}
                      >
                        <span>
                          <strong>{item.name}</strong>
                          <small>
                            {isActive
                              ? schemeDirty
                                ? "正在编辑，有未保存修改"
                                : "正在编辑"
                              : item.modifiedAt
                                ? formatDate(item.modifiedAt)
                                : "未知时间"}
                          </small>
                        </span>
                      </button>
                      <button
                        type="button"
                        className="scheme-dropdown__rename"
                        onClick={(event) => {
                          event.stopPropagation();
                          void handleRenameSchemeItem(item);
                        }}
                        disabled={busy || runInProgress}
                        title="重命名"
                        aria-label={`重命名 ${item.name}`}
                      >
                        <Pencil size={15} />
                      </button>
                      <button
                        type="button"
                        className="scheme-dropdown__delete"
                        onClick={(event) => {
                          event.stopPropagation();
                          void handleDeleteSchemeItem(item);
                        }}
                        disabled={busy || runInProgress}
                        aria-label={`删除 ${item.name}`}
                      >
                        <Trash2 size={15} />
                      </button>
                    </div>
                  );
                })}
              </div>
            ) : null}
          </div>
          <button type="button" onClick={handleNewScheme} disabled={busy || runInProgress}>
            <FilePlus2 size={16} />
            创建方案
          </button>
          <button type="button" onClick={handleSaveScheme} disabled={busy || runInProgress}>
            <Save size={16} />
            保存方案
          </button>
        </div>
        <div className="workflow-actions">
          <button type="button" onClick={handleValidate} disabled={busy || runInProgress}>
            <CheckCircle2 size={16} />
            校验
          </button>
          <button className="primary" type="button" onClick={handleOpenRunDialog} disabled={busy}>
            <Play size={16} />
            运行
          </button>
        </div>
      </section>

      <div className="notice-bar">
        <div className={validationClass}>
          {validation.valid ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
          <span>{validation.valid ? "流程有效" : validation.messages.join(" ")}</span>
        </div>
        <span>{notice}</span>
      </div>

      {activeTab === "flow" ? (
        <section className="tab-page flow-workspace">
          <aside className="module-rail">
            <div className="section-title">
              <Database size={16} />
              <span>组件库</span>
            </div>
            <div className="library-tabs" role="tablist" aria-label="组件类型">
              <button
                type="button"
                role="tab"
                aria-selected={activeLibraryTab === "modules"}
                className={activeLibraryTab === "modules" ? "active" : ""}
                onClick={() => setActiveLibraryTab("modules")}
              >
                模块
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={activeLibraryTab === "data"}
                className={activeLibraryTab === "data" ? "active" : ""}
                onClick={() => setActiveLibraryTab("data")}
              >
                数据节点
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={activeLibraryTab === "tools"}
                className={activeLibraryTab === "tools" ? "active" : ""}
                onClick={() => setActiveLibraryTab("tools")}
              >
                工具
              </button>
            </div>
            <div className="module-list">
              {activeLibraryTab !== "tools" && activeLibraryItems.length === 0 ? (
                <div className="module-empty">
                  <FolderInput size={18} />
                  <span>{activeLibraryTab === "modules" ? "暂无可用模块" : "暂无数据节点"}</span>
                </div>
              ) : null}
              {activeLibraryTab === "data" ? (
                dataNodeGroups.map((group) => (
                  <div className="module-group" key={group.id}>
                    <div className="module-group__title">{group.title}</div>
                    <div className="module-group__items">
                      {group.items.map((module) => (
                        <div
                          className="module-tile"
                          draggable={false}
                          key={module.id}
                          onPointerDown={(event) => handleModulePointerDown(event, module)}
                        >
                          <div className="module-tile__main">
                            <ModuleIcon
                              icon={module.icon}
                              iconPath={module.iconPath}
                              iconDataUrl={module.iconDataUrl}
                              size={18}
                            />
                            <div>
                              <strong>{module.name}</strong>
                              <span>{module.summary}</span>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ))
              ) : activeLibraryTab === "tools" ? (
                canvasToolItems.map((tool) => (
                  <div
                    className="module-tile tool-tile"
                    draggable={false}
                    key={tool.type}
                    onPointerDown={(event) => handleToolPointerDown(event, tool)}
                  >
                    <div className="module-tile__main">
                      <ModuleIcon icon={tool.icon} size={18} />
                      <div>
                        <strong>{tool.name}</strong>
                        <span>{tool.summary}</span>
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                activeLibraryItems.map((module) => (
                  <div
                    className="module-tile"
                    draggable={false}
                    key={module.id}
                    onPointerDown={(event) => handleModulePointerDown(event, module)}
                  >
                    <div className="module-tile__main">
                      <ModuleIcon
                        icon={module.icon}
                        iconPath={module.iconPath}
                        iconDataUrl={module.iconDataUrl}
                        size={18}
                      />
                      <div>
                        <strong>{module.name}</strong>
                        <span>{module.summary}</span>
                      </div>
                    </div>
                  </div>
                ))
              )}
              {activeLibraryTab === "modules" ? (
                <button className="module-import-button" type="button" onClick={handleImportModule} disabled={busy}>
                  <FolderInput size={18} />
                  <div>
                    <strong>导入模块</strong>
                    <span>选择模块文件夹</span>
                  </div>
                </button>
              ) : null}
            </div>
          </aside>

          <section className="flow-panel" ref={flowPanelRef}>
            <div className="flow-tools">
              <button
                className="flow-tool-button"
                type="button"
                onClick={handleAutoAlign}
                disabled={busy || nodes.length === 0}
              >
                <SlidersHorizontal size={16} />
                自动对齐
              </button>
              <div className="history-popover">
                <button
                  className="flow-tool-button"
                  type="button"
                  onClick={() => setHistoryOpen((open) => !open)}
                  disabled={historyEntries.length === 0}
                >
                  <History size={16} />
                  操作记录
                </button>
                {historyOpen ? (
                  <div className="history-menu">
                    <div className="history-menu__actions">
                      <button type="button" onClick={handleUndo} disabled={historyIndex <= 0}>
                        <Undo2 size={15} />
                        撤销
                      </button>
                      <button type="button" onClick={handleRedo} disabled={historyIndex >= historyEntries.length - 1}>
                        <Redo2 size={15} />
                        反撤销
                      </button>
                    </div>
                    <div className="history-menu__list">
                      {historyEntries.map((entry, index) => (
                        <button
                          type="button"
                          key={entry.id}
                          className={`history-menu__item ${index === historyIndex ? "is-current" : ""} ${index > historyIndex ? "is-redo" : ""}`}
                          onClick={() => handleHistoryJump(index)}
                        >
                          <span>{index + 1}</span>
                          <div>
                            <strong>{entry.label}</strong>
                            <small>{historyTimeLabel(entry.timestamp)}</small>
                          </div>
                        </button>
                      ))}
                    </div>
                  </div>
                ) : null}
              </div>
            </div>
            <ReactFlow<AuditNode, Edge>
              nodes={nodes}
              edges={edges}
              edgeTypes={edgeTypes}
              nodeTypes={nodeTypes}
              fitView
              onConnect={handleConnect}
              onConnectEnd={handleConnectEnd}
              onClickConnectEnd={handleConnectEnd}
              onDrop={handleDrop}
              onDragOver={handleDragOver}
              onEdgesChange={handleEdgesChange}
              onNodesChange={handleNodeChange}
              onNodesDelete={(deletedNodes) => {
                if (deletedNodes.some((node) => node.id === selectedNodeId)) {
                  setSelectedNodeId(null);
                }
                setValidation({ valid: true, messages: [] });
                setNotice(`已删除 ${deletedNodes.length} 个节点`);
              }}
              onNodeClick={(_, node) => {
                setPendingConnectionMenu(null);
                setSelectedNodeId(node.id);
              }}
              onPaneClick={() => {
                if (Date.now() - pendingConnectionOpenedAtRef.current < 300) return;
                setPendingConnectionMenu(null);
                setSelectedNodeId(null);
              }}
              connectionRadius={36}
              onInit={(instance) => {
                void instance.fitView();
              }}
              deleteKeyCode={["Backspace", "Delete"]}
              panOnDrag={[1, 2]}
              selectionOnDrag
              selectionMode={SelectionMode.Partial}
            >
              <Background gap={28} size={1} />
              <MiniMap pannable zoomable nodeStrokeWidth={3} />
              <Controls />
            </ReactFlow>
            {pendingConnectionMenu ? (
              <div
                className="connection-menu"
                style={{ left: pendingConnectionMenu.x, top: pendingConnectionMenu.y }}
              >
                <div className="connection-menu__header">
                  <strong>连接到</strong>
                  <button type="button" onClick={() => setPendingConnectionMenu(null)}>
                    <XCircle size={15} />
                  </button>
                </div>
                {pendingConnectionMenu.options.length === 0 ? (
                  <div className="connection-menu__empty">没有可连接的入口</div>
                ) : (
                  <>
                    {pendingConnectionMenu.options.some((option) => option.group === "existing") ? (
                      <div className="connection-menu__group">
                        <span>已有节点</span>
                        {pendingConnectionMenu.options
                          .filter((option) => option.group === "existing")
                          .map((option) => (
                            <button
                              type="button"
                              key={option.id}
                              onClick={() => handlePendingConnectionOption(option)}
                            >
                              <strong>{option.label}</strong>
                              <small>{option.detail}</small>
                            </button>
                          ))}
                      </div>
                    ) : null}
                    {pendingConnectionMenu.options.some((option) => option.group === "new") ? (
                      <div className="connection-menu__group">
                        <span>新增节点</span>
                        {pendingConnectionMenu.options
                          .filter((option) => option.group === "new")
                          .map((option) => (
                            <button
                              type="button"
                              key={option.id}
                              onClick={() => handlePendingConnectionOption(option)}
                            >
                              <strong>{option.label}</strong>
                              <small>{option.detail}</small>
                            </button>
                          ))}
                      </div>
                    ) : null}
                  </>
                )}
              </div>
            ) : null}
          </section>

          <aside className="inspector">
            <div className="section-title">
              <FileCheck2 size={16} />
              <span>{selectedCanvasToolNode ? (isCanvasGroupKind(String(selectedCanvasToolNode.data.moduleKind)) ? "分组" : "注释") : "步骤参数"}</span>
            </div>
            {selectedCanvasToolNode && selectedCanvasToolConfig ? (
              <div className="inspector-form">
                <label className="param-field">
                  <span>{isCanvasGroupKind(String(selectedCanvasToolNode.data.moduleKind)) ? "分组名称" : "标题"}</span>
                  <input
                    value={String(selectedCanvasToolConfig.title)}
                    onChange={(event) => {
                      const title = event.target.value;
                      updateSelectedNode({
                        label: title || (isCanvasGroupKind(String(selectedCanvasToolNode.data.moduleKind)) ? "分组" : "注释"),
                        config: {
                          ...selectedCanvasToolConfig,
                          title,
                        },
                      }, "编辑名称", 700);
                    }}
                  />
                </label>
                {isCanvasNoteKind(String(selectedCanvasToolNode.data.moduleKind)) ? (
                  <label className="param-field">
                    <span>正文</span>
                    <textarea
                      className="annotation-editor-textarea"
                      value={String(selectedCanvasToolConfig.text ?? "")}
                      onChange={(event) => updateSelectedConfig("text", event.target.value)}
                      placeholder="写下注释内容"
                    />
                  </label>
                ) : null}
                <label className="param-field">
                  <span>颜色</span>
                  <select
                    value={annotationTone(selectedCanvasToolConfig.color)}
                    onChange={(event) => updateSelectedConfig("color", event.target.value)}
                  >
                    <option value="blue">蓝色</option>
                    <option value="green">绿色</option>
                    <option value="yellow">黄色</option>
                    <option value="red">红色</option>
                    <option value="gray">灰色</option>
                  </select>
                </label>
                <button className="danger" type="button" onClick={handleDeleteSelectedNode}>
                  <Trash2 size={16} />
                  删除{isCanvasGroupKind(String(selectedCanvasToolNode.data.moduleKind)) ? "分组" : "注释"}
                </button>
              </div>
            ) : selectedNode && selectedModule ? (
              <div className="inspector-form">
                <label className="param-field">
                  <span>步骤名称</span>
                  <input
                    value={String(selectedNode.data.label)}
                    readOnly={isFixedSystemNode(selectedNode)}
                    onChange={(event) => updateSelectedNode({ label: event.target.value })}
                  />
                </label>
                <label className="param-field">
                  <span>模块</span>
                  <input value={`${selectedModule.name} · ${moduleSourceLabel(selectedModule.source)}`} readOnly />
                </label>
                {isFixedSystemNode(selectedNode) ? (
                  <div className="system-node-note">
                    这是流程固定节点，用来保证每个审核流都有明确入口和最终输出，不能删除。
                  </div>
                ) : (
                  <>
                    <div className="parameter-stack">
                      {selectedModule.parameters.map((parameter) => (
                        <ParameterField
                          key={parameter.key}
                          parameter={parameter}
                          value={selectedNode.data.config[parameter.key] ?? parameter.defaultValue}
                          onChange={(value) => updateSelectedConfig(parameter.key, value)}
                        />
                      ))}
                    </div>
                    <details className="advanced-config">
                      <summary>高级 JSON</summary>
                      <textarea
                        value={configText}
                        onChange={(event) => setConfigText(event.target.value)}
                        spellCheck={false}
                      />
                      <button type="button" onClick={handleSaveConfig}>
                        <Save size={16} />
                        应用 JSON
                      </button>
                    </details>
                    <button className="danger" type="button" onClick={handleDeleteSelectedNode}>
                      <Trash2 size={16} />
                      删除步骤
                    </button>
                  </>
                )}
              </div>
            ) : (
              <div className="empty-state">
                <XCircle size={18} />
                <span>未选择步骤</span>
              </div>
            )}
          </aside>
        </section>
      ) : null}

      {activeTab === "results" ? (
        <section className="tab-page result-page">
          <div className={`result-main ${highlightRunId && currentRun?.id === highlightRunId ? "result-main--flash" : ""}`}>
            <div className="page-title">
              <h2>运行结果</h2>
              <p>
                {currentRun
                  ? `${currentRun.id} · 素材 ${currentRun.assets.length} 个`
                  : "还没有运行结果"}
              </p>
              {currentRun?.artifactDir ? <p>产物目录：{currentRun.artifactDir}</p> : null}
            </div>
            <section className="performance-panel">
              <div className="section-title">
                <Activity size={16} />
                <span>性能开销</span>
              </div>
              {performanceSummary && performanceSteps.length > 0 ? (
                <>
                  <div className="performance-summary-grid">
                    <div className="performance-summary-card">
                      <Cpu size={17} />
                      <span>最大开销</span>
                      <strong>{primaryPerformanceLeader?.label ?? "暂无"}</strong>
                      <small>
                        {performanceSummary.cpuLeader
                          ? formatCpuTimeMs(performanceSummary.cpuLeader.value)
                          : primaryPerformanceLeader
                            ? formatDurationMs(primaryPerformanceLeader.value)
                            : "暂无"}
                      </small>
                    </div>
                    <div className="performance-summary-card">
                      <Timer size={17} />
                      <span>最耗时</span>
                      <strong>{performanceSummary.durationLeader?.label ?? "暂无"}</strong>
                      <small>{formatDurationMs(performanceSummary.durationLeader?.value ?? 0)}</small>
                    </div>
                    <div className="performance-summary-card">
                      <HardDrive size={17} />
                      <span>最高内存</span>
                      <strong>{performanceSummary.memoryLeader?.label ?? "暂无"}</strong>
                      <small>{formatBytes(performanceSummary.memoryLeader?.value ?? 0)}</small>
                    </div>
                  </div>
                  <div className="performance-note">
                    <span>已采集 {performanceSummary.measuredSteps} 个模块</span>
                    <span>总耗时 {formatDurationMs(performanceSummary.totalDurationMs)}</span>
                    <span>CPU 估算 {formatCpuTimeMs(performanceSummary.totalCpuTimeMs)}</span>
                    <span>NVIDIA GPU：{performanceSummary.gpuAvailable ? "已尝试采集" : "未采集"}</span>
                  </div>
                  <div className="performance-table">
                    <div className="performance-table__head">
                      <span>步骤</span>
                      <span>耗时</span>
                      <span>CPU 占比</span>
                      <span>峰值内存</span>
                      <span>产物</span>
                      <span>GPU</span>
                    </div>
                    {performanceSteps.map((step) => (
                      <div className="performance-table__row" key={`${step.stepId}-performance`}>
                        <span>{step.label}</span>
                        <span>{formatDurationMs(step.performance?.durationMs)}</span>
                        <span>{formatPercent(step.performance?.cpuSharePercent)}</span>
                        <span>{formatBytes(step.performance?.peakMemoryBytes)}</span>
                        <span>{formatBytes(step.performance?.artifactBytes)}</span>
                        <span>{performanceGpuText(step)}</span>
                      </div>
                    ))}
                  </div>
                </>
              ) : (
                <div className="performance-empty">这次运行没有可显示的性能数据。</div>
              )}
            </section>
            <div className="step-table">
              <div className="table-head">
                <span>步骤</span>
                <span>模块</span>
                <span>状态</span>
                <span>结论</span>
                <span>文件</span>
                <span>说明</span>
              </div>
              {(currentRun?.steps ?? []).map((step) => (
                <div className="table-row" key={step.stepId}>
                  <span>{step.label}</span>
                  <span>{step.moduleName}</span>
                  <span>{statusText(step.status)}</span>
                  <span>{verdictText(step.verdict)}</span>
                  <span>
                    {(step.processedFiles ?? 0).toString()} / {(step.matchedFiles ?? 0).toString()}
                  </span>
                  <span>{step.message}</span>
                </div>
              ))}
              {!currentRun ? <div className="table-empty">暂无运行结果</div> : null}
            </div>
            <section className="result-report">
              <div className="section-title">
                <FileText size={16} />
                <span>报告</span>
              </div>
              <MarkdownViewer markdown={report} runId={currentRun?.id} />
            </section>
          </div>
          <aside className="runs-list">
            <div className="section-title">
              <TableProperties size={16} />
              <span>历史记录</span>
              <button
                type="button"
                className="runs-list__clear"
                onClick={() => void handleDeleteAllRuns()}
                disabled={busy || runs.length === 0}
              >
                清空
              </button>
            </div>
            {runs.map((run) => (
              <div
                className={[
                  "run-history-item",
                  currentRun?.id === run.id ? "run-history-item--active" : "",
                  highlightRunId === run.id ? "run-history-item--flash" : "",
                  historyUnreadRunIdSet.has(run.id) ? "run-history-item--unread" : "",
                ].filter(Boolean).join(" ")}
                key={run.id}
              >
                <button type="button" className="run-history-item__main" onClick={() => void handleSelectRun(run)}>
                  <span>
                    {run.flowName}
                    {historyUnreadRunIdSet.has(run.id) ? <i className="run-history-item__badge" aria-label="未查看" /> : null}
                  </span>
                  <small>
                    {formatDate(run.createdAt)} · {verdictText(run.verdict)}
                  </small>
                </button>
                <button
                  type="button"
                  className="run-history-item__delete"
                  onClick={() => void handleDeleteRun(run)}
                  disabled={busy}
                  title="删除历史记录"
                  aria-label={`删除历史记录 ${run.flowName}`}
                >
                  <Trash2 size={15} />
                </button>
              </div>
            ))}
          </aside>
        </section>
      ) : null}

      {activeTab === "modules" ? (
        <section className="tab-page modules-page">
          <div className="page-title page-title--with-actions">
            <div>
              <h2>模块管理</h2>
              <p>这里列出可拖入流程的审核模块，开始和输出结果节点由流程自动维护。</p>
            </div>
            <button type="button" onClick={handleImportModule} disabled={busy}>
              <FolderInput size={16} />
              导入模块
            </button>
          </div>
          <div className="module-cards">
            {customModules.length === 0 ? (
              <div className="module-empty-state">
                <FolderInput size={20} />
                <span>暂无导入模块</span>
              </div>
            ) : null}
            {customModules.map((module) => {
              const moduleInUse = nodes.some((node) => String(node.data.moduleId) === module.id);
              return (
                <article className="module-card" key={module.id}>
                  <div className="module-card__head">
                    <span className="module-card__icon" aria-hidden="true">
                      <ModuleIcon
                        icon={module.icon}
                        iconPath={module.iconPath}
                        iconDataUrl={module.iconDataUrl}
                        size={20}
                      />
                    </span>
                    <div>
                      <h3>{module.name}</h3>
                      <p>{module.summary}</p>
                    </div>
                    <div className="module-card__actions">
                      <span>{moduleSourceLabel(module.source).replace("模块", "")}</span>
                      <button type="button" onClick={() => void handleOpenModuleFolder(module)}>
                        <FolderOpen size={15} />
                        打开文件夹
                      </button>
                      {module.source === "custom" ? (
                        <button
                          className="danger"
                          type="button"
                          onClick={() => void handleRemoveModule(module)}
                          disabled={busy || moduleInUse}
                          title={moduleInUse ? "先从流程里删除相关步骤" : undefined}
                        >
                          <Trash2 size={15} />
                          移除
                        </button>
                      ) : null}
                    </div>
                  </div>
                  <div className="module-folder-row">
                    <span>定义文件夹</span>
                    <code>{module.definitionDir || "未生成"}</code>
                  </div>
                  <div className="module-launch-row">
                    <span>启动方式</span>
                    <strong>{launchTypeLabel(module.launch.launchType)}</strong>
                    <code>{launchDetail(module)}</code>
                    <small>{module.launch.notes}</small>
                  </div>
                  <div className="parameter-table">
                    <div className="parameter-table__head">
                      <span>参数</span>
                      <span>类型</span>
                      <span>默认值</span>
                      <span>说明</span>
                    </div>
                    {module.parameters.map((parameter) => (
                      <div className="parameter-table__row" key={parameter.key}>
                        <span>{parameter.name}</span>
                        <span>{parameter.parameterType}</span>
                        <span>{JSON.stringify(parameter.defaultValue)}</span>
                        <span>{parameter.description}</span>
                      </div>
                    ))}
                  </div>
                </article>
              );
            })}
          </div>
        </section>
      ) : null}

      {activeTab === "settings" ? (
        <section className="tab-page settings-page">
          <div className="settings-shell">
            <aside className="settings-nav" aria-label="设置分类">
              <button
                type="button"
                className={activeSettingsTab === "runtime" ? "active" : ""}
                onClick={() => setActiveSettingsTab("runtime")}
              >
                <HardDrive size={16} />
                运行环境
              </button>
              <button
                type="button"
                className={activeSettingsTab === "app" ? "active" : ""}
                onClick={() => setActiveSettingsTab("app")}
              >
                <FileText size={16} />
                应用信息
              </button>
            </aside>

            <section className="settings-content">
              {activeSettingsTab === "runtime" ? (
                <>
                  <div className="page-title page-title--with-actions">
                    <div>
                      <h2>运行环境</h2>
                      <p>{runtimeStatus ? runtimeSourceText(runtimeStatus.runtimeSource) : "正在读取"}</p>
                    </div>
                    <button type="button" onClick={() => void refreshRuntimeStatus()}>
                      <RefreshCw size={16} />
                      刷新
                    </button>
                  </div>

                  <div className="runtime-overview">
                    <div className="runtime-summary">
                      <span>Python</span>
                      <strong>
                        {runtimeStatus?.pythonInstalled
                          ? runtimeStatus.pythonVersion ?? "已就绪"
                          : "未准备"}
                      </strong>
                      <code>{runtimeStatus?.pythonPath ?? "..."}</code>
                      <button type="button" onClick={() => void handleOpenRuntimePythonFolder()}>
                        <FolderOpen size={15} />
                        打开文件夹
                      </button>
                    </div>
                    <div className="runtime-summary">
                      <span>运行时目录</span>
                      <strong>{runtimeStatus ? runtimeSourceText(runtimeStatus.runtimeSource) : "..."}</strong>
                      <code>{runtimeStatus?.runtimeRoot ?? "..."}</code>
                    </div>
                  </div>

                  <div className="app-info-list__field">
                    <span>依赖存放默认路径</span>
                    <div className="settings-path-row">
                      <input
                        value={dependencyRootDraft}
                        onChange={(event) => setDependencyRootDraft(event.target.value)}
                        placeholder={appSettings?.dependencyRoot ?? "选择依赖存放默认路径"}
                      />
                      <button type="button" onClick={() => void handleSelectDependencyRoot()}>
                        <FolderOpen size={15} />
                        选择
                      </button>
                      <button type="button" onClick={() => void handleSaveDependencyRoot()}>
                        <Save size={15} />
                        保存
                      </button>
                    </div>
                    {dependencyRootHasUnsavedChanges ? (
                      <small>路径已改动，保存后会从新路径重新检查依赖。</small>
                    ) : null}
                  </div>

                  <div className="dependency-grid">
                    {(runtimeStatus?.dependencies ?? []).map((dependency) => {
                      const isBusy = runtimeBusyDependency === dependency.id;
                      const actionLocked = runtimeBusyDependency !== null;
                      return (
                        <article className="dependency-card" key={dependency.id}>
                          <div className="dependency-card__head">
                            <div>
                              <h3>{dependency.name}</h3>
                              <span className={dependency.installed ? "status-pill is-ok" : "status-pill"}>
                                {dependencyStateText(dependency)}
                              </span>
                            </div>
                            <button
                              type="button"
                              onClick={() => void handleOpenRuntimeDependencyFolder(dependency)}
                              disabled={dependencyRootHasUnsavedChanges}
                            >
                              <FolderOpen size={15} />
                              打开文件夹
                            </button>
                          </div>
                          <code>{dependency.folder}</code>
                          <div className="dependency-actions">
                            <button
                              type="button"
                              onClick={() => void handleInstallRuntimeDependency(dependency)}
                              disabled={isBusy || actionLocked || dependencyRootHasUnsavedChanges}
                            >
                              <Download size={15} />
                              {isBusy ? "安装中" : dependency.installed ? "重新安装" : "安装"}
                            </button>
                          </div>
                        </article>
                      );
                    })}
                  </div>
                </>
              ) : (
                <>
                  <div className="page-title">
                    <h2>应用信息</h2>
                    <p>{dataRoot || "..."}</p>
                  </div>
                  <div className="app-info-list">
                    <div className="app-info-list__field">
                      <span>审核产物默认生成路径</span>
                      <div className="settings-path-row">
                        <input
                          value={artifactRootDraft}
                          onChange={(event) => setArtifactRootDraft(event.target.value)}
                          placeholder={appSettings?.artifactRoot ?? "选择审核产物默认生成路径"}
                        />
                        <button type="button" onClick={() => void handleSelectArtifactRoot()}>
                          <FolderOpen size={15} />
                          选择
                        </button>
                        <button type="button" onClick={() => void handleSaveArtifactRoot()}>
                          <Save size={15} />
                          保存
                        </button>
                      </div>
                    </div>
                    <div>
                      <span>数据目录</span>
                      <code>{dataRoot || "..."}</code>
                    </div>
                    <div>
                      <span>运行时目录</span>
                      <code>{runtimeStatus?.runtimeRoot ?? "..."}</code>
                    </div>
                  </div>
                </>
              )}
            </section>
          </div>
        </section>
      ) : null}

      {showRunFloatingButton ? (
        <button
          type="button"
          className={`run-floating-button ${runInProgress ? "run-floating-button--active" : ""}`}
          style={{ left: runBubblePosition.x, top: runBubblePosition.y }}
          onClick={handleRunBubbleClick}
          onPointerDown={handleRunBubblePointerDown}
          aria-label="打开运行中任务"
        >
          <span className="run-floating-button__icon">
            <Play size={20} />
          </span>
          <span>{runBubbleText}</span>
          {entryUnreadCount > 0 ? <b className="run-floating-button__badge">{entryUnreadCount}</b> : null}
          {runInProgress ? <i aria-hidden="true" /> : null}
        </button>
      ) : null}

      {runOverlayOpen ? (
        <div
          className="run-overlay-backdrop"
          role="presentation"
          onClick={() => setRunOverlayOpen(false)}
        >
          <section
            className="run-overlay-panel"
            role="dialog"
            aria-modal="true"
            aria-label="运行中任务"
            onClick={(event) => event.stopPropagation()}
          >
            {runOverlayHasActiveTask ? (
              <section className="run-page run-page--overlay">
                <div className="run-page__header">
                  <div className="page-title">
                    <h2>任务队列</h2>
                    <p>
                      {activeQueueTask
                        ? `正在运行：${activeQueueTask.name}`
                        : visibleQueueTasks.length > 0
                          ? `队列中 ${visibleQueueTasks.length} 个任务`
                          : "没有等待中的任务"}
                    </p>
                  </div>
                  <div className="run-page__actions">
                    <button type="button" onClick={handleCreateTaskFromRunOverlay}>
                      <Play size={16} />
                      新建任务
                    </button>
                    <button type="button" onClick={() => setRunOverlayOpen(false)}>
                      关闭
                    </button>
                  </div>
                </div>
                <div className="run-page__body run-page__body--queue">
                  <aside className="task-queue-panel">
                    <div className="section-title">
                      <TableProperties size={16} />
                      <span>任务</span>
                    </div>
                    <div className="task-queue-list">
                      {visibleQueueTasks.map((task) => {
                        const pendingIndex = taskQueue.filter((item) => item.status === "pending").findIndex((item) => item.id === task.id);
                        const canMoveEarlier = task.status === "pending" && pendingIndex > 0;
                        const isSelected = selectedQueueTask?.id === task.id;
                        const isUnread = Boolean(task.runId && historyUnreadRunIdSet.has(task.runId));
                        return (
                          <div
                            className={[
                              "task-queue-item",
                              `task-queue-item--${task.status}`,
                              isSelected ? "is-selected" : "",
                              dragOverQueueTaskId === task.id ? "is-drag-over" : "",
                            ].filter(Boolean).join(" ")}
                            key={task.id}
                            draggable={task.status === "pending"}
                            onDragStart={(event) => handleQueueTaskDragStart(event, task)}
                            onDragOver={(event) => handleQueueTaskDragOver(event, task)}
                            onDrop={(event) => handleQueueTaskDrop(event, task)}
                            onDragEnd={handleQueueTaskDragEnd}
                          >
                            <button
                              type="button"
                              className="task-queue-item__main"
                              onClick={() => setSelectedQueueTaskId(task.id)}
                            >
                              <span className="task-queue-item__drag" aria-hidden="true">
                                {task.status === "pending" ? <GripVertical size={15} /> : null}
                              </span>
                              <span>
                                <strong>{task.name}</strong>
                                <small>{queueTaskStatusText(task.status)}{task.runId ? ` · ${task.runId}` : ""}</small>
                              </span>
                              {isUnread ? <i className="task-queue-item__badge" aria-label="未查看" /> : null}
                            </button>
                            <div className="task-queue-item__actions">
                              {task.status === "pending" ? (
                                <>
                                  <button
                                    type="button"
                                    onClick={() => handleMoveQueueTaskEarlier(task.id)}
                                    disabled={!canMoveEarlier}
                                    title="提前"
                                    aria-label={`提前 ${task.name}`}
                                  >
                                    <ArrowUp size={14} />
                                  </button>
                                  <button
                                    type="button"
                                    onClick={() => handleDeleteQueueTask(task.id)}
                                    title="删除"
                                    aria-label={`删除 ${task.name}`}
                                  >
                                    <Trash2 size={14} />
                                  </button>
                                </>
                              ) : task.status === "running" ? (
                                <button
                                  type="button"
                                  className="danger"
                                  onClick={handleCancelRun}
                                  disabled={!runInProgress || runCancelling || !activeRunId}
                                  title="中断"
                                  aria-label={`中断 ${task.name}`}
                                >
                                  <XCircle size={14} />
                                </button>
                              ) : (
                                <>
                                  {task.runId ? (
                                    <button
                                      type="button"
                                      onClick={() => handleQuickViewTask(task)}
                                      title="快速查看"
                                      aria-label={`快速查看 ${task.name}`}
                                    >
                                      <Eye size={14} />
                                    </button>
                                  ) : null}
                                  <button
                                    type="button"
                                    onClick={() => handleDeleteQueueTask(task.id)}
                                    title="删除"
                                    aria-label={`删除 ${task.name}`}
                                  >
                                    <Trash2 size={14} />
                                  </button>
                                </>
                              )}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </aside>
                  <section className="run-flow-panel">
                    <div className="run-flow-panel__header">
                      <div>
                        <strong>{selectedQueueTask?.name ?? "未选择任务"}</strong>
                        <span>{selectedQueueTask ? queueTaskStatusText(selectedQueueTask.status) : "请选择左侧任务"}</span>
                      </div>
                      <div className="run-flow-panel__actions">
                        {selectedQueueTask?.runId && isTerminalQueueStatus(selectedQueueTask.status) ? (
                          <button type="button" onClick={() => handleQuickViewTask(selectedQueueTask)}>
                            <Eye size={15} />
                            快速查看
                          </button>
                        ) : null}
                        {selectedQueueTask?.status === "running" ? (
                          <button
                            type="button"
                            className="danger"
                            onClick={handleCancelRun}
                            disabled={!runInProgress || runCancelling || !activeRunId}
                          >
                            <XCircle size={15} />
                            {runCancelling ? "正在中断" : "中断运行"}
                          </button>
                        ) : null}
                      </div>
                    </div>
                    <ReactFlowProvider>
                      <ReactFlow<AuditNode, Edge>
                        nodes={displayRunNodes}
                        edges={displayRunEdges}
                        edgeTypes={edgeTypes}
                        nodeTypes={nodeTypes}
                        fitView
                        nodesDraggable={false}
                        nodesConnectable={false}
                        nodesFocusable={false}
                        edgesFocusable={false}
                        elementsSelectable={false}
                        deleteKeyCode={null}
                        onInit={(instance) => {
                          void instance.fitView();
                        }}
                      >
                        <Background gap={28} size={1} />
                        <MiniMap pannable zoomable nodeStrokeWidth={3} />
                        <Controls />
                      </ReactFlow>
                    </ReactFlowProvider>
                  </section>
                  <aside className="run-side-panel">
                    <div className="section-title">
                      <TableProperties size={16} />
                      <span>步骤状态</span>
                    </div>
                    <div className="run-step-list">
                      {displayRunNodes
                        .filter((node) => !isPassiveCanvasKind(String(node.data.moduleKind)))
                        .map((node) => {
                          const state = selectedQueueTaskNodeStates[node.id] ?? { status: "pending", progress: 0, message: "" };
                          return (
                            <div className="run-step-item" key={node.id}>
                              <div>
                                <strong>{String(node.data.label)}</strong>
                                <span>{runStatusLabel(state.status) || "等待中"}</span>
                              </div>
                              <b>{Math.round((state.progress ?? 0) * 100)}%</b>
                              {state.message ? <small>{state.message}</small> : null}
                            </div>
                          );
                        })}
                    </div>
                  </aside>
                </div>
              </section>
            ) : (
              <div className="run-overlay-empty">
                <Play size={32} />
                <h2>队列为空</h2>
                <p>可以创建一个新的审核任务。</p>
                <button className="primary" type="button" onClick={handleCreateTaskFromRunOverlay}>
                  <Play size={16} />
                  创建任务
                </button>
              </div>
            )}
          </section>
        </div>
      ) : null}

      {completionToasts.length > 0 ? (
        <div className="completion-toast-stack" role="status" aria-live="polite">
          {completionToasts.map((toast) => (
            <section className={`completion-toast completion-toast--${toast.status}`} key={toast.id}>
              <span className="completion-toast__icon" aria-hidden="true">
                <Bell size={16} />
              </span>
              <div>
                <strong>{toast.name}</strong>
                <p>
                  {toast.status === "completed"
                    ? `任务完成，结果：${verdictText(toast.verdict ?? "review")}`
                    : toast.status === "cancelled"
                      ? "任务已中断"
                      : toast.message || "任务失败"}
                </p>
              </div>
              {toast.runId ? (
                <button type="button" onClick={() => void openRunResult(toast.runId ?? "", true)}>
                  快速查看
                </button>
              ) : null}
              <button
                type="button"
                className="completion-toast__close"
                onClick={() => setCompletionToasts((current) => current.filter((item) => item.id !== toast.id))}
                aria-label="关闭提示"
              >
                <XCircle size={15} />
              </button>
            </section>
          ))}
        </div>
      ) : null}

      {runDialogOpen ? (
        <div className="run-dialog-backdrop" role="presentation">
          <section className="run-dialog" role="dialog" aria-modal="true" aria-label="运行审核任务">
            <div className="run-dialog__header">
              <div>
                <h2>运行审核任务</h2>
                <p>选择本次要检测的文件夹和使用的审核方案</p>
              </div>
              <button type="button" onClick={() => setRunDialogOpen(false)} disabled={runDialogLocked}>
                关闭
              </button>
            </div>

            <div className="run-dialog__body">
              <label className={`run-dialog__field ${!hasRunDialogDirectory && runDialogError ? "is-error" : ""}`}>
                <span>要检测的文件夹</span>
                <div className="folder-picker-row">
                  <code>{runDialogAssets[0]?.path ?? "未选择"}</code>
                  <button type="button" onClick={() => void handleSelectRunDialogDirectory()} disabled={runDialogLocked}>
                    <FolderInput size={16} />
                    选择文件夹
                  </button>
                </div>
              </label>

              <label className="run-dialog__field">
                <span>使用的审核方案</span>
                <select
                  value={runDialogSchemePath}
                  onChange={(event) => setRunDialogSchemePath(event.target.value)}
                  disabled={runDialogLocked}
                >
                  <option value={CURRENT_SCHEME_VALUE}>{schemeName || "当前编辑中的方案"}</option>
                  {schemeLibraryItems.map((item) => (
                    <option key={item.path} value={item.path}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </label>

              <label className="run-dialog__field">
                <span>任务名称</span>
                <input
                  value={runTaskName}
                  onChange={(event) => setRunTaskName(event.target.value)}
                  placeholder="输入本次任务名称"
                  disabled={runDialogLocked}
                />
              </label>

              <label className="run-dialog__field">
                <span>任务说明</span>
                <textarea
                  value={runTaskNote}
                  onChange={(event) => setRunTaskNote(event.target.value)}
                  placeholder="可选：输入本次审核说明"
                  disabled={runDialogLocked}
                />
              </label>
              {runDialogError ? (
                <div className="run-dialog__error" role="alert">
                  <AlertTriangle size={16} />
                  <span>{runDialogError}</span>
                </div>
              ) : null}
            </div>

            <div className="run-dialog__actions">
              <button type="button" onClick={() => setRunDialogOpen(false)} disabled={runDialogLocked}>
                取消
              </button>
              <button
                className="primary"
                type="button"
                onClick={() => void handleConfirmRunDialog()}
                disabled={runDialogLocked || !hasRunDialogDirectory}
                title={!hasRunDialogDirectory ? "请先选择要检测的文件夹" : undefined}
              >
                <Play size={16} />
                {runStarting ? "正在加入" : "加入队列"}
              </button>
            </div>
          </section>
        </div>
      ) : null}

      {runtimeTerminalCollapsed ? (
        <button
          type="button"
          className="runtime-terminal-toggle"
          onClick={() => setRuntimeTerminalCollapsed(false)}
          aria-label="展开 Python 输出"
        >
          <PythonMark />
          <span>Python</span>
        </button>
      ) : null}

      <section className="runtime-terminal" aria-label="Python 输出">
        <header>
          <div className="runtime-terminal__title">
            <PythonMark />
            <span>Python 输出</span>
          </div>
          <div className="runtime-terminal__actions">
            <button
              type="button"
              onClick={() => setRuntimeTerminalCollapsed(true)}
              aria-label="折叠 Python 输出"
            >
              折叠
            </button>
            <button type="button" onClick={() => setRuntimeLogs([])} disabled={runtimeLogs.length === 0}>
              清空
            </button>
          </div>
        </header>
        <div className="terminal-lines">
          {runtimeLogs.length === 0 ? (
            <div className="terminal-empty">暂无输出</div>
          ) : (
            runtimeLogs.map((line, index) => (
              <div className={`terminal-line terminal-line--${line.stream}`} key={`${line.timestamp}-${index}`}>
                <span>{formatLogTime(line.timestamp)}</span>
                <strong>{line.scope}</strong>
                <code>{line.line}</code>
              </div>
            ))
          )}
        </div>
      </section>

      {moduleDragPreview ? (
        <div
          className={`module-drag-preview ${moduleDragPreview.overCanvas ? "is-over-canvas" : ""}`}
          style={{
            transform: `translate(${moduleDragPreview.x + 12}px, ${moduleDragPreview.y + 12}px)`,
          }}
        >
          <ModuleIcon
            icon={moduleDragPreview.icon}
            iconPath={moduleDragPreview.iconPath}
            iconDataUrl={moduleDragPreview.iconDataUrl}
            size={16}
          />
          <span>{moduleDragPreview.name}</span>
        </div>
      ) : null}
    </main>
  );
}
