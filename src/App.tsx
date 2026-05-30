import {
  addEdge,
  Background,
  Connection,
  Controls,
  Edge,
  Handle,
  MiniMap,
  Node,
  NodeChange,
  NodeProps,
  Position as HandlePosition,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "@xyflow/react";
import {
  AlertTriangle,
  CheckCircle2,
  ClipboardList,
  Database,
  FileCheck2,
  FileText,
  FolderInput,
  FolderOpen,
  Play,
  Save,
  ScanText,
  ShieldAlert,
  SlidersHorizontal,
  TableProperties,
  Trash2,
  XCircle,
} from "lucide-react";
import {
  DragEvent,
  PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  defaultConfigForModule,
  getDataRoot,
  listModules,
  listRuns,
  loadFlow,
  openModuleDefinitionFolder,
  readRunReport,
  saveFlow,
  selectAssetDirectory,
  selectAssetFiles,
  startRun,
  validateFlow,
} from "./api";
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

type AppTab = "flow" | "results" | "report" | "modules";
type JsonObject = Record<string, JsonValue>;

type AuditNodeData = Record<string, unknown> & {
  label: string;
  moduleId: string;
  moduleName: string;
  moduleKind: string;
  moduleIcon: string;
  source: string;
  config: JsonObject;
};

type AuditNode = Node<AuditNodeData>;

type ModuleDragPreview = {
  name: string;
  x: number;
  y: number;
  overCanvas: boolean;
};

type MarkdownBlock =
  | { type: "heading"; level: number; text: string }
  | { type: "paragraph"; text: string }
  | { type: "list"; items: string[] }
  | { type: "table"; headers: string[]; rows: string[][] }
  | { type: "code"; text: string }
  | { type: "rule" };

const nodeTypes = {
  audit: AuditNodeCard,
};

function iconFor(name: string) {
  if (name === "play-circle") return Play;
  if (name === "file-output") return FileText;
  if (name === "scan-text") return ScanText;
  if (name === "shield-alert") return ShieldAlert;
  return FileCheck2;
}

function verdictText(verdict: string) {
  if (verdict === "pass") return "通过";
  if (verdict === "reject") return "不通过";
  if (verdict === "error") return "失败";
  return "复审";
}

function statusText(status: string) {
  if (status === "system") return "系统节点";
  if (status === "ready") return "已配置";
  if (status === "invalid_model_path") return "路径不可用";
  if (status === "needs_model") return "未配置";
  return "已记录";
}

function formatDate(seconds: number) {
  return new Date(seconds * 1000).toLocaleString("zh-CN", { hour12: false });
}

function asObject(value: JsonValue | undefined): JsonObject {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return value as JsonObject;
}

function moduleById(modules: ModuleInfo[], moduleId: string) {
  return modules.find((module) => module.id === moduleId);
}

function moduleSourceLabel(source: ModuleInfo["source"] | string) {
  if (source === "system") return "流程系统节点";
  if (source === "custom") return "自定义模块";
  return "预置自定义模块";
}

function launchTypeLabel(launchType: string) {
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
  };
}

function configForNode(module: ModuleInfo, config: JsonValue): JsonObject {
  return {
    ...defaultConfigForModule(module),
    ...asObject(config),
  };
}

function toReactNodes(flow: FlowDefinition, modules: ModuleInfo[]): AuditNode[] {
  return flow.nodes.map((node) => {
    const module = moduleById(modules, node.moduleId) ?? fallbackModule(node.moduleId);
    return {
      id: node.id,
      type: "audit",
      position: node.position,
      deletable: module.source !== "system",
      data: {
        label: node.label,
        moduleId: node.moduleId,
        moduleName: module.name,
        moduleKind: module.kind,
        moduleIcon: module.icon,
        source: module.source,
        config: configForNode(module, node.config),
      },
    };
  });
}

function toReactEdges(flow: FlowDefinition): Edge[] {
  return flow.edges.map((edge) => ({
    id: edge.id,
    source: edge.from,
    target: edge.to,
    type: "smoothstep",
  }));
}

function buildFlow(nodes: AuditNode[], edges: Edge[]): FlowDefinition {
  return {
    id: "flow.default.image-audit",
    name: "图片 UGC 默认审核",
    version: 1,
    nodes: nodes.map((node) => ({
      id: node.id,
      moduleId: String(node.data.moduleId),
      label: String(node.data.label),
      position: node.position,
      config: node.data.config,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      from: edge.source,
      to: edge.target,
    })),
  };
}

function modelConfigured(config: JsonObject) {
  return typeof config.modelPath === "string" && config.modelPath.trim().length > 0;
}

function isSystemNode(node: AuditNode | null | undefined) {
  return node?.data.source === "system";
}

function isTableSeparator(line: string) {
  return /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line);
}

function splitTableRow(line: string) {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isBlockStart(line: string, nextLine?: string) {
  return (
    /^#{1,6}\s+/.test(line) ||
    /^-\s+/.test(line) ||
    /^```/.test(line) ||
    /^\s*---+\s*$/.test(line) ||
    (line.trim().startsWith("|") && Boolean(nextLine && isTableSeparator(nextLine)))
  );
}

function parseMarkdown(source: string): MarkdownBlock[] {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const blocks: MarkdownBlock[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    const trimmed = line.trim();

    if (!trimmed) {
      index += 1;
      continue;
    }

    if (/^```/.test(trimmed)) {
      const codeLines: string[] = [];
      index += 1;
      while (index < lines.length && !/^```/.test(lines[index].trim())) {
        codeLines.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) index += 1;
      blocks.push({ type: "code", text: codeLines.join("\n") });
      continue;
    }

    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (heading) {
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2] });
      index += 1;
      continue;
    }

    if (/^\s*---+\s*$/.test(line)) {
      blocks.push({ type: "rule" });
      index += 1;
      continue;
    }

    if (line.trim().startsWith("|") && lines[index + 1] && isTableSeparator(lines[index + 1])) {
      const headers = splitTableRow(line);
      const rows: string[][] = [];
      index += 2;
      while (index < lines.length && lines[index].trim().startsWith("|")) {
        rows.push(splitTableRow(lines[index]));
        index += 1;
      }
      blocks.push({ type: "table", headers, rows });
      continue;
    }

    if (/^-\s+/.test(trimmed)) {
      const items: string[] = [];
      while (index < lines.length && /^-\s+/.test(lines[index].trim())) {
        items.push(lines[index].trim().replace(/^-\s+/, ""));
        index += 1;
      }
      blocks.push({ type: "list", items });
      continue;
    }

    const paragraphLines = [trimmed];
    index += 1;
    while (
      index < lines.length &&
      lines[index].trim() &&
      !isBlockStart(lines[index], lines[index + 1])
    ) {
      paragraphLines.push(lines[index].trim());
      index += 1;
    }
    blocks.push({ type: "paragraph", text: paragraphLines.join(" ") });
  }

  return blocks;
}

function MarkdownViewer({ markdown }: { markdown: string }) {
  const blocks = useMemo(() => parseMarkdown(markdown), [markdown]);

  if (!markdown.trim()) {
    return <div className="markdown-empty">暂无报告</div>;
  }

  return (
    <article className="markdown-viewer">
      {blocks.map((block, index) => {
        if (block.type === "heading") {
          if (block.level === 1) return <h1 key={index}>{block.text}</h1>;
          if (block.level === 2) return <h2 key={index}>{block.text}</h2>;
          if (block.level === 3) return <h3 key={index}>{block.text}</h3>;
          return <h4 key={index}>{block.text}</h4>;
        }
        if (block.type === "paragraph") return <p key={index}>{block.text}</p>;
        if (block.type === "list") {
          return (
            <ul key={index}>
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>{item}</li>
              ))}
            </ul>
          );
        }
        if (block.type === "table") {
          return (
            <div className="markdown-table-wrap" key={index}>
              <table>
                <thead>
                  <tr>
                    {block.headers.map((header, headerIndex) => (
                      <th key={headerIndex}>{header}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {block.rows.map((row, rowIndex) => (
                    <tr key={rowIndex}>
                      {block.headers.map((_, cellIndex) => (
                        <td key={cellIndex}>{row[cellIndex] ?? ""}</td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
        }
        if (block.type === "code") return <pre key={index}>{block.text}</pre>;
        return <hr key={index} />;
      })}
    </article>
  );
}

function AuditNodeCard({ data, selected }: NodeProps<AuditNode>) {
  const Icon = iconFor(data.moduleIcon);
  const configured = modelConfigured(data.config);
  const isSystem = data.source === "system";
  const isStart = data.moduleKind === "flow_start";
  const isOutput = data.moduleKind === "flow_output";
  const pillText = isSystem
    ? isStart
      ? "开始节点"
      : "输出结果"
    : configured
      ? "本地入口已配置"
      : "未配置模型";

  return (
    <div className={`audit-node ${isSystem ? "audit-node--system" : ""} ${selected ? "is-selected" : ""}`}>
      {!isStart ? <Handle type="target" position={HandlePosition.Left} /> : null}
      <div className="audit-node__header">
        <span className="audit-node__icon" aria-hidden="true">
          <Icon size={18} />
        </span>
        <span className="audit-node__title">{data.label}</span>
      </div>
      <div className="audit-node__meta">{data.moduleName}</div>
      <div className="node-source">{moduleSourceLabel(data.source)}</div>
      <div className={isSystem ? "node-pill node-pill--system" : configured ? "node-pill node-pill--ready" : "node-pill"}>
        {pillText}
      </div>
      {!isOutput ? <Handle type="source" position={HandlePosition.Right} /> : null}
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
  const { screenToFlowPosition } = useReactFlow();
  const flowPanelRef = useRef<HTMLElement | null>(null);
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [nodes, setNodes, onNodesChange] = useNodesState<AuditNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [validation, setValidation] = useState<ValidationResult>({ valid: true, messages: [] });
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [currentRun, setCurrentRun] = useState<RunRecord | null>(null);
  const [report, setReport] = useState("");
  const [dataRoot, setDataRoot] = useState("");
  const [inputNote, setInputNote] = useState("");
  const [selectedAssets, setSelectedAssets] = useState<AuditAsset[]>([]);
  const [configText, setConfigText] = useState("{}");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [activeTab, setActiveTab] = useState<AppTab>("flow");
  const [moduleDragPreview, setModuleDragPreview] = useState<ModuleDragPreview | null>(null);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId],
  );
  const selectedModule = selectedNode
    ? moduleById(modules, String(selectedNode.data.moduleId))
    : null;

  const hydrate = useCallback(async () => {
    setBusy(true);
    try {
      const [nextModules, flow, nextRuns, root] = await Promise.all([
        listModules(),
        loadFlow(),
        listRuns(),
        getDataRoot(),
      ]);
      setModules(nextModules);
      setNodes(toReactNodes(flow, nextModules));
      setEdges(toReactEdges(flow));
      setRuns(nextRuns);
      setDataRoot(root);
      setNotice("已加载");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [setEdges, setNodes]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    setNodes((current) =>
      current.map((node) => {
        const module = moduleById(modules, String(node.data.moduleId));
        if (!module) return node;
        return {
          ...node,
          deletable: module.source !== "system",
          data: {
            ...node.data,
            moduleName: module.name,
            moduleKind: module.kind,
            moduleIcon: module.icon,
            source: module.source,
            config: {
              ...defaultConfigForModule(module),
              ...node.data.config,
            },
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

  const currentFlow = useCallback(() => buildFlow(nodes, edges), [nodes, edges]);

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
      setNodes(toReactNodes(flow, modules));
      setEdges(toReactEdges(flow));
      setValidation({ valid: true, messages: [] });
      setNotice("流程已保存");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [currentFlow, modules, setEdges, setNodes]);

  const handleRun = useCallback(async () => {
    setBusy(true);
    try {
      const result = await handleValidate();
      if (!result.valid) return;
      const run = await startRun(currentFlow(), inputNote, selectedAssets);
      setCurrentRun(run);
      setReport(await readRunReport(run.id));
      setRuns(await listRuns());
      setActiveTab("report");
      setNotice("运行完成");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [currentFlow, handleValidate, inputNote, selectedAssets]);

  const handleConnect = useCallback(
    (connection: Connection) => {
      const source = nodes.find((node) => node.id === connection.source);
      const target = nodes.find((node) => node.id === connection.target);
      if (source?.data.moduleKind === "flow_output") {
        setNotice("输出结果节点不能再连接到其他步骤");
        return;
      }
      if (target?.data.moduleKind === "flow_start") {
        setNotice("开始节点不能有输入连线");
        return;
      }
      setEdges((current) =>
        addEdge(
          {
            ...connection,
            id: `edge_${connection.source}_${connection.target}_${Date.now()}`,
            type: "smoothstep",
          },
          current,
        ),
      );
    },
    [nodes, setEdges],
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
      if (module.source === "system") return;
      const position = screenToFlowPosition({ x: clientX, y: clientY });
      const id = `${module.kind}_${Date.now()}`;
      const nextNode: AuditNode = {
        id,
        type: "audit",
        position,
        deletable: true,
        data: {
          label: module.name,
          moduleId: module.id,
          moduleName: module.name,
          moduleKind: module.kind,
          moduleIcon: module.icon,
          source: module.source,
          config: defaultConfigForModule(module),
        },
      };
      setNodes((current) => [...current, nextNode]);
      setSelectedNodeId(id);
      setValidation({ valid: true, messages: [] });
      setNotice(`已添加步骤：${module.name}`);
    },
    [screenToFlowPosition, setNodes],
  );

  const handleModulePointerDown = (
    event: ReactPointerEvent<HTMLDivElement>,
    module: ModuleInfo,
  ) => {
    if (event.button !== 0 || module.source === "system") return;
    event.preventDefault();

    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const updatePreview = (clientX: number, clientY: number) => {
      const overCanvas = isPointInFlowPanel(clientX, clientY);
      setModuleDragPreview({
        name: module.name,
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

  const handleDrop = useCallback(
    (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const moduleId = event.dataTransfer.getData("application/ugc-module");
      const module = moduleById(modules, moduleId);
      if (!module) return;
      if (module.source === "system") return;
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
        return !isSystemNode(node);
      });
      if (filtered.length !== changes.length) {
        setNotice("开始和输出结果节点不能删除");
      }
      onNodesChange(filtered);
    },
    [nodes, onNodesChange],
  );

  const updateSelectedNode = (patch: Partial<AuditNodeData>) => {
    if (!selectedNodeId) return;
    setNodes((current) =>
      current.map((node) =>
        node.id === selectedNodeId ? { ...node, data: { ...node.data, ...patch } } : node,
      ),
    );
  };

  const updateSelectedConfig = (key: string, value: JsonValue) => {
    if (!selectedNode) return;
    const nextConfig = {
      ...selectedNode.data.config,
      [key]: value,
    };
    updateSelectedNode({ config: nextConfig });
    setConfigText(JSON.stringify(nextConfig, null, 2));
  };

  const handleSaveConfig = () => {
    try {
      const parsed = JSON.parse(configText) as JsonValue;
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        setNotice("步骤配置必须是 JSON 对象");
        return;
      }
      updateSelectedNode({ config: parsed as JsonObject });
      setNotice("步骤配置已更新");
    } catch {
      setNotice("步骤配置不是有效 JSON");
    }
  };

  const handleDeleteSelectedNode = () => {
    if (!selectedNodeId) return;
    const deletedNode = nodes.find((node) => node.id === selectedNodeId);
    if (isSystemNode(deletedNode)) {
      setNotice("开始和输出结果节点不能删除");
      return;
    }
    setNodes((current) => current.filter((node) => node.id !== selectedNodeId));
    setEdges((current) =>
      current.filter((edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId),
    );
    setSelectedNodeId(null);
    setValidation({ valid: true, messages: [] });
    setNotice(deletedNode ? `已删除步骤：${deletedNode.data.label}` : "已删除步骤");
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

  const handleSelectFiles = async () => {
    try {
      appendAssets(await selectAssetFiles());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
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
    setActiveTab("report");
    setReport(await readRunReport(run.id));
  };

  const handleOpenModuleFolder = async (module: ModuleInfo) => {
    try {
      await openModuleDefinitionFolder(module.id);
      setNotice(`已打开模块文件夹：${module.name}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const validationClass = validation.valid ? "validation validation--ok" : "validation";

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <ClipboardList size={22} />
          <div>
            <h1>UGCAudit</h1>
            <p>{dataRoot || "..."}</p>
          </div>
        </div>
        <nav className="main-tabs" aria-label="主功能">
          <button
            type="button"
            className={activeTab === "flow" ? "active" : ""}
            onClick={() => setActiveTab("flow")}
          >
            <SlidersHorizontal size={16} />
            流程设计
          </button>
          <button
            type="button"
            className={activeTab === "results" ? "active" : ""}
            onClick={() => setActiveTab("results")}
          >
            <TableProperties size={16} />
            运行结果
          </button>
          <button
            type="button"
            className={activeTab === "report" ? "active" : ""}
            onClick={() => setActiveTab("report")}
          >
            <FileText size={16} />
            Markdown 报告
          </button>
          <button
            type="button"
            className={activeTab === "modules" ? "active" : ""}
            onClick={() => setActiveTab("modules")}
          >
            <Database size={16} />
            模块管理
          </button>
        </nav>
        <label className="input-note">
          <span>补充说明</span>
          <input
            value={inputNote}
            onChange={(event) => setInputNote(event.target.value)}
            placeholder="可选：输入本次审核说明"
          />
        </label>
        <div className="asset-actions">
          <button type="button" onClick={handleSelectFiles} disabled={busy}>
            <FileText size={16} />
            选择文件
          </button>
          <button type="button" onClick={handleSelectDirectory} disabled={busy}>
            <FolderInput size={16} />
            选择文件夹
          </button>
          <button type="button" onClick={() => setSelectedAssets([])} disabled={busy || selectedAssets.length === 0}>
            清空
          </button>
        </div>
        <div className="topbar-actions">
          <button type="button" onClick={handleValidate} disabled={busy}>
            <CheckCircle2 size={16} />
            校验
          </button>
          <button type="button" onClick={handleSaveFlow} disabled={busy}>
            <Save size={16} />
            保存
          </button>
          <button className="primary" type="button" onClick={handleRun} disabled={busy}>
            <Play size={16} />
            运行
          </button>
        </div>
      </header>

      <div className="notice-bar">
        <div className={validationClass}>
          {validation.valid ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
          <span>{validation.valid ? "流程有效" : validation.messages.join(" ")}</span>
        </div>
        <div className="asset-list">
          {selectedAssets.length === 0 ? (
            <span>未选择本地素材</span>
          ) : (
            selectedAssets.map((asset) => (
              <button type="button" className="asset-chip" key={asset.id} onClick={() => removeAsset(asset.id)}>
                <span>{asset.kind === "directory" ? "文件夹" : "文件"}</span>
                <strong>{asset.name}</strong>
                <XCircle size={13} />
              </button>
            ))
          )}
        </div>
        <span>{notice}</span>
      </div>

      {activeTab === "flow" ? (
        <section className="tab-page flow-workspace">
          <aside className="module-rail">
            <div className="section-title">
              <Database size={16} />
              <span>预置自定义模块</span>
            </div>
            <div className="module-list">
              {modules.filter((module) => module.source !== "system").map((module) => {
                const Icon = iconFor(module.icon);
                return (
                  <div
                    className="module-tile"
                    draggable={false}
                    key={module.id}
                    onPointerDown={(event) => handleModulePointerDown(event, module)}
                  >
                    <div className="module-tile__main">
                      <Icon size={18} />
                      <div>
                        <strong>{module.name}</strong>
                        <span>{module.summary}</span>
                      </div>
                    </div>
                    <span className="module-badge">{module.source === "custom" ? "custom" : "preset"}</span>
                  </div>
                );
              })}
              <div className="module-tile module-tile--disabled">
                <FolderInput size={18} />
                <div>
                  <strong>导入自定义模块</strong>
                  <span>下一阶段接入</span>
                </div>
              </div>
            </div>
          </aside>

          <section className="flow-panel" ref={flowPanelRef}>
            <ReactFlow<AuditNode, Edge>
              nodes={nodes}
              edges={edges}
              nodeTypes={nodeTypes}
              fitView
              onConnect={handleConnect}
              onDrop={handleDrop}
              onDragOver={handleDragOver}
              onEdgesChange={onEdgesChange}
              onNodesChange={handleNodeChange}
              onNodesDelete={(deletedNodes) => {
                if (deletedNodes.some((node) => node.id === selectedNodeId)) {
                  setSelectedNodeId(null);
                }
                setValidation({ valid: true, messages: [] });
                setNotice(`已删除 ${deletedNodes.length} 个步骤`);
              }}
              onNodeClick={(_, node) => setSelectedNodeId(node.id)}
              onPaneClick={() => setSelectedNodeId(null)}
              onInit={(instance) => {
                void instance.fitView();
              }}
              deleteKeyCode={["Backspace", "Delete"]}
            >
              <Background gap={28} size={1} />
              <MiniMap pannable zoomable nodeStrokeWidth={3} />
              <Controls />
            </ReactFlow>
          </section>

          <aside className="inspector">
            <div className="section-title">
              <FileCheck2 size={16} />
              <span>步骤参数</span>
            </div>
            {selectedNode && selectedModule ? (
              <div className="inspector-form">
                <label className="param-field">
                  <span>步骤名称</span>
                  <input
                    value={String(selectedNode.data.label)}
                    readOnly={isSystemNode(selectedNode)}
                    onChange={(event) => updateSelectedNode({ label: event.target.value })}
                  />
                </label>
                <label className="param-field">
                  <span>模块</span>
                  <input value={`${selectedModule.name} · ${moduleSourceLabel(selectedModule.source)}`} readOnly />
                </label>
                {isSystemNode(selectedNode) ? (
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
          <div className="result-main">
            <div className="page-title">
              <h2>运行结果</h2>
              <p>
                {currentRun
                  ? `${currentRun.id} · 素材 ${currentRun.assets.length} 个`
                  : "还没有运行结果"}
              </p>
            </div>
            <div className="step-table">
              <div className="table-head">
                <span>步骤</span>
                <span>模块</span>
                <span>状态</span>
                <span>结论</span>
                <span>说明</span>
              </div>
              {(currentRun?.steps ?? []).map((step) => (
                <div className="table-row" key={step.stepId}>
                  <span>{step.label}</span>
                  <span>{step.moduleName}</span>
                  <span>{statusText(step.status)}</span>
                  <span>{verdictText(step.verdict)}</span>
                  <span>{step.message}</span>
                </div>
              ))}
              {!currentRun ? <div className="table-empty">暂无运行结果</div> : null}
            </div>
          </div>
          <aside className="runs-list">
            <div className="section-title">
              <TableProperties size={16} />
              <span>历史记录</span>
            </div>
            {runs.map((run) => (
              <button type="button" key={run.id} onClick={() => void handleSelectRun(run)}>
                <span>{run.flowName}</span>
                <small>
                  {formatDate(run.createdAt)} · {verdictText(run.verdict)}
                </small>
              </button>
            ))}
          </aside>
        </section>
      ) : null}

      {activeTab === "report" ? (
        <section className="tab-page report-page">
          <MarkdownViewer markdown={report} />
        </section>
      ) : null}

      {activeTab === "modules" ? (
        <section className="tab-page modules-page">
          <div className="page-title">
            <h2>模块管理</h2>
            <p>这里列出可拖入流程的审核模块，开始和输出结果节点由流程自动维护。</p>
          </div>
          <div className="module-cards">
            {modules.filter((module) => module.source !== "system").map((module) => {
              const Icon = iconFor(module.icon);
              return (
                <article className="module-card" key={module.id}>
                  <div className="module-card__head">
                    <Icon size={20} />
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

      {moduleDragPreview ? (
        <div
          className={`module-drag-preview ${moduleDragPreview.overCanvas ? "is-over-canvas" : ""}`}
          style={{
            transform: `translate(${moduleDragPreview.x + 12}px, ${moduleDragPreview.y + 12}px)`,
          }}
        >
          <FileCheck2 size={16} />
          <span>{moduleDragPreview.name}</span>
        </div>
      ) : null}
    </main>
  );
}
