use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    id: String,
    name: String,
    kind: String,
    summary: String,
    model_label: String,
    icon: String,
    built_in: bool,
    source: String,
    #[serde(default)]
    definition_dir: String,
    model_path: Option<String>,
    model_configured: bool,
    parameters: Vec<ModuleParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleParameterOption {
    label: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleParameter {
    key: String,
    name: String,
    description: String,
    parameter_type: String,
    default_value: Value,
    required: bool,
    options: Vec<ModuleParameterOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    id: String,
    module_id: String,
    label: String,
    position: Position,
    #[serde(default)]
    config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    id: String,
    from: String,
    to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDefinition {
    id: String,
    name: String,
    version: u32,
    nodes: Vec<FlowNode>,
    edges: Vec<FlowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    valid: bool,
    messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRun {
    step_id: String,
    module_id: String,
    module_name: String,
    label: String,
    status: String,
    verdict: String,
    message: String,
    execution_group: usize,
    outputs: Value,
    report_section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditAsset {
    id: String,
    kind: String,
    path: String,
    name: String,
    extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    id: String,
    flow_id: String,
    flow_name: String,
    created_at: u64,
    status: String,
    verdict: String,
    input_note: String,
    #[serde(default)]
    assets: Vec<AuditAsset>,
    data_root: String,
    run_dir: String,
    report_path: String,
    steps: Vec<StepRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    id: String,
    flow_name: String,
    created_at: u64,
    status: String,
    verdict: String,
    report_path: String,
}

type ModelPaths = HashMap<String, String>;

const START_NODE_ID: &str = "flow_start";
const OUTPUT_NODE_ID: &str = "flow_output";
const START_MODULE_ID: &str = "system.start";
const OUTPUT_MODULE_ID: &str = "system.output";

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn data_root(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("data")
}

fn ensure_data_dirs(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let root = data_root(app);
    for dir in [
        root.join("flows"),
        root.join("settings"),
        root.join("runs"),
        root.join("modules"),
    ] {
        fs::create_dir_all(&dir)
            .map_err(|error| format!("无法创建数据目录 {}: {error}", dir.display()))?;
    }
    Ok(root)
}

fn read_json<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("无法读取 {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("无法解析 {}: {error}", path.display()))
}

fn read_required_json<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)
        .map_err(|error| format!("无法读取 {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("无法解析 {}: {error}", path.display()))
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建目录 {}: {error}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("无法序列化 {}: {error}", path.display()))?;
    fs::write(path, content).map_err(|error| format!("无法写入 {}: {error}", path.display()))
}

fn option(label: &str, value: &str) -> ModuleParameterOption {
    ModuleParameterOption {
        label: label.to_string(),
        value: value.to_string(),
    }
}

fn param(
    key: &str,
    name: &str,
    description: &str,
    parameter_type: &str,
    default_value: Value,
    required: bool,
    options: Vec<ModuleParameterOption>,
) -> ModuleParameter {
    ModuleParameter {
        key: key.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        parameter_type: parameter_type.to_string(),
        default_value,
        required,
        options,
    }
}

fn module_default_config(module: &ModuleInfo) -> Value {
    let mut map = serde_json::Map::new();
    for parameter in &module.parameters {
        map.insert(parameter.key.clone(), parameter.default_value.clone());
    }
    Value::Object(map)
}

fn model_path_from_config(config: &Value) -> Option<String> {
    config
        .get("modelPath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
}

fn normalize_module_id(module_id: &str) -> String {
    match module_id {
        "builtin.paddleocr" => "preset.custom.paddleocr",
        "builtin.shieldgemma2" => "preset.custom.shieldgemma2",
        "builtin.qwen3guard" => "preset.custom.qwen3guard",
        _ => module_id,
    }
    .to_string()
}

fn merge_config(defaults: &Value, config: &Value) -> Value {
    match (defaults, config) {
        (Value::Object(default_map), Value::Object(config_map)) => {
            let mut merged = default_map.clone();
            for (key, value) in config_map {
                merged.insert(key.clone(), value.clone());
            }
            Value::Object(merged)
        }
        _ => defaults.clone(),
    }
}

fn builtin_module_definitions() -> Vec<ModuleInfo> {
    vec![
        ModuleInfo {
            id: START_MODULE_ID.to_string(),
            name: "开始".to_string(),
            kind: "flow_start".to_string(),
            summary: "流程入口，接收本次审核素材".to_string(),
            model_label: "无需模型".to_string(),
            icon: "play-circle".to_string(),
            built_in: true,
            source: "system".to_string(),
            definition_dir: String::new(),
            model_path: None,
            model_configured: true,
            parameters: vec![],
        },
        ModuleInfo {
            id: OUTPUT_MODULE_ID.to_string(),
            name: "输出结果".to_string(),
            kind: "flow_output".to_string(),
            summary: "汇总所有步骤并生成 Markdown 报告".to_string(),
            model_label: "无需模型".to_string(),
            icon: "file-output".to_string(),
            built_in: true,
            source: "system".to_string(),
            definition_dir: String::new(),
            model_path: None,
            model_configured: true,
            parameters: vec![],
        },
        ModuleInfo {
            id: "preset.custom.paddleocr".to_string(),
            name: "图片文字识别".to_string(),
            kind: "image_ocr".to_string(),
            summary: "预置自定义模块，面向 PaddleOCR 本地运行入口".to_string(),
            model_label: "PaddleOCR 本地目录".to_string(),
            icon: "scan-text".to_string(),
            built_in: true,
            source: "preset".to_string(),
            definition_dir: String::new(),
            model_path: None,
            model_configured: false,
            parameters: vec![
                param("modelPath", "PaddleOCR 本地目录", "PaddleOCR 模型或运行环境所在目录。", "path", json!(""), true, vec![]),
                param("profile", "识别模式", "mobile 速度更快，server 更适合高精度。", "select", json!("mobile"), true, vec![option("mobile", "mobile"), option("server", "server")]),
                param("language", "识别语言", "传给模块的语言代码。", "select", json!("ch"), true, vec![option("中文", "ch"), option("英文", "en"), option("多语言", "multi")]),
                param("minConfidence", "最低置信度", "低于该值的文字会被标记为低可信。", "number", json!(0.5), false, vec![]),
                param("drawBoxes", "输出标注图", "是否要求模块输出 OCR 标注图片。", "boolean", json!(true), false, vec![]),
            ],
        },
        ModuleInfo {
            id: "preset.custom.shieldgemma2".to_string(),
            name: "图片合规检测".to_string(),
            kind: "image_safety".to_string(),
            summary: "预置自定义模块，面向 ShieldGemma 2 本地入口".to_string(),
            model_label: "ShieldGemma 2 模型目录".to_string(),
            icon: "shield-alert".to_string(),
            built_in: true,
            source: "preset".to_string(),
            definition_dir: String::new(),
            model_path: None,
            model_configured: false,
            parameters: vec![
                param("modelPath", "ShieldGemma 2 模型目录", "ShieldGemma 2 本地模型目录。", "path", json!(""), true, vec![]),
                param("policies", "检测策略", "模块需要检测的图片风险类别。", "multiSelect", json!(["sexual", "violence_gore", "dangerous"]), true, vec![option("色情", "sexual"), option("暴力/血腥", "violence_gore"), option("危险内容", "dangerous")]),
                param("threshold", "风险阈值", "高于该分值时进入人工复审。", "number", json!(0.7), false, vec![]),
                param("policyPrompt", "策略说明", "传给模型的策略文本，可按业务调整。", "textarea", json!("检查图片是否包含色情、暴力血腥或危险内容。"), false, vec![]),
            ],
        },
        ModuleInfo {
            id: "preset.custom.qwen3guard".to_string(),
            name: "文本合规检测".to_string(),
            kind: "text_safety".to_string(),
            summary: "预置自定义模块，面向 Qwen3Guard 本地入口".to_string(),
            model_label: "Qwen3Guard 模型目录".to_string(),
            icon: "file-check".to_string(),
            built_in: true,
            source: "preset".to_string(),
            definition_dir: String::new(),
            model_path: None,
            model_configured: false,
            parameters: vec![
                param("modelPath", "Qwen3Guard 模型目录", "Qwen3Guard 本地模型目录。", "path", json!(""), true, vec![]),
                param("input", "输入来源", "文本来源字段，例如 OCR 全文。", "string", json!("$steps.image_ocr.outputs.fullText"), true, vec![]),
                param("modelSize", "模型尺寸", "传给模块的模型尺寸标记。", "select", json!("0.6b"), true, vec![option("0.6B", "0.6b"), option("4B", "4b"), option("8B", "8b")]),
                param("categories", "风险类别", "文本模块要关注的风险类别。", "multiSelect", json!(["sexual", "violence", "illegal", "privacy"]), false, vec![option("色情", "sexual"), option("暴力", "violence"), option("违法", "illegal"), option("隐私", "privacy"), option("自伤", "self_harm")]),
                param("rejectUnsafe", "Unsafe 直接拒绝", "模型返回 Unsafe 时是否直接判为不通过。", "boolean", json!(true), false, vec![]),
            ],
        },
    ]
}

fn module_folder(root: &Path, module_id: &str) -> PathBuf {
    root.join("modules").join(module_id)
}

fn module_definition_file(folder: &Path) -> PathBuf {
    folder.join("module.json")
}

fn module_readme(module: &ModuleInfo) -> String {
    format!(
        "# {}\n\n{}\n\n- 模块 ID：{}\n- 类型：{}\n- 来源：{}\n\n这个文件夹是模块定义目录。`module.json` 描述模块参数和入口信息。\n",
        module.name, module.summary, module.id, module.kind, module.source
    )
}

fn write_module_definition(path: &Path, module: &ModuleInfo) -> Result<(), String> {
    let mut value = serde_json::to_value(module)
        .map_err(|error| format!("无法序列化模块定义 {}: {error}", module.id))?;
    if let Value::Object(map) = &mut value {
        map.remove("definitionDir");
    }
    write_json(path, &value)
}

fn ensure_builtin_module_folders(root: &Path) -> Result<(), String> {
    for mut module in builtin_module_definitions() {
        let folder = module_folder(root, &module.id);
        module.definition_dir = folder.display().to_string();
        fs::create_dir_all(&folder)
            .map_err(|error| format!("无法创建模块目录 {}: {error}", folder.display()))?;

        let definition_file = module_definition_file(&folder);
        if !definition_file.exists() {
            write_module_definition(&definition_file, &module)?;
        }

        let readme_file = folder.join("README.md");
        if !readme_file.exists() {
            fs::write(&readme_file, module_readme(&module))
                .map_err(|error| format!("无法写入模块说明 {}: {error}", readme_file.display()))?;
        }
    }
    Ok(())
}

fn apply_model_path(mut module: ModuleInfo, paths: &ModelPaths) -> ModuleInfo {
    if module.source == "system" {
        module.model_path = None;
        module.model_configured = true;
        return module;
    }

    if let Some(path) = paths.get(&module.id).map(|value| value.trim()).filter(|value| !value.is_empty()) {
        module.model_path = Some(path.to_string());
        module.model_configured = true;
    }
    module
}

fn module_order(module_id: &str) -> usize {
    match module_id {
        START_MODULE_ID => 0,
        OUTPUT_MODULE_ID => 1,
        "preset.custom.paddleocr" => 10,
        "preset.custom.shieldgemma2" => 11,
        "preset.custom.qwen3guard" => 12,
        _ => 100,
    }
}

fn load_modules(root: &Path, paths: &ModelPaths) -> Result<Vec<ModuleInfo>, String> {
    ensure_builtin_module_folders(root)?;

    let modules_dir = root.join("modules");
    let mut modules = Vec::new();
    for entry in fs::read_dir(&modules_dir)
        .map_err(|error| format!("无法读取模块目录 {}: {error}", modules_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("无法读取模块目录项: {error}"))?;
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let definition_file = module_definition_file(&folder);
        if !definition_file.exists() {
            continue;
        }
        let mut module: ModuleInfo = read_required_json(&definition_file)?;
        module.id = normalize_module_id(&module.id);
        module.definition_dir = folder.display().to_string();
        modules.push(apply_model_path(module, paths));
    }

    modules.sort_by(|left, right| {
        module_order(&left.id)
            .cmp(&module_order(&right.id))
            .then(left.name.cmp(&right.name))
    });
    Ok(modules)
}

fn modules_by_id(root: &Path, paths: &ModelPaths) -> Result<HashMap<String, ModuleInfo>, String> {
    Ok(load_modules(root, paths)?
        .into_iter()
        .map(|module| (module.id.clone(), module))
        .collect())
}

fn normalize_flow(mut flow: FlowDefinition, modules: &HashMap<String, ModuleInfo>) -> FlowDefinition {
    for node in &mut flow.nodes {
        node.module_id = normalize_module_id(&node.module_id);
        if let Some(module) = modules.get(&node.module_id) {
            let defaults = module_default_config(module);
            node.config = merge_config(&defaults, &node.config);
        }
    }
    ensure_system_nodes(flow)
}

fn default_flow() -> FlowDefinition {
    FlowDefinition {
        id: "flow.default.image-audit".to_string(),
        name: "图片 UGC 默认审核".to_string(),
        version: 1,
        nodes: vec![
            FlowNode {
                id: START_NODE_ID.to_string(),
                module_id: START_MODULE_ID.to_string(),
                label: "开始".to_string(),
                position: Position { x: 120.0, y: 220.0 },
                config: json!({}),
            },
            FlowNode {
                id: OUTPUT_NODE_ID.to_string(),
                module_id: OUTPUT_MODULE_ID.to_string(),
                label: "输出结果".to_string(),
                position: Position { x: 520.0, y: 220.0 },
                config: json!({}),
            },
        ],
        edges: vec![FlowEdge {
            id: "edge_flow_start_output".to_string(),
            from: START_NODE_ID.to_string(),
            to: OUTPUT_NODE_ID.to_string(),
        }],
    }
}

fn has_system_node(flow: &FlowDefinition, module_id: &str) -> bool {
    flow.nodes.iter().any(|node| node.module_id == module_id)
}

fn ensure_system_nodes(flow: FlowDefinition) -> FlowDefinition {
    let has_start = has_system_node(&flow, START_MODULE_ID);
    let has_output = has_system_node(&flow, OUTPUT_MODULE_ID);

    if !has_start && !has_output {
        return default_flow();
    }

    let FlowDefinition {
        id,
        name,
        version,
        mut nodes,
        mut edges,
    } = flow;

    if !has_start {
        nodes.insert(
            0,
            FlowNode {
                id: START_NODE_ID.to_string(),
                module_id: START_MODULE_ID.to_string(),
                label: "开始".to_string(),
                position: Position { x: 120.0, y: 220.0 },
                config: json!({}),
            },
        );
    }

    if !has_output {
        nodes.push(FlowNode {
            id: OUTPUT_NODE_ID.to_string(),
            module_id: OUTPUT_MODULE_ID.to_string(),
            label: "输出结果".to_string(),
            position: Position { x: 520.0, y: 220.0 },
            config: json!({}),
        });
    }

    let start_id = nodes
        .iter()
        .find(|node| node.module_id == START_MODULE_ID)
        .map(|node| node.id.clone())
        .unwrap_or_else(|| START_NODE_ID.to_string());
    let output_id = nodes
        .iter()
        .find(|node| node.module_id == OUTPUT_MODULE_ID)
        .map(|node| node.id.clone())
        .unwrap_or_else(|| OUTPUT_NODE_ID.to_string());

    if nodes.len() == 2 && !edges.iter().any(|edge| edge.from == start_id && edge.to == output_id)
    {
        edges.push(FlowEdge {
            id: "edge_flow_start_output".to_string(),
            from: start_id,
            to: output_id,
        });
    }

    FlowDefinition {
        id,
        name,
        version,
        nodes,
        edges,
    }
}

fn model_paths_file(root: &Path) -> PathBuf {
    root.join("settings").join("model-paths.json")
}

fn default_flow_file(root: &Path) -> PathBuf {
    root.join("flows").join("default.json")
}

fn load_model_paths(root: &Path) -> Result<ModelPaths, String> {
    read_json(&model_paths_file(root))
}

fn validate_flow_inner(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
) -> ValidationResult {
    let mut messages = Vec::new();

    if flow.nodes.is_empty() {
        messages.push("流程至少需要一个步骤。".to_string());
    }

    let start_nodes = flow
        .nodes
        .iter()
        .filter(|node| node.module_id == START_MODULE_ID)
        .collect::<Vec<_>>();
    let output_nodes = flow
        .nodes
        .iter()
        .filter(|node| node.module_id == OUTPUT_MODULE_ID)
        .collect::<Vec<_>>();

    if start_nodes.len() != 1 {
        messages.push("流程必须有且只能有一个开始节点。".to_string());
    }
    if output_nodes.len() != 1 {
        messages.push("流程必须有且只能有一个输出结果节点。".to_string());
    }

    let mut seen_nodes = HashSet::new();
    for node in &flow.nodes {
        if node.id.trim().is_empty() {
            messages.push("存在没有 ID 的步骤。".to_string());
        }
        if !seen_nodes.insert(node.id.clone()) {
            messages.push(format!("步骤 ID 重复：{}", node.id));
        }
        if !modules.contains_key(&node.module_id) {
            messages.push(format!("步骤 {} 使用了未知模块。", node.label));
        }
    }

    let node_ids: HashSet<String> = flow.nodes.iter().map(|node| node.id.clone()).collect();
    for edge in &flow.edges {
        if !node_ids.contains(&edge.from) {
            messages.push(format!("连线 {} 的起点不存在。", edge.id));
        }
        if !node_ids.contains(&edge.to) {
            messages.push(format!("连线 {} 的终点不存在。", edge.id));
        }
        if edge.from == edge.to {
            messages.push(format!("连线 {} 指向了同一个步骤。", edge.id));
        }
    }

    if let Some(start) = start_nodes.first() {
        if flow.edges.iter().any(|edge| edge.to == start.id) {
            messages.push("开始节点不能有输入连线。".to_string());
        }
    }
    if let Some(output) = output_nodes.first() {
        if flow.edges.iter().any(|edge| edge.from == output.id) {
            messages.push("输出结果节点不能有输出连线。".to_string());
        }
    }
    if let (Some(start), Some(output)) = (start_nodes.first(), output_nodes.first()) {
        if !has_path(flow, &start.id, &output.id) {
            messages.push("开始节点必须能连到输出结果节点。".to_string());
        }
    }

    if messages.is_empty() && topological_order(flow).is_err() {
        messages.push("流程不能形成闭环。".to_string());
    }

    ValidationResult {
        valid: messages.is_empty(),
        messages,
    }
}

fn has_path(flow: &FlowDefinition, from: &str, to: &str) -> bool {
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &flow.edges {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([from.to_string()]);
    while let Some(current) = queue.pop_front() {
        if current == to {
            return true;
        }
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(children) = outgoing.get(&current) {
            for child in children {
                queue.push_back(child.clone());
            }
        }
    }

    false
}

fn topological_order(flow: &FlowDefinition) -> Result<Vec<(FlowNode, usize)>, String> {
    let mut by_id: HashMap<String, FlowNode> = flow
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut indegree: HashMap<String, usize> = flow
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0_usize))
        .collect();
    let mut group: HashMap<String, usize> = flow
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0_usize))
        .collect();

    for edge in &flow.edges {
        if by_id.contains_key(&edge.from) && by_id.contains_key(&edge.to) {
            outgoing
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            *indegree.entry(edge.to.clone()).or_default() += 1;
        }
    }

    let mut ready: VecDeque<String> = sorted_ready_ids(flow, &indegree)
        .into_iter()
        .collect::<VecDeque<_>>();
    let mut ordered = Vec::new();

    while let Some(id) = ready.pop_front() {
        let node = by_id
            .remove(&id)
            .ok_or_else(|| format!("找不到步骤 {id}"))?;
        let current_group = *group.get(&id).unwrap_or(&0);
        ordered.push((node, current_group));

        if let Some(children) = outgoing.get(&id) {
            for child in children {
                if let Some(value) = indegree.get_mut(child) {
                    *value = value.saturating_sub(1);
                    let next_group = current_group + 1;
                    let child_group = group.entry(child.clone()).or_default();
                    if next_group > *child_group {
                        *child_group = next_group;
                    }
                    if *value == 0 {
                        ready.push_back(child.clone());
                    }
                }
            }
            sort_queue(flow, &mut ready);
        }
    }

    if ordered.len() != flow.nodes.len() {
        return Err("流程不能形成闭环。".to_string());
    }

    Ok(ordered)
}

fn sorted_ready_ids(flow: &FlowDefinition, indegree: &HashMap<String, usize>) -> Vec<String> {
    let mut ids = flow
        .nodes
        .iter()
        .filter(|node| indegree.get(&node.id).copied().unwrap_or(0) == 0)
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();

    ids.sort_by(|left, right| {
        let left_node = flow.nodes.iter().find(|node| node.id == *left);
        let right_node = flow.nodes.iter().find(|node| node.id == *right);
        left_node
            .and_then(|node| right_node.map(|other| (node, other)))
            .map(|(left_node, right_node)| {
                left_node
                    .position
                    .x
                    .total_cmp(&right_node.position.x)
                    .then(left_node.position.y.total_cmp(&right_node.position.y))
            })
            .unwrap_or_else(|| left.cmp(right))
    });

    ids
}

fn sort_queue(flow: &FlowDefinition, queue: &mut VecDeque<String>) {
    let mut ids: Vec<String> = queue.drain(..).collect();
    let position = |id: &str| {
        flow.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| (node.position.x, node.position.y))
            .unwrap_or((f64::MAX, f64::MAX))
    };
    ids.sort_by(|left, right| {
        let left_position = position(left);
        let right_position = position(right);
        left_position
            .0
            .total_cmp(&right_position.0)
            .then(left_position.1.total_cmp(&right_position.1))
            .then(left.cmp(right))
    });
    queue.extend(ids);
}

fn module_step_result(
    module: &ModuleInfo,
    node: &FlowNode,
    execution_group: usize,
) -> StepRun {
    if module.source == "system" {
        let message = if module.id == START_MODULE_ID {
            "流程开始，已接收本次审核素材。"
        } else {
            "流程结束，审核结果将汇总到 Markdown 报告。"
        };
        let report_section = format!(
            "### {}\n\n- 模块：{}\n- 模块来源：流程系统节点\n- 结论：通过\n- 状态：系统节点\n- 说明：{}\n",
            node.label, module.name, message
        );

        return StepRun {
            step_id: node.id.clone(),
            module_id: module.id.clone(),
            module_name: module.name.clone(),
            label: node.label.clone(),
            status: "system".to_string(),
            verdict: "pass".to_string(),
            message: message.to_string(),
            execution_group,
            outputs: json!({
                "moduleKind": module.kind.clone(),
                "summary": message,
                "params": node.config.clone()
            }),
            report_section,
        };
    }

    let model_path = model_path_from_config(&node.config);
    let (status, verdict, message) = match model_path {
        None => (
            "needs_model",
            "review",
            format!("{} 未配置，本轮没有执行真实识别，也没有下载模型。", module.model_label),
        ),
        Some(ref path) if path.trim().is_empty() => (
            "needs_model",
            "review",
            format!("{} 未配置，本轮没有执行真实识别，也没有下载模型。", module.model_label),
        ),
        Some(ref path) if !Path::new(path).exists() => (
            "invalid_model_path",
            "review",
            format!("配置的本地路径不存在：{path}。本轮没有执行真实识别。"),
        ),
        Some(ref path) => (
            "ready",
            "review",
            format!("已收到模块参数，本地入口：{path}。首版只完成入口检查，尚未执行真实识别。"),
        ),
    };

    let report_section = format!(
        "### {}\n\n- 模块：{}\n- 模块来源：预置自定义模块\n- 结论：需要人工复审\n- 状态：{}\n- 说明：{}\n- 参数：`{}`\n",
        node.label,
        module.name,
        status_label(status),
        message,
        table_cell(&node.config.to_string())
    );

    StepRun {
        step_id: node.id.clone(),
        module_id: module.id.clone(),
        module_name: module.name.clone(),
        label: node.label.clone(),
        status: status.to_string(),
        verdict: verdict.to_string(),
        message: message.clone(),
        execution_group,
        outputs: json!({
            "moduleKind": module.kind.clone(),
            "modelConfigured": status == "ready",
            "summary": message,
            "params": node.config.clone()
        }),
        report_section,
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "system" => "系统节点",
        "ready" => "本地入口已配置",
        "needs_model" => "未配置模型",
        "invalid_model_path" => "路径不可用",
        _ => "已记录",
    }
}

fn verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "pass" => "通过",
        "reject" => "不通过",
        "error" => "执行失败",
        _ => "需要人工复审",
    }
}

fn table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn asset_kind_label(kind: &str) -> &'static str {
    match kind {
        "directory" => "文件夹",
        "note" => "文字说明",
        _ => "文件",
    }
}

fn build_report(run: &RunRecord) -> String {
    let mut report = String::new();
    report.push_str("# UGC 审核报告\n\n");
    report.push_str("## 总结\n\n");
    report.push_str(&format!("- 最终结论：{}\n", verdict_label(&run.verdict)));
    report.push_str(&format!("- 运行编号：{}\n", run.id));
    report.push_str(&format!("- 流程：{}\n", run.flow_name));
    report.push_str(&format!("- 输入：{}\n", run.input_note));
    report.push_str(&format!("- 素材数量：{}\n", run.assets.len()));
    report.push_str("- 模型下载：本次运行未触发任何模型下载。\n\n");

    report.push_str("## 输入素材\n\n");
    if run.assets.is_empty() {
        report.push_str("- 未选择本地素材。\n");
        report.push_str(&format!("- 说明：{}\n\n", run.input_note));
    } else {
        report.push_str("| 类型 | 名称 | 路径 |\n");
        report.push_str("| --- | --- | --- |\n");
        for asset in &run.assets {
            report.push_str(&format!(
                "| {} | {} | {} |\n",
                asset_kind_label(&asset.kind),
                table_cell(&asset.name),
                table_cell(&asset.path)
            ));
        }
        report.push('\n');
    }

    report.push_str("## 流程结果\n\n");
    report.push_str("| 步骤 | 模块 | 状态 | 结论 | 说明 |\n");
    report.push_str("| --- | --- | --- | --- | --- |\n");
    for step in &run.steps {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            table_cell(&step.label),
            table_cell(&step.module_name),
            status_label(&step.status),
            verdict_label(&step.verdict),
            table_cell(&step.message)
        ));
    }

    report.push_str("\n## 模块结论\n\n");
    for step in &run.steps {
        report.push_str(&step.report_section);
        report.push('\n');
    }

    report.push_str("## 本地文件\n\n");
    report.push_str(&format!("- 运行目录：{}\n", run.run_dir));
    report.push_str(&format!("- 报告文件：{}\n", run.report_path));
    report
}

#[tauri::command]
fn get_data_root(app: tauri::AppHandle) -> Result<String, String> {
    ensure_data_dirs(&app).map(|path| path.display().to_string())
}

#[tauri::command]
fn list_modules(app: tauri::AppHandle) -> Result<Vec<ModuleInfo>, String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    load_modules(&root, &paths)
}

#[tauri::command]
fn get_model_paths(app: tauri::AppHandle) -> Result<ModelPaths, String> {
    let root = ensure_data_dirs(&app)?;
    load_model_paths(&root)
}

#[tauri::command]
fn save_model_path(
    app: tauri::AppHandle,
    module_id: String,
    path: String,
) -> Result<Vec<ModuleInfo>, String> {
    let root = ensure_data_dirs(&app)?;
    let mut paths = load_model_paths(&root)?;
    if path.trim().is_empty() {
        paths.remove(&module_id);
    } else {
        paths.insert(module_id, path.trim().to_string());
    }
    write_json(&model_paths_file(&root), &paths)?;
    load_modules(&root, &paths)
}

#[tauri::command]
fn open_module_definition_folder(app: tauri::AppHandle, module_id: String) -> Result<(), String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let module = modules
        .get(&module_id)
        .ok_or_else(|| format!("找不到模块 {}", module_id))?;
    let folder = PathBuf::from(&module.definition_dir);
    if !folder.exists() {
        return Err(format!("模块定义文件夹不存在：{}", folder.display()));
    }
    tauri_plugin_opener::open_path(&folder, None::<&str>)
        .map_err(|error| format!("无法打开模块文件夹 {}: {error}", folder.display()))
}

#[tauri::command]
fn load_flow(app: tauri::AppHandle) -> Result<FlowDefinition, String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let flow_path = default_flow_file(&root);
    if !flow_path.exists() {
        let flow = default_flow();
        write_json(&flow_path, &flow)?;
        return Ok(flow);
    }

    let content = fs::read_to_string(&flow_path)
        .map_err(|error| format!("无法读取流程文件 {}: {error}", flow_path.display()))?;
    let flow: FlowDefinition = serde_json::from_str(&content)
        .map_err(|error| format!("无法解析流程文件 {}: {error}", flow_path.display()))?;
    let normalized = normalize_flow(flow, &modules);
    write_json(&flow_path, &normalized)?;
    Ok(normalized)
}

#[tauri::command]
fn save_flow(app: tauri::AppHandle, flow: FlowDefinition) -> Result<FlowDefinition, String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let normalized = normalize_flow(flow, &modules);
    let result = validate_flow_inner(&normalized, &modules);
    if !result.valid {
        return Err(result.messages.join(" "));
    }
    write_json(&default_flow_file(&root), &normalized)?;
    Ok(normalized)
}

#[tauri::command]
fn validate_flow(app: tauri::AppHandle, flow: FlowDefinition) -> ValidationResult {
    let Ok(root) = ensure_data_dirs(&app) else {
        return ValidationResult {
            valid: false,
            messages: vec!["无法读取本地数据目录。".to_string()],
        };
    };
    let Ok(paths) = load_model_paths(&root) else {
        return ValidationResult {
            valid: false,
            messages: vec!["无法读取模型路径配置。".to_string()],
        };
    };
    let Ok(modules) = modules_by_id(&root, &paths) else {
        return ValidationResult {
            valid: false,
            messages: vec!["无法读取模块定义。".to_string()],
        };
    };
    validate_flow_inner(&normalize_flow(flow, &modules), &modules)
}

#[tauri::command]
fn start_run(
    app: tauri::AppHandle,
    flow: FlowDefinition,
    input_note: String,
    assets: Vec<AuditAsset>,
) -> Result<RunRecord, String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let flow = normalize_flow(flow, &modules);
    let validation = validate_flow_inner(&flow, &modules);
    if !validation.valid {
        return Err(validation.messages.join(" "));
    }

    let ordered_nodes = topological_order(&flow)?;
    let run_id = format!("run_{}", now_millis());
    let run_dir = root.join("runs").join(&run_id);
    let steps_dir = run_dir.join("steps");

    fs::create_dir_all(&steps_dir)
        .map_err(|error| format!("无法创建运行目录 {}: {error}", steps_dir.display()))?;
    write_json(&run_dir.join("flow.snapshot.json"), &flow)?;

    let mut steps = Vec::new();
    for (node, execution_group) in ordered_nodes {
        let module = modules
            .get(&node.module_id)
            .ok_or_else(|| format!("找不到模块 {}", node.module_id))?;
        let step = module_step_result(module, &node, execution_group);
        let step_dir = steps_dir.join(&node.id);
        fs::create_dir_all(&step_dir)
            .map_err(|error| format!("无法创建步骤目录 {}: {error}", step_dir.display()))?;
        write_json(
            &step_dir.join("input.json"),
            &json!({
                "runId": run_id,
                "stepId": node.id,
                "moduleId": node.module_id,
                "params": node.config,
                "inputNote": &input_note,
                "assets": &assets,
                "executionGroup": execution_group
            }),
        )?;
        write_json(&step_dir.join("output.json"), &step)?;
        steps.push(step);
    }

    let verdict = if steps.iter().any(|step| step.verdict == "reject") {
        "reject"
    } else if steps.iter().any(|step| step.verdict == "error") {
        "error"
    } else if steps.iter().any(|step| step.verdict == "review") {
        "review"
    } else {
        "pass"
    };

    let report_path = run_dir.join("report.md");
    let mut run = RunRecord {
        id: run_id,
        flow_id: flow.id,
        flow_name: flow.name,
        created_at: now_seconds(),
        status: "completed".to_string(),
        verdict: verdict.to_string(),
        input_note: if input_note.trim().is_empty() {
            "未填写输入说明".to_string()
        } else {
            input_note.trim().to_string()
        },
        assets,
        data_root: root.display().to_string(),
        run_dir: run_dir.display().to_string(),
        report_path: report_path.display().to_string(),
        steps,
    };

    let report = build_report(&run);
    fs::write(&report_path, report)
        .map_err(|error| format!("无法写入报告 {}: {error}", report_path.display()))?;
    write_json(&run_dir.join("run.json"), &run)?;
    run.report_path = report_path.display().to_string();

    Ok(run)
}

#[tauri::command]
fn list_runs(app: tauri::AppHandle) -> Result<Vec<RunSummary>, String> {
    let root = ensure_data_dirs(&app)?;
    let runs_dir = root.join("runs");
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in fs::read_dir(&runs_dir)
        .map_err(|error| format!("无法读取运行目录 {}: {error}", runs_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("无法读取运行记录: {error}"))?;
        let run_path = entry.path().join("run.json");
        if run_path.exists() {
            let run: RunRecord = read_required_json(&run_path)?;
            runs.push(RunSummary {
                id: run.id,
                flow_name: run.flow_name,
                created_at: run.created_at,
                status: run.status,
                verdict: run.verdict,
                report_path: run.report_path,
            });
        }
    }
    runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(runs)
}

#[tauri::command]
fn read_run_report(app: tauri::AppHandle, run_id: String) -> Result<String, String> {
    let root = ensure_data_dirs(&app)?;
    let report_path = root.join("runs").join(run_id).join("report.md");
    fs::read_to_string(&report_path)
        .map_err(|error| format!("无法读取报告 {}: {error}", report_path.display()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_data_root,
            list_modules,
            get_model_paths,
            save_model_path,
            open_module_definition_folder,
            load_flow,
            save_flow,
            validate_flow,
            start_run,
            list_runs,
            read_run_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
