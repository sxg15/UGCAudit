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
  builtIn: boolean;
  source: "system" | "preset" | "custom";
  definitionDir: string;
  modelPath?: string | null;
  modelConfigured: boolean;
  launch: ModuleLaunch;
  parameters: ModuleParameter[];
};

export type ModuleParameterOption = {
  label: string;
  value: string;
};

export type ModuleParameter = {
  key: string;
  name: string;
  description: string;
  parameterType: "string" | "number" | "boolean" | "select" | "multiSelect" | "path" | "textarea";
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
};

export type FlowDefinition = {
  id: string;
  name: string;
  version: number;
  nodes: FlowNodeDefinition[];
  edges: FlowEdgeDefinition[];
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
  reportSection: string;
};

export type RunRecord = {
  id: string;
  flowId: string;
  flowName: string;
  createdAt: number;
  status: string;
  verdict: string;
  inputNote: string;
  assets: AuditAsset[];
  dataRoot: string;
  runDir: string;
  resourceRoot: string;
  reportPath: string;
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
