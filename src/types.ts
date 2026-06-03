export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type ModuleLaunch = {
  launchType: string;
  command?: string | null;
  url?: string | null;
  method?: string | null;
  args: string[];
  notes: string;
};

export type ModuleInfo = {
  id: string;
  name: string;
  kind: string;
  summary: string;
  modelLabel: string;
  icon: string;
  iconPath?: string | null;
  iconDataUrl?: string | null;
  builtIn: boolean;
  source: "system" | "preset" | "custom";
  definitionDir: string;
  modelPath?: string | null;
  modelConfigured: boolean;
  launch: ModuleLaunch;
  parameters: ModuleParameter[];
  dataOutputs?: ModuleDataOutput[];
};

export type ModuleDataOutput = {
  handle: string;
  name: string;
  dataType: "imageCollection" | "textCollection" | "folder";
};

export type ModuleParameterOption = {
  label: string;
  value: string;
};

export type ModuleParameter = {
  key: string;
  name: string;
  description: string;
  parameterType: "string" | "number" | "boolean" | "select" | "multiSelect" | "stringList" | "policyList" | "path" | "textarea";
  defaultValue: JsonValue;
  required: boolean;
  options: ModuleParameterOption[];
};

export type FlowNodeDefinition = {
  id: string;
  moduleId: string;
  label: string;
  position: {
    x: number;
    y: number;
  };
  config: JsonValue;
};

export type FlowEdgeDefinition = {
  id: string;
  from: string;
  to: string;
  edgeType?: "sequence" | "data";
  fromHandle?: string | null;
  toHandle?: string | null;
};

export type FlowDefinition = {
  id: string;
  name: string;
  version: number;
  nodes: FlowNodeDefinition[];
  edges: FlowEdgeDefinition[];
};

export type AuditScheme = {
  schemaVersion: number;
  kind: "ugcAuditScheme";
  id: string;
  name: string;
  flow: FlowDefinition;
};

export type SchemeListItem = {
  id: string;
  name: string;
  path: string;
  modifiedAt?: number | null;
};

export type SavedAuditScheme = {
  path: string;
  scheme: AuditScheme;
};

export type ValidationResult = {
  valid: boolean;
  messages: string[];
};

export type AuditAsset = {
  id: string;
  kind: "file" | "directory" | "note";
  path: string;
  name: string;
  extension: string;
};

export type StepRun = {
  stepId: string;
  moduleId: string;
  moduleName: string;
  label: string;
  status: string;
  verdict: string;
  message: string;
  executionGroup: number;
  progress?: number;
  processedFiles?: number;
  matchedFiles?: number;
  artifactCount?: number;
  performance?: StepPerformance | null;
  reportSection: string;
};

export type StepPerformance = {
  startTime: number;
  endTime: number;
  durationMs: number;
  sampleCount: number;
  cpuTimeMs: number;
  cpuSharePercent: number;
  averageCpuPercent: number;
  peakCpuPercent: number;
  peakMemoryBytes: number;
  artifactBytes: number;
  gpuAvailable: boolean;
  gpuSampleCount: number;
  peakGpuMemoryBytes?: number | null;
  samplingNote: string;
};

export type PerformanceLeader = {
  stepId: string;
  label: string;
  moduleName: string;
  value: number;
};

export type RunPerformanceSummary = {
  totalDurationMs: number;
  totalCpuTimeMs: number;
  totalArtifactBytes: number;
  measuredSteps: number;
  gpuAvailable: boolean;
  gpuSampled: boolean;
  cpuLeader?: PerformanceLeader | null;
  durationLeader?: PerformanceLeader | null;
  memoryLeader?: PerformanceLeader | null;
  samplingNote: string;
};

export type RunRecord = {
  id: string;
  flowId: string;
  flowName: string;
  createdAt: number;
  status: string;
  verdict: string;
  taskName?: string;
  inputNote: string;
  assets: AuditAsset[];
  dataRoot: string;
  runDir: string;
  resourceRoot: string;
  artifactRoot?: string;
  artifactDir?: string;
  reportPath: string;
  performanceSummary?: RunPerformanceSummary | null;
  steps: StepRun[];
};

export type RunSummary = {
  id: string;
  flowName: string;
  createdAt: number;
  status: string;
  verdict: string;
  reportPath: string;
};

export type RunStartResponse = {
  runId: string;
};

export type RunProgressEvent = {
  runId: string;
  nodeId?: string | null;
  status: string;
  progress?: number | null;
  message: string;
  processed?: number | null;
  total?: number | null;
  step?: StepRun | null;
  run?: RunRecord | null;
};

export type RuntimeDependencyStatus = {
  id: "torch" | "transformers" | "pillow" | "accelerate";
  name: string;
  installed: boolean;
  version?: string | null;
  folder: string;
  sitePackages: string;
};

export type RuntimeStatus = {
  runtimeRoot: string;
  runtimeSource: "program" | "data" | "override" | string;
  dependencyRoot: string;
  pythonDir: string;
  pythonPath: string;
  pythonInstalled: boolean;
  pythonVersion?: string | null;
  dependencies: RuntimeDependencyStatus[];
};

export type RuntimeLogLine = {
  timestamp: number;
  scope: string;
  stream: "stdout" | "stderr" | "info" | "error" | string;
  line: string;
};

export type AppSettings = {
  artifactRoot: string;
  dependencyRoot: string;
};
