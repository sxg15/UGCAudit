use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleLaunch {
    #[serde(default)]
    launch_type: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    notes: String,
}

impl Default for ModuleLaunch {
    fn default() -> Self {
        Self {
            launch_type: "manual".to_string(),
            command: None,
            url: None,
            method: None,
            args: Vec::new(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    id: String,
    name: String,
    kind: String,
    #[serde(default)]
    summary: String,
    #[serde(default = "default_model_label")]
    model_label: String,
    icon: String,
    #[serde(default)]
    built_in: bool,
    #[serde(default = "default_module_source")]
    source: String,
    #[serde(default)]
    definition_dir: String,
    #[serde(default)]
    icon_path: Option<String>,
    #[serde(default)]
    icon_data_url: Option<String>,
    #[serde(default)]
    model_path: Option<String>,
    #[serde(default)]
    model_configured: bool,
    #[serde(default)]
    launch: ModuleLaunch,
    #[serde(default)]
    parameters: Vec<ModuleParameter>,
    #[serde(default)]
    data_outputs: Vec<ModuleDataOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDataOutput {
    handle: String,
    name: String,
    data_type: String,
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
    #[serde(default)]
    required: bool,
    #[serde(default)]
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
    #[serde(default = "default_edge_type")]
    edge_type: String,
    #[serde(default)]
    from_handle: Option<String>,
    #[serde(default)]
    to_handle: Option<String>,
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
pub struct AuditScheme {
    #[serde(default = "default_scheme_schema_version")]
    schema_version: u32,
    #[serde(default = "default_scheme_kind")]
    kind: String,
    #[serde(default)]
    id: String,
    name: String,
    flow: FlowDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeListItem {
    id: String,
    name: String,
    path: String,
    #[serde(default)]
    modified_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAuditScheme {
    path: String,
    scheme: AuditScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    valid: bool,
    messages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPerformance {
    #[serde(default)]
    start_time: u64,
    #[serde(default)]
    end_time: u64,
    #[serde(default)]
    duration_ms: u64,
    #[serde(default)]
    sample_count: usize,
    #[serde(default)]
    cpu_time_ms: f64,
    #[serde(default)]
    cpu_share_percent: f64,
    #[serde(default)]
    average_cpu_percent: f64,
    #[serde(default)]
    peak_cpu_percent: f64,
    #[serde(default)]
    peak_memory_bytes: u64,
    #[serde(default)]
    artifact_bytes: u64,
    #[serde(default)]
    gpu_available: bool,
    #[serde(default)]
    gpu_sample_count: usize,
    #[serde(default)]
    peak_gpu_memory_bytes: Option<u64>,
    #[serde(default)]
    sampling_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceLeader {
    step_id: String,
    label: String,
    module_name: String,
    value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPerformanceSummary {
    #[serde(default)]
    total_duration_ms: u64,
    #[serde(default)]
    total_cpu_time_ms: f64,
    #[serde(default)]
    total_artifact_bytes: u64,
    #[serde(default)]
    measured_steps: usize,
    #[serde(default)]
    gpu_available: bool,
    #[serde(default)]
    gpu_sampled: bool,
    #[serde(default)]
    cpu_leader: Option<PerformanceLeader>,
    #[serde(default)]
    duration_leader: Option<PerformanceLeader>,
    #[serde(default)]
    memory_leader: Option<PerformanceLeader>,
    #[serde(default)]
    sampling_note: String,
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
    #[serde(default)]
    progress: f64,
    #[serde(default)]
    processed_files: usize,
    #[serde(default)]
    matched_files: usize,
    #[serde(default)]
    artifact_count: usize,
    #[serde(default)]
    performance: Option<StepPerformance>,
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
pub struct AuditFile {
    path: String,
    name: String,
    extension: String,
    file_type: String,
    source_asset_id: String,
    source_asset_name: String,
    relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCollectionItem {
    path: String,
    name: String,
    extension: String,
    source_asset_id: String,
    source_asset_name: String,
    relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCollectionItem {
    source_type: String,
    path: String,
    name: String,
    relative_path: String,
    text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataPortValue {
    data_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    items: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleStepOutput {
    #[serde(default = "default_output_status")]
    status: String,
    #[serde(default = "default_output_verdict")]
    verdict: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    processed_files: usize,
    #[serde(default)]
    matched_files: usize,
    #[serde(default)]
    artifact_count: usize,
    #[serde(default)]
    report_section: String,
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
    #[serde(default)]
    task_name: String,
    input_note: String,
    #[serde(default)]
    assets: Vec<AuditAsset>,
    data_root: String,
    run_dir: String,
    #[serde(default)]
    resource_root: String,
    #[serde(default)]
    artifact_root: String,
    #[serde(default)]
    artifact_dir: String,
    report_path: String,
    #[serde(default)]
    performance_summary: Option<RunPerformanceSummary>,
    steps: Vec<StepRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStartResponse {
    run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProgressEvent {
    run_id: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    progress: Option<f64>,
    #[serde(default)]
    message: String,
    #[serde(default)]
    processed: Option<usize>,
    #[serde(default)]
    total: Option<usize>,
    #[serde(default)]
    step: Option<StepRun>,
    #[serde(default)]
    run: Option<RunRecord>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDependencyStatus {
    id: String,
    name: String,
    installed: bool,
    version: Option<String>,
    folder: String,
    site_packages: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    runtime_root: String,
    runtime_source: String,
    dependency_root: String,
    python_dir: String,
    python_path: String,
    python_installed: bool,
    python_version: Option<String>,
    dependencies: Vec<RuntimeDependencyStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogLine {
    timestamp: u64,
    scope: String,
    stream: String,
    line: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    artifact_root: String,
    #[serde(default)]
    dependency_root: String,
}

type ModelPaths = HashMap<String, String>;
type ImportedModulePaths = Vec<String>;

const START_NODE_ID: &str = "flow_start";
const OUTPUT_NODE_ID: &str = "flow_output";
const START_MODULE_ID: &str = "system.start";
const OUTPUT_MODULE_ID: &str = "system.output";
const DATA_ALL_IMAGES_MODULE_ID: &str = "system.data.images.all";
const DATA_ALL_TEXTS_MODULE_ID: &str = "system.data.texts.all";
const DATA_ARTIFACT_IMAGES_MODULE_ID: &str = "system.data.images.artifacts";
const DATA_ARTIFACT_TEXTS_MODULE_ID: &str = "system.data.texts.artifacts";
const DATA_RELATIVE_IMAGES_MODULE_ID: &str = "system.data.images.relative";
const DATA_RELATIVE_TEXTS_MODULE_ID: &str = "system.data.texts.relative";
const DATA_AUDIT_FOLDER_MODULE_ID: &str = "system.data.folder.audit";
const DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID: &str = "system.data.folder.audit.relative";
const DATA_ARTIFACT_FOLDER_MODULE_ID: &str = "system.data.folder.artifact";
const DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID: &str =
    "system.data.folder.artifact.relative";
const DATA_MERGE_IMAGES_MODULE_ID: &str = "system.data.images.merge";
const DATA_MERGE_TEXTS_MODULE_ID: &str = "system.data.texts.merge";
const ANNOTATION_MODULE_ID: &str = "system.annotation.comment";
const CANVAS_GROUP_MODULE_ID: &str = "system.canvas.group";
const CANVAS_NOTE_MODULE_ID: &str = "system.canvas.note";
const EDGE_TYPE_SEQUENCE: &str = "sequence";
const EDGE_TYPE_DATA: &str = "data";
const HANDLE_SEQUENCE_IN: &str = "sequence-in";
const HANDLE_SEQUENCE_OUT: &str = "sequence-out";
const HANDLE_IMAGES_IN: &str = "images";
const HANDLE_IMAGES_OUT: &str = "images";
const HANDLE_TEXTS_IN: &str = "texts";
const HANDLE_TEXTS_OUT: &str = "texts";
const HANDLE_FOLDER_IN: &str = "folder";
const HANDLE_FOLDER_OUT: &str = "folder";
const HANDLE_IMAGES_A_IN: &str = "images-a";
const HANDLE_IMAGES_B_IN: &str = "images-b";
const HANDLE_TEXTS_A_IN: &str = "texts-a";
const HANDLE_TEXTS_B_IN: &str = "texts-b";
const MAX_PARALLEL_MODULES: usize = 2;
const DATA_TYPE_IMAGES: &str = "imageCollection";
const DATA_TYPE_TEXTS: &str = "textCollection";
const DATA_TYPE_FOLDER: &str = "folder";
#[allow(dead_code)]
const DEFAULT_MODEL_ROOT: &str = "D:\\UGCAuditModels";
const RUNTIME_DIR_NAME: &str = "Runtime";
const RUNTIME_PYTHON_DIR_NAME: &str = "Python312";
const RUNTIME_PACKAGES_DIR_NAME: &str = "Packages";
const SCHEME_LIBRARY_DIR_NAME: &str = "Schemes";
const ARTIFACTS_DIR_NAME: &str = "审核产物";
const PERFORMANCE_SAMPLE_INTERVAL: Duration = Duration::from_millis(1000);
#[cfg(windows)]
const WINDOWS_CREATE_NO_WINDOW: u32 = 0x08000000;

fn hide_child_console_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(WINDOWS_CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

fn default_output_status() -> String {
    "completed".to_string()
}

fn default_output_verdict() -> String {
    "review".to_string()
}

fn default_edge_type() -> String {
    EDGE_TYPE_SEQUENCE.to_string()
}

fn default_scheme_schema_version() -> u32 {
    1
}

fn default_scheme_kind() -> String {
    "ugcAuditScheme".to_string()
}

fn default_model_label() -> String {
    "本地运行目录".to_string()
}

fn default_module_source() -> String {
    "custom".to_string()
}

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

fn cli_data_root() -> PathBuf {
    if let Ok(path) = env::var("UGCAUDIT_DATA_ROOT") {
        let path = PathBuf::from(path.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }

    if let Ok(appdata) = env::var("APPDATA") {
        return PathBuf::from(appdata)
            .join("com.ugcaudit.portable")
            .join("data");
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data")
}

fn ensure_data_dirs_at(root: &Path) -> Result<PathBuf, String> {
    for dir in [
        root.join("flows"),
        root.join("settings"),
        root.join("runs"),
        root.join("modules"),
    ] {
        fs::create_dir_all(&dir)
            .map_err(|error| format!("无法创建数据目录 {}: {error}", dir.display()))?;
    }
    Ok(root.to_path_buf())
}

fn ensure_data_dirs(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    ensure_data_dirs_at(&data_root(app))
}

fn program_root() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn scheme_library_dir() -> PathBuf {
    if let Ok(path) = env::var("UGCAUDIT_SCHEME_LIBRARY_DIR") {
        let path = PathBuf::from(path.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    program_root().join(SCHEME_LIBRARY_DIR_NAME)
}

fn ensure_scheme_library_dir() -> Result<PathBuf, String> {
    let dir = scheme_library_dir();
    fs::create_dir_all(&dir)
        .map_err(|error| format!("无法创建审核方案目录 {}: {error}", dir.display()))?;
    Ok(dir)
}

fn default_artifact_root() -> PathBuf {
    program_root().join(ARTIFACTS_DIR_NAME)
}

fn default_dependency_root(root: &Path) -> PathBuf {
    if let Ok(path) = env::var("UGCAUDIT_DEPENDENCY_ROOT") {
        let path = PathBuf::from(path.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(app_dir) = exe.parent() {
            let is_packaged_app_dir = app_dir
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("App"))
                .unwrap_or(false);
            if is_packaged_app_dir {
                if let Some(publish_root) = app_dir.parent() {
                    let is_publish_output = publish_root
                        .parent()
                        .and_then(|parent| parent.file_name())
                        .and_then(|name| name.to_str())
                        .map(|name| name.eq_ignore_ascii_case("Publish"))
                        .unwrap_or(false);
                    if is_publish_output {
                        if let Some(project_root) =
                            publish_root.parent().and_then(|parent| parent.parent())
                        {
                            return project_root
                                .join(RUNTIME_DIR_NAME)
                                .join(RUNTIME_PACKAGES_DIR_NAME);
                        }
                    }
                }
            }
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if current_dir.join("package.json").is_file() || current_dir.join("src-tauri").is_dir() {
            return current_dir
                .join(RUNTIME_DIR_NAME)
                .join(RUNTIME_PACKAGES_DIR_NAME);
        }
    }

    root.join(RUNTIME_DIR_NAME).join(RUNTIME_PACKAGES_DIR_NAME)
}

fn app_settings_file(root: &Path) -> PathBuf {
    root.join("settings").join("app-settings.json")
}

fn resolve_app_settings(root: &Path, mut settings: AppSettings) -> AppSettings {
    if settings.artifact_root.trim().is_empty() {
        settings.artifact_root = default_artifact_root().display().to_string();
    }
    if settings.dependency_root.trim().is_empty() {
        settings.dependency_root = default_dependency_root(root).display().to_string();
    }
    settings
}

fn load_app_settings(root: &Path) -> Result<AppSettings, String> {
    let settings = read_json(&app_settings_file(root))?;
    Ok(resolve_app_settings(root, settings))
}

fn save_app_settings_inner(root: &Path, settings: AppSettings) -> Result<AppSettings, String> {
    let mut resolved = resolve_app_settings(root, settings);
    let artifact_root = PathBuf::from(resolved.artifact_root.trim());
    fs::create_dir_all(&artifact_root)
        .map_err(|error| format!("无法创建审核产物目录 {}: {error}", artifact_root.display()))?;
    resolved.artifact_root = artifact_root.display().to_string();
    let dependency_root = PathBuf::from(resolved.dependency_root.trim());
    fs::create_dir_all(&dependency_root).map_err(|error| {
        format!(
            "无法创建依赖存放目录 {}: {error}",
            dependency_root.display()
        )
    })?;
    resolved.dependency_root = dependency_root.display().to_string();
    write_json(&app_settings_file(root), &resolved)?;
    Ok(resolved)
}

fn artifact_root_from_settings(root: &Path) -> Result<PathBuf, String> {
    let settings = load_app_settings(root)?;
    let artifact_root = PathBuf::from(settings.artifact_root.trim());
    fs::create_dir_all(&artifact_root)
        .map_err(|error| format!("无法创建审核产物目录 {}: {error}", artifact_root.display()))?;
    Ok(artifact_root)
}

fn dependency_root_from_settings(root: &Path) -> Result<PathBuf, String> {
    let settings = load_app_settings(root)?;
    let dependency_root = PathBuf::from(settings.dependency_root.trim());
    fs::create_dir_all(&dependency_root).map_err(|error| {
        format!(
            "无法创建依赖存放目录 {}: {error}",
            dependency_root.display()
        )
    })?;
    Ok(dependency_root)
}

fn sanitize_path_segment(value: &str, fallback: &str) -> String {
    let mut segment = value
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches(|ch: char| ch.is_whitespace() || ch == '.')
        .to_string();
    while segment.contains("__") {
        segment = segment.replace("__", "_");
    }
    if segment.is_empty() {
        segment = fallback.to_string();
    }
    segment.chars().take(80).collect()
}

fn task_name_from_note(input_note: &str, fallback: &str) -> String {
    for line in input_note.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("任务名称：") {
            let name = value.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        if let Some(value) = trimmed.strip_prefix("任务名称:") {
            let name = value.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    let fallback = fallback.trim();
    if fallback.is_empty() {
        "审核任务".to_string()
    } else {
        fallback.to_string()
    }
}

#[derive(Debug, Clone)]
struct RunArtifactPaths {
    task_name: String,
    artifact_root: PathBuf,
    artifact_dir: PathBuf,
    report_path: PathBuf,
}

fn prepare_run_artifacts(
    root: &Path,
    flow_name: &str,
    input_note: &str,
    run_id: &str,
    task_name_override: Option<String>,
    artifact_dir_override: Option<PathBuf>,
) -> Result<RunArtifactPaths, String> {
    let task_name = task_name_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| task_name_from_note(input_note, flow_name));
    let artifact_dir = if let Some(path) = artifact_dir_override {
        path
    } else {
        let artifact_root = artifact_root_from_settings(root)?;
        artifact_root.join(format!(
            "{}-{}",
            sanitize_path_segment(&task_name, "审核任务"),
            sanitize_path_segment(run_id, "run")
        ))
    };
    fs::create_dir_all(&artifact_dir)
        .map_err(|error| format!("无法创建审核产物目录 {}: {error}", artifact_dir.display()))?;
    let artifact_root = artifact_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| artifact_dir.clone());
    Ok(RunArtifactPaths {
        task_name,
        artifact_root,
        report_path: artifact_dir.join("report.md"),
        artifact_dir,
    })
}

fn percent_encode_path(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn markdown_link_text(value: &str) -> String {
    table_cell(value)
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn report_target_url(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("ugcaudit://reveal?path={}", percent_encode_path(trimmed))
    }
}

fn report_file_link(label: &str, path: &str) -> String {
    let url = report_target_url(path);
    if url.is_empty() {
        markdown_link_text(label)
    } else {
        format!("[{}]({})", markdown_link_text(label), url)
    }
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

#[derive(Debug, Clone)]
struct RuntimeDependencySpec {
    id: &'static str,
    name: &'static str,
    seed_packages: &'static [&'static str],
    install_args: &'static [&'static str],
}

struct CommandRunOutput {
    success: bool,
    cancelled: bool,
    stdout: String,
    stderr: String,
    performance: StepPerformance,
}

#[derive(Default)]
struct PerformanceSampleState {
    sample_count: usize,
    cpu_observation_count: usize,
    cpu_time_ms: f64,
    cpu_percent_sum: f64,
    peak_cpu_percent: f64,
    peak_memory_bytes: u64,
    gpu_available: bool,
    gpu_sample_count: usize,
    peak_gpu_memory_bytes: Option<u64>,
}

struct PerformanceSampler {
    stop: Arc<AtomicBool>,
    handle: thread::JoinHandle<PerformanceSampleState>,
    start_time: u64,
    start_instant: Instant,
}

impl PerformanceSampler {
    fn start(root_pid: u32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let start_time = now_millis() as u64;
        let start_instant = Instant::now();
        let handle = thread::spawn(move || sample_process_performance(root_pid, thread_stop));
        Self {
            stop,
            handle,
            start_time,
            start_instant,
        }
    }

    fn finish(self) -> StepPerformance {
        self.stop.store(true, Ordering::SeqCst);
        let end_time = now_millis() as u64;
        let duration_ms = self.start_instant.elapsed().as_millis() as u64;
        let sample = self.handle.join().unwrap_or_default();
        StepPerformance {
            start_time: self.start_time,
            end_time,
            duration_ms,
            sample_count: sample.sample_count,
            cpu_time_ms: sample.cpu_time_ms.max(0.0),
            cpu_share_percent: 0.0,
            average_cpu_percent: if sample.cpu_observation_count == 0 {
                0.0
            } else {
                sample.cpu_percent_sum / sample.cpu_observation_count as f64
            },
            peak_cpu_percent: sample.peak_cpu_percent,
            peak_memory_bytes: sample.peak_memory_bytes,
            artifact_bytes: 0,
            gpu_available: sample.gpu_available,
            gpu_sample_count: sample.gpu_sample_count,
            peak_gpu_memory_bytes: sample.peak_gpu_memory_bytes,
            sampling_note: performance_sampling_note(),
        }
    }
}

fn performance_sampling_note() -> String {
    "约每 1 秒采样；CPU 和内存统计当前模块进程及其子进程；NVIDIA GPU 仅统计可读取到的进程显存。"
        .to_string()
}

fn sample_process_performance(root_pid: u32, stop: Arc<AtomicBool>) -> PerformanceSampleState {
    let root_pid = Pid::from_u32(root_pid);
    let mut system = System::new();
    let refresh_kind = ProcessRefreshKind::nothing().with_cpu().with_memory();
    let gpu_available = nvidia_smi_available();
    let mut state = PerformanceSampleState {
        gpu_available,
        ..PerformanceSampleState::default()
    };
    let mut last_sample = Instant::now();

    loop {
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);
        let tracked_pids = tracked_process_ids(&system, root_pid);
        if !tracked_pids.is_empty() {
            let mut cpu_percent = 0.0_f64;
            let mut memory_bytes = 0_u64;
            for pid in &tracked_pids {
                if let Some(process) = system.process(*pid) {
                    cpu_percent += f64::from(process.cpu_usage());
                    memory_bytes = memory_bytes.saturating_add(process.memory());
                }
            }
            let elapsed_ms = last_sample.elapsed().as_millis() as f64;
            if elapsed_ms > 0.0 {
                let normalized_cpu_percent = cpu_percent / logical_cpu_count() as f64;
                state.cpu_time_ms += (cpu_percent / 100.0) * elapsed_ms;
                state.cpu_percent_sum += normalized_cpu_percent;
                state.cpu_observation_count += 1;
                state.peak_cpu_percent = state.peak_cpu_percent.max(normalized_cpu_percent);
            }
            state.peak_memory_bytes = state.peak_memory_bytes.max(memory_bytes);
            state.sample_count += 1;

            if gpu_available {
                if let Some(gpu_memory_bytes) = nvidia_process_gpu_memory(&tracked_pids) {
                    state.gpu_sample_count += 1;
                    let peak = state.peak_gpu_memory_bytes.unwrap_or(0);
                    state.peak_gpu_memory_bytes = Some(peak.max(gpu_memory_bytes));
                }
            }
        }
        last_sample = Instant::now();
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let sleep_start = Instant::now();
        while sleep_start.elapsed() < PERFORMANCE_SAMPLE_INTERVAL {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    state
}

fn logical_cpu_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .max(1)
}

fn tracked_process_ids(system: &System, root_pid: Pid) -> HashSet<Pid> {
    system
        .processes()
        .keys()
        .copied()
        .filter(|pid| process_is_descendant(system, *pid, root_pid))
        .collect()
}

fn process_is_descendant(system: &System, pid: Pid, root_pid: Pid) -> bool {
    if pid == root_pid {
        return true;
    }
    let mut current = pid;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        let Some(process) = system.process(current) else {
            return false;
        };
        let Some(parent) = process.parent() else {
            return false;
        };
        if parent == root_pid {
            return true;
        }
        current = parent;
    }
    false
}

fn nvidia_smi_available() -> bool {
    let mut command = Command::new("nvidia-smi");
    hide_child_console_window(&mut command);
    command.arg("--help").stdout(Stdio::null()).stderr(Stdio::null());
    command.status().map(|status| status.success()).unwrap_or(false)
}

fn nvidia_process_gpu_memory(tracked_pids: &HashSet<Pid>) -> Option<u64> {
    let mut command = Command::new("nvidia-smi");
    hide_child_console_window(&mut command);
    let output = command
        .args([
            "--query-compute-apps=pid,used_memory",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return Some(0);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let tracked = tracked_pids
        .iter()
        .map(|pid| pid.as_u32())
        .collect::<HashSet<_>>();
    let mut total_mib = 0_u64;
    for line in stdout.lines() {
        let mut parts = line.split(',').map(str::trim);
        let Some(pid_text) = parts.next() else {
            continue;
        };
        let Some(memory_text) = parts.next() else {
            continue;
        };
        let Ok(pid) = pid_text.parse::<u32>() else {
            continue;
        };
        if !tracked.contains(&pid) {
            continue;
        }
        let memory_digits = memory_text
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if let Ok(mib) = memory_digits.parse::<u64>() {
            total_mib = total_mib.saturating_add(mib);
        }
    }
    Some(total_mib.saturating_mul(1024 * 1024))
}

fn directory_size_bytes(path: &Path) -> u64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| directory_size_bytes(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

fn attach_step_performance(
    mut step: StepRun,
    output_value: &mut Value,
    mut performance: StepPerformance,
    step_artifacts_dir: &Path,
) -> StepRun {
    performance.artifact_bytes = directory_size_bytes(step_artifacts_dir);
    if let Value::Object(map) = output_value {
        map.insert(
            "performance".to_string(),
            serde_json::to_value(&performance).unwrap_or(Value::Null),
        );
    }
    step.performance = Some(performance);
    step
}

#[derive(Default)]
struct RunRegistry {
    controls: Mutex<HashMap<String, Arc<RunControl>>>,
}

#[derive(Default)]
struct RunControl {
    cancelled: AtomicBool,
}

impl RunControl {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

type SharedRunRegistry = Arc<RunRegistry>;

fn runtime_dependency_specs() -> &'static [RuntimeDependencySpec] {
    &[
        RuntimeDependencySpec {
            id: "torch",
            name: "Torch",
            seed_packages: &["torch", "torchvision"],
            install_args: &[
                "--index-url",
                "https://download.pytorch.org/whl/cu126",
                "torch",
                "torchvision",
            ],
        },
        RuntimeDependencySpec {
            id: "transformers",
            name: "Transformers",
            seed_packages: &["transformers", "tokenizers", "safetensors"],
            install_args: &[
                "-i",
                "https://mirrors.aliyun.com/pypi/simple/",
                "transformers>=4.53.0",
                "tokenizers",
                "safetensors",
            ],
        },
        RuntimeDependencySpec {
            id: "pillow",
            name: "Pillow",
            seed_packages: &["pillow"],
            install_args: &["-i", "https://mirrors.aliyun.com/pypi/simple/", "pillow"],
        },
        RuntimeDependencySpec {
            id: "accelerate",
            name: "Accelerate",
            seed_packages: &["accelerate"],
            install_args: &[
                "-i",
                "https://mirrors.aliyun.com/pypi/simple/",
                "accelerate",
            ],
        },
    ]
}

fn runtime_dependency_spec(dependency_id: &str) -> Result<&'static RuntimeDependencySpec, String> {
    runtime_dependency_specs()
        .iter()
        .find(|spec| spec.id == dependency_id)
        .ok_or_else(|| format!("未知运行依赖：{dependency_id}"))
}

fn emit_runtime_log(app: &tauri::AppHandle, scope: &str, stream: &str, line: &str) {
    let _ = app.emit(
        "runtime_log",
        RuntimeLogLine {
            timestamp: now_seconds(),
            scope: scope.to_string(),
            stream: stream.to_string(),
            line: line.to_string(),
        },
    );
}

fn emit_run_event(app: &tauri::AppHandle, event_name: &str, event: RunProgressEvent) {
    let _ = app.emit(event_name, event);
}

fn run_event(run_id: &str, node_id: Option<&str>, status: &str, message: &str) -> RunProgressEvent {
    RunProgressEvent {
        run_id: run_id.to_string(),
        node_id: node_id.map(ToString::to_string),
        status: status.to_string(),
        progress: None,
        message: message.to_string(),
        processed: None,
        total: None,
        step: None,
        run: None,
    }
}

fn emit_runtime_status_changed(app: &tauri::AppHandle) {
    let _ = app.emit(
        "runtime_status_changed",
        json!({ "timestamp": now_seconds() }),
    );
}

fn runtime_parent_writable(parent: &Path) -> bool {
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let test_file = parent.join(format!(".ugc_audit_runtime_write_{}", now_millis()));
    match fs::write(&test_file, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

fn runtime_root(app: &tauri::AppHandle) -> Result<(PathBuf, String), String> {
    if let Ok(path) = env::var("UGCAUDIT_RUNTIME_ROOT") {
        let path = PathBuf::from(path.trim());
        return Ok((path, "override".to_string()));
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            if runtime_parent_writable(parent) {
                return Ok((parent.join(RUNTIME_DIR_NAME), "program".to_string()));
            }
        }
    }

    let root = ensure_data_dirs(app)?.join(RUNTIME_DIR_NAME);
    Ok((root, "data".to_string()))
}

fn runtime_root_for_data_root(data_root: &Path) -> (PathBuf, String) {
    if let Ok(path) = env::var("UGCAUDIT_RUNTIME_ROOT") {
        let path = PathBuf::from(path.trim());
        if !path.as_os_str().is_empty() {
            return (path, "override".to_string());
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            if runtime_parent_writable(parent) {
                return (parent.join(RUNTIME_DIR_NAME), "program".to_string());
            }
        }
    }

    (data_root.join(RUNTIME_DIR_NAME), "data".to_string())
}

fn runtime_python_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(runtime_root(app)?.0.join(RUNTIME_PYTHON_DIR_NAME))
}

fn runtime_python_exe(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(runtime_python_dir(app)?.join("python.exe"))
}

fn runtime_packages_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let root = ensure_data_dirs(app)?;
    dependency_root_from_settings(&root)
}

fn dependency_folder(app: &tauri::AppHandle, dependency_id: &str) -> Result<PathBuf, String> {
    Ok(runtime_packages_dir(app)?.join(dependency_id))
}

fn dependency_site_packages(
    app: &tauri::AppHandle,
    dependency_id: &str,
) -> Result<PathBuf, String> {
    Ok(dependency_folder(app, dependency_id)?.join("site-packages"))
}

fn dependency_site_packages_for_dependency_root(
    dependency_root: &Path,
    dependency_id: &str,
) -> PathBuf {
    dependency_root.join(dependency_id).join("site-packages")
}

fn normalize_package_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .replace('-', "_")
        .replace('.', "_")
}

fn site_package_entry_matches(entry_name: &str, package_name: &str) -> bool {
    let entry = normalize_package_name(entry_name);
    let package = normalize_package_name(package_name);
    entry == package
        || entry == format!("{package}.py")
        || entry.starts_with(&format!("{package}_"))
        || entry.starts_with(&format!("{package}-"))
}

fn site_packages_has_package(site_packages: &Path, package_name: &str) -> bool {
    fs::read_dir(site_packages)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| site_package_entry_matches(name, package_name))
                .unwrap_or(false)
        })
}

fn site_packages_has_any_file(site_packages: &Path) -> bool {
    fs::read_dir(site_packages)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| entry.path().is_file() || entry.path().is_dir())
}

fn dependency_installed(app: &tauri::AppHandle, spec: &RuntimeDependencySpec) -> bool {
    let Ok(site_packages) = dependency_site_packages(app, spec.id) else {
        return false;
    };
    spec.seed_packages
        .iter()
        .any(|package| site_packages_has_package(&site_packages, package))
}

fn bundled_python_source() -> Option<PathBuf> {
    if let Ok(path) = env::var("UGCAUDIT_PYTHON_SOURCE") {
        let path = PathBuf::from(path.trim());
        if path.join("python.exe").is_file() {
            return Some(path);
        }
    }
    None
}

fn copy_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("无法创建目录 {}: {error}", parent.display()))?;
        }
        fs::copy(source, destination).map_err(|error| {
            format!(
                "无法复制 {} 到 {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }

    fs::create_dir_all(destination)
        .map_err(|error| format!("无法创建目录 {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("无法读取目录 {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("无法读取目录项: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_recursive(&source_path, &destination_path)?;
        } else if source_path.is_file() {
            copy_recursive(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn ensure_runtime_python(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let python = runtime_python_exe(app)?;
    if python.is_file() {
        return Ok(python);
    }

    let source = bundled_python_source().ok_or_else(|| {
        "未找到客户端 Python 3.12。请随客户端放置 Runtime\\Python312，或设置 UGCAUDIT_PYTHON_SOURCE。"
            .to_string()
    })?;
    let destination = runtime_python_dir(app)?;
    emit_runtime_log(
        app,
        "runtime",
        "info",
        &format!("正在准备客户端 Python：{}", destination.display()),
    );
    copy_recursive(&source, &destination)?;
    Ok(python)
}

fn python_version(python: &Path) -> Option<String> {
    let mut command = Command::new(python);
    hide_child_console_window(&mut command);
    let output = command.arg("--version").output().ok()?;
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    Some(text.trim().to_string()).filter(|value| !value.is_empty())
}

fn ensure_pip_available(app: &tauri::AppHandle, python: &Path) -> Result<(), String> {
    let mut command = Command::new(python);
    hide_child_console_window(&mut command);
    let check = command
        .args(["-m", "pip", "--version"])
        .output()
        .map_err(|error| format!("无法检查 pip：{error}"))?;
    if check.status.success() {
        return Ok(());
    }

    let args = vec![
        "-m".to_string(),
        "ensurepip".to_string(),
        "--upgrade".to_string(),
    ];
    let output = run_command_with_logs(Some(app), "runtime", python, &args, None, &[])?;
    if output.success {
        Ok(())
    } else {
        Err("客户端 Python 中无法启用 pip。".to_string())
    }
}

fn dependency_version(
    python: &Path,
    site_packages: &Path,
    spec: &RuntimeDependencySpec,
) -> Option<String> {
    if !python.is_file() || !site_packages.exists() {
        return None;
    }
    let names = spec.seed_packages.join(",");
    let mut command = Command::new(python);
    hide_child_console_window(&mut command);
    let output = command
        .arg("-c")
        .arg(
            r#"import importlib.metadata as m, sys
for name in sys.argv[1].split(","):
    try:
        print(m.version(name))
        raise SystemExit(0)
    except Exception:
        pass
"#,
        )
        .arg(names)
        .env("PYTHONPATH", site_packages)
        .env("PYTHONNOUSERSITE", "1")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!version.is_empty()).then_some(version)
}

fn runtime_dependency_status(
    app: &tauri::AppHandle,
    spec: &RuntimeDependencySpec,
    python: &Path,
) -> RuntimeDependencyStatus {
    let folder = dependency_folder(app, spec.id).unwrap_or_default();
    let site_packages = dependency_site_packages(app, spec.id).unwrap_or_default();
    let installed = dependency_installed(app, spec) || site_packages_has_any_file(&site_packages);
    RuntimeDependencyStatus {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        installed,
        version: dependency_version(python, &site_packages, spec),
        folder: folder.display().to_string(),
        site_packages: site_packages.display().to_string(),
    }
}

fn runtime_status_inner(app: &tauri::AppHandle) -> Result<RuntimeStatus, String> {
    let (root, source) = runtime_root(app)?;
    let dependency_root = runtime_packages_dir(app)?;
    let python_dir = runtime_python_dir(app)?;
    let python_path = runtime_python_exe(app)?;
    let python_installed = python_path.is_file();
    let dependencies = runtime_dependency_specs()
        .iter()
        .map(|spec| runtime_dependency_status(app, spec, &python_path))
        .collect();

    Ok(RuntimeStatus {
        runtime_root: root.display().to_string(),
        runtime_source: source,
        dependency_root: dependency_root.display().to_string(),
        python_dir: python_dir.display().to_string(),
        python_path: python_path.display().to_string(),
        python_installed,
        python_version: python_installed
            .then(|| python_version(&python_path))
            .flatten(),
        dependencies,
    })
}

fn dependency_pythonpath(app: &tauri::AppHandle) -> Result<std::ffi::OsString, String> {
    let mut paths = Vec::new();
    for spec in runtime_dependency_specs() {
        let site_packages = dependency_site_packages(app, spec.id)?;
        if site_packages_has_any_file(&site_packages) {
            paths.push(site_packages);
        }
    }
    if paths.is_empty() {
        return Ok(std::ffi::OsString::new());
    }
    env::join_paths(paths).map_err(|error| format!("无法拼接 Python 依赖路径：{error}"))
}

fn append_runtime_env(
    app: &tauri::AppHandle,
    envs: &mut Vec<(String, String)>,
    module_dir: Option<&Path>,
) -> Result<(), String> {
    let (root, _) = runtime_root(app)?;
    let python = runtime_python_path(Some(app))?;
    envs.push((
        "UGCAUDIT_CLIENT_PYTHON".to_string(),
        python.display().to_string(),
    ));
    envs.push((
        "UGCAUDIT_RUNTIME_ROOT".to_string(),
        root.display().to_string(),
    ));
    envs.push((
        "UGCAUDIT_DEPENDENCY_ROOT".to_string(),
        runtime_packages_dir(app)?.display().to_string(),
    ));
    envs.push(("PYTHONNOUSERSITE".to_string(), "1".to_string()));
    if let Some(module_dir) = module_dir {
        envs.push((
            "UGCAUDIT_MODULE_DIR".to_string(),
            module_dir.display().to_string(),
        ));
    }
    envs.push((
        "PYTHONPATH".to_string(),
        dependency_pythonpath(app)?.to_string_lossy().to_string(),
    ));
    Ok(())
}

fn append_run_module_env(
    envs: &mut Vec<(String, String)>,
    run_id: &str,
    task_name: &str,
    resource_dir: &Path,
    artifact_dir: &Path,
    step_artifact_dir: &Path,
) {
    envs.push(("UGCAUDIT_RUN_ID".to_string(), run_id.to_string()));
    envs.push(("UGCAUDIT_TASK_NAME".to_string(), task_name.to_string()));
    envs.push((
        "UGCAUDIT_RESOURCE_ROOT".to_string(),
        resource_dir.display().to_string(),
    ));
    envs.push((
        "UGCAUDIT_ARTIFACT_DIR".to_string(),
        artifact_dir.display().to_string(),
    ));
    envs.push((
        "UGCAUDIT_STEP_ARTIFACT_DIR".to_string(),
        step_artifact_dir.display().to_string(),
    ));
}

fn dependency_pythonpath_for_dependency_root(
    dependency_root: &Path,
) -> Result<std::ffi::OsString, String> {
    let mut paths = Vec::new();
    for spec in runtime_dependency_specs() {
        let site_packages = dependency_site_packages_for_dependency_root(dependency_root, spec.id);
        if site_packages_has_any_file(&site_packages) {
            paths.push(site_packages);
        }
    }
    if paths.is_empty() {
        return Ok(std::ffi::OsString::new());
    }
    env::join_paths(paths).map_err(|error| format!("无法拼接 Python 依赖路径：{error}"))
}

fn configure_cli_runtime_env(data_root: &Path) -> Result<(), String> {
    let data_root = ensure_data_dirs_at(data_root)?;
    let (runtime_root, _) = runtime_root_for_data_root(&data_root);
    let dependency_root = dependency_root_from_settings(&data_root)?;
    let python = runtime_root
        .join(RUNTIME_PYTHON_DIR_NAME)
        .join("python.exe");
    if python.is_file() {
        env::set_var("UGCAUDIT_PYTHON", &python);
        env::set_var("UGCAUDIT_CLIENT_PYTHON", &python);
    }
    env::set_var("UGCAUDIT_RUNTIME_ROOT", &runtime_root);
    env::set_var("UGCAUDIT_DEPENDENCY_ROOT", &dependency_root);
    env::set_var("PYTHONNOUSERSITE", "1");
    env::set_var(
        "PYTHONPATH",
        dependency_pythonpath_for_dependency_root(&dependency_root)?,
    );
    Ok(())
}

fn read_pipe_to_log<R: std::io::Read + Send + 'static>(
    app: Option<tauri::AppHandle>,
    scope: String,
    stream: &'static str,
    reader: R,
    output: Arc<Mutex<String>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffered = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match buffered.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                    if !trimmed.is_empty() {
                        if let Some(app) = app.as_ref() {
                            emit_runtime_log(app, &scope, stream, &trimmed);
                        }
                    }
                    if let Ok(mut output) = output.lock() {
                        output.push_str(&line);
                    }
                }
                Err(error) => {
                    if let Some(app) = app.as_ref() {
                        emit_runtime_log(app, &scope, "error", &format!("读取输出失败：{error}"));
                    }
                    break;
                }
            }
        }
    })
}

fn run_command_with_logs(
    app: Option<&tauri::AppHandle>,
    scope: &str,
    command: &Path,
    args: &[String],
    cwd: Option<&Path>,
    envs: &[(String, String)],
) -> Result<CommandRunOutput, String> {
    if let Some(app) = app {
        emit_runtime_log(
            app,
            scope,
            "info",
            &format!("启动：{} {}", command.display(), args.join(" ")),
        );
    }
    let mut process = Command::new(command);
    hide_child_console_window(&mut process);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    for (key, value) in envs {
        process.env(key, value);
    }
    let mut child = process
        .spawn()
        .map_err(|error| format!("无法启动命令 {}: {error}", command.display()))?;
    let sampler = PerformanceSampler::start(child.id());

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取标准输出。".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法读取错误输出。".to_string())?;
    let stdout_output = Arc::new(Mutex::new(String::new()));
    let stderr_output = Arc::new(Mutex::new(String::new()));
    let stdout_thread = read_pipe_to_log(
        app.cloned(),
        scope.to_string(),
        "stdout",
        stdout,
        stdout_output.clone(),
    );
    let stderr_thread = read_pipe_to_log(
        app.cloned(),
        scope.to_string(),
        "stderr",
        stderr,
        stderr_output.clone(),
    );

    let status = child
        .wait()
        .map_err(|error| format!("等待命令结束失败 {}: {error}", command.display()))?;
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    let performance = sampler.finish();

    if let Some(app) = app {
        emit_runtime_log(
            app,
            scope,
            if status.success() { "info" } else { "error" },
            &format!("退出码：{}", status.code().unwrap_or(-1)),
        );
    }
    Ok(CommandRunOutput {
        success: status.success(),
        cancelled: false,
        stdout: stdout_output
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default(),
        stderr: stderr_output
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default(),
        performance,
    })
}

fn progress_value(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|number| usize::try_from(number).ok())
}

fn poll_progress_file(
    app: &tauri::AppHandle,
    run_id: &str,
    node_id: &str,
    progress_path: &Path,
    seen_lines: &mut usize,
) {
    let Ok(content) = fs::read_to_string(progress_path) else {
        return;
    };
    for line in content.lines().skip(*seen_lines) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let progress = value
            .get("progress")
            .and_then(Value::as_f64)
            .map(|number| number.clamp(0.0, 1.0));
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        emit_run_event(
            app,
            "step_progress",
            RunProgressEvent {
                run_id: run_id.to_string(),
                node_id: Some(node_id.to_string()),
                status: "running".to_string(),
                progress,
                message,
                processed: progress_value(&value, "processed"),
                total: progress_value(&value, "total"),
                step: None,
                run: None,
            },
        );
    }
    *seen_lines = content.lines().count();
}

#[allow(clippy::too_many_arguments)]
fn run_command_with_live_logs(
    app: &tauri::AppHandle,
    run_id: &str,
    node_id: &str,
    scope: &str,
    command: &Path,
    args: &[String],
    cwd: Option<&Path>,
    envs: &[(String, String)],
    progress_path: &Path,
    cancel_path: &Path,
    control: &RunControl,
) -> Result<CommandRunOutput, String> {
    emit_runtime_log(
        app,
        scope,
        "info",
        &format!("启动：{} {}", command.display(), args.join(" ")),
    );
    let mut process = Command::new(command);
    hide_child_console_window(&mut process);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    for (key, value) in envs {
        process.env(key, value);
    }
    let mut child = process
        .spawn()
        .map_err(|error| format!("无法启动命令 {}: {error}", command.display()))?;
    let sampler = PerformanceSampler::start(child.id());

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取标准输出。".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法读取错误输出。".to_string())?;
    let stdout_output = Arc::new(Mutex::new(String::new()));
    let stderr_output = Arc::new(Mutex::new(String::new()));
    let stdout_thread = read_pipe_to_log(
        Some(app.clone()),
        scope.to_string(),
        "stdout",
        stdout,
        stdout_output.clone(),
    );
    let stderr_thread = read_pipe_to_log(
        Some(app.clone()),
        scope.to_string(),
        "stderr",
        stderr,
        stderr_output.clone(),
    );

    let mut seen_progress_lines = 0_usize;
    let mut cancel_deadline: Option<Instant> = None;
    let mut cancelled = false;
    let mut killed = false;
    let status = loop {
        poll_progress_file(
            app,
            run_id,
            node_id,
            progress_path,
            &mut seen_progress_lines,
        );
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("等待命令结束失败 {}: {error}", command.display()))?
        {
            break status;
        }
        if control.is_cancelled() {
            cancelled = true;
            if cancel_deadline.is_none() {
                let _ = fs::write(cancel_path, "cancelled");
                emit_runtime_log(app, scope, "info", "已通知模块中断。");
                cancel_deadline = Some(Instant::now() + Duration::from_secs(3));
            } else if !killed
                && cancel_deadline
                    .map(|deadline| Instant::now() >= deadline)
                    .unwrap_or(false)
            {
                emit_runtime_log(app, scope, "error", "模块未及时退出，已强制停止。");
                let _ = child.kill();
                killed = true;
            }
        }
        thread::sleep(Duration::from_millis(120));
    };
    poll_progress_file(
        app,
        run_id,
        node_id,
        progress_path,
        &mut seen_progress_lines,
    );
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    let performance = sampler.finish();

    emit_runtime_log(
        app,
        scope,
        if status.success() && !cancelled {
            "info"
        } else {
            "error"
        },
        &format!("退出码：{}", status.code().unwrap_or(-1)),
    );
    Ok(CommandRunOutput {
        success: status.success() && !cancelled,
        cancelled,
        stdout: stdout_output
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default(),
        stderr: stderr_output
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default(),
        performance,
    })
}

fn install_dependency_inner(
    app: &tauri::AppHandle,
    dependency_id: &str,
) -> Result<RuntimeStatus, String> {
    let spec = runtime_dependency_spec(dependency_id)?;
    let python = ensure_runtime_python(app)?;
    ensure_pip_available(app, &python)?;
    let site_packages = dependency_site_packages(app, spec.id)?;
    fs::create_dir_all(&site_packages)
        .map_err(|error| format!("无法创建依赖目录 {}: {error}", site_packages.display()))?;

    let mut args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--upgrade".to_string(),
        "--target".to_string(),
        site_packages.display().to_string(),
    ];
    args.extend(spec.install_args.iter().map(|value| value.to_string()));
    let output = run_command_with_logs(Some(app), spec.id, &python, &args, None, &[])?;
    if !output.success {
        return Err(format!("安装 {} 失败。", spec.name));
    }
    emit_runtime_status_changed(app);
    runtime_status_inner(app)
}

fn system_launch(notes: &str) -> ModuleLaunch {
    ModuleLaunch {
        launch_type: "system".to_string(),
        notes: notes.to_string(),
        ..ModuleLaunch::default()
    }
}

fn relative_path_parameter(default_value: &str) -> ModuleParameter {
    ModuleParameter {
        key: "relativePath".to_string(),
        name: "相对路径".to_string(),
        description: "只收集待测项目中这个相对路径下的文件。".to_string(),
        parameter_type: "string".to_string(),
        default_value: json!(default_value),
        required: true,
        options: vec![],
    }
}

fn system_module(
    id: &str,
    name: &str,
    kind: &str,
    summary: &str,
    icon: &str,
    notes: &str,
    parameters: Vec<ModuleParameter>,
) -> ModuleInfo {
    ModuleInfo {
        id: id.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        summary: summary.to_string(),
        model_label: "无需模型".to_string(),
        icon: icon.to_string(),
        built_in: true,
        source: "system".to_string(),
        definition_dir: String::new(),
        icon_path: None,
        icon_data_url: None,
        model_path: None,
        model_configured: true,
        launch: system_launch(notes),
        parameters,
        data_outputs: Vec::new(),
    }
}

fn module_default_config(module: &ModuleInfo) -> Value {
    let mut map = serde_json::Map::new();
    for parameter in &module.parameters {
        let value = if parameter.key == "modelPath" {
            module
                .model_path
                .as_ref()
                .filter(|path| !path.trim().is_empty())
                .map(|path| json!(path))
                .unwrap_or_else(|| parameter.default_value.clone())
        } else {
            parameter.default_value.clone()
        };
        map.insert(parameter.key.clone(), value);
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

fn normalized_edge_type(value: &str) -> String {
    if value == EDGE_TYPE_DATA {
        EDGE_TYPE_DATA.to_string()
    } else {
        EDGE_TYPE_SEQUENCE.to_string()
    }
}

fn normalize_flow_edge(edge: &mut FlowEdge) {
    edge.edge_type = normalized_edge_type(&edge.edge_type);
    if edge.edge_type == EDGE_TYPE_SEQUENCE {
        if edge.from_handle.as_deref().unwrap_or("").trim().is_empty() {
            edge.from_handle = Some(HANDLE_SEQUENCE_OUT.to_string());
        }
        if edge.to_handle.as_deref().unwrap_or("").trim().is_empty() {
            edge.to_handle = Some(HANDLE_SEQUENCE_IN.to_string());
        }
    }
}

fn builtin_module_definitions() -> Vec<ModuleInfo> {
    vec![
        system_module(
            START_MODULE_ID,
            "开始",
            "flow_start",
            "流程入口，接收本次审核素材",
            "play-circle",
            "流程入口，不启动外部模块。",
            vec![],
        ),
        system_module(
            OUTPUT_MODULE_ID,
            "输出结果",
            "flow_output",
            "汇总所有步骤并生成 Markdown 报告",
            "file-output",
            "报告汇总节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_ALL_IMAGES_MODULE_ID,
            "待测项目中所有图片",
            "data_all_images",
            "提供本次待测项目里的全部图片集合",
            "database",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_ALL_TEXTS_MODULE_ID,
            "待测项目中所有文本",
            "data_all_texts",
            "提供本次待测项目里的全部文本集合",
            "file-text",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_ARTIFACT_IMAGES_MODULE_ID,
            "产物文件夹中所有图片",
            "data_artifact_images",
            "提供本次审核产物文件夹里的全部图片集合",
            "hard-drive",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_ARTIFACT_TEXTS_MODULE_ID,
            "产物文件夹中所有文本",
            "data_artifact_texts",
            "提供本次审核产物文件夹里的全部文本集合",
            "hard-drive",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_RELATIVE_IMAGES_MODULE_ID,
            "待测项目相对路径下所有图片",
            "data_relative_images",
            "提供本次待测项目指定相对路径下的图片集合",
            "folder-open",
            "数据提供节点，不启动外部模块。",
            vec![relative_path_parameter("images")],
        ),
        system_module(
            DATA_RELATIVE_TEXTS_MODULE_ID,
            "待测项目相对路径下所有文本",
            "data_relative_texts",
            "提供本次待测项目指定相对路径下的文本集合",
            "folder-open",
            "数据提供节点，不启动外部模块。",
            vec![relative_path_parameter("texts")],
        ),
        system_module(
            DATA_AUDIT_FOLDER_MODULE_ID,
            "待审核文件夹",
            "data_audit_folder",
            "提供本次待审核文件夹",
            "folder-open",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID,
            "待审核文件夹下相对路径文件夹",
            "data_audit_relative_folder",
            "提供本次待审核文件夹下指定相对路径的文件夹",
            "folder-open",
            "数据提供节点，不启动外部模块。",
            vec![relative_path_parameter("Assets")],
        ),
        system_module(
            DATA_ARTIFACT_FOLDER_MODULE_ID,
            "产物文件夹",
            "data_artifact_folder",
            "提供本次审核产物文件夹",
            "hard-drive",
            "数据提供节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID,
            "待产物文件夹下相对路径文件夹",
            "data_artifact_relative_folder",
            "提供本次审核产物文件夹下指定相对路径的文件夹",
            "hard-drive",
            "数据提供节点，不启动外部模块。",
            vec![relative_path_parameter("outputs")],
        ),
        system_module(
            DATA_MERGE_IMAGES_MODULE_ID,
            "将两个图片集合合并",
            "data_merge_images",
            "把两个图片集合合并并去重",
            "database",
            "数据合并节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            DATA_MERGE_TEXTS_MODULE_ID,
            "将两个文本集合合并",
            "data_merge_texts",
            "把两个文本集合合并并去重",
            "database",
            "数据合并节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            ANNOTATION_MODULE_ID,
            "注释框",
            "annotation_comment",
            "画布注释和分组框",
            "file-text",
            "注释节点，不启动外部模块。",
            vec![],
        ),
        system_module(
            CANVAS_GROUP_MODULE_ID,
            "分组",
            "canvas_group",
            "创建带名称的画布分组框",
            "group",
            "画布工具组件，不启动外部模块。",
            vec![],
        ),
        system_module(
            CANVAS_NOTE_MODULE_ID,
            "注释",
            "canvas_note",
            "创建独立注释便签",
            "sticky-note",
            "画布工具组件，不启动外部模块。",
            vec![],
        ),
    ]
}

fn module_definition_file(folder: &Path) -> PathBuf {
    folder.join("module.json")
}

fn module_folder_name(module_id: &str) -> String {
    module_id
        .trim()
        .chars()
        .map(|character| match character {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ if character.is_control() => '_',
            _ => character,
        })
        .collect::<String>()
        .trim_matches(|character| character == ' ' || character == '.')
        .to_string()
}

fn module_folder(root: &Path, module_id: &str) -> PathBuf {
    root.join("modules").join(module_folder_name(module_id))
}

fn is_builtin_module_id(module_id: &str) -> bool {
    matches!(
        module_id,
        START_MODULE_ID
            | OUTPUT_MODULE_ID
            | DATA_ALL_IMAGES_MODULE_ID
            | DATA_ALL_TEXTS_MODULE_ID
            | DATA_ARTIFACT_IMAGES_MODULE_ID
            | DATA_ARTIFACT_TEXTS_MODULE_ID
            | DATA_RELATIVE_IMAGES_MODULE_ID
            | DATA_RELATIVE_TEXTS_MODULE_ID
            | DATA_AUDIT_FOLDER_MODULE_ID
            | DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID
            | DATA_ARTIFACT_FOLDER_MODULE_ID
            | DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID
            | DATA_MERGE_IMAGES_MODULE_ID
            | DATA_MERGE_TEXTS_MODULE_ID
            | ANNOTATION_MODULE_ID
            | CANVAS_GROUP_MODULE_ID
            | CANVAS_NOTE_MODULE_ID
    )
}

fn is_pure_data_module_id(module_id: &str) -> bool {
    matches!(
        module_id,
        DATA_ALL_IMAGES_MODULE_ID
            | DATA_ALL_TEXTS_MODULE_ID
            | DATA_ARTIFACT_IMAGES_MODULE_ID
            | DATA_ARTIFACT_TEXTS_MODULE_ID
            | DATA_RELATIVE_IMAGES_MODULE_ID
            | DATA_RELATIVE_TEXTS_MODULE_ID
            | DATA_AUDIT_FOLDER_MODULE_ID
            | DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID
            | DATA_ARTIFACT_FOLDER_MODULE_ID
            | DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID
            | DATA_MERGE_IMAGES_MODULE_ID
            | DATA_MERGE_TEXTS_MODULE_ID
            | ANNOTATION_MODULE_ID
            | CANVAS_GROUP_MODULE_ID
            | CANVAS_NOTE_MODULE_ID
    )
}

fn is_legacy_preset_module_id(module_id: &str) -> bool {
    matches!(
        module_id,
        "preset.custom.paddleocr" | "preset.custom.shieldgemma2" | "preset.custom.qwen3guard"
    )
}

fn is_pure_data_module_kind(kind: &str) -> bool {
    matches!(
        kind,
        "data_all_images"
            | "data_all_texts"
            | "data_artifact_images"
            | "data_artifact_texts"
            | "data_relative_images"
            | "data_relative_texts"
            | "data_audit_folder"
            | "data_audit_relative_folder"
            | "data_artifact_folder"
            | "data_artifact_relative_folder"
            | "data_merge_images"
            | "data_merge_texts"
            | "annotation_comment"
            | "canvas_group"
            | "canvas_note"
    )
}

fn has_sequence_input(module: &ModuleInfo) -> bool {
    !matches!(module.kind.as_str(), "flow_start") && !is_pure_data_module_kind(&module.kind)
}

fn has_sequence_output(module: &ModuleInfo) -> bool {
    !matches!(module.kind.as_str(), "flow_output") && !is_pure_data_module_kind(&module.kind)
}

fn known_data_type(data_type: &str) -> Option<&'static str> {
    match data_type {
        DATA_TYPE_IMAGES => Some(DATA_TYPE_IMAGES),
        DATA_TYPE_TEXTS => Some(DATA_TYPE_TEXTS),
        DATA_TYPE_FOLDER => Some(DATA_TYPE_FOLDER),
        _ => None,
    }
}

fn data_input_type(module: &ModuleInfo, handle: &str) -> Option<&'static str> {
    match module.kind.as_str() {
        "image_ocr" | "image_safety" if handle == HANDLE_IMAGES_IN => Some(DATA_TYPE_IMAGES),
        "text_safety" if handle == HANDLE_TEXTS_IN => Some(DATA_TYPE_TEXTS),
        "data_merge_images" if matches!(handle, HANDLE_IMAGES_A_IN | HANDLE_IMAGES_B_IN) => {
            Some(DATA_TYPE_IMAGES)
        }
        "data_merge_texts" if matches!(handle, HANDLE_TEXTS_A_IN | HANDLE_TEXTS_B_IN) => {
            Some(DATA_TYPE_TEXTS)
        }
        "folder_processor" if handle == HANDLE_FOLDER_IN => Some(DATA_TYPE_FOLDER),
        _ => None,
    }
}

fn declared_data_output_type(module: &ModuleInfo, handle: &str) -> Option<String> {
    module
        .data_outputs
        .iter()
        .find(|output| output.handle == handle)
        .and_then(|output| known_data_type(&output.data_type))
        .map(ToString::to_string)
}

fn data_output_type(module: &ModuleInfo, handle: &str) -> Option<String> {
    if let Some(data_type) = declared_data_output_type(module, handle) {
        return Some(data_type);
    }
    match module.kind.as_str() {
        "image_ocr" if handle == HANDLE_TEXTS_OUT => Some(DATA_TYPE_TEXTS.to_string()),
        "data_all_images"
        | "data_artifact_images"
        | "data_relative_images"
        | "data_merge_images"
            if handle == HANDLE_IMAGES_OUT =>
        {
            Some(DATA_TYPE_IMAGES.to_string())
        }
        "data_all_texts" | "data_artifact_texts" | "data_relative_texts" | "data_merge_texts"
            if handle == HANDLE_TEXTS_OUT =>
        {
            Some(DATA_TYPE_TEXTS.to_string())
        }
        "data_audit_folder"
        | "data_audit_relative_folder"
        | "data_artifact_folder"
        | "data_artifact_relative_folder"
            if handle == HANDLE_FOLDER_OUT =>
        {
            Some(DATA_TYPE_FOLDER.to_string())
        }
        _ => None,
    }
}

fn required_data_inputs(module: &ModuleInfo) -> Vec<(&'static str, &'static str)> {
    match module.kind.as_str() {
        "image_ocr" | "image_safety" => vec![(HANDLE_IMAGES_IN, DATA_TYPE_IMAGES)],
        "text_safety" => vec![(HANDLE_TEXTS_IN, DATA_TYPE_TEXTS)],
        "data_merge_images" => vec![
            (HANDLE_IMAGES_A_IN, DATA_TYPE_IMAGES),
            (HANDLE_IMAGES_B_IN, DATA_TYPE_IMAGES),
        ],
        "data_merge_texts" => vec![
            (HANDLE_TEXTS_A_IN, DATA_TYPE_TEXTS),
            (HANDLE_TEXTS_B_IN, DATA_TYPE_TEXTS),
        ],
        "folder_processor" => vec![(HANDLE_FOLDER_IN, DATA_TYPE_FOLDER)],
        _ => Vec::new(),
    }
}

fn has_required_relative_path(module: &ModuleInfo, config: &Value) -> bool {
    if !matches!(
        module.kind.as_str(),
        "data_relative_images"
            | "data_relative_texts"
            | "data_audit_relative_folder"
            | "data_artifact_relative_folder"
    ) {
        return true;
    }
    config
        .get("relativePath")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

fn is_image_icon_reference(icon: &str) -> bool {
    let trimmed = icon.trim();
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.starts_with('.') {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    matches!(
        Path::new(&lower)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico")
    )
}

fn resolve_module_icon_path(folder: &Path, icon: &str) -> Option<String> {
    let trimmed = icon.trim();
    if !is_image_icon_reference(trimmed) {
        return None;
    }

    let icon_path = Path::new(trimmed);
    let resolved = if icon_path.is_absolute() {
        icon_path.to_path_buf()
    } else {
        folder.join(icon_path)
    };

    resolved.is_file().then(|| resolved.display().to_string())
}

fn icon_mime_type(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("svg") => Some("image/svg+xml"),
        Some("gif") => Some("image/gif"),
        Some("bmp") => Some("image/bmp"),
        Some("ico") => Some("image/x-icon"),
        _ => None,
    }
}

fn module_icon_data_url(icon_path: &Path) -> Option<String> {
    let mime_type = icon_mime_type(icon_path)?;
    let bytes = fs::read(icon_path).ok()?;
    Some(format!(
        "data:{mime_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

fn normalize_module_definition(
    mut module: ModuleInfo,
    folder: &Path,
) -> Result<ModuleInfo, String> {
    module.id = normalize_module_id(module.id.trim());
    module.name = module.name.trim().to_string();
    module.kind = module.kind.trim().to_string();
    module.icon = module.icon.trim().to_string();
    module.definition_dir = folder.display().to_string();
    module.icon_path = resolve_module_icon_path(folder, &module.icon);
    module.icon_data_url = module
        .icon_path
        .as_ref()
        .and_then(|icon_path| module_icon_data_url(Path::new(icon_path)));
    if module
        .model_path
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        let default_model_dir = folder.join("Model");
        if default_model_dir.is_dir() {
            module.model_path = Some(default_model_dir.display().to_string());
            module.model_configured = true;
        }
    }

    if module.id.is_empty() {
        return Err(format!(
            "模块定义 {} 缺少 id。",
            module_definition_file(folder).display()
        ));
    }
    if module.name.is_empty() {
        return Err(format!("模块 {} 缺少 name。", module.id));
    }
    if module.kind.is_empty() {
        return Err(format!("模块 {} 缺少 kind。", module.id));
    }
    if module.icon.is_empty() {
        return Err(format!("模块 {} 缺少 icon。", module.id));
    }
    for output in &mut module.data_outputs {
        output.handle = output.handle.trim().to_string();
        output.name = output.name.trim().to_string();
        output.data_type = output.data_type.trim().to_string();
        if output.handle.is_empty() {
            return Err(format!("模块 {} 存在空的数据输出口。", module.id));
        }
        if output.name.is_empty() {
            output.name = output.handle.clone();
        }
        if known_data_type(&output.data_type).is_none() {
            return Err(format!(
                "模块 {} 的数据输出口 {} 使用了未知数据类型：{}",
                module.id, output.handle, output.data_type
            ));
        }
    }
    if is_image_icon_reference(&module.icon) && module.icon_path.is_none() {
        return Err(format!(
            "模块 {} 的图标文件不存在：{}",
            module.id, module.icon
        ));
    }

    Ok(module)
}

fn module_readme(module: &ModuleInfo) -> String {
    format!(
        "# {}\n\n{}\n\n- 模块 ID：{}\n- 类型：{}\n- 来源：{}\n- 启动方式：{}\n\n这个文件夹是模块定义目录。`module.json` 描述模块参数和入口信息。\n",
        module.name, module.summary, module.id, module.kind, module.source, module.launch.launch_type
    )
}

fn write_module_definition(path: &Path, module: &ModuleInfo) -> Result<(), String> {
    let mut value = serde_json::to_value(module)
        .map_err(|error| format!("无法序列化模块定义 {}: {error}", module.id))?;
    if let Value::Object(map) = &mut value {
        map.remove("definitionDir");
        map.remove("iconPath");
        map.remove("iconDataUrl");
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
        write_module_definition(&definition_file, &module)?;

        let readme_file = folder.join("README.md");
        fs::write(&readme_file, module_readme(&module))
            .map_err(|error| format!("无法写入模块说明 {}: {error}", readme_file.display()))?;
    }
    Ok(())
}

fn apply_model_path(mut module: ModuleInfo, paths: &ModelPaths) -> ModuleInfo {
    if module.source == "system" {
        module.model_path = None;
        module.model_configured = true;
        return module;
    }

    if let Some(path) = paths
        .get(&module.id)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        module.model_path = Some(path.to_string());
        module.model_configured = true;
    }
    module
}

fn load_module_from_folder(folder: &Path, force_custom: bool) -> Result<ModuleInfo, String> {
    let definition_file = module_definition_file(folder);
    let module: ModuleInfo = read_required_json(&definition_file)?;
    let mut module = normalize_module_definition(module, folder)?;
    if force_custom {
        module.built_in = false;
        module.source = "custom".to_string();
    }
    Ok(module)
}

fn imported_modules_file(root: &Path) -> PathBuf {
    root.join("settings").join("imported-modules.json")
}

fn load_imported_module_paths(root: &Path) -> Result<ImportedModulePaths, String> {
    read_json(&imported_modules_file(root))
}

fn write_imported_module_paths(root: &Path, paths: &ImportedModulePaths) -> Result<(), String> {
    write_json(&imported_modules_file(root), paths)
}

fn default_preset_module_folders() -> Vec<PathBuf> {
    #[cfg(test)]
    let root = match env::var("UGCAUDIT_MODEL_ROOT") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return Vec::new(),
    };
    #[cfg(not(test))]
    let root = env::var("UGCAUDIT_MODEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_MODEL_ROOT));

    [
        "preset.custom.paddleocr",
        "preset.custom.qwen3guard",
        "preset.custom.shieldgemma2",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .filter(|folder| module_definition_file(folder).is_file())
    .collect()
}

fn register_imported_module_path(
    root: &Path,
    folder: &Path,
    module_id: &str,
) -> Result<(), String> {
    let canonical = folder
        .canonicalize()
        .map_err(|error| format!("无法读取模块文件夹 {}: {error}", folder.display()))?;
    let canonical_string = canonical.display().to_string();
    let mut paths = load_imported_module_paths(root)?;
    paths.retain(|path| {
        let folder = Path::new(path);
        let is_same_path = folder
            .canonicalize()
            .map(|current| current == canonical)
            .unwrap_or(false);
        let is_same_module = folder.is_dir()
            && load_module_from_folder(folder, true)
                .map(|module| module.id == module_id)
                .unwrap_or(false);
        !is_same_path && !is_same_module
    });
    paths.push(canonical_string);
    paths.sort();
    write_imported_module_paths(root, &paths)
}

fn unregister_imported_module_id(root: &Path, module_id: &str) -> Result<bool, String> {
    let paths = load_imported_module_paths(root)?;
    let mut removed = false;
    let mut next_paths = Vec::new();

    for path in paths {
        let folder = PathBuf::from(&path);
        let should_remove = folder.is_dir()
            && load_module_from_folder(&folder, true)
                .map(|module| module.id == module_id)
                .unwrap_or(false);
        if should_remove {
            removed = true;
        } else {
            next_paths.push(path);
        }
    }

    if removed {
        write_imported_module_paths(root, &next_paths)?;
    }
    Ok(removed)
}

fn module_order(module_id: &str) -> usize {
    match module_id {
        START_MODULE_ID => 0,
        OUTPUT_MODULE_ID => 1,
        DATA_ALL_IMAGES_MODULE_ID | DATA_ALL_TEXTS_MODULE_ID => 2,
        DATA_ARTIFACT_IMAGES_MODULE_ID | DATA_ARTIFACT_TEXTS_MODULE_ID => 3,
        DATA_RELATIVE_IMAGES_MODULE_ID | DATA_RELATIVE_TEXTS_MODULE_ID => 4,
        DATA_AUDIT_FOLDER_MODULE_ID | DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID => 5,
        DATA_ARTIFACT_FOLDER_MODULE_ID | DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID => 6,
        DATA_MERGE_IMAGES_MODULE_ID | DATA_MERGE_TEXTS_MODULE_ID => 7,
        _ => 100,
    }
}

fn load_modules(root: &Path, paths: &ModelPaths) -> Result<Vec<ModuleInfo>, String> {
    ensure_builtin_module_folders(root)?;

    let modules_dir = root.join("modules");
    let mut modules = HashMap::new();
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
        let module = load_module_from_folder(&folder, false)?;
        if module.source == "preset" && is_legacy_preset_module_id(&module.id) {
            continue;
        }
        modules.insert(module.id.clone(), apply_model_path(module, paths));
    }

    for folder in default_preset_module_folders() {
        let module = load_module_from_folder(&folder, false)?;
        if is_builtin_module_id(&module.id) || module.source == "system" {
            continue;
        }
        modules.insert(module.id.clone(), apply_model_path(module, paths));
    }

    for folder in load_imported_module_paths(root)?
        .into_iter()
        .map(PathBuf::from)
        .filter(|folder| folder.is_dir())
    {
        let module = load_module_from_folder(&folder, true)?;
        if is_builtin_module_id(&module.id) || module.source == "system" {
            continue;
        }
        modules.insert(module.id.clone(), apply_model_path(module, paths));
    }

    let mut modules = modules.into_values().collect::<Vec<_>>();
    modules.sort_by(|left, right| {
        module_order(&left.id)
            .cmp(&module_order(&right.id))
            .then(left.name.cmp(&right.name))
    });
    Ok(modules)
}

fn import_module_folder_inner(root: &Path, source_folder: &Path) -> Result<String, String> {
    if !source_folder.is_dir() {
        return Err(format!("请选择模块文件夹：{}", source_folder.display()));
    }

    let source_definition = module_definition_file(source_folder);
    if !source_definition.exists() {
        return Err(format!(
            "不是模块文件夹，缺少 module.json：{}",
            source_folder.display()
        ));
    }

    let module = load_module_from_folder(source_folder, false)?;
    if is_builtin_module_id(&module.id) || module.source == "system" {
        return Err(format!("不能导入内置模块 ID：{}", module.id));
    }

    if module_folder_name(&module.id).is_empty() {
        return Err(format!("模块 {} 不能作为文件夹名称。", module.id));
    }

    register_imported_module_path(root, source_folder, &module.id)?;
    Ok(module.id)
}

fn remove_module_inner(root: &Path, module_id: &str) -> Result<String, String> {
    let normalized_module_id = normalize_module_id(module_id.trim());
    if normalized_module_id.is_empty() {
        return Err("请选择要移除的模块。".to_string());
    }
    if is_builtin_module_id(&normalized_module_id) {
        return Err("预置模块不能移除。".to_string());
    }

    let paths = HashMap::new();
    let modules = modules_by_id(root, &paths)?;
    let module = modules
        .get(&normalized_module_id)
        .ok_or_else(|| format!("找不到模块 {}", normalized_module_id))?;
    if module.source != "custom" || module.built_in {
        return Err("只能移除导入的自定义模块。".to_string());
    }

    let flow_path = default_flow_file(root);
    if flow_path.exists() {
        let flow: FlowDefinition = read_required_json(&flow_path)?;
        if flow
            .nodes
            .iter()
            .any(|node| normalize_module_id(&node.module_id) == normalized_module_id)
        {
            return Err(format!(
                "模块 {} 已在当前流程中使用，请先从流程里删除相关步骤。",
                module.name
            ));
        }
    }

    if unregister_imported_module_id(root, &normalized_module_id)? {
        return Ok(normalized_module_id);
    }

    let modules_dir = root.join("modules");
    let modules_root = modules_dir
        .canonicalize()
        .map_err(|error| format!("无法读取模块根目录 {}: {error}", modules_dir.display()))?;
    let folder = PathBuf::from(&module.definition_dir);
    let folder_canonical = folder
        .canonicalize()
        .map_err(|error| format!("无法读取模块文件夹 {}: {error}", folder.display()))?;
    if !folder_canonical.starts_with(&modules_root) {
        return Err(format!(
            "拒绝移除模块目录外的文件夹：{}",
            folder_canonical.display()
        ));
    }

    fs::remove_dir_all(&folder_canonical)
        .map_err(|error| format!("无法移除模块文件夹 {}: {error}", folder_canonical.display()))?;

    Ok(normalized_module_id)
}

fn modules_by_id(root: &Path, paths: &ModelPaths) -> Result<HashMap<String, ModuleInfo>, String> {
    Ok(load_modules(root, paths)?
        .into_iter()
        .map(|module| (module.id.clone(), module))
        .collect())
}

fn normalize_flow(
    mut flow: FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
) -> FlowDefinition {
    for node in &mut flow.nodes {
        node.module_id = normalize_module_id(&node.module_id);
        if let Some(module) = modules.get(&node.module_id) {
            let defaults = module_default_config(module);
            node.config = merge_config(&defaults, &node.config);
            if module
                .parameters
                .iter()
                .any(|parameter| parameter.key == "modelPath")
            {
                if let (Value::Object(map), Some(model_path)) =
                    (&mut node.config, &module.model_path)
                {
                    let current = map
                        .get("modelPath")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or("");
                    if current.is_empty() && !model_path.trim().is_empty() {
                        map.insert("modelPath".to_string(), json!(model_path));
                    }
                }
            }
            if let Value::Object(map) = &mut node.config {
                match module.id.as_str() {
                    "preset.custom.paddleocr" => {
                        map.remove("modelPath");
                        map.remove("profile");
                    }
                    "preset.custom.qwen3guard" => {
                        map.remove("input");
                        map.remove("modelPath");
                        map.remove("modelSize");
                    }
                    "preset.custom.shieldgemma2" => {
                        map.remove("modelPath");
                    }
                    _ => {}
                }
            }
        }
    }
    for edge in &mut flow.edges {
        normalize_flow_edge(edge);
    }
    let flow = ensure_system_nodes(flow);
    if is_minimal_system_flow(&flow) {
        default_flow()
    } else {
        upgrade_legacy_default_data_flow(flow)
    }
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
                position: Position { x: 120.0, y: 260.0 },
                config: json!({}),
            },
            FlowNode {
                id: "all_images".to_string(),
                module_id: DATA_ALL_IMAGES_MODULE_ID.to_string(),
                label: "待测项目中所有图片".to_string(),
                position: Position { x: 360.0, y: 60.0 },
                config: json!({}),
            },
            FlowNode {
                id: "image_ocr".to_string(),
                module_id: "preset.custom.paddleocr".to_string(),
                label: "图片文字识别".to_string(),
                position: Position { x: 520.0, y: 160.0 },
                config: json!({}),
            },
            FlowNode {
                id: "image_safety".to_string(),
                module_id: "preset.custom.shieldgemma2".to_string(),
                label: "图片合规检测".to_string(),
                position: Position { x: 520.0, y: 360.0 },
                config: json!({}),
            },
            FlowNode {
                id: "text_safety".to_string(),
                module_id: "preset.custom.qwen3guard".to_string(),
                label: "文本合规检测".to_string(),
                position: Position { x: 820.0, y: 160.0 },
                config: json!({}),
            },
            FlowNode {
                id: OUTPUT_NODE_ID.to_string(),
                module_id: OUTPUT_MODULE_ID.to_string(),
                label: "输出结果".to_string(),
                position: Position {
                    x: 1120.0,
                    y: 260.0,
                },
                config: json!({}),
            },
        ],
        edges: vec![
            sequence_edge("edge_flow_start_image_ocr", START_NODE_ID, "image_ocr"),
            sequence_edge(
                "edge_flow_start_image_safety",
                START_NODE_ID,
                "image_safety",
            ),
            sequence_edge("edge_seq_image_ocr_text_safety", "image_ocr", "text_safety"),
            sequence_edge("edge_image_safety_output", "image_safety", OUTPUT_NODE_ID),
            sequence_edge("edge_text_safety_output", "text_safety", OUTPUT_NODE_ID),
            data_edge(
                "edge_all_images_image_ocr",
                "all_images",
                HANDLE_IMAGES_OUT,
                "image_ocr",
                HANDLE_IMAGES_IN,
            ),
            data_edge(
                "edge_all_images_image_safety",
                "all_images",
                HANDLE_IMAGES_OUT,
                "image_safety",
                HANDLE_IMAGES_IN,
            ),
            data_edge(
                "edge_data_image_ocr_text_safety",
                "image_ocr",
                HANDLE_TEXTS_OUT,
                "text_safety",
                HANDLE_TEXTS_IN,
            ),
        ],
    }
}

fn first_node_id_by_module(flow: &FlowDefinition, module_id: &str) -> Option<String> {
    flow.nodes
        .iter()
        .find(|node| node.module_id == module_id)
        .map(|node| node.id.clone())
}

fn flow_has_data_edges(flow: &FlowDefinition) -> bool {
    flow.edges
        .iter()
        .any(|edge| edge.edge_type == EDGE_TYPE_DATA)
}

fn unique_node_id(flow: &FlowDefinition, preferred: &str) -> String {
    if !flow.nodes.iter().any(|node| node.id == preferred) {
        return preferred.to_string();
    }
    let mut index = 2;
    loop {
        let candidate = format!("{preferred}_{index}");
        if !flow.nodes.iter().any(|node| node.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn upgrade_legacy_default_data_flow(mut flow: FlowDefinition) -> FlowDefinition {
    if flow.id != "flow.default.image-audit" || flow_has_data_edges(&flow) {
        return flow;
    }

    let Some(ocr_id) = first_node_id_by_module(&flow, "preset.custom.paddleocr") else {
        return flow;
    };
    let Some(image_safety_id) = first_node_id_by_module(&flow, "preset.custom.shieldgemma2") else {
        return flow;
    };
    let Some(text_safety_id) = first_node_id_by_module(&flow, "preset.custom.qwen3guard") else {
        return flow;
    };

    let image_source_id = unique_node_id(&flow, "all_images");
    flow.nodes.push(FlowNode {
        id: image_source_id.clone(),
        module_id: DATA_ALL_IMAGES_MODULE_ID.to_string(),
        label: "待测项目中所有图片".to_string(),
        position: Position { x: 360.0, y: 60.0 },
        config: json!({}),
    });
    flow.edges.push(data_edge(
        "edge_all_images_image_ocr",
        &image_source_id,
        HANDLE_IMAGES_OUT,
        &ocr_id,
        HANDLE_IMAGES_IN,
    ));
    flow.edges.push(data_edge(
        "edge_all_images_image_safety",
        &image_source_id,
        HANDLE_IMAGES_OUT,
        &image_safety_id,
        HANDLE_IMAGES_IN,
    ));
    flow.edges.push(data_edge(
        "edge_data_image_ocr_text_safety",
        &ocr_id,
        HANDLE_TEXTS_OUT,
        &text_safety_id,
        HANDLE_TEXTS_IN,
    ));
    flow
}

fn has_system_node(flow: &FlowDefinition, module_id: &str) -> bool {
    flow.nodes.iter().any(|node| node.module_id == module_id)
}

fn is_minimal_system_flow(flow: &FlowDefinition) -> bool {
    if flow.nodes.len() != 2 {
        return false;
    }
    let start = flow
        .nodes
        .iter()
        .find(|node| node.module_id == START_MODULE_ID);
    let output = flow
        .nodes
        .iter()
        .find(|node| node.module_id == OUTPUT_MODULE_ID);
    matches!((start, output), (Some(start), Some(output)) if flow
        .edges
        .iter()
        .any(|edge| edge.from == start.id && edge.to == output.id))
}

fn sequence_edge(id: &str, from: &str, to: &str) -> FlowEdge {
    FlowEdge {
        id: id.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        edge_type: EDGE_TYPE_SEQUENCE.to_string(),
        from_handle: Some(HANDLE_SEQUENCE_OUT.to_string()),
        to_handle: Some(HANDLE_SEQUENCE_IN.to_string()),
    }
}

fn data_edge(id: &str, from: &str, from_handle: &str, to: &str, to_handle: &str) -> FlowEdge {
    FlowEdge {
        id: id.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        edge_type: EDGE_TYPE_DATA.to_string(),
        from_handle: Some(from_handle.to_string()),
        to_handle: Some(to_handle.to_string()),
    }
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

    if nodes.len() == 2
        && !edges
            .iter()
            .any(|edge| edge.from == start_id && edge.to == output_id)
    {
        edges.push(sequence_edge(
            "edge_flow_start_output",
            &start_id,
            &output_id,
        ));
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

fn default_scheme_id() -> String {
    format!("scheme_{}", now_millis())
}

fn scheme_from_flow(flow: FlowDefinition) -> AuditScheme {
    AuditScheme {
        schema_version: default_scheme_schema_version(),
        kind: default_scheme_kind(),
        id: default_scheme_id(),
        name: if flow.name.trim().is_empty() {
            "未命名审核方案".to_string()
        } else {
            flow.name.clone()
        },
        flow,
    }
}

fn normalize_scheme(mut scheme: AuditScheme, modules: &HashMap<String, ModuleInfo>) -> AuditScheme {
    scheme.schema_version = default_scheme_schema_version();
    scheme.kind = default_scheme_kind();
    if scheme.id.trim().is_empty() {
        scheme.id = default_scheme_id();
    }
    if scheme.name.trim().is_empty() {
        scheme.name = "未命名审核方案".to_string();
    }
    scheme.flow.name = scheme.name.clone();
    scheme.flow = normalize_flow(scheme.flow, modules);
    scheme
}

fn scheme_path_with_default_extension(path: &Path) -> PathBuf {
    if path.extension().is_none() {
        path.with_extension("ugcaudit")
    } else {
        path.to_path_buf()
    }
}

fn scheme_file_stem(name: &str) -> String {
    let mut stem = name
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    stem = stem
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    while stem.ends_with('.') || stem.ends_with(' ') {
        stem.pop();
    }
    if stem.is_empty() {
        stem = "审核方案".to_string();
    }
    stem.chars().take(80).collect()
}

fn unique_scheme_library_path(dir: &Path, scheme_name: &str) -> PathBuf {
    let stem = scheme_file_stem(scheme_name);
    let mut path = dir.join(format!("{stem}.ugcaudit"));
    if !path.exists() {
        return path;
    }

    for index in 2.. {
        path = dir.join(format!("{stem}-{index}.ugcaudit"));
        if !path.exists() {
            return path;
        }
    }
    unreachable!("scheme library path search should always find a candidate")
}

fn scheme_list_item_from_path(path: &Path) -> Option<SchemeListItem> {
    let extension = path.extension()?.to_string_lossy();
    if !extension.eq_ignore_ascii_case("ugcaudit") {
        return None;
    }

    let file_stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名审核方案")
        .to_string();
    let content = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/flow/id").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/flow/name").and_then(Value::as_str))
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.trim().to_string())
        .unwrap_or(file_stem);
    let modified_at = fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());

    Some(SchemeListItem {
        id,
        name,
        path: path.display().to_string(),
        modified_at,
    })
}

fn list_scheme_files_inner() -> Result<Vec<SchemeListItem>, String> {
    let dir = ensure_scheme_library_dir()?;
    let mut items = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("无法读取审核方案目录 {}: {error}", dir.display()))?
    {
        let Ok(entry) = entry else {
            continue;
        };
        if let Some(item) = scheme_list_item_from_path(&entry.path()) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(items)
}

fn read_scheme_file(
    path: &Path,
    modules: &HashMap<String, ModuleInfo>,
) -> Result<AuditScheme, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("无法读取审核方案 {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .map_err(|error| format!("无法解析审核方案 {}: {error}", path.display()))?;
    let scheme = if value.get("flow").is_some() {
        serde_json::from_value::<AuditScheme>(value)
            .map_err(|error| format!("无法解析审核方案 {}: {error}", path.display()))?
    } else {
        let flow = serde_json::from_value::<FlowDefinition>(value)
            .map_err(|error| format!("无法解析旧流程文件 {}: {error}", path.display()))?;
        scheme_from_flow(flow)
    };
    if scheme.kind != default_scheme_kind() {
        return Err(format!("不是 UGCAudit 审核方案文件：{}", path.display()));
    }
    Ok(normalize_scheme(scheme, modules))
}

fn validate_scheme_for_save(
    root: &Path,
    scheme: AuditScheme,
) -> Result<(AuditScheme, ValidationResult), String> {
    let paths = load_model_paths(root)?;
    let modules = modules_by_id(root, &paths)?;
    let normalized = normalize_scheme(scheme, &modules);
    let validation = validate_flow_inner(&normalized.flow, &modules);
    Ok((normalized, validation))
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
    let nodes_by_id: HashMap<String, &FlowNode> = flow
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut seen_edges = HashSet::new();
    let mut data_targets = HashSet::new();
    for edge in &flow.edges {
        if !seen_edges.insert(edge.id.clone()) {
            messages.push(format!("连线 ID 重复：{}", edge.id));
        }
        if !node_ids.contains(&edge.from) {
            messages.push(format!("连线 {} 的起点不存在。", edge.id));
        }
        if !node_ids.contains(&edge.to) {
            messages.push(format!("连线 {} 的终点不存在。", edge.id));
        }
        if edge.from == edge.to {
            messages.push(format!("连线 {} 指向了同一个步骤。", edge.id));
        }

        let Some(source_node) = nodes_by_id.get(&edge.from) else {
            continue;
        };
        let Some(target_node) = nodes_by_id.get(&edge.to) else {
            continue;
        };
        let Some(source_module) = modules.get(&source_node.module_id) else {
            continue;
        };
        let Some(target_module) = modules.get(&target_node.module_id) else {
            continue;
        };

        if edge.edge_type == EDGE_TYPE_DATA {
            let from_handle = edge.from_handle.as_deref().unwrap_or("");
            let to_handle = edge.to_handle.as_deref().unwrap_or("");
            let output_type = data_output_type(source_module, from_handle);
            let input_type = data_input_type(target_module, to_handle);
            if output_type.is_none() {
                messages.push(format!("连线 {} 的输出口不是有效数据口。", edge.id));
            }
            if input_type.is_none() {
                messages.push(format!("连线 {} 的输入口不是有效数据口。", edge.id));
            }
            if let (Some(output_type), Some(input_type)) = (output_type, input_type) {
                if output_type != input_type {
                    messages.push(format!("连线 {} 的数据类型不匹配。", edge.id));
                }
            }
            if !data_targets.insert((edge.to.clone(), to_handle.to_string())) {
                messages.push(format!(
                    "步骤 {} 的输入口 {} 被连接了多次。",
                    target_node.label, to_handle
                ));
            }
        } else {
            let from_handle = edge.from_handle.as_deref().unwrap_or("");
            let to_handle = edge.to_handle.as_deref().unwrap_or("");
            if from_handle != HANDLE_SEQUENCE_OUT || to_handle != HANDLE_SEQUENCE_IN {
                messages.push(format!("连线 {} 的顺序口不正确。", edge.id));
            }
            if !has_sequence_output(source_module) {
                messages.push(format!("步骤 {} 没有顺序输出口。", source_node.label));
            }
            if !has_sequence_input(target_module) {
                messages.push(format!("步骤 {} 没有顺序输入口。", target_node.label));
            }
        }
    }

    if let Some(start) = start_nodes.first() {
        if flow
            .edges
            .iter()
            .any(|edge| edge.edge_type == EDGE_TYPE_SEQUENCE && edge.to == start.id)
        {
            messages.push("开始节点不能有输入连线。".to_string());
        }
    }
    if let Some(output) = output_nodes.first() {
        if flow
            .edges
            .iter()
            .any(|edge| edge.edge_type == EDGE_TYPE_SEQUENCE && edge.from == output.id)
        {
            messages.push("输出结果节点不能有输出连线。".to_string());
        }
    }
    if let (Some(start), Some(output)) = (start_nodes.first(), output_nodes.first()) {
        if !has_path(flow, &start.id, &output.id) {
            messages.push("开始节点必须能连到输出结果节点。".to_string());
        }
    }

    for node in &flow.nodes {
        let Some(module) = modules.get(&node.module_id) else {
            continue;
        };
        if !has_required_relative_path(module, &node.config) {
            messages.push(format!("步骤 {} 需要填写相对路径。", node.label));
        }
        for (handle, _) in required_data_inputs(module) {
            let connected = flow.edges.iter().any(|edge| {
                edge.edge_type == EDGE_TYPE_DATA
                    && edge.to == node.id
                    && edge.to_handle.as_deref() == Some(handle)
            });
            if !connected {
                messages.push(format!(
                    "步骤 {} 缺少必需的数据输入：{}。",
                    node.label, handle
                ));
            }
        }
    }

    if messages.is_empty() && execution_plan(flow, modules).is_err() {
        messages.push("流程不能形成闭环。".to_string());
    }

    ValidationResult {
        valid: messages.is_empty(),
        messages,
    }
}

fn has_path(flow: &FlowDefinition, from: &str, to: &str) -> bool {
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_SEQUENCE)
    {
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

fn executable_data_consumers(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
    source_id: &str,
) -> HashSet<String> {
    let nodes_by_id: HashMap<String, &FlowNode> = flow
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut data_outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_DATA)
    {
        data_outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut consumers = HashSet::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from(data_outgoing.get(source_id).cloned().unwrap_or_default());
    while let Some(current) = queue.pop_front() {
        if !seen.insert(current.clone()) {
            continue;
        }
        let Some(node) = nodes_by_id.get(&current) else {
            continue;
        };
        let Some(module) = modules.get(&node.module_id) else {
            continue;
        };
        if is_pure_data_module_kind(&module.kind) {
            if let Some(children) = data_outgoing.get(&current) {
                for child in children {
                    queue.push_back(child.clone());
                }
            }
        } else {
            consumers.insert(current);
        }
    }

    consumers
}

#[allow(dead_code)]
fn topological_order(flow: &FlowDefinition) -> Result<Vec<(FlowNode, usize)>, String> {
    let mut by_id: HashMap<String, FlowNode> = flow
        .nodes
        .iter()
        .filter(|node| !is_pure_data_module_id(&node.module_id))
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut indegree: HashMap<String, usize> =
        by_id.keys().map(|id| (id.clone(), 0_usize)).collect();
    let mut group: HashMap<String, usize> = by_id.keys().map(|id| (id.clone(), 0_usize)).collect();

    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_SEQUENCE)
    {
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
        let expected = flow
            .nodes
            .iter()
            .filter(|node| !is_pure_data_module_id(&node.module_id))
            .count();
        if ordered.len() == expected {
            return Ok(ordered);
        }
        return Err("流程不能形成闭环。".to_string());
    }

    Ok(ordered)
}

fn sorted_ready_ids(flow: &FlowDefinition, indegree: &HashMap<String, usize>) -> Vec<String> {
    let mut ids = flow
        .nodes
        .iter()
        .filter(|node| !is_pure_data_module_id(&node.module_id))
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

#[derive(Debug, Clone)]
struct ExecutionPlan {
    ordered: Vec<(FlowNode, usize)>,
    dependencies: HashMap<String, HashSet<String>>,
    dependents: HashMap<String, Vec<String>>,
}

fn add_execution_dependency(
    nodes: &HashMap<String, FlowNode>,
    dependencies: &mut HashMap<String, HashSet<String>>,
    dependents: &mut HashMap<String, Vec<String>>,
    from: &str,
    to: &str,
) {
    if from == to || !nodes.contains_key(from) || !nodes.contains_key(to) {
        return;
    }
    if dependencies
        .entry(to.to_string())
        .or_default()
        .insert(from.to_string())
    {
        dependents
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }
}

fn execution_plan(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
) -> Result<ExecutionPlan, String> {
    let nodes: HashMap<String, FlowNode> = flow
        .nodes
        .iter()
        .filter(|node| !is_pure_data_module_id(&node.module_id))
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let mut dependencies: HashMap<String, HashSet<String>> = nodes
        .keys()
        .map(|id| (id.clone(), HashSet::new()))
        .collect();
    let mut dependents: HashMap<String, Vec<String>> =
        nodes.keys().map(|id| (id.clone(), Vec::new())).collect();

    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_SEQUENCE)
    {
        add_execution_dependency(
            &nodes,
            &mut dependencies,
            &mut dependents,
            &edge.from,
            &edge.to,
        );
    }

    for source_id in nodes.keys() {
        for consumer_id in executable_data_consumers(flow, modules, source_id) {
            add_execution_dependency(
                &nodes,
                &mut dependencies,
                &mut dependents,
                source_id,
                &consumer_id,
            );
        }
    }

    let mut indegree: HashMap<String, usize> = dependencies
        .iter()
        .map(|(id, deps)| (id.clone(), deps.len()))
        .collect();
    let mut group: HashMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0_usize)).collect();
    let mut ready = sorted_ready_ids(flow, &indegree)
        .into_iter()
        .collect::<VecDeque<_>>();
    let mut ordered = Vec::new();

    while let Some(id) = ready.pop_front() {
        let node = nodes
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("找不到步骤 {id}"))?;
        let current_group = *group.get(&id).unwrap_or(&0);
        ordered.push((node, current_group));

        if let Some(children) = dependents.get(&id) {
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

    if ordered.len() != nodes.len() {
        return Err("流程不能形成闭环。".to_string());
    }

    Ok(ExecutionPlan {
        ordered,
        dependencies,
        dependents,
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_string()
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn file_type_from_extension(extension: &str) -> &'static str {
    match extension {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => "image",
        "txt" | "md" | "json" => "text",
        _ => "other",
    }
}

fn relative_path_for_file(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
}

fn collect_file_for_asset(
    file: &Path,
    asset: &AuditAsset,
    asset_root: &Path,
    files: &mut Vec<AuditFile>,
) -> Result<(), String> {
    let canonical = file
        .canonicalize()
        .map_err(|error| format!("无法读取素材文件 {}: {error}", file.display()))?;
    let extension = file_extension(&canonical);
    let file_type = file_type_from_extension(&extension).to_string();
    files.push(AuditFile {
        path: canonical.display().to_string(),
        name: file_name(&canonical),
        extension,
        file_type,
        source_asset_id: asset.id.clone(),
        source_asset_name: asset.name.clone(),
        relative_path: relative_path_for_file(asset_root, &canonical),
    });
    Ok(())
}

fn collect_directory_files(
    dir: &Path,
    asset: &AuditAsset,
    asset_root: &Path,
    files: &mut Vec<AuditFile>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|error| format!("无法读取素材文件夹 {}: {error}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取素材文件夹项: {error}"))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("无法读取素材类型 {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_directory_files(&path, asset, asset_root, files)?;
        } else if file_type.is_file() {
            collect_file_for_asset(&path, asset, asset_root, files)?;
        }
    }
    Ok(())
}

fn collect_audit_files(assets: &[AuditAsset]) -> Result<Vec<AuditFile>, String> {
    let mut files = Vec::new();
    for asset in assets {
        let path = PathBuf::from(asset.path.trim());
        if asset.kind == "directory" {
            let root = path
                .canonicalize()
                .map_err(|error| format!("无法读取素材文件夹 {}: {error}", path.display()))?;
            collect_directory_files(&root, asset, &root, &mut files)?;
        } else if asset.kind == "file" {
            if path.is_file() {
                let asset_root = path.parent().unwrap_or_else(|| Path::new(""));
                collect_file_for_asset(&path, asset, asset_root, &mut files)?;
            } else {
                return Err(format!("素材文件不存在：{}", path.display()));
            }
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);
    Ok(files)
}

fn collect_artifact_files(artifact_dir: &Path) -> Result<Vec<AuditFile>, String> {
    if !artifact_dir.exists() {
        return Ok(Vec::new());
    }
    if !artifact_dir.is_dir() {
        return Err(format!("产物路径不是文件夹：{}", artifact_dir.display()));
    }
    let root = artifact_dir
        .canonicalize()
        .map_err(|error| format!("无法读取产物文件夹 {}: {error}", artifact_dir.display()))?;
    let asset = AuditAsset {
        id: "artifact_dir".to_string(),
        kind: "directory".to_string(),
        path: root.display().to_string(),
        name: "产物文件夹".to_string(),
        extension: String::new(),
    };
    let mut files = Vec::new();
    collect_directory_files(&root, &asset, &root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);
    Ok(files)
}

fn read_text_content(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
}

fn image_item_from_file(file: &AuditFile) -> Value {
    json!(ImageCollectionItem {
        path: file.path.clone(),
        name: file.name.clone(),
        extension: file.extension.clone(),
        source_asset_id: file.source_asset_id.clone(),
        source_asset_name: file.source_asset_name.clone(),
        relative_path: file.relative_path.clone(),
    })
}

fn text_item_from_file(file: &AuditFile) -> Value {
    json!(TextCollectionItem {
        source_type: "file".to_string(),
        path: file.path.clone(),
        name: file.name.clone(),
        relative_path: file.relative_path.clone(),
        text: read_text_content(Path::new(&file.path)),
    })
}

fn normalized_relative_path(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').trim().to_string()
}

fn relative_path_matches(path: &str, prefix: &str) -> bool {
    let path = normalized_relative_path(path);
    let prefix = normalized_relative_path(prefix);
    if prefix.is_empty() {
        return true;
    }
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn collection_from_items(data_type: &str, items: Vec<Value>) -> DataPortValue {
    DataPortValue {
        data_type: data_type.to_string(),
        items,
        ..DataPortValue::default()
    }
}

fn folder_port_value(path: &Path, relative_path: &str) -> Result<DataPortValue, String> {
    if !path.is_dir() {
        return Err(format!("文件夹不存在：{}", path.display()));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("无法读取文件夹 {}: {error}", path.display()))?;
    Ok(DataPortValue {
        data_type: DATA_TYPE_FOLDER.to_string(),
        path: Some(canonical.display().to_string()),
        name: Some(file_name(&canonical)),
        relative_path: Some(normalized_relative_path(relative_path)),
        ..DataPortValue::default()
    })
}

fn audit_folder_value(assets: &[AuditAsset]) -> Result<DataPortValue, String> {
    let asset = assets
        .iter()
        .find(|asset| asset.kind == "directory" && !asset.path.trim().is_empty())
        .ok_or_else(|| "本次运行没有选择待审核文件夹。".to_string())?;
    folder_port_value(Path::new(asset.path.trim()), "")
}

fn relative_folder_value(
    root: &Path,
    relative_path: &str,
    root_label: &str,
) -> Result<DataPortValue, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("无法读取{root_label} {}: {error}", root.display()))?;
    let normalized = normalized_relative_path(relative_path);
    let candidate = root.join(&normalized);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("无法读取相对路径文件夹 {}: {error}", candidate.display()))?;
    if !canonical.starts_with(&root) {
        return Err(format!(
            "相对路径文件夹不能超出{root_label}：{}",
            candidate.display()
        ));
    }
    folder_port_value(&canonical, &normalized)
}

fn audit_relative_folder_value(
    assets: &[AuditAsset],
    relative_path: &str,
) -> Result<DataPortValue, String> {
    let root = assets
        .iter()
        .find(|asset| asset.kind == "directory" && !asset.path.trim().is_empty())
        .map(|asset| PathBuf::from(asset.path.trim()))
        .ok_or_else(|| "本次运行没有选择待审核文件夹。".to_string())?;
    relative_folder_value(&root, relative_path, "待审核文件夹")
}

fn artifact_relative_folder_value(
    artifact_dir: &Path,
    relative_path: &str,
) -> Result<DataPortValue, String> {
    relative_folder_value(artifact_dir, relative_path, "产物文件夹")
}

fn all_images_collection(files: &[AuditFile]) -> DataPortValue {
    collection_from_items(
        DATA_TYPE_IMAGES,
        files
            .iter()
            .filter(|file| file.file_type == "image")
            .map(image_item_from_file)
            .collect(),
    )
}

fn all_texts_collection(files: &[AuditFile]) -> DataPortValue {
    collection_from_items(
        DATA_TYPE_TEXTS,
        files
            .iter()
            .filter(|file| file.file_type == "text")
            .map(text_item_from_file)
            .collect(),
    )
}

fn artifact_collection(artifact_dir: &Path, data_type: &str) -> Result<DataPortValue, String> {
    let files = collect_artifact_files(artifact_dir)?;
    let value = if data_type == DATA_TYPE_IMAGES {
        all_images_collection(&files)
    } else {
        all_texts_collection(&files)
    };
    Ok(value)
}

fn relative_collection(files: &[AuditFile], data_type: &str, relative_path: &str) -> DataPortValue {
    if data_type == DATA_TYPE_IMAGES {
        collection_from_items(
            DATA_TYPE_IMAGES,
            files
                .iter()
                .filter(|file| file.file_type == "image")
                .filter(|file| relative_path_matches(&file.relative_path, relative_path))
                .map(image_item_from_file)
                .collect(),
        )
    } else {
        collection_from_items(
            DATA_TYPE_TEXTS,
            files
                .iter()
                .filter(|file| file.file_type == "text")
                .filter(|file| relative_path_matches(&file.relative_path, relative_path))
                .map(text_item_from_file)
                .collect(),
        )
    }
}

fn merge_collections(left: DataPortValue, right: DataPortValue) -> DataPortValue {
    let mut seen = HashSet::new();
    let data_type = left.data_type.clone();
    let mut items = Vec::new();
    for item in left.items.into_iter().chain(right.items) {
        let key = if data_type == DATA_TYPE_IMAGES {
            item.get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        } else {
            format!(
                "{}|{}|{}",
                item.get("sourceType").and_then(Value::as_str).unwrap_or(""),
                item.get("path").and_then(Value::as_str).unwrap_or(""),
                item.get("text").and_then(Value::as_str).unwrap_or("")
            )
        };
        if seen.insert(key) {
            items.push(item);
        }
    }
    DataPortValue {
        data_type,
        items,
        ..DataPortValue::default()
    }
}

fn module_output_as_text_collection(node: &FlowNode, output: &Value) -> DataPortValue {
    let mut items = Vec::new();
    if let Some(results) = output.get("results").and_then(Value::as_array) {
        for result in results {
            let text = result
                .get("fullText")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() {
                continue;
            }
            let path = result
                .get("textPath")
                .or_else(|| result.get("path"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("previous:{}", node.id));
            let name = result
                .get("textName")
                .or_else(|| result.get("name"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| node.label.clone());
            let relative_path = result
                .get("textRelativePath")
                .or_else(|| result.get("relativePath"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| path.clone());
            items.push(json!(TextCollectionItem {
                source_type: "ocr".to_string(),
                path: path.clone(),
                name,
                relative_path,
                text,
            }));
        }
    }

    if items.is_empty() {
        let text = output
            .get("outputs")
            .and_then(|outputs| outputs.get("fullText"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if !text.is_empty() {
            items.push(json!(TextCollectionItem {
                source_type: "ocr".to_string(),
                path: format!("previous:{}", node.id),
                name: node.label.clone(),
                relative_path: format!("previous:{}", node.id),
                text,
            }));
        }
    }

    collection_from_items(DATA_TYPE_TEXTS, items)
}

fn module_output_as_declared_collection(
    output: &Value,
    handle: &str,
    data_type: &str,
) -> DataPortValue {
    let port_value = output
        .get("outputs")
        .and_then(|outputs| outputs.get(handle))
        .or_else(|| output.get(handle));
    let items = port_value
        .and_then(|value| value.get("items"))
        .or(port_value)
        .and_then(Value::as_array)
        .map(|items| items.to_vec())
        .unwrap_or_default();
    collection_from_items(data_type, items)
}

fn module_data_output(
    module: &ModuleInfo,
    node: &FlowNode,
    output: &Value,
    handle: &str,
) -> Option<DataPortValue> {
    if let Some(data_type) = declared_data_output_type(module, handle) {
        return Some(module_output_as_declared_collection(
            output, handle, &data_type,
        ));
    }
    match module.kind.as_str() {
        "image_ocr" if handle == HANDLE_TEXTS_OUT => {
            Some(module_output_as_text_collection(node, output))
        }
        _ => None,
    }
}

fn node_by_id<'a>(flow: &'a FlowDefinition, node_id: &str) -> Option<&'a FlowNode> {
    flow.nodes.iter().find(|node| node.id == node_id)
}

fn incoming_data_edge<'a>(
    flow: &'a FlowDefinition,
    target_id: &str,
    target_handle: &str,
) -> Option<&'a FlowEdge> {
    flow.edges.iter().find(|edge| {
        edge.edge_type == EDGE_TYPE_DATA
            && edge.to == target_id
            && edge.to_handle.as_deref() == Some(target_handle)
    })
}

fn resolve_data_value(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
    assets: &[AuditAsset],
    files: &[AuditFile],
    artifact_dir: &Path,
    module_outputs: &HashMap<String, Value>,
    cache: &mut HashMap<String, DataPortValue>,
    visiting: &mut HashSet<String>,
    node_id: &str,
    handle: &str,
) -> Result<DataPortValue, String> {
    let cache_key = format!("{node_id}:{handle}");
    if let Some(value) = cache.get(&cache_key) {
        return Ok(value.clone());
    }
    if !visiting.insert(cache_key.clone()) {
        return Err("数据节点之间形成了闭环。".to_string());
    }

    let node = node_by_id(flow, node_id).ok_or_else(|| format!("找不到数据来源节点 {node_id}"))?;
    let module = modules
        .get(&node.module_id)
        .ok_or_else(|| format!("找不到模块 {}", node.module_id))?;

    let value = match module.kind.as_str() {
        "data_all_images" => all_images_collection(files),
        "data_all_texts" => all_texts_collection(files),
        "data_artifact_images" => artifact_collection(artifact_dir, DATA_TYPE_IMAGES)?,
        "data_artifact_texts" => artifact_collection(artifact_dir, DATA_TYPE_TEXTS)?,
        "data_audit_folder" => audit_folder_value(assets)?,
        "data_audit_relative_folder" => audit_relative_folder_value(
            assets,
            node.config
                .get("relativePath")
                .and_then(Value::as_str)
                .unwrap_or(""),
        )?,
        "data_artifact_folder" => folder_port_value(artifact_dir, "")?,
        "data_artifact_relative_folder" => artifact_relative_folder_value(
            artifact_dir,
            node.config
                .get("relativePath")
                .and_then(Value::as_str)
                .unwrap_or(""),
        )?,
        "data_relative_images" => relative_collection(
            files,
            DATA_TYPE_IMAGES,
            node.config
                .get("relativePath")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        "data_relative_texts" => relative_collection(
            files,
            DATA_TYPE_TEXTS,
            node.config
                .get("relativePath")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        "data_merge_images" => {
            let left_edge = incoming_data_edge(flow, node_id, HANDLE_IMAGES_A_IN)
                .ok_or_else(|| format!("步骤 {} 缺少图片集合 A。", node.label))?;
            let right_edge = incoming_data_edge(flow, node_id, HANDLE_IMAGES_B_IN)
                .ok_or_else(|| format!("步骤 {} 缺少图片集合 B。", node.label))?;
            let left = resolve_data_value(
                flow,
                modules,
                assets,
                files,
                artifact_dir,
                module_outputs,
                cache,
                visiting,
                &left_edge.from,
                left_edge.from_handle.as_deref().unwrap_or(""),
            )?;
            let right = resolve_data_value(
                flow,
                modules,
                assets,
                files,
                artifact_dir,
                module_outputs,
                cache,
                visiting,
                &right_edge.from,
                right_edge.from_handle.as_deref().unwrap_or(""),
            )?;
            merge_collections(left, right)
        }
        "data_merge_texts" => {
            let left_edge = incoming_data_edge(flow, node_id, HANDLE_TEXTS_A_IN)
                .ok_or_else(|| format!("步骤 {} 缺少文本集合 A。", node.label))?;
            let right_edge = incoming_data_edge(flow, node_id, HANDLE_TEXTS_B_IN)
                .ok_or_else(|| format!("步骤 {} 缺少文本集合 B。", node.label))?;
            let left = resolve_data_value(
                flow,
                modules,
                assets,
                files,
                artifact_dir,
                module_outputs,
                cache,
                visiting,
                &left_edge.from,
                left_edge.from_handle.as_deref().unwrap_or(""),
            )?;
            let right = resolve_data_value(
                flow,
                modules,
                assets,
                files,
                artifact_dir,
                module_outputs,
                cache,
                visiting,
                &right_edge.from,
                right_edge.from_handle.as_deref().unwrap_or(""),
            )?;
            merge_collections(left, right)
        }
        _ => {
            let output = module_outputs
                .get(node_id)
                .ok_or_else(|| format!("数据来源 {} 尚未运行完成，无法传给下游。", node.label))?;
            module_data_output(module, node, output, handle)
                .ok_or_else(|| format!("步骤 {} 没有可用的数据输出口。", node.label))?
        }
    };

    visiting.remove(&cache_key);
    cache.insert(cache_key, value.clone());
    Ok(value)
}

fn data_inputs_for_node(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
    assets: &[AuditAsset],
    files: &[AuditFile],
    artifact_dir: &Path,
    module_outputs: &HashMap<String, Value>,
    cache: &mut HashMap<String, DataPortValue>,
    node: &FlowNode,
) -> Result<HashMap<String, DataPortValue>, String> {
    let mut inputs = HashMap::new();
    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_DATA && edge.to == node.id)
    {
        let to_handle = edge.to_handle.as_deref().unwrap_or("").to_string();
        let value = resolve_data_value(
            flow,
            modules,
            assets,
            files,
            artifact_dir,
            module_outputs,
            cache,
            &mut HashSet::new(),
            &edge.from,
            edge.from_handle.as_deref().unwrap_or(""),
        )?;
        inputs.insert(to_handle, value);
    }
    Ok(inputs)
}

fn compatible_files_from_inputs(inputs: &HashMap<String, DataPortValue>) -> Vec<AuditFile> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for value in inputs.values() {
        if !matches!(value.data_type.as_str(), DATA_TYPE_IMAGES | DATA_TYPE_TEXTS) {
            continue;
        }
        for item in &value.items {
            let path = item
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if path.is_empty() || path.starts_with("previous:") || !seen.insert(path.clone()) {
                continue;
            }
            let file_type = if value.data_type == DATA_TYPE_IMAGES {
                "image"
            } else {
                "text"
            };
            files.push(AuditFile {
                path,
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                extension: item
                    .get("extension")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        Path::new(item.get("path").and_then(Value::as_str).unwrap_or(""))
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase()
                    }),
                file_type: file_type.to_string(),
                source_asset_id: item
                    .get("sourceAssetId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                source_asset_name: item
                    .get("sourceAssetName")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                relative_path: item
                    .get("relativePath")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    files
}

fn collect_previous_output_sources(
    flow: &FlowDefinition,
    modules: &HashMap<String, ModuleInfo>,
    module_outputs: &HashMap<String, Value>,
    node_id: &str,
    result: &mut HashMap<String, Value>,
    visited: &mut HashSet<String>,
) {
    for edge in flow
        .edges
        .iter()
        .filter(|edge| edge.edge_type == EDGE_TYPE_DATA && edge.to == node_id)
    {
        if !visited.insert(format!(
            "{}:{}",
            edge.from,
            edge.from_handle.as_deref().unwrap_or("")
        )) {
            continue;
        }
        let Some(source_node) = node_by_id(flow, &edge.from) else {
            continue;
        };
        let Some(source_module) = modules.get(&source_node.module_id) else {
            continue;
        };
        if is_pure_data_module_kind(&source_module.kind) {
            collect_previous_output_sources(
                flow,
                modules,
                module_outputs,
                &source_node.id,
                result,
                visited,
            );
        } else if let Some(output) = module_outputs.get(&source_node.id) {
            result.insert(source_node.id.clone(), output.clone());
        }
    }
}

fn module_source_label(source: &str) -> &'static str {
    match source {
        "system" => "流程系统节点",
        "preset" => "预置模块",
        _ => "自定义模块",
    }
}

fn runtime_python_path(app: Option<&tauri::AppHandle>) -> Result<PathBuf, String> {
    if let Some(app) = app {
        let path = runtime_python_exe(app)?;
        if path.is_file() {
            return Ok(path);
        }
        if bundled_python_source().is_some() {
            return ensure_runtime_python(app);
        }
    }

    if let Ok(path) = env::var("UGCAUDIT_PYTHON") {
        let path = PathBuf::from(path.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "UGCAUDIT_PYTHON 指向的 Python 不存在：{}",
            path.display()
        ));
    }

    Err(
        "找不到模型运行环境。请随客户端放置 Runtime\\Python312，或设置 UGCAUDIT_PYTHON。"
            .to_string(),
    )
}

fn runtime_python_launcher_path(app: Option<&tauri::AppHandle>) -> Result<PathBuf, String> {
    runtime_python_path(app)
}

fn resolve_module_command(
    app: Option<&tauri::AppHandle>,
    module: &ModuleInfo,
) -> Result<(PathBuf, Vec<String>), String> {
    let command = module
        .launch
        .command
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("模块 {} 未配置启动命令。", module.name))?;

    let command_path = PathBuf::from(command);
    let resolved_command = if command_path.is_absolute() {
        command_path
    } else {
        PathBuf::from(&module.definition_dir).join(command_path)
    };

    let extension = resolved_command
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if extension == "py" {
        if !resolved_command.is_file() {
            return Err(format!(
                "模块 {} 的启动脚本不存在：{}",
                module.name,
                resolved_command.display()
            ));
        }
        Ok((
            runtime_python_launcher_path(app)?,
            vec![resolved_command.display().to_string()],
        ))
    } else if resolved_command.is_file() {
        Ok((resolved_command, Vec::new()))
    } else {
        Err(format!(
            "模块 {} 的启动文件不存在：{}",
            module.name,
            resolved_command.display()
        ))
    }
}

fn replace_arg_placeholders(
    arg: &str,
    input_path: &Path,
    output_path: &Path,
    resource_dir: &Path,
    params_json: &str,
) -> String {
    arg.replace("{inputJson}", &input_path.display().to_string())
        .replace("{outputJson}", &output_path.display().to_string())
        .replace("{resourceRoot}", &resource_dir.display().to_string())
        .replace("{paramsJson}", params_json)
}

fn step_from_module_output(
    module: &ModuleInfo,
    node: &FlowNode,
    execution_group: usize,
    output: ModuleStepOutput,
) -> StepRun {
    let report_section = if output.report_section.trim().is_empty() {
        format!(
            "### {}\n\n- 模块：{}\n- 模块来源：{}\n- 结论：{}\n- 状态：{}\n- 处理文件：{}\n- 命中文件：{}\n- 说明：{}\n",
            node.label,
            module.name,
            module_source_label(&module.source),
            verdict_label(&output.verdict),
            status_label(&output.status),
            output.processed_files,
            output.matched_files,
            output.message
        )
    } else {
        output.report_section.clone()
    };

    StepRun {
        step_id: node.id.clone(),
        module_id: module.id.clone(),
        module_name: module.name.clone(),
        label: node.label.clone(),
        status: output.status,
        verdict: output.verdict,
        message: output.message,
        execution_group,
        progress: 1.0,
        processed_files: output.processed_files,
        matched_files: output.matched_files,
        artifact_count: output.artifact_count,
        performance: None,
        report_section,
    }
}

fn stderr_line_is_warning(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains("UserWarning:")
        || trimmed.starts_with("warnings.warn(")
        || trimmed.starts_with("WARNING:")
        || trimmed.starts_with("INFO:")
        || trimmed.starts_with("I0")
}

fn stderr_line_is_error(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains("Error")
        || trimmed.contains("Exception")
        || trimmed.contains("Traceback")
        || trimmed.contains("ModuleNotFoundError")
        || trimmed.contains("ImportError")
        || trimmed.contains("RuntimeError")
}

fn stderr_failure_detail(stderr: &str) -> String {
    let lines = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if let Some(line) = lines.iter().rev().find(|line| stderr_line_is_error(line)) {
        return (*line).to_string();
    }

    if let Some(line) = lines
        .iter()
        .rev()
        .find(|line| !stderr_line_is_warning(line))
    {
        return (*line).to_string();
    }

    String::new()
}

fn module_failure_message(module_name: &str, stderr: &str, missing_output: bool) -> String {
    let detail = stderr_failure_detail(stderr);
    if !detail.is_empty() {
        return format!("模块 {module_name} 执行失败：{detail}");
    }
    if missing_output {
        format!("模块 {module_name} 没有生成结果文件，进程可能异常退出。")
    } else {
        format!("模块 {module_name} 执行失败。")
    }
}

fn step_output_value(step: &StepRun) -> Value {
    json!({
        "status": &step.status,
        "verdict": &step.verdict,
        "message": &step.message,
        "progress": step.progress,
        "processedFiles": step.processed_files,
        "matchedFiles": step.matched_files,
        "artifactCount": step.artifact_count,
        "performance": &step.performance,
        "reportSection": &step.report_section,
    })
}

fn synthetic_step_result(
    module: &ModuleInfo,
    node: &FlowNode,
    execution_group: usize,
    status: &str,
    verdict: &str,
    message: &str,
    progress: f64,
) -> StepRun {
    let report_section = format!(
        "### {}\n\n- 模块：{}\n- 模块来源：{}\n- 结论：{}\n- 状态：{}\n- 说明：{}\n",
        node.label,
        module.name,
        module_source_label(&module.source),
        verdict_label(verdict),
        status_label(status),
        message
    );
    StepRun {
        step_id: node.id.clone(),
        module_id: module.id.clone(),
        module_name: module.name.clone(),
        label: node.label.clone(),
        status: status.to_string(),
        verdict: verdict.to_string(),
        message: message.to_string(),
        execution_group,
        progress,
        processed_files: 0,
        matched_files: 0,
        artifact_count: 0,
        performance: None,
        report_section,
    }
}

#[allow(dead_code)]
fn execute_module_step(
    app: Option<&tauri::AppHandle>,
    module: &ModuleInfo,
    node: &FlowNode,
    execution_group: usize,
    run_id: &str,
    task_name: &str,
    resource_dir: &Path,
    artifact_dir: &Path,
    step_dir: &Path,
    files: &[AuditFile],
    data_inputs: &HashMap<String, DataPortValue>,
    previous_outputs: &HashMap<String, Value>,
) -> Result<(StepRun, Value), String> {
    let step_artifacts_dir = step_dir.join("artifacts");
    fs::create_dir_all(&step_artifacts_dir).map_err(|error| {
        format!(
            "无法创建步骤证据目录 {}: {error}",
            step_artifacts_dir.display()
        )
    })?;

    let input_path = step_dir.join("input.json");
    let output_path = step_dir.join("output.json");
    let params_json = serde_json::to_string(&node.config)
        .map_err(|error| format!("无法序列化模块参数 {}: {error}", node.label))?;
    let model_path = model_path_from_config(&node.config)
        .or_else(|| module.model_path.clone())
        .unwrap_or_default();

    let input = json!({
        "runId": run_id,
        "stepId": &node.id,
        "moduleId": &module.id,
        "moduleKind": &module.kind,
        "moduleName": &module.name,
        "workDir": step_dir.display().to_string(),
        "taskName": task_name,
        "artifactDir": artifact_dir.display().to_string(),
        "stepArtifactDir": step_artifacts_dir.display().to_string(),
        "resourceRoot": resource_dir.display().to_string(),
        "inputs": data_inputs,
        "files": files,
        "params": &node.config,
        "modelPath": model_path,
        "previous": previous_outputs,
    });
    write_json(&input_path, &input)?;

    let (command, mut prefix_args) = resolve_module_command(app, module)?;
    let mut args = if module.launch.args.is_empty() {
        vec![
            "--input".to_string(),
            "{inputJson}".to_string(),
            "--output".to_string(),
            "{outputJson}".to_string(),
        ]
    } else {
        module.launch.args.clone()
    };
    let has_input = args.iter().any(|arg| arg.contains("{inputJson}"));
    let has_output = args.iter().any(|arg| arg.contains("{outputJson}"));
    if !has_input {
        args.extend(["--input".to_string(), "{inputJson}".to_string()]);
    }
    if !has_output {
        args.extend(["--output".to_string(), "{outputJson}".to_string()]);
    }
    prefix_args.extend(args.into_iter().map(|arg| {
        replace_arg_placeholders(&arg, &input_path, &output_path, resource_dir, &params_json)
    }));

    let mut envs = Vec::new();
    if let Some(app) = app {
        append_runtime_env(app, &mut envs, Some(Path::new(&module.definition_dir)))?;
    }
    append_run_module_env(
        &mut envs,
        run_id,
        task_name,
        resource_dir,
        artifact_dir,
        &step_artifacts_dir,
    );
    let command_output = run_command_with_logs(
        app,
        &module.id,
        &command,
        &prefix_args,
        Some(Path::new(&module.definition_dir)),
        &envs,
    )
    .map_err(|error| format!("无法启动模块 {}: {error}", module.name))?;

    fs::write(step_dir.join("stdout.log"), &command_output.stdout)
        .map_err(|error| format!("无法写入模块输出日志 {}: {error}", node.label))?;
    fs::write(step_dir.join("stderr.log"), &command_output.stderr)
        .map_err(|error| format!("无法写入模块错误日志 {}: {error}", node.label))?;

    let mut output_value = if output_path.is_file() {
        read_required_json::<Value>(&output_path)?
    } else {
        json!({
            "status": "error",
            "verdict": "error",
            "message": module_failure_message(&module.name, &command_output.stderr, true)
        })
    };

    let output: ModuleStepOutput = serde_json::from_value(output_value.clone())
        .map_err(|error| format!("无法解析模块 {} 输出：{error}", module.name))?;
    let step = step_from_module_output(module, node, execution_group, output);
    let step = attach_step_performance(
        step,
        &mut output_value,
        command_output.performance,
        &step_artifacts_dir,
    );
    write_json(&output_path, &output_value)?;
    fs::write(step_dir.join("result.md"), &step.report_section)
        .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
    Ok((step, output_value))
}

#[allow(clippy::too_many_arguments)]
fn execute_module_step_live(
    app: &tauri::AppHandle,
    control: &RunControl,
    module: &ModuleInfo,
    node: &FlowNode,
    execution_group: usize,
    run_id: &str,
    task_name: &str,
    resource_dir: &Path,
    artifact_dir: &Path,
    step_dir: &Path,
    files: &[AuditFile],
    data_inputs: &HashMap<String, DataPortValue>,
    previous_outputs: &HashMap<String, Value>,
) -> Result<(StepRun, Value, bool), String> {
    let step_artifacts_dir = step_dir.join("artifacts");
    fs::create_dir_all(&step_artifacts_dir).map_err(|error| {
        format!(
            "无法创建步骤证据目录 {}: {error}",
            step_artifacts_dir.display()
        )
    })?;

    let input_path = step_dir.join("input.json");
    let output_path = step_dir.join("output.json");
    let progress_path = step_dir.join("progress.jsonl");
    let cancel_path = step_dir.join("cancel.flag");
    let params_json = serde_json::to_string(&node.config)
        .map_err(|error| format!("无法序列化模块参数 {}: {error}", node.label))?;
    let model_path = model_path_from_config(&node.config)
        .or_else(|| module.model_path.clone())
        .unwrap_or_default();

    let input = json!({
        "runId": run_id,
        "stepId": &node.id,
        "moduleId": &module.id,
        "moduleKind": &module.kind,
        "moduleName": &module.name,
        "workDir": step_dir.display().to_string(),
        "taskName": task_name,
        "artifactDir": artifact_dir.display().to_string(),
        "stepArtifactDir": step_artifacts_dir.display().to_string(),
        "resourceRoot": resource_dir.display().to_string(),
        "progressPath": progress_path.display().to_string(),
        "cancelPath": cancel_path.display().to_string(),
        "inputs": data_inputs,
        "files": files,
        "params": &node.config,
        "modelPath": model_path,
        "previous": previous_outputs,
    });
    write_json(&input_path, &input)?;

    let (command, mut prefix_args) = resolve_module_command(Some(app), module)?;
    let mut args = if module.launch.args.is_empty() {
        vec![
            "--input".to_string(),
            "{inputJson}".to_string(),
            "--output".to_string(),
            "{outputJson}".to_string(),
        ]
    } else {
        module.launch.args.clone()
    };
    let has_input = args.iter().any(|arg| arg.contains("{inputJson}"));
    let has_output = args.iter().any(|arg| arg.contains("{outputJson}"));
    if !has_input {
        args.extend(["--input".to_string(), "{inputJson}".to_string()]);
    }
    if !has_output {
        args.extend(["--output".to_string(), "{outputJson}".to_string()]);
    }
    prefix_args.extend(args.into_iter().map(|arg| {
        replace_arg_placeholders(&arg, &input_path, &output_path, resource_dir, &params_json)
    }));

    let mut envs = Vec::new();
    append_runtime_env(app, &mut envs, Some(Path::new(&module.definition_dir)))?;
    append_run_module_env(
        &mut envs,
        run_id,
        task_name,
        resource_dir,
        artifact_dir,
        &step_artifacts_dir,
    );
    envs.push((
        "UGCAUDIT_PROGRESS_FILE".to_string(),
        progress_path.display().to_string(),
    ));
    envs.push((
        "UGCAUDIT_CANCEL_FILE".to_string(),
        cancel_path.display().to_string(),
    ));

    let command_output = run_command_with_live_logs(
        app,
        run_id,
        &node.id,
        &module.id,
        &command,
        &prefix_args,
        Some(Path::new(&module.definition_dir)),
        &envs,
        &progress_path,
        &cancel_path,
        control,
    )
    .map_err(|error| format!("无法启动模块 {}: {error}", module.name))?;

    fs::write(step_dir.join("stdout.log"), &command_output.stdout)
        .map_err(|error| format!("无法写入模块输出日志 {}: {error}", node.label))?;
    fs::write(step_dir.join("stderr.log"), &command_output.stderr)
        .map_err(|error| format!("无法写入模块错误日志 {}: {error}", node.label))?;

    let mut output_value = if output_path.is_file() {
        read_required_json::<Value>(&output_path)?
    } else {
        json!({
            "status": "error",
            "verdict": "error",
            "message": module_failure_message(&module.name, &command_output.stderr, true)
        })
    };

    if command_output.cancelled
        || output_value
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "cancelled")
    {
        let message = output_value
            .get("message")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("模块已中断。")
            .to_string();
        let step = synthetic_step_result(
            module,
            node,
            execution_group,
            "cancelled",
            "review",
            &message,
            0.0,
        );
        output_value = step_output_value(&step);
        let step = attach_step_performance(
            step,
            &mut output_value,
            command_output.performance,
            &step_artifacts_dir,
        );
        write_json(&output_path, &output_value)?;
        fs::write(step_dir.join("result.md"), &step.report_section)
            .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
        return Ok((step, output_value, true));
    }

    let output: ModuleStepOutput = serde_json::from_value(output_value.clone())
        .map_err(|error| format!("无法解析模块 {} 输出：{error}", module.name))?;
    let step = step_from_module_output(module, node, execution_group, output);
    let step = attach_step_performance(
        step,
        &mut output_value,
        command_output.performance,
        &step_artifacts_dir,
    );
    write_json(&output_path, &output_value)?;
    fs::write(step_dir.join("result.md"), &step.report_section)
        .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
    Ok((step, output_value, false))
}

fn module_step_result(module: &ModuleInfo, node: &FlowNode, execution_group: usize) -> StepRun {
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
            progress: 1.0,
            processed_files: 0,
            matched_files: 0,
            artifact_count: 0,
            performance: None,
            report_section,
        };
    }

    let model_path = model_path_from_config(&node.config);
    let (status, verdict, message) = match model_path {
        None => (
            "needs_model",
            "review",
            format!(
                "{} 未配置，本轮没有执行真实识别，也没有下载模型。",
                module.model_label
            ),
        ),
        Some(ref path) if path.trim().is_empty() => (
            "needs_model",
            "review",
            format!(
                "{} 未配置，本轮没有执行真实识别，也没有下载模型。",
                module.model_label
            ),
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
        "### {}\n\n- 模块：{}\n- 模块来源：自定义模块\n- 结论：需要人工复审\n- 状态：{}\n- 说明：{}\n",
        node.label,
        module.name,
        status_label(status),
        message
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
        progress: 1.0,
        processed_files: 0,
        matched_files: 0,
        artifact_count: 0,
        performance: None,
        report_section,
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "system" => "系统节点",
        "ready" => "本地入口已配置",
        "completed" => "已完成",
        "skipped" => "已跳过",
        "cancelled" => "已中断",
        "error" => "执行失败",
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

fn performance_leader(step: &StepRun, value: f64) -> PerformanceLeader {
    PerformanceLeader {
        step_id: step.step_id.clone(),
        label: step.label.clone(),
        module_name: step.module_name.clone(),
        value,
    }
}

fn finalize_run_performance(run: &mut RunRecord) {
    let measured_steps = run
        .steps
        .iter()
        .filter(|step| step.performance.is_some())
        .count();
    if measured_steps == 0 {
        run.performance_summary = Some(RunPerformanceSummary {
            sampling_note: performance_sampling_note(),
            ..RunPerformanceSummary::default()
        });
        return;
    }

    let total_cpu_time_ms = run
        .steps
        .iter()
        .filter_map(|step| step.performance.as_ref())
        .map(|performance| performance.cpu_time_ms.max(0.0))
        .sum::<f64>();
    let total_duration_ms = run
        .steps
        .iter()
        .filter_map(|step| step.performance.as_ref())
        .map(|performance| performance.duration_ms)
        .sum::<u64>();
    let total_artifact_bytes = run
        .steps
        .iter()
        .filter_map(|step| step.performance.as_ref())
        .map(|performance| performance.artifact_bytes)
        .sum::<u64>();

    let mut cpu_leader: Option<PerformanceLeader> = None;
    let mut duration_leader: Option<PerformanceLeader> = None;
    let mut memory_leader: Option<PerformanceLeader> = None;
    let mut gpu_available = false;
    let mut gpu_sampled = false;

    for step in &mut run.steps {
        let Some(performance) = step.performance.as_mut() else {
            continue;
        };
        if total_cpu_time_ms > 0.0 {
            performance.cpu_share_percent =
                (performance.cpu_time_ms.max(0.0) / total_cpu_time_ms) * 100.0;
        }
        gpu_available |= performance.gpu_available;
        gpu_sampled |= performance.peak_gpu_memory_bytes.unwrap_or(0) > 0;
    }

    for step in &run.steps {
        let Some(performance) = step.performance.as_ref() else {
            continue;
        };
        if performance.cpu_time_ms > cpu_leader.as_ref().map(|item| item.value).unwrap_or(-1.0) {
            cpu_leader = Some(performance_leader(step, performance.cpu_time_ms));
        }
        let duration_value = performance.duration_ms as f64;
        if duration_value > duration_leader.as_ref().map(|item| item.value).unwrap_or(-1.0) {
            duration_leader = Some(performance_leader(step, duration_value));
        }
        let memory_value = performance.peak_memory_bytes as f64;
        if memory_value > memory_leader.as_ref().map(|item| item.value).unwrap_or(-1.0) {
            memory_leader = Some(performance_leader(step, memory_value));
        }
    }

    if total_cpu_time_ms <= 0.0 {
        cpu_leader = None;
    }
    if memory_leader.as_ref().is_some_and(|leader| leader.value <= 0.0) {
        memory_leader = None;
    }

    run.performance_summary = Some(RunPerformanceSummary {
        total_duration_ms,
        total_cpu_time_ms,
        total_artifact_bytes,
        measured_steps,
        gpu_available,
        gpu_sampled,
        cpu_leader,
        duration_leader,
        memory_leader,
        sampling_note: performance_sampling_note(),
    });
}

fn format_duration_ms(duration_ms: u64) -> String {
    if duration_ms < 1000 {
        format!("{duration_ms} ms")
    } else {
        format!("{:.2} s", duration_ms as f64 / 1000.0)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GB", value / GIB)
    } else if value >= MIB {
        format!("{:.2} MB", value / MIB)
    } else if value >= KIB {
        format!("{:.2} KB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn format_percent(value: f64) -> String {
    format!("{:.1}%", value.max(0.0))
}

fn format_cpu_time_ms(value: f64) -> String {
    if value < 1000.0 {
        format!("{:.0} ms", value.max(0.0))
    } else {
        format!("{:.2} s", value / 1000.0)
    }
}

fn gpu_performance_text(performance: &StepPerformance) -> String {
    if !performance.gpu_available {
        return "未采集".to_string();
    }
    match performance.peak_gpu_memory_bytes {
        Some(bytes) if bytes > 0 => format_bytes(bytes),
        _ => "0 B".to_string(),
    }
}

fn append_performance_report(report: &mut String, run: &RunRecord) {
    report.push_str("## 性能开销\n\n");
    let measured_steps = run
        .steps
        .iter()
        .filter(|step| step.performance.is_some())
        .collect::<Vec<_>>();
    let Some(summary) = run.performance_summary.as_ref() else {
        report.push_str("未采集到模块性能数据。\n\n");
        return;
    };
    if measured_steps.is_empty() {
        report.push_str("未采集到外部模块性能数据。\n\n");
        return;
    }

    report.push_str(&format!(
        "- 已采集模块：{} 个\n",
        summary.measured_steps
    ));
    report.push_str(&format!(
        "- 模块总耗时：{}\n",
        format_duration_ms(summary.total_duration_ms)
    ));
    report.push_str(&format!(
        "- CPU 估算总量：{}\n",
        format_cpu_time_ms(summary.total_cpu_time_ms)
    ));
    if let Some(leader) = summary.cpu_leader.as_ref() {
        report.push_str(&format!(
            "- 最大 CPU 开销：{}（{}）\n",
            leader.label,
            format_cpu_time_ms(leader.value)
        ));
    } else if let Some(leader) = summary.duration_leader.as_ref() {
        report.push_str(&format!(
            "- 最大开销模块：{}（按耗时判断，{}）\n",
            leader.label,
            format_duration_ms(leader.value as u64)
        ));
    }
    if let Some(leader) = summary.duration_leader.as_ref() {
        report.push_str(&format!(
            "- 最耗时模块：{}（{}）\n",
            leader.label,
            format_duration_ms(leader.value as u64)
        ));
    }
    if let Some(leader) = summary.memory_leader.as_ref() {
        report.push_str(&format!(
            "- 峰值内存最高：{}（{}）\n",
            leader.label,
            format_bytes(leader.value as u64)
        ));
    }
    report.push_str(&format!(
        "- NVIDIA GPU：{}\n",
        if summary.gpu_available {
            "已尝试采集"
        } else {
            "未采集"
        }
    ));
    report.push_str(&format!("- 采样说明：{}\n\n", summary.sampling_note));

    report.push_str("| 步骤 | 模块 | 耗时 | CPU 占本次运行 | CPU 估算 | 平均 CPU | 峰值内存 | 产物大小 | NVIDIA GPU |\n");
    report.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for step in measured_steps {
        let Some(performance) = step.performance.as_ref() else {
            continue;
        };
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            table_cell(&step.label),
            table_cell(&step.module_name),
            format_duration_ms(performance.duration_ms),
            format_percent(performance.cpu_share_percent),
            format_cpu_time_ms(performance.cpu_time_ms),
            format_percent(performance.average_cpu_percent),
            format_bytes(performance.peak_memory_bytes),
            format_bytes(performance.artifact_bytes),
            gpu_performance_text(performance)
        ));
    }
    report.push('\n');
}

fn build_report(run: &RunRecord) -> String {
    let mut report = String::new();
    report.push_str("# UGC 审核报告\n\n");
    report.push_str("## 总结\n\n");
    report.push_str(&format!("- 最终结论：{}\n", verdict_label(&run.verdict)));
    report.push_str(&format!("- 运行编号：{}\n", run.id));
    if !run.task_name.trim().is_empty() {
        report.push_str(&format!("- 任务名称：{}\n", run.task_name));
    }
    report.push_str(&format!("- 流程：{}\n", run.flow_name));
    report.push_str(&format!("- 输入：{}\n", run.input_note));
    report.push_str(&format!("- 素材数量：{}\n", run.assets.len()));
    if !run.resource_root.is_empty() {
        report.push_str(&format!("- 资源根目录：{}\n", run.resource_root));
    }
    report.push_str("- 模型来源：本地模型目录\n\n");

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
                report_file_link(&asset.path, &asset.path)
            ));
        }
        report.push('\n');
    }

    report.push_str("## 流程结果\n\n");
    report.push_str("| 步骤 | 模块 | 状态 | 结论 | 处理文件 | 命中文件 | 说明 |\n");
    report.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for step in &run.steps {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            table_cell(&step.label),
            table_cell(&step.module_name),
            status_label(&step.status),
            verdict_label(&step.verdict),
            step.processed_files,
            step.matched_files,
            table_cell(&step.message)
        ));
    }

    report.push('\n');
    append_performance_report(&mut report, run);

    report.push_str("\n## 模块结论\n\n");
    for step in &run.steps {
        report.push_str(&step.report_section);
        report.push('\n');
    }

    report.push_str("## 本地文件\n\n");
    report.push_str(&format!(
        "- 运行目录：{}\n",
        report_file_link(&run.run_dir, &run.run_dir)
    ));
    if !run.artifact_dir.trim().is_empty() {
        report.push_str(&format!(
            "- 产物目录：{}\n",
            report_file_link(&run.artifact_dir, &run.artifact_dir)
        ));
    }
    if !run.resource_root.is_empty() {
        report.push_str(&format!(
            "- 资源根目录：{}\n",
            report_file_link(&run.resource_root, &run.resource_root)
        ));
    }
    report.push_str(&format!(
        "- 报告文件：{}\n",
        report_file_link(&run.report_path, &run.report_path)
    ));
    report
}

#[tauri::command]
fn get_data_root(app: tauri::AppHandle) -> Result<String, String> {
    ensure_data_dirs(&app).map(|path| path.display().to_string())
}

#[tauri::command]
fn get_app_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let root = ensure_data_dirs(&app)?;
    load_app_settings(&root)
}

#[tauri::command]
fn save_app_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let root = ensure_data_dirs(&app)?;
    save_app_settings_inner(&root, settings)
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
fn import_module_folder(
    app: tauri::AppHandle,
    folder_path: String,
) -> Result<Vec<ModuleInfo>, String> {
    let root = ensure_data_dirs(&app)?;
    let mut paths = load_model_paths(&root)?;
    let module_id = import_module_folder_inner(&root, &PathBuf::from(folder_path.trim()))?;
    paths.remove(&module_id);
    write_json(&model_paths_file(&root), &paths)?;
    load_modules(&root, &paths)
}

#[tauri::command]
fn remove_module(app: tauri::AppHandle, module_id: String) -> Result<Vec<ModuleInfo>, String> {
    let root = ensure_data_dirs(&app)?;
    let removed_module_id = remove_module_inner(&root, &module_id)?;
    let mut paths = load_model_paths(&root)?;
    paths.remove(&removed_module_id);
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

fn path_matches_relative_file(asset_path: &Path, relative_path: &str) -> bool {
    let normalized_relative = normalized_relative_path(relative_path);
    if normalized_relative.is_empty() {
        return false;
    }
    let file_name_matches = asset_path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|file_name| normalized_relative == normalized_relative_path(file_name));
    file_name_matches
        || normalized_relative_path(&asset_path.display().to_string())
            .ends_with(&normalized_relative)
}

fn resolve_report_target_path(
    app: &tauri::AppHandle,
    path: &str,
    run_id: Option<&str>,
) -> Result<PathBuf, String> {
    let direct = PathBuf::from(path);
    if direct.is_absolute() {
        return Ok(direct);
    }

    let Some(run_id) = run_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(direct);
    };

    let root = ensure_data_dirs(app)?;
    let run_dir = safe_run_dir(&root, run_id)?;
    let run: RunRecord = read_required_json(&run_dir.join("run.json"))?;
    let normalized_target = normalized_relative_path(path);

    for asset in &run.assets {
        let asset_path = PathBuf::from(asset.path.trim());
        if asset.kind == "directory" {
            let candidate = asset_path.join(&normalized_target);
            if candidate.exists() {
                return Ok(candidate);
            }
        } else if asset.kind == "file"
            && path_matches_relative_file(&asset_path, &normalized_target)
        {
            return Ok(asset_path);
        }
    }

    if !run.resource_root.trim().is_empty() {
        let candidate = PathBuf::from(run.resource_root.trim()).join(&normalized_target);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    if !run.artifact_dir.trim().is_empty() {
        let candidate = PathBuf::from(run.artifact_dir.trim()).join(&normalized_target);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let candidate = run_dir.join(&normalized_target);
    if candidate.exists() {
        return Ok(candidate);
    }

    Ok(direct)
}

#[tauri::command]
fn reveal_report_target(
    app: tauri::AppHandle,
    path: String,
    run_id: Option<String>,
) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("文件路径为空。".to_string());
    }

    let target = resolve_report_target_path(&app, trimmed, run_id.as_deref())?;
    if !target.exists() {
        return Err(format!("文件不存在：{}", target.display()));
    }

    if target.is_dir() {
        return tauri_plugin_opener::open_path(&target, None::<&str>)
            .map_err(|error| format!("无法打开文件夹 {}: {error}", target.display()));
    }

    let normalized = target.canonicalize().unwrap_or_else(|_| target.clone());
    tauri_plugin_opener::reveal_item_in_dir(&normalized)
        .map_err(|error| format!("无法定位文件 {}: {error}", target.display()))
}

#[tauri::command]
fn get_runtime_status(app: tauri::AppHandle) -> Result<RuntimeStatus, String> {
    runtime_status_inner(&app)
}

#[tauri::command]
async fn install_runtime_dependency(
    app: tauri::AppHandle,
    dependency_id: String,
) -> Result<RuntimeStatus, String> {
    tauri::async_runtime::spawn_blocking(move || install_dependency_inner(&app, &dependency_id))
        .await
        .map_err(|error| format!("安装任务异常结束：{error}"))?
}

#[tauri::command]
fn open_runtime_dependency_folder(
    app: tauri::AppHandle,
    dependency_id: String,
) -> Result<(), String> {
    runtime_dependency_spec(&dependency_id)?;
    let folder = dependency_folder(&app, &dependency_id)?;
    fs::create_dir_all(folder.join("site-packages"))
        .map_err(|error| format!("无法创建依赖文件夹 {}: {error}", folder.display()))?;
    tauri_plugin_opener::open_path(&folder, None::<&str>)
        .map_err(|error| format!("无法打开依赖文件夹 {}: {error}", folder.display()))
}

#[tauri::command]
fn open_runtime_python_folder(app: tauri::AppHandle) -> Result<(), String> {
    let folder = runtime_python_dir(&app)?;
    fs::create_dir_all(&folder)
        .map_err(|error| format!("无法创建 Python 文件夹 {}: {error}", folder.display()))?;
    tauri_plugin_opener::open_path(&folder, None::<&str>)
        .map_err(|error| format!("无法打开 Python 文件夹 {}: {error}", folder.display()))
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
fn load_scheme_file(app: tauri::AppHandle, path: String) -> Result<AuditScheme, String> {
    let root = ensure_data_dirs(&app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    read_scheme_file(Path::new(path.trim()), &modules)
}

#[tauri::command]
fn save_scheme_file(
    app: tauri::AppHandle,
    path: String,
    scheme: AuditScheme,
) -> Result<AuditScheme, String> {
    let root = ensure_data_dirs(&app)?;
    let (normalized, validation) = validate_scheme_for_save(&root, scheme)?;
    if !validation.valid {
        return Err(validation.messages.join(" "));
    }
    let path = scheme_path_with_default_extension(Path::new(path.trim()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建审核方案目录 {}: {error}", parent.display()))?;
    }
    write_json(&path, &normalized)?;
    Ok(normalized)
}

#[tauri::command]
fn get_scheme_library_dir() -> Result<String, String> {
    Ok(ensure_scheme_library_dir()?.display().to_string())
}

#[tauri::command]
fn list_scheme_files() -> Result<Vec<SchemeListItem>, String> {
    list_scheme_files_inner()
}

#[tauri::command]
fn delete_scheme_file(path: String) -> Result<Vec<SchemeListItem>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("审核方案路径为空。".to_string());
    }
    let target = PathBuf::from(trimmed);
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !extension.eq_ignore_ascii_case("ugcaudit") {
        return Err("只能删除 .ugcaudit 审核方案文件。".to_string());
    }

    let library = ensure_scheme_library_dir()?
        .canonicalize()
        .map_err(|error| format!("无法读取审核方案目录：{error}"))?;
    let canonical_target = target
        .canonicalize()
        .map_err(|error| format!("无法读取审核方案 {}: {error}", target.display()))?;
    if !canonical_target.starts_with(&library) {
        return Err("只能从默认方案目录删除审核方案。".to_string());
    }

    fs::remove_file(&canonical_target)
        .map_err(|error| format!("无法删除审核方案 {}: {error}", canonical_target.display()))?;
    list_scheme_files_inner()
}

#[tauri::command]
fn save_scheme_to_library(
    app: tauri::AppHandle,
    scheme: AuditScheme,
) -> Result<SavedAuditScheme, String> {
    let root = ensure_data_dirs(&app)?;
    let (normalized, validation) = validate_scheme_for_save(&root, scheme)?;
    if !validation.valid {
        return Err(validation.messages.join(" "));
    }

    let dir = ensure_scheme_library_dir()?;
    let path = unique_scheme_library_path(&dir, &normalized.name);
    write_json(&path, &normalized)?;
    Ok(SavedAuditScheme {
        path: path.display().to_string(),
        scheme: normalized,
    })
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

#[allow(dead_code)]
fn start_run_inner(
    app: Option<&tauri::AppHandle>,
    root: &Path,
    flow: FlowDefinition,
    input_note: String,
    assets: Vec<AuditAsset>,
    task_name_override: Option<String>,
    artifact_dir_override: Option<PathBuf>,
) -> Result<RunRecord, String> {
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let flow = normalize_flow(flow, &modules);
    let validation = validate_flow_inner(&flow, &modules);
    if !validation.valid {
        return Err(validation.messages.join(" "));
    }

    let ordered_nodes = topological_order(&flow)?;
    let run_id = format!("run_{}", now_millis());
    let run_artifacts = prepare_run_artifacts(
        root,
        &flow.name,
        &input_note,
        &run_id,
        task_name_override,
        artifact_dir_override,
    )?;
    let run_dir = root.join("runs").join(&run_id);
    let resource_dir = run_dir.join("resources");
    let steps_dir = run_dir.join("steps");
    let audit_files = collect_audit_files(&assets)?;

    fs::create_dir_all(&steps_dir)
        .map_err(|error| format!("无法创建运行目录 {}: {error}", steps_dir.display()))?;
    fs::create_dir_all(&resource_dir)
        .map_err(|error| format!("无法创建资源目录 {}: {error}", resource_dir.display()))?;
    write_json(&run_dir.join("flow.snapshot.json"), &flow)?;
    write_json(
        &resource_dir.join("manifest.json"),
        &json!({
            "runId": run_id,
            "taskName": &run_artifacts.task_name,
            "artifactDir": run_artifacts.artifact_dir.display().to_string(),
            "inputNote": &input_note,
            "assets": &assets,
            "files": &audit_files
        }),
    )?;
    write_json(&resource_dir.join("files.json"), &audit_files)?;

    let mut steps = Vec::new();
    let mut module_outputs: HashMap<String, Value> = HashMap::new();
    for (node, execution_group) in ordered_nodes {
        let module = modules
            .get(&node.module_id)
            .ok_or_else(|| format!("找不到模块 {}", node.module_id))?;
        let step_dir = steps_dir.join(&node.id);
        fs::create_dir_all(&step_dir)
            .map_err(|error| format!("无法创建步骤目录 {}: {error}", step_dir.display()))?;

        let (step, output_value) = if module.source == "system" {
            let step = module_step_result(module, &node, execution_group);
            let output_value = json!({
                "status": &step.status,
                "verdict": &step.verdict,
                "message": &step.message,
                "processedFiles": step.processed_files,
                "matchedFiles": step.matched_files,
                "artifactCount": step.artifact_count,
                "reportSection": &step.report_section,
            });
            write_json(
                &step_dir.join("input.json"),
                &json!({
                    "resourceRoot": resource_dir.display().to_string(),
                    "artifactDir": run_artifacts.artifact_dir.display().to_string(),
                    "taskName": &run_artifacts.task_name,
                    "files": &audit_files,
                    "params": &node.config,
                }),
            )?;
            write_json(&step_dir.join("output.json"), &output_value)?;
            fs::write(step_dir.join("result.md"), &step.report_section)
                .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
            (step, output_value)
        } else {
            let mut data_cache: HashMap<String, DataPortValue> = HashMap::new();
            let data_inputs = data_inputs_for_node(
                &flow,
                &modules,
                &assets,
                &audit_files,
                &run_artifacts.artifact_dir,
                &module_outputs,
                &mut data_cache,
                &node,
            )?;
            let compatible_files = if data_inputs.is_empty() {
                audit_files.clone()
            } else {
                compatible_files_from_inputs(&data_inputs)
            };
            let mut previous_for_node = HashMap::new();
            collect_previous_output_sources(
                &flow,
                &modules,
                &module_outputs,
                &node.id,
                &mut previous_for_node,
                &mut HashSet::new(),
            );
            execute_module_step(
                app,
                module,
                &node,
                execution_group,
                &run_id,
                &run_artifacts.task_name,
                &resource_dir,
                &run_artifacts.artifact_dir,
                &step_dir,
                &compatible_files,
                &data_inputs,
                &previous_for_node,
            )?
        };
        module_outputs.insert(node.id.clone(), output_value);
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

    let report_path = run_artifacts.report_path.clone();
    let mut run = RunRecord {
        id: run_id,
        flow_id: flow.id,
        flow_name: flow.name,
        created_at: now_seconds(),
        status: "completed".to_string(),
        verdict: verdict.to_string(),
        task_name: run_artifacts.task_name.clone(),
        input_note: if input_note.trim().is_empty() {
            "未填写输入说明".to_string()
        } else {
            input_note.trim().to_string()
        },
        assets,
        data_root: root.display().to_string(),
        run_dir: run_dir.display().to_string(),
        resource_root: resource_dir.display().to_string(),
        artifact_root: run_artifacts.artifact_root.display().to_string(),
        artifact_dir: run_artifacts.artifact_dir.display().to_string(),
        report_path: report_path.display().to_string(),
        performance_summary: None,
        steps,
    };

    finalize_run_performance(&mut run);
    let report = build_report(&run);
    fs::write(&report_path, report)
        .map_err(|error| format!("无法写入报告 {}: {error}", report_path.display()))?;
    write_json(&run_dir.join("run.json"), &run)?;
    run.report_path = report_path.display().to_string();

    Ok(run)
}

#[derive(Debug)]
struct NodeExecutionResult {
    node_id: String,
    step: StepRun,
    output: Value,
    cancelled: bool,
}

fn verdict_from_steps(steps: &[StepRun]) -> String {
    if steps.iter().any(|step| step.status == "cancelled") {
        "review".to_string()
    } else if steps.iter().any(|step| step.verdict == "reject") {
        "reject".to_string()
    } else if steps.iter().any(|step| step.verdict == "error") {
        "error".to_string()
    } else if steps.iter().any(|step| step.verdict == "review") {
        "review".to_string()
    } else {
        "pass".to_string()
    }
}

fn emit_step_finished(app: &tauri::AppHandle, run_id: &str, step: &StepRun, cancelled: bool) {
    let event_name = if cancelled || step.status == "cancelled" {
        "step_cancelled"
    } else if step.status == "error" || step.verdict == "error" {
        "step_failed"
    } else {
        "step_completed"
    };
    emit_run_event(
        app,
        event_name,
        RunProgressEvent {
            run_id: run_id.to_string(),
            node_id: Some(step.step_id.clone()),
            status: step.status.clone(),
            progress: Some(step.progress),
            message: step.message.clone(),
            processed: Some(step.processed_files),
            total: None,
            step: Some(step.clone()),
            run: None,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn execute_live_node(
    app: tauri::AppHandle,
    control: Arc<RunControl>,
    flow: Arc<FlowDefinition>,
    modules: Arc<HashMap<String, ModuleInfo>>,
    assets: Arc<Vec<AuditAsset>>,
    audit_files: Arc<Vec<AuditFile>>,
    resource_dir: PathBuf,
    artifact_dir: PathBuf,
    steps_dir: PathBuf,
    run_id: String,
    task_name: String,
    node: FlowNode,
    execution_group: usize,
    module_outputs: HashMap<String, Value>,
) -> NodeExecutionResult {
    let module = modules
        .get(&node.module_id)
        .cloned()
        .unwrap_or_else(|| ModuleInfo {
            id: node.module_id.clone(),
            name: node.label.clone(),
            kind: "unknown".to_string(),
            summary: String::new(),
            model_label: default_model_label(),
            icon: "file-check".to_string(),
            built_in: false,
            source: "custom".to_string(),
            definition_dir: String::new(),
            icon_path: None,
            icon_data_url: None,
            model_path: None,
            model_configured: false,
            launch: ModuleLaunch::default(),
            parameters: Vec::new(),
            data_outputs: Vec::new(),
        });
    emit_run_event(
        &app,
        "step_started",
        RunProgressEvent {
            run_id: run_id.clone(),
            node_id: Some(node.id.clone()),
            status: "running".to_string(),
            progress: Some(0.0),
            message: "开始执行。".to_string(),
            processed: None,
            total: None,
            step: None,
            run: None,
        },
    );

    let step_dir = steps_dir.join(&node.id);
    let result = (|| -> Result<(StepRun, Value, bool), String> {
        fs::create_dir_all(&step_dir)
            .map_err(|error| format!("无法创建步骤目录 {}: {error}", step_dir.display()))?;

        if control.is_cancelled() {
            let step = synthetic_step_result(
                &module,
                &node,
                execution_group,
                "cancelled",
                "review",
                "流程已中断，步骤未启动。",
                0.0,
            );
            let output = step_output_value(&step);
            write_json(&step_dir.join("output.json"), &output)?;
            fs::write(step_dir.join("result.md"), &step.report_section)
                .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
            return Ok((step, output, true));
        }

        if module.source == "system" {
            let step = module_step_result(&module, &node, execution_group);
            let output = step_output_value(&step);
            write_json(
                &step_dir.join("input.json"),
                &json!({
                    "resourceRoot": resource_dir.display().to_string(),
                    "artifactDir": artifact_dir.display().to_string(),
                    "taskName": &task_name,
                    "files": audit_files.as_ref(),
                    "params": &node.config,
                }),
            )?;
            write_json(&step_dir.join("output.json"), &output)?;
            fs::write(step_dir.join("result.md"), &step.report_section)
                .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
            return Ok((step, output, false));
        }

        let mut data_cache = HashMap::new();
        let data_inputs = data_inputs_for_node(
            &flow,
            &modules,
            &assets,
            &audit_files,
            &artifact_dir,
            &module_outputs,
            &mut data_cache,
            &node,
        )?;
        let compatible_files = if data_inputs.is_empty() {
            audit_files.as_ref().clone()
        } else {
            compatible_files_from_inputs(&data_inputs)
        };
        let mut previous_for_node = HashMap::new();
        collect_previous_output_sources(
            &flow,
            &modules,
            &module_outputs,
            &node.id,
            &mut previous_for_node,
            &mut HashSet::new(),
        );
        execute_module_step_live(
            &app,
            &control,
            &module,
            &node,
            execution_group,
            &run_id,
            &task_name,
            &resource_dir,
            &artifact_dir,
            &step_dir,
            &compatible_files,
            &data_inputs,
            &previous_for_node,
        )
    })();

    let (step, output, cancelled) = match result {
        Ok(value) => value,
        Err(error) => {
            let step = synthetic_step_result(
                &module,
                &node,
                execution_group,
                "error",
                "error",
                &error,
                1.0,
            );
            let output = step_output_value(&step);
            let _ = write_json(&step_dir.join("output.json"), &output);
            let _ = fs::write(step_dir.join("result.md"), &step.report_section);
            (step, output, false)
        }
    };
    emit_step_finished(&app, &run_id, &step, cancelled);
    NodeExecutionResult {
        node_id: node.id,
        step,
        output,
        cancelled,
    }
}

fn start_run_live_inner(
    app: &tauri::AppHandle,
    run_id: String,
    flow: FlowDefinition,
    input_note: String,
    assets: Vec<AuditAsset>,
    control: Arc<RunControl>,
    task_name_override: Option<String>,
    artifact_dir_override: Option<PathBuf>,
) -> Result<RunRecord, String> {
    let root = ensure_data_dirs(app)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let flow = normalize_flow(flow, &modules);
    let validation = validate_flow_inner(&flow, &modules);
    if !validation.valid {
        let message = validation.messages.join(" ");
        emit_run_event(
            app,
            "run_failed",
            run_event(&run_id, None, "error", &message),
        );
        return Err(message);
    }

    let plan = execution_plan(&flow, &modules)?;
    let run_artifacts = prepare_run_artifacts(
        &root,
        &flow.name,
        &input_note,
        &run_id,
        task_name_override,
        artifact_dir_override,
    )?;
    let run_dir = root.join("runs").join(&run_id);
    let resource_dir = run_dir.join("resources");
    let steps_dir = run_dir.join("steps");
    let audit_files = collect_audit_files(&assets)?;

    fs::create_dir_all(&steps_dir)
        .map_err(|error| format!("无法创建运行目录 {}: {error}", steps_dir.display()))?;
    fs::create_dir_all(&resource_dir)
        .map_err(|error| format!("无法创建资源目录 {}: {error}", resource_dir.display()))?;
    write_json(&run_dir.join("flow.snapshot.json"), &flow)?;
    write_json(
        &resource_dir.join("manifest.json"),
        &json!({
            "runId": run_id,
            "taskName": &run_artifacts.task_name,
            "artifactDir": run_artifacts.artifact_dir.display().to_string(),
            "inputNote": &input_note,
            "assets": &assets,
            "files": &audit_files
        }),
    )?;
    write_json(&resource_dir.join("files.json"), &audit_files)?;

    emit_run_event(
        app,
        "run_started",
        RunProgressEvent {
            run_id: run_id.clone(),
            node_id: None,
            status: "running".to_string(),
            progress: Some(0.0),
            message: "流程开始运行。".to_string(),
            processed: None,
            total: Some(plan.ordered.len()),
            step: None,
            run: None,
        },
    );

    let ordered_nodes = plan
        .ordered
        .iter()
        .map(|(node, group)| (node.id.clone(), (node.clone(), *group)))
        .collect::<HashMap<_, _>>();
    let mut remaining_deps = plan
        .dependencies
        .iter()
        .map(|(id, deps)| (id.clone(), deps.len()))
        .collect::<HashMap<_, _>>();
    let mut ready = sorted_ready_ids(&flow, &remaining_deps)
        .into_iter()
        .filter(|id| ordered_nodes.contains_key(id))
        .collect::<VecDeque<_>>();
    let mut module_outputs: HashMap<String, Value> = HashMap::new();
    let mut steps_by_id: HashMap<String, StepRun> = HashMap::new();
    let mut completed = HashSet::new();
    let mut running = 0_usize;
    let (tx, rx) = mpsc::channel::<NodeExecutionResult>();
    let app_arc = app.clone();
    let flow_arc = Arc::new(flow.clone());
    let modules_arc = Arc::new(modules.clone());
    let assets_arc = Arc::new(assets.clone());
    let audit_files_arc = Arc::new(audit_files.clone());
    let artifact_dir = run_artifacts.artifact_dir.clone();

    while completed.len() < ordered_nodes.len() {
        while !control.is_cancelled() && running < MAX_PARALLEL_MODULES {
            let Some(next_id) = ready.pop_front() else {
                break;
            };
            if completed.contains(&next_id) {
                continue;
            }
            let Some((node, execution_group)) = ordered_nodes.get(&next_id).cloned() else {
                continue;
            };
            running += 1;
            let tx = tx.clone();
            let app = app_arc.clone();
            let control = control.clone();
            let flow = flow_arc.clone();
            let modules = modules_arc.clone();
            let assets = assets_arc.clone();
            let audit_files = audit_files_arc.clone();
            let resource_dir = resource_dir.clone();
            let artifact_dir = artifact_dir.clone();
            let steps_dir = steps_dir.clone();
            let run_id_for_node = run_id.clone();
            let task_name_for_node = run_artifacts.task_name.clone();
            let outputs_snapshot = module_outputs.clone();
            thread::spawn(move || {
                let result = execute_live_node(
                    app,
                    control,
                    flow,
                    modules,
                    assets,
                    audit_files,
                    resource_dir,
                    artifact_dir,
                    steps_dir,
                    run_id_for_node,
                    task_name_for_node,
                    node,
                    execution_group,
                    outputs_snapshot,
                );
                let _ = tx.send(result);
            });
        }

        if running == 0 {
            break;
        }

        let result = rx
            .recv()
            .map_err(|error| format!("运行任务异常结束：{error}"))?;
        running = running.saturating_sub(1);
        completed.insert(result.node_id.clone());
        module_outputs.insert(result.node_id.clone(), result.output);
        steps_by_id.insert(result.node_id.clone(), result.step);
        if result.cancelled {
            control.cancel();
        }

        if !control.is_cancelled() {
            if let Some(children) = plan.dependents.get(&result.node_id) {
                for child in children {
                    if let Some(count) = remaining_deps.get_mut(child) {
                        *count = count.saturating_sub(1);
                        if *count == 0 && !completed.contains(child) {
                            ready.push_back(child.clone());
                        }
                    }
                }
                sort_queue(&flow, &mut ready);
            }
        }
    }

    if control.is_cancelled() {
        for (node_id, (node, execution_group)) in &ordered_nodes {
            if completed.contains(node_id) || steps_by_id.contains_key(node_id) {
                continue;
            }
            let Some(module) = modules.get(&node.module_id) else {
                continue;
            };
            let step_dir = steps_dir.join(node_id);
            fs::create_dir_all(&step_dir)
                .map_err(|error| format!("无法创建步骤目录 {}: {error}", step_dir.display()))?;
            let step = synthetic_step_result(
                module,
                node,
                *execution_group,
                "cancelled",
                "review",
                "流程已中断，步骤未启动。",
                0.0,
            );
            let output = step_output_value(&step);
            write_json(&step_dir.join("output.json"), &output)?;
            fs::write(step_dir.join("result.md"), &step.report_section)
                .map_err(|error| format!("无法写入步骤结果 {}: {error}", step_dir.display()))?;
            emit_step_finished(app, &run_id, &step, true);
            steps_by_id.insert(node_id.clone(), step);
        }
    }

    let mut steps = plan
        .ordered
        .iter()
        .filter_map(|(node, _)| steps_by_id.get(&node.id).cloned())
        .collect::<Vec<_>>();
    steps.sort_by(|left, right| {
        left.execution_group
            .cmp(&right.execution_group)
            .then(left.label.cmp(&right.label))
    });
    let verdict = verdict_from_steps(&steps);
    let report_path = run_artifacts.report_path.clone();
    let mut run = RunRecord {
        id: run_id.clone(),
        flow_id: flow.id,
        flow_name: flow.name,
        created_at: now_seconds(),
        status: if control.is_cancelled() {
            "cancelled".to_string()
        } else {
            "completed".to_string()
        },
        verdict,
        task_name: run_artifacts.task_name.clone(),
        input_note: if input_note.trim().is_empty() {
            "未填写输入说明".to_string()
        } else {
            input_note.trim().to_string()
        },
        assets,
        data_root: root.display().to_string(),
        run_dir: run_dir.display().to_string(),
        resource_root: resource_dir.display().to_string(),
        artifact_root: run_artifacts.artifact_root.display().to_string(),
        artifact_dir: run_artifacts.artifact_dir.display().to_string(),
        report_path: report_path.display().to_string(),
        performance_summary: None,
        steps,
    };
    finalize_run_performance(&mut run);
    let report = build_report(&run);
    fs::write(&report_path, report)
        .map_err(|error| format!("无法写入报告 {}: {error}", report_path.display()))?;
    write_json(&run_dir.join("run.json"), &run)?;
    run.report_path = report_path.display().to_string();

    emit_run_event(
        app,
        if run.status == "cancelled" {
            "run_cancelled"
        } else {
            "run_completed"
        },
        RunProgressEvent {
            run_id,
            node_id: None,
            status: run.status.clone(),
            progress: Some(1.0),
            message: if run.status == "cancelled" {
                "流程已中断。".to_string()
            } else {
                "流程运行完成。".to_string()
            },
            processed: Some(run.steps.len()),
            total: Some(run.steps.len()),
            step: None,
            run: Some(run.clone()),
        },
    );

    Ok(run)
}

#[tauri::command]
async fn start_run(
    app: tauri::AppHandle,
    flow: FlowDefinition,
    input_note: String,
    assets: Vec<AuditAsset>,
) -> Result<RunRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let run_id = format!("run_{}", now_millis());
        start_run_live_inner(
            &app,
            run_id,
            flow,
            input_note,
            assets,
            Arc::new(RunControl::default()),
            None,
            None,
        )
    })
    .await
    .map_err(|error| format!("运行任务异常结束：{error}"))?
}

#[tauri::command]
async fn start_run_live(
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedRunRegistry>,
    flow: FlowDefinition,
    input_note: String,
    assets: Vec<AuditAsset>,
) -> Result<RunStartResponse, String> {
    let run_id = format!("run_{}", now_millis());
    let control = Arc::new(RunControl::default());
    registry
        .controls
        .lock()
        .map_err(|_| "运行状态被占用，无法启动流程。".to_string())?
        .insert(run_id.clone(), control.clone());

    let registry = registry.inner().clone();
    let app_for_task = app.clone();
    let run_id_for_task = run_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = start_run_live_inner(
            &app_for_task,
            run_id_for_task.clone(),
            flow,
            input_note,
            assets,
            control,
            None,
            None,
        );
        if let Err(error) = result {
            emit_run_event(
                &app_for_task,
                "run_failed",
                run_event(&run_id_for_task, None, "error", &error),
            );
        }
        if let Ok(mut controls) = registry.controls.lock() {
            controls.remove(&run_id_for_task);
        }
    });

    Ok(RunStartResponse { run_id })
}

#[tauri::command]
fn cancel_run(registry: tauri::State<'_, SharedRunRegistry>, run_id: String) -> Result<(), String> {
    let controls = registry
        .controls
        .lock()
        .map_err(|_| "运行状态被占用，无法中断流程。".to_string())?;
    let Some(control) = controls.get(&run_id) else {
        return Err("没有找到正在运行的流程。".to_string());
    };
    control.cancel();
    Ok(())
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
fn read_run_record(app: tauri::AppHandle, run_id: String) -> Result<RunRecord, String> {
    let root = ensure_data_dirs(&app)?;
    let run_path = root.join("runs").join(run_id).join("run.json");
    read_required_json(&run_path)
}

#[tauri::command]
fn read_run_report(app: tauri::AppHandle, run_id: String) -> Result<String, String> {
    let root = ensure_data_dirs(&app)?;
    let run_dir = safe_run_dir(&root, &run_id)?;
    let run_path = run_dir.join("run.json");
    let report_path = if run_path.exists() {
        let run: RunRecord = read_required_json(&run_path)?;
        let configured = PathBuf::from(run.report_path.trim());
        if !configured.as_os_str().is_empty() {
            configured
        } else {
            run_dir.join("report.md")
        }
    } else {
        run_dir.join("report.md")
    };
    fs::read_to_string(&report_path)
        .map_err(|error| format!("无法读取报告 {}: {error}", report_path.display()))
}

fn safe_run_dir(root: &Path, run_id: &str) -> Result<PathBuf, String> {
    if run_id.trim().is_empty()
        || run_id.contains('/')
        || run_id.contains('\\')
        || run_id.contains("..")
    {
        return Err("运行记录 ID 不合法。".to_string());
    }
    Ok(root.join("runs").join(run_id))
}

#[tauri::command]
fn delete_run(app: tauri::AppHandle, run_id: String) -> Result<Vec<RunSummary>, String> {
    let root = ensure_data_dirs(&app)?;
    let run_dir = safe_run_dir(&root, &run_id)?;
    if run_dir.exists() {
        fs::remove_dir_all(&run_dir)
            .map_err(|error| format!("无法删除历史记录 {}: {error}", run_id))?;
    }
    list_runs(app)
}

#[tauri::command]
fn delete_all_runs(app: tauri::AppHandle) -> Result<Vec<RunSummary>, String> {
    let root = ensure_data_dirs(&app)?;
    let runs_dir = root.join("runs");
    if runs_dir.exists() {
        for entry in fs::read_dir(&runs_dir)
            .map_err(|error| format!("无法读取运行目录 {}: {error}", runs_dir.display()))?
        {
            let entry = entry.map_err(|error| format!("无法读取运行记录: {error}"))?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|error| format!("无法删除历史记录 {}: {error}", path.display()))?;
            }
        }
    }
    Ok(Vec::new())
}

#[derive(Debug, Clone)]
struct CliRunArgs {
    scheme: PathBuf,
    input: PathBuf,
    output: Option<PathBuf>,
    task_name: Option<String>,
    note: String,
}

#[derive(Debug, Clone)]
enum CliCommand {
    Run(CliRunArgs),
    Help,
}

fn parse_cli_args(args: Vec<String>) -> Result<Option<CliCommand>, String> {
    if args.is_empty() {
        return Ok(None);
    }
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h" | "help") {
        return Ok(Some(CliCommand::Help));
    }
    if args[0] != "run" {
        return Err("未知命令。请使用 run --scheme <方案文件> --input <待审文件夹>。".to_string());
    }

    let mut scheme = None;
    let mut input = None;
    let mut output = None;
    let mut task_name = None;
    let mut note = String::new();
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        let Some(value) = args.get(index + 1) else {
            return Err(format!("参数 {flag} 缺少值。"));
        };
        match flag {
            "--scheme" => scheme = Some(PathBuf::from(value)),
            "--input" => input = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--task-name" => task_name = Some(value.clone()),
            "--note" => note = value.clone(),
            _ => return Err(format!("未知参数：{flag}")),
        }
        index += 2;
    }

    let scheme = scheme.ok_or_else(|| "缺少参数 --scheme。".to_string())?;
    let input = input.ok_or_else(|| "缺少参数 --input。".to_string())?;
    Ok(Some(CliCommand::Run(CliRunArgs {
        scheme,
        input,
        output,
        task_name,
        note,
    })))
}

fn cli_asset_from_input(path: &Path) -> Result<AuditAsset, String> {
    if !path.is_dir() {
        return Err(format!(
            "CLI 只支持审核文件夹，路径不存在或不是文件夹：{}",
            path.display()
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("无法读取待审文件夹 {}: {error}", path.display()))?;
    Ok(AuditAsset {
        id: "cli_input".to_string(),
        kind: "directory".to_string(),
        path: canonical.display().to_string(),
        name: file_name(&canonical),
        extension: String::new(),
    })
}

fn cli_exit_code_for_run(run: &RunRecord) -> i32 {
    match run.verdict.as_str() {
        "pass" => 0,
        "review" | "reject" => 2,
        _ => 1,
    }
}

fn write_cli_outputs(
    run: &RunRecord,
    args: &CliRunArgs,
    output_dir: &Path,
    exit_code: i32,
) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("无法创建 CLI 输出目录 {}: {error}", output_dir.display()))?;

    let mut exported_run = run.clone();
    exported_run.artifact_dir = output_dir.display().to_string();
    exported_run.artifact_root = output_dir
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| output_dir.display().to_string());
    exported_run.report_path = output_dir.join("report.md").display().to_string();
    write_json(&output_dir.join("run.json"), &exported_run)?;
    fs::write(output_dir.join("report.md"), build_report(&exported_run))
        .map_err(|error| format!("无法写入 CLI 报告 {}: {error}", output_dir.display()))?;
    write_json(
        &output_dir.join("cli-result.json"),
        &json!({
            "exitCode": exit_code,
            "runId": exported_run.id,
            "status": exported_run.status,
            "verdict": exported_run.verdict,
            "schemePath": args.scheme.display().to_string(),
            "inputPath": args.input.display().to_string(),
            "outputPath": output_dir.display().to_string(),
            "artifactDir": output_dir.display().to_string(),
            "runPath": output_dir.join("run.json").display().to_string(),
            "reportPath": output_dir.join("report.md").display().to_string(),
        }),
    )
}

fn write_cli_error_output(args: Option<&CliRunArgs>, message: &str) {
    let Some(args) = args else {
        return;
    };
    let Some(output_dir) = args.output.as_ref() else {
        return;
    };
    if fs::create_dir_all(output_dir).is_err() {
        return;
    }
    let _ = write_json(
        &output_dir.join("cli-result.json"),
        &json!({
            "exitCode": 1,
            "status": "error",
            "verdict": "error",
            "message": message,
            "schemePath": args.scheme.display().to_string(),
            "inputPath": args.input.display().to_string(),
            "outputPath": output_dir.display().to_string(),
            "artifactDir": output_dir.display().to_string(),
        }),
    );
}

fn run_cli_command(command: CliCommand) -> i32 {
    match command {
        CliCommand::Help => 0,
        CliCommand::Run(args) => match run_cli_audit(&args) {
            Ok(code) => code,
            Err(error) => {
                write_cli_error_output(Some(&args), &error);
                1
            }
        },
    }
}

fn run_cli_audit(args: &CliRunArgs) -> Result<i32, String> {
    let root = ensure_data_dirs_at(&cli_data_root())?;
    configure_cli_runtime_env(&root)?;
    let paths = load_model_paths(&root)?;
    let modules = modules_by_id(&root, &paths)?;
    let scheme = read_scheme_file(&args.scheme, &modules)?;
    let validation = validate_flow_inner(&scheme.flow, &modules);
    if !validation.valid {
        return Err(validation.messages.join(" "));
    }
    let asset = cli_asset_from_input(&args.input)?;
    let task_name = args
        .task_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&scheme.name)
        .to_string();
    let input_note = if args.note.trim().is_empty() {
        format!("任务名称：{task_name}")
    } else {
        format!("任务名称：{task_name}\n任务说明：{}", args.note.trim())
    };
    let run = start_run_inner(
        None,
        &root,
        scheme.flow,
        input_note,
        vec![asset],
        Some(task_name),
        args.output.clone(),
    )?;
    let exit_code = cli_exit_code_for_run(&run);
    let output_dir = if let Some(output_dir) = args.output.as_ref() {
        output_dir.clone()
    } else {
        PathBuf::from(&run.artifact_dir)
    };
    write_cli_outputs(&run, args, &output_dir, exit_code)?;
    Ok(exit_code)
}

pub fn run_cli_from_args() -> Option<i32> {
    match parse_cli_args(env::args().skip(1).collect()) {
        Ok(None) => None,
        Ok(Some(command)) => Some(run_cli_command(command)),
        Err(error) => {
            eprintln!("{error}");
            Some(1)
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn FreeConsole() -> i32;
}

pub fn detach_console_for_gui() {
    #[cfg(windows)]
    unsafe {
        let _ = FreeConsole();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ugc_audit_{}_{}_{}",
            name,
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&root).expect("create test root");
        root
    }

    fn test_asset(path: &Path, kind: &str) -> AuditAsset {
        AuditAsset {
            id: format!("asset_{}", now_millis()),
            kind: kind.to_string(),
            path: path.display().to_string(),
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("asset")
                .to_string(),
            extension: path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_string(),
        }
    }

    fn test_module(id: &str, name: &str, kind: &str) -> ModuleInfo {
        ModuleInfo {
            id: id.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            summary: String::new(),
            model_label: "Model".to_string(),
            icon: "file-check".to_string(),
            built_in: true,
            source: "preset".to_string(),
            definition_dir: String::new(),
            icon_path: None,
            icon_data_url: None,
            model_path: None,
            model_configured: true,
            launch: system_launch("test"),
            parameters: Vec::new(),
            data_outputs: Vec::new(),
        }
    }

    fn test_modules() -> HashMap<String, ModuleInfo> {
        let mut modules = HashMap::new();
        for module in builtin_module_definitions() {
            modules.insert(module.id.clone(), module);
        }
        for module in [
            test_module("preset.custom.paddleocr", "图片文字识别", "image_ocr"),
            test_module("preset.custom.qwen3guard", "文本合规检测", "text_safety"),
            test_module("preset.custom.shieldgemma2", "图片合规检测", "image_safety"),
        ] {
            modules.insert(module.id.clone(), module);
        }
        modules
    }

    #[test]
    fn cli_args_parse_run_command_with_ugcaudit_scheme() {
        let command = parse_cli_args(vec![
            "run".to_string(),
            "--scheme".to_string(),
            "D:\\AuditSchemes\\image.ugcaudit".to_string(),
            "--input".to_string(),
            "D:\\UGCProject".to_string(),
            "--output".to_string(),
            "D:\\AuditRuns\\run-001".to_string(),
            "--task-name".to_string(),
            "每日审核".to_string(),
        ])
        .expect("parse")
        .expect("command");

        let CliCommand::Run(args) = command else {
            panic!("expected run command");
        };
        assert_eq!(
            args.scheme,
            PathBuf::from("D:\\AuditSchemes\\image.ugcaudit")
        );
        assert_eq!(args.input, PathBuf::from("D:\\UGCProject"));
        assert_eq!(args.output, Some(PathBuf::from("D:\\AuditRuns\\run-001")));
        assert_eq!(args.task_name.as_deref(), Some("每日审核"));
    }

    #[test]
    fn scheme_reader_accepts_scheme_file_and_normalizes_flow() {
        let root = test_root("scheme_reader");
        let scheme_path = root.join("default.ugcaudit");
        let scheme = AuditScheme {
            schema_version: 1,
            kind: "ugcAuditScheme".to_string(),
            id: "scheme_test".to_string(),
            name: "测试方案".to_string(),
            flow: default_flow(),
        };
        write_json(&scheme_path, &scheme).expect("write scheme");

        let loaded = read_scheme_file(&scheme_path, &test_modules()).expect("read scheme");
        assert_eq!(loaded.kind, "ugcAuditScheme");
        assert_eq!(loaded.name, "测试方案");
        assert_eq!(loaded.flow.name, "测试方案");
        assert!(validate_flow_inner(&loaded.flow, &test_modules()).valid);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scheme_library_names_are_safe_and_unique() {
        let root = test_root("scheme_library");
        fs::create_dir_all(&root).expect("create scheme library");
        let first = unique_scheme_library_path(&root, "默认:审核/方案");
        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("默认_审核_方案.ugcaudit")
        );
        fs::write(&first, "{}").expect("write first scheme");
        let second = unique_scheme_library_path(&root, "默认:审核/方案");
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("默认_审核_方案-2.ugcaudit")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_folder_name_uses_task_name_and_run_id() {
        let root = test_root("artifact_paths");
        let artifact_root = root.join("audit-products");
        save_app_settings_inner(
            &root,
            AppSettings {
                artifact_root: artifact_root.display().to_string(),
                ..AppSettings::default()
            },
        )
        .expect("save settings");
        let paths = prepare_run_artifacts(
            &root,
            "默认方案",
            "任务名称：每日:审核/任务",
            "run_123",
            None,
            None,
        )
        .expect("prepare artifact paths");

        assert!(paths.artifact_dir.exists());
        assert_eq!(paths.task_name, "每日:审核/任务");
        assert!(paths
            .artifact_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .starts_with("每日_审核_任务-run_123"));
        assert_eq!(paths.report_path, paths.artifact_dir.join("report.md"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn app_settings_persist_artifact_root() {
        let root = test_root("app_settings");
        let artifact_root = root.join("outputs");
        let dependency_root = root.join("dependencies");
        let saved = save_app_settings_inner(
            &root,
            AppSettings {
                artifact_root: artifact_root.display().to_string(),
                dependency_root: dependency_root.display().to_string(),
                ..AppSettings::default()
            },
        )
        .expect("save settings");
        assert_eq!(saved.artifact_root, artifact_root.display().to_string());
        assert_eq!(saved.dependency_root, dependency_root.display().to_string());
        assert!(artifact_root.exists());
        assert!(dependency_root.exists());

        let loaded = load_app_settings(&root).expect("load settings");
        assert_eq!(loaded.artifact_root, saved.artifact_root);
        assert_eq!(loaded.dependency_root, saved.dependency_root);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dependency_pythonpath_uses_only_selected_dependency_root() {
        let root = test_root("dependency_pythonpath");
        let selected_root = root.join("selected");
        let selected_site_packages = selected_root.join("torch").join("site-packages");
        let old_root = root.join("old-runtime").join("Packages");
        fs::create_dir_all(selected_site_packages.join("torch")).expect("create selected package");
        fs::create_dir_all(old_root.join("torch").join("site-packages").join("torch"))
            .expect("create old package");

        let previous_pythonpath = env::var_os("PYTHONPATH");
        env::set_var("PYTHONPATH", &old_root);
        let pythonpath =
            dependency_pythonpath_for_dependency_root(&selected_root).expect("dependency path");
        let pythonpath_text = pythonpath.to_string_lossy();
        assert!(pythonpath_text.contains(&selected_site_packages.display().to_string()));
        assert!(!pythonpath_text.contains(&old_root.display().to_string()));
        if let Some(previous_pythonpath) = previous_pythonpath {
            env::set_var("PYTHONPATH", previous_pythonpath);
        } else {
            env::remove_var("PYTHONPATH");
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn python_launcher_uses_console_python_for_module_output_capture() {
        let root = test_root("python_launcher");
        let python = root.join("python.exe");
        let pythonw = root.join("pythonw.exe");
        fs::write(&python, b"").expect("create python");
        fs::write(&pythonw, b"").expect("create pythonw");

        let previous_python = env::var_os("UGCAUDIT_PYTHON");
        env::set_var("UGCAUDIT_PYTHON", &python);
        let launcher = runtime_python_launcher_path(None).expect("launcher path");
        assert_eq!(launcher, python);
        if let Some(previous_python) = previous_python {
            env::set_var("UGCAUDIT_PYTHON", previous_python);
        } else {
            env::remove_var("UGCAUDIT_PYTHON");
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cli_exit_codes_match_pipeline_contract() {
        let mut run = RunRecord {
            id: "run_test".to_string(),
            flow_id: "flow".to_string(),
            flow_name: "flow".to_string(),
            created_at: now_seconds(),
            status: "completed".to_string(),
            verdict: "pass".to_string(),
            task_name: String::new(),
            input_note: String::new(),
            assets: Vec::new(),
            data_root: String::new(),
            run_dir: String::new(),
            resource_root: String::new(),
            artifact_root: String::new(),
            artifact_dir: String::new(),
            report_path: String::new(),
            performance_summary: None,
            steps: Vec::new(),
        };
        assert_eq!(cli_exit_code_for_run(&run), 0);
        run.verdict = "review".to_string();
        assert_eq!(cli_exit_code_for_run(&run), 2);
        run.verdict = "error".to_string();
        assert_eq!(cli_exit_code_for_run(&run), 1);
    }

    #[test]
    fn performance_summary_assigns_cpu_share_and_report_section() {
        let module = test_module("custom.perf", "性能测试模块", "text_safety");
        let node = FlowNode {
            id: "perf_step".to_string(),
            module_id: module.id.clone(),
            label: "性能步骤".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            config: json!({}),
        };
        let mut step = synthetic_step_result(
            &module,
            &node,
            1,
            "completed",
            "pass",
            "done",
            1.0,
        );
        step.performance = Some(StepPerformance {
            start_time: 1,
            end_time: 2001,
            duration_ms: 2000,
            sample_count: 2,
            cpu_time_ms: 1200.0,
            average_cpu_percent: 18.0,
            peak_cpu_percent: 35.0,
            peak_memory_bytes: 128 * 1024 * 1024,
            artifact_bytes: 4096,
            sampling_note: performance_sampling_note(),
            ..StepPerformance::default()
        });

        let mut run = RunRecord {
            id: "run_perf".to_string(),
            flow_id: "flow".to_string(),
            flow_name: "flow".to_string(),
            created_at: now_seconds(),
            status: "completed".to_string(),
            verdict: "pass".to_string(),
            task_name: "性能测试".to_string(),
            input_note: String::new(),
            assets: Vec::new(),
            data_root: String::new(),
            run_dir: String::new(),
            resource_root: String::new(),
            artifact_root: String::new(),
            artifact_dir: String::new(),
            report_path: String::new(),
            performance_summary: None,
            steps: vec![step],
        };

        finalize_run_performance(&mut run);
        let summary = run.performance_summary.as_ref().expect("summary");
        assert_eq!(summary.measured_steps, 1);
        assert_eq!(summary.total_duration_ms, 2000);
        assert_eq!(
            run.steps[0]
                .performance
                .as_ref()
                .expect("step performance")
                .cpu_share_percent
                .round(),
            100.0
        );
        let report = build_report(&run);
        assert!(report.contains("## 性能开销"));
        assert!(report.contains("性能步骤"));
        assert!(report.contains("CPU 占本次运行"));
    }

    #[test]
    fn run_record_accepts_missing_performance_fields() {
        let run: RunRecord = serde_json::from_value(json!({
            "id": "run_old",
            "flowId": "flow",
            "flowName": "flow",
            "createdAt": 1,
            "status": "completed",
            "verdict": "pass",
            "inputNote": "",
            "assets": [],
            "dataRoot": "",
            "runDir": "",
            "resourceRoot": "",
            "artifactRoot": "",
            "artifactDir": "",
            "reportPath": "",
            "steps": []
        }))
        .expect("old run record");
        assert!(run.performance_summary.is_none());
    }

    #[test]
    fn module_failure_summary_ignores_warning_only_stderr() {
        let message = module_failure_message(
            "图片文字识别",
            "D:\\UGCAuditModels\\preset.custom.paddleocr\\main.py:63: UserWarning: warning\n  return PaddleOCR(\n",
            true,
        );
        assert!(message.contains("没有生成结果文件"));
        assert!(!message.contains("UserWarning"));
    }

    #[test]
    fn module_failure_summary_prefers_real_error() {
        let message = module_failure_message(
            "图片文字识别",
            "some.py:1: UserWarning: warning\nTraceback (most recent call last):\nModuleNotFoundError: No module named 'x'\n",
            true,
        );
        assert!(message.contains("ModuleNotFoundError"));
    }

    #[test]
    fn default_flow_has_valid_typed_data_edges() {
        let flow = default_flow();
        let result = validate_flow_inner(&flow, &test_modules());
        assert!(result.valid, "{:?}", result.messages);
        assert!(flow
            .edges
            .iter()
            .any(|edge| edge.edge_type == EDGE_TYPE_DATA));
        assert!(flow
            .nodes
            .iter()
            .any(|node| node.module_id == DATA_ALL_IMAGES_MODULE_ID));
    }

    #[test]
    fn rejects_mismatched_data_port_types() {
        let flow = FlowDefinition {
            id: "flow.bad".to_string(),
            name: "Bad".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: START_NODE_ID.to_string(),
                    module_id: START_MODULE_ID.to_string(),
                    label: "开始".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "all_images".to_string(),
                    module_id: DATA_ALL_IMAGES_MODULE_ID.to_string(),
                    label: "图片".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "text_safety".to_string(),
                    module_id: "preset.custom.qwen3guard".to_string(),
                    label: "文本合规检测".to_string(),
                    position: Position { x: 1.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: OUTPUT_NODE_ID.to_string(),
                    module_id: OUTPUT_MODULE_ID.to_string(),
                    label: "输出".to_string(),
                    position: Position { x: 2.0, y: 0.0 },
                    config: json!({}),
                },
            ],
            edges: vec![
                sequence_edge("start_text", START_NODE_ID, "text_safety"),
                sequence_edge("text_output", "text_safety", OUTPUT_NODE_ID),
                data_edge(
                    "bad_data",
                    "all_images",
                    HANDLE_IMAGES_OUT,
                    "text_safety",
                    HANDLE_TEXTS_IN,
                ),
            ],
        };
        let result = validate_flow_inner(&flow, &test_modules());
        assert!(!result.valid);
        assert!(result
            .messages
            .iter()
            .any(|message| message.contains("数据类型不匹配")));
    }

    #[test]
    fn merge_collections_deduplicates_by_path() {
        let left = collection_from_items(
            DATA_TYPE_IMAGES,
            vec![
                json!({"path": "a.png", "name": "a.png"}),
                json!({"path": "b.png", "name": "b.png"}),
            ],
        );
        let right = collection_from_items(
            DATA_TYPE_IMAGES,
            vec![
                json!({"path": "a.png", "name": "a copy.png"}),
                json!({"path": "c.png", "name": "c.png"}),
            ],
        );
        let merged = merge_collections(left, right);
        assert_eq!(merged.items.len(), 3);
    }

    #[test]
    fn ocr_output_feeds_text_collection_input() {
        let flow = default_flow();
        let modules = test_modules();
        let mut outputs = HashMap::new();
        outputs.insert(
            "image_ocr".to_string(),
            json!({
                "outputs": {"fullText": "hello from image"},
                "results": [{"path": "a.png", "name": "a.png", "fullText": "hello from image"}]
            }),
        );
        let text_node = flow
            .nodes
            .iter()
            .find(|node| node.id == "text_safety")
            .expect("text node");
        let mut cache = HashMap::new();
        let artifact_dir = test_root("ocr-output-artifact-dir");
        fs::create_dir_all(&artifact_dir).expect("create artifact dir");
        let inputs = data_inputs_for_node(
            &flow,
            &modules,
            &[],
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            text_node,
        )
        .expect("resolve inputs");
        let texts = inputs.get(HANDLE_TEXTS_IN).expect("texts input");
        assert_eq!(texts.data_type, DATA_TYPE_TEXTS);
        assert_eq!(
            texts.items[0].get("text").and_then(Value::as_str),
            Some("hello from image")
        );
        let _ = fs::remove_dir_all(artifact_dir);
    }

    #[test]
    fn declared_module_image_output_feeds_image_collection_input() {
        let flow = FlowDefinition {
            id: "flow.preview".to_string(),
            name: "Preview".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: "preview".to_string(),
                    module_id: "custom.ap.level-preview-images".to_string(),
                    label: "AP-关卡预览图".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "image_safety".to_string(),
                    module_id: "preset.custom.shieldgemma2".to_string(),
                    label: "图片合规检测".to_string(),
                    position: Position { x: 1.0, y: 0.0 },
                    config: json!({}),
                },
            ],
            edges: vec![data_edge(
                "preview_images",
                "preview",
                HANDLE_IMAGES_OUT,
                "image_safety",
                HANDLE_IMAGES_IN,
            )],
        };
        let mut modules = test_modules();
        let mut preview = test_module(
            "custom.ap.level-preview-images",
            "AP-关卡预览图",
            "folder_processor",
        );
        preview.data_outputs = vec![ModuleDataOutput {
            handle: HANDLE_IMAGES_OUT.to_string(),
            name: "图片".to_string(),
            data_type: DATA_TYPE_IMAGES.to_string(),
        }];
        modules.insert(preview.id.clone(), preview);
        let mut outputs = HashMap::new();
        outputs.insert(
            "preview".to_string(),
            json!({
                "outputs": {
                    "images": {
                        "dataType": "imageCollection",
                        "items": [{
                            "path": "D:/out/level/all_color.png",
                            "name": "all_color.png",
                            "extension": "png",
                            "sourceAssetId": "ap-level-preview",
                            "sourceAssetName": "AP-关卡预览图",
                            "relativePath": "关卡预览图/level/all_color.png"
                        }]
                    }
                }
            }),
        );
        let image_node = flow
            .nodes
            .iter()
            .find(|node| node.id == "image_safety")
            .expect("image node");
        let mut cache = HashMap::new();
        let artifact_dir = test_root("declared-image-output-artifact-dir");
        fs::create_dir_all(&artifact_dir).expect("create artifact dir");
        let inputs = data_inputs_for_node(
            &flow,
            &modules,
            &[],
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            image_node,
        )
        .expect("resolve inputs");
        let images = inputs.get(HANDLE_IMAGES_IN).expect("images input");
        assert_eq!(images.data_type, DATA_TYPE_IMAGES);
        assert_eq!(images.items.len(), 1);
        assert_eq!(
            images.items[0].get("name").and_then(Value::as_str),
            Some("all_color.png")
        );
        let _ = fs::remove_dir_all(artifact_dir);
    }

    #[test]
    fn artifact_data_nodes_read_current_artifact_folder() {
        let artifact_dir = test_root("artifact-data-node");
        let text_dir = artifact_dir.join("图文识别结果");
        fs::create_dir_all(&text_dir).expect("create artifact text dir");
        fs::write(text_dir.join("a.txt"), "hello artifact").expect("write artifact text");
        fs::write(artifact_dir.join("chart.png"), "fake image").expect("write artifact image");

        let files = collect_artifact_files(&artifact_dir).expect("collect artifact files");
        assert_eq!(
            files.iter().filter(|file| file.file_type == "text").count(),
            1
        );
        assert_eq!(
            files
                .iter()
                .filter(|file| file.file_type == "image")
                .count(),
            1
        );

        let texts =
            artifact_collection(&artifact_dir, DATA_TYPE_TEXTS).expect("artifact text collection");
        assert_eq!(texts.data_type, DATA_TYPE_TEXTS);
        assert_eq!(texts.items.len(), 1);
        assert_eq!(
            texts.items[0].get("text").and_then(Value::as_str),
            Some("hello artifact")
        );
        assert_eq!(
            texts.items[0].get("relativePath").and_then(Value::as_str),
            Some("图文识别结果\\a.txt")
        );

        let images = artifact_collection(&artifact_dir, DATA_TYPE_IMAGES)
            .expect("artifact image collection");
        assert_eq!(images.data_type, DATA_TYPE_IMAGES);
        assert_eq!(images.items.len(), 1);

        let _ = fs::remove_dir_all(artifact_dir);
    }

    #[test]
    fn folder_data_nodes_resolve_single_folders() {
        let root = test_root("folder-data-node");
        let audit_dir = root.join("input");
        let audit_subdir = audit_dir.join("Assets");
        let artifact_dir = root.join("artifacts");
        let artifact_subdir = artifact_dir.join("outputs");
        fs::create_dir_all(&audit_subdir).expect("create audit subdir");
        fs::create_dir_all(&artifact_subdir).expect("create artifact subdir");

        let flow = FlowDefinition {
            id: "flow.folder-test".to_string(),
            name: "folder test".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: "audit_folder".to_string(),
                    module_id: DATA_AUDIT_FOLDER_MODULE_ID.to_string(),
                    label: "待审核文件夹".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "audit_relative_folder".to_string(),
                    module_id: DATA_AUDIT_RELATIVE_FOLDER_MODULE_ID.to_string(),
                    label: "待审核文件夹下相对路径文件夹".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({ "relativePath": "Assets" }),
                },
                FlowNode {
                    id: "artifact_folder".to_string(),
                    module_id: DATA_ARTIFACT_FOLDER_MODULE_ID.to_string(),
                    label: "产物文件夹".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "artifact_relative_folder".to_string(),
                    module_id: DATA_ARTIFACT_RELATIVE_FOLDER_MODULE_ID.to_string(),
                    label: "待产物文件夹下相对路径文件夹".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({ "relativePath": "outputs" }),
                },
            ],
            edges: vec![],
        };
        let modules = test_modules();
        let assets = vec![test_asset(&audit_dir, "directory")];
        let outputs = HashMap::new();
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        let expected_audit_dir = audit_dir
            .canonicalize()
            .expect("canonical audit")
            .display()
            .to_string();
        let expected_audit_subdir = audit_subdir
            .canonicalize()
            .expect("canonical audit subdir")
            .display()
            .to_string();
        let expected_artifact_dir = artifact_dir
            .canonicalize()
            .expect("canonical artifact")
            .display()
            .to_string();
        let expected_artifact_subdir = artifact_subdir
            .canonicalize()
            .expect("canonical artifact subdir")
            .display()
            .to_string();

        let audit_folder = resolve_data_value(
            &flow,
            &modules,
            &assets,
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            &mut visiting,
            "audit_folder",
            HANDLE_FOLDER_OUT,
        )
        .expect("audit folder");
        assert_eq!(audit_folder.data_type, DATA_TYPE_FOLDER);
        assert!(audit_folder.items.is_empty());
        assert_eq!(audit_folder.path.as_deref(), Some(expected_audit_dir.as_str()));

        let audit_relative_folder = resolve_data_value(
            &flow,
            &modules,
            &assets,
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            &mut visiting,
            "audit_relative_folder",
            HANDLE_FOLDER_OUT,
        )
        .expect("audit relative folder");
        assert_eq!(audit_relative_folder.data_type, DATA_TYPE_FOLDER);
        assert_eq!(
            audit_relative_folder.path.as_deref(),
            Some(expected_audit_subdir.as_str())
        );
        assert_eq!(audit_relative_folder.relative_path.as_deref(), Some("Assets"));

        let artifact_folder = resolve_data_value(
            &flow,
            &modules,
            &assets,
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            &mut visiting,
            "artifact_folder",
            HANDLE_FOLDER_OUT,
        )
        .expect("artifact folder");
        assert_eq!(artifact_folder.data_type, DATA_TYPE_FOLDER);
        assert_eq!(
            artifact_folder.path.as_deref(),
            Some(expected_artifact_dir.as_str())
        );

        let artifact_relative_folder = resolve_data_value(
            &flow,
            &modules,
            &assets,
            &[],
            &artifact_dir,
            &outputs,
            &mut cache,
            &mut visiting,
            "artifact_relative_folder",
            HANDLE_FOLDER_OUT,
        )
        .expect("artifact relative folder");
        assert_eq!(artifact_relative_folder.data_type, DATA_TYPE_FOLDER);
        assert_eq!(
            artifact_relative_folder.path.as_deref(),
            Some(expected_artifact_subdir.as_str())
        );
        assert_eq!(artifact_relative_folder.relative_path.as_deref(), Some("outputs"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn module_data_output_can_flow_through_data_node_without_sequence_to_data_node() {
        let flow = FlowDefinition {
            id: "flow.merge-text".to_string(),
            name: "Merge Text".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: START_NODE_ID.to_string(),
                    module_id: START_MODULE_ID.to_string(),
                    label: "开始".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "all_images".to_string(),
                    module_id: DATA_ALL_IMAGES_MODULE_ID.to_string(),
                    label: "待测项目中所有图片".to_string(),
                    position: Position { x: 0.0, y: 1.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "all_texts".to_string(),
                    module_id: DATA_ALL_TEXTS_MODULE_ID.to_string(),
                    label: "待测项目中所有文本".to_string(),
                    position: Position { x: 1.0, y: -1.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "image_ocr".to_string(),
                    module_id: "preset.custom.paddleocr".to_string(),
                    label: "图片文字识别".to_string(),
                    position: Position { x: 1.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "merge_texts".to_string(),
                    module_id: DATA_MERGE_TEXTS_MODULE_ID.to_string(),
                    label: "将两个文本集合合并".to_string(),
                    position: Position { x: 2.0, y: -1.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "text_safety".to_string(),
                    module_id: "preset.custom.qwen3guard".to_string(),
                    label: "文本合规检测".to_string(),
                    position: Position { x: 3.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: OUTPUT_NODE_ID.to_string(),
                    module_id: OUTPUT_MODULE_ID.to_string(),
                    label: "输出结果".to_string(),
                    position: Position { x: 4.0, y: 0.0 },
                    config: json!({}),
                },
            ],
            edges: vec![
                sequence_edge("start_ocr", START_NODE_ID, "image_ocr"),
                sequence_edge("ocr_text", "image_ocr", "text_safety"),
                sequence_edge("text_output", "text_safety", OUTPUT_NODE_ID),
                data_edge(
                    "images_to_ocr",
                    "all_images",
                    HANDLE_IMAGES_OUT,
                    "image_ocr",
                    HANDLE_IMAGES_IN,
                ),
                data_edge(
                    "texts_to_merge",
                    "all_texts",
                    HANDLE_TEXTS_OUT,
                    "merge_texts",
                    HANDLE_TEXTS_A_IN,
                ),
                data_edge(
                    "ocr_to_merge",
                    "image_ocr",
                    HANDLE_TEXTS_OUT,
                    "merge_texts",
                    HANDLE_TEXTS_B_IN,
                ),
                data_edge(
                    "merge_to_text",
                    "merge_texts",
                    HANDLE_TEXTS_OUT,
                    "text_safety",
                    HANDLE_TEXTS_IN,
                ),
            ],
        };

        let result = validate_flow_inner(&flow, &test_modules());
        assert!(result.valid, "{:?}", result.messages);
    }

    #[test]
    fn output_waits_for_parallel_branches() {
        let flow = default_flow();
        let ordered = topological_order(&flow).expect("ordered");
        let groups: HashMap<String, usize> = ordered
            .into_iter()
            .map(|(node, group)| (node.id, group))
            .collect();
        assert!(groups[OUTPUT_NODE_ID] > groups["image_safety"]);
        assert!(groups[OUTPUT_NODE_ID] > groups["text_safety"]);
    }

    #[test]
    fn execution_plan_adds_module_data_dependency_without_sequence_edge() {
        let flow = FlowDefinition {
            id: "flow.data-dependency".to_string(),
            name: "Data Dependency".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: START_NODE_ID.to_string(),
                    module_id: START_MODULE_ID.to_string(),
                    label: "开始".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "all_images".to_string(),
                    module_id: DATA_ALL_IMAGES_MODULE_ID.to_string(),
                    label: "待测项目中所有图片".to_string(),
                    position: Position { x: 0.0, y: 1.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "image_ocr".to_string(),
                    module_id: "preset.custom.paddleocr".to_string(),
                    label: "图片文字识别".to_string(),
                    position: Position { x: 1.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: "text_safety".to_string(),
                    module_id: "preset.custom.qwen3guard".to_string(),
                    label: "文本合规检测".to_string(),
                    position: Position { x: 2.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: OUTPUT_NODE_ID.to_string(),
                    module_id: OUTPUT_MODULE_ID.to_string(),
                    label: "输出结果".to_string(),
                    position: Position { x: 3.0, y: 0.0 },
                    config: json!({}),
                },
            ],
            edges: vec![
                sequence_edge("start_ocr", START_NODE_ID, "image_ocr"),
                sequence_edge("start_text", START_NODE_ID, "text_safety"),
                sequence_edge("text_output", "text_safety", OUTPUT_NODE_ID),
                data_edge(
                    "images_to_ocr",
                    "all_images",
                    HANDLE_IMAGES_OUT,
                    "image_ocr",
                    HANDLE_IMAGES_IN,
                ),
                data_edge(
                    "ocr_to_text",
                    "image_ocr",
                    HANDLE_TEXTS_OUT,
                    "text_safety",
                    HANDLE_TEXTS_IN,
                ),
            ],
        };
        let modules = test_modules();
        let result = validate_flow_inner(&flow, &modules);
        assert!(result.valid, "{:?}", result.messages);
        let plan = execution_plan(&flow, &modules).expect("execution plan");
        assert!(plan
            .dependencies
            .get("text_safety")
            .expect("text dependencies")
            .contains("image_ocr"));
    }

    #[test]
    fn execution_plan_keeps_multiple_sequence_inputs_waiting() {
        let flow = default_flow();
        let plan = execution_plan(&flow, &test_modules()).expect("execution plan");
        let output_deps = plan
            .dependencies
            .get(OUTPUT_NODE_ID)
            .expect("output dependencies");
        assert!(output_deps.contains("image_safety"));
        assert!(output_deps.contains("text_safety"));
    }

    #[test]
    fn run_control_can_be_cancelled() {
        let control = RunControl::default();
        assert!(!control.is_cancelled());
        control.cancel();
        assert!(control.is_cancelled());
    }

    #[test]
    fn progress_value_reads_valid_counts() {
        let value = json!({"processed": 3, "total": 8});
        assert_eq!(progress_value(&value, "processed"), Some(3));
        assert_eq!(progress_value(&value, "total"), Some(8));
        assert_eq!(progress_value(&value, "missing"), None);
    }

    #[test]
    fn recursively_collects_image_text_and_other_files() {
        let root = test_root("collect-files");
        let assets_dir = root.join("assets");
        let nested = assets_dir.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(assets_dir.join("safe.txt"), "hello").expect("write text");
        fs::write(nested.join("image.png"), "not really an image").expect("write image");
        fs::write(nested.join("ignore.bin"), "binary").expect("write other");

        let files = collect_audit_files(&[test_asset(&assets_dir, "directory")])
            .expect("collect audit files");
        assert_eq!(files.len(), 3);
        assert_eq!(
            files
                .iter()
                .filter(|file| file.file_type == "image")
                .count(),
            1
        );
        assert_eq!(
            files.iter().filter(|file| file.file_type == "text").count(),
            1
        );
        assert_eq!(
            files
                .iter()
                .filter(|file| file.file_type == "other")
                .count(),
            1
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn uses_model_folder_as_portable_default_model_path() {
        let root = test_root("model-folder");
        let source = root.join("portable-module");
        fs::create_dir_all(source.join("Model")).expect("create model folder");
        fs::write(
            source.join("module.json"),
            r#"{
  "id": "custom.portable",
  "name": "Portable Module",
  "kind": "text_safety",
  "summary": "Portable module",
  "icon": "file-check",
  "parameters": [
    {
      "key": "modelPath",
      "name": "Model",
      "description": "Model folder",
      "parameterType": "path",
      "defaultValue": "",
      "required": true,
      "options": []
    }
  ],
  "launch": {
    "launchType": "python",
    "command": "main.py",
    "args": ["--input", "{inputJson}", "--output", "{outputJson}"],
    "notes": "Portable"
  }
}"#,
        )
        .expect("write module definition");

        let module = load_module_from_folder(&source, true).expect("load module");
        let model_path = module.model_path.as_deref().expect("model path");
        assert!(model_path.ends_with("Model"));
        let config = module_default_config(&module);
        assert_eq!(
            config.get("modelPath").and_then(Value::as_str),
            Some(model_path)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn upgrades_minimal_flow_to_real_default_flow() {
        let minimal = FlowDefinition {
            id: "flow.default.image-audit".to_string(),
            name: "图片 UGC 默认审核".to_string(),
            version: 1,
            nodes: vec![
                FlowNode {
                    id: START_NODE_ID.to_string(),
                    module_id: START_MODULE_ID.to_string(),
                    label: "开始".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    config: json!({}),
                },
                FlowNode {
                    id: OUTPUT_NODE_ID.to_string(),
                    module_id: OUTPUT_MODULE_ID.to_string(),
                    label: "输出结果".to_string(),
                    position: Position { x: 1.0, y: 0.0 },
                    config: json!({}),
                },
            ],
            edges: vec![sequence_edge(
                "edge_flow_start_output",
                START_NODE_ID,
                OUTPUT_NODE_ID,
            )],
        };
        let modules = HashMap::new();
        let upgraded = normalize_flow(minimal, &modules);
        assert!(upgraded
            .nodes
            .iter()
            .any(|node| node.module_id == "preset.custom.paddleocr"));
        assert!(upgraded
            .nodes
            .iter()
            .any(|node| node.module_id == "preset.custom.qwen3guard"));
        assert!(upgraded
            .nodes
            .iter()
            .any(|node| node.module_id == "preset.custom.shieldgemma2"));
    }

    #[test]
    fn upgrades_legacy_default_flow_with_data_edges() {
        let mut legacy = default_flow();
        legacy
            .nodes
            .retain(|node| node.module_id != DATA_ALL_IMAGES_MODULE_ID);
        legacy.edges.retain(|edge| edge.edge_type != EDGE_TYPE_DATA);
        let upgraded = normalize_flow(legacy, &test_modules());
        assert!(upgraded
            .edges
            .iter()
            .any(|edge| edge.edge_type == EDGE_TYPE_DATA));
        assert!(validate_flow_inner(&upgraded, &test_modules()).valid);
    }

    #[test]
    fn module_execution_receives_previous_outputs() {
        let root = test_root("module-exec");
        let module_dir = root.join("module");
        let step_dir = root.join("step");
        let resource_dir = root.join("resources");
        let artifact_dir = root.join("artifacts");
        fs::create_dir_all(&module_dir).expect("create module");
        fs::create_dir_all(&step_dir).expect("create step");
        fs::create_dir_all(&resource_dir).expect("create resources");
        fs::write(
            module_dir.join("runner.ps1"),
            r####"param([string]$InputPath, [string]$OutputPath)
$inputData = Get-Content -LiteralPath $InputPath -Raw | ConvertFrom-Json
$hasPrevious = $false
if ($null -ne $inputData.previous -and $inputData.previous.PSObject.Properties.Count -gt 0) {
  $hasPrevious = $true
}
$processed = @($inputData.files).Count
$matched = if ($hasPrevious) { 1 } else { 0 }
$result = [ordered]@{
  status = "completed"
  verdict = "pass"
  message = "dummy"
  processedFiles = $processed
  matchedFiles = $matched
  artifactCount = 0
  reportSection = "### Dummy"
}
$result | ConvertTo-Json -Depth 8 | Out-File -LiteralPath $OutputPath -Encoding ascii
"####,
        )
        .expect("write runner");

        let module = ModuleInfo {
            id: "custom.dummy".to_string(),
            name: "Dummy".to_string(),
            kind: "text_safety".to_string(),
            summary: String::new(),
            model_label: "Model".to_string(),
            icon: "file-check".to_string(),
            built_in: false,
            source: "custom".to_string(),
            definition_dir: module_dir.display().to_string(),
            icon_path: None,
            icon_data_url: None,
            model_path: None,
            model_configured: true,
            launch: ModuleLaunch {
                launch_type: "exe".to_string(),
                command: Some(
                    "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".to_string(),
                ),
                url: None,
                method: None,
                args: vec![
                    "-NoProfile".to_string(),
                    "-ExecutionPolicy".to_string(),
                    "Bypass".to_string(),
                    "-File".to_string(),
                    "runner.ps1".to_string(),
                    "-InputPath".to_string(),
                    "{inputJson}".to_string(),
                    "-OutputPath".to_string(),
                    "{outputJson}".to_string(),
                ],
                notes: String::new(),
            },
            parameters: Vec::new(),
            data_outputs: Vec::new(),
        };
        let node = FlowNode {
            id: "dummy_step".to_string(),
            module_id: "custom.dummy".to_string(),
            label: "Dummy".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            config: json!({}),
        };
        let files = vec![AuditFile {
            path: root.join("a.txt").display().to_string(),
            name: "a.txt".to_string(),
            extension: "txt".to_string(),
            file_type: "text".to_string(),
            source_asset_id: "asset".to_string(),
            source_asset_name: "asset".to_string(),
            relative_path: "a.txt".to_string(),
        }];
        let mut previous = HashMap::new();
        previous.insert(
            "ocr".to_string(),
            json!({"outputs": {"fullText": "OCR text"}}),
        );

        let (step, output) = execute_module_step(
            None,
            &module,
            &node,
            1,
            "run_test",
            "测试任务",
            &resource_dir,
            &artifact_dir,
            &step_dir,
            &files,
            &HashMap::new(),
            &previous,
        )
        .expect("execute module");
        assert_eq!(step.processed_files, 1);
        assert_eq!(step.matched_files, 1);
        assert_eq!(output.get("matchedFiles").and_then(Value::as_u64), Some(1));
        let input_value: Value =
            read_required_json(&step_dir.join("input.json")).expect("read module input");
        let expected_artifact_dir = artifact_dir.display().to_string();
        let expected_step_artifact_dir = step_dir.join("artifacts").display().to_string();
        assert_eq!(
            input_value
                .get("artifactDir")
                .and_then(Value::as_str)
                .unwrap_or(""),
            expected_artifact_dir
        );
        assert_eq!(
            input_value
                .get("stepArtifactDir")
                .and_then(Value::as_str)
                .unwrap_or(""),
            expected_step_artifact_dir
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn module_execution_keeps_valid_output_when_process_exit_is_nonzero() {
        let root = test_root("module-exit");
        let module_dir = root.join("module");
        let step_dir = root.join("step");
        let resource_dir = root.join("resources");
        let artifact_dir = root.join("artifacts");
        fs::create_dir_all(&module_dir).expect("create module");
        fs::create_dir_all(&step_dir).expect("create step");
        fs::create_dir_all(&resource_dir).expect("create resources");
        fs::write(
            module_dir.join("runner.ps1"),
            r####"param([string]$InputPath, [string]$OutputPath)
$result = [ordered]@{
  status = "completed"
  verdict = "pass"
  message = "wrote result before nonzero exit"
  processedFiles = 2
  matchedFiles = 1
  artifactCount = 0
  reportSection = "### Dummy"
}
$result | ConvertTo-Json -Depth 8 | Out-File -LiteralPath $OutputPath -Encoding ascii
exit 7
"####,
        )
        .expect("write runner");

        let module = ModuleInfo {
            id: "custom.dummy".to_string(),
            name: "Dummy".to_string(),
            kind: "text_safety".to_string(),
            summary: String::new(),
            model_label: "Model".to_string(),
            icon: "file-check".to_string(),
            built_in: false,
            source: "custom".to_string(),
            definition_dir: module_dir.display().to_string(),
            icon_path: None,
            icon_data_url: None,
            model_path: None,
            model_configured: true,
            launch: ModuleLaunch {
                launch_type: "exe".to_string(),
                command: Some(
                    "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".to_string(),
                ),
                url: None,
                method: None,
                args: vec![
                    "-NoProfile".to_string(),
                    "-ExecutionPolicy".to_string(),
                    "Bypass".to_string(),
                    "-File".to_string(),
                    "runner.ps1".to_string(),
                    "-InputPath".to_string(),
                    "{inputJson}".to_string(),
                    "-OutputPath".to_string(),
                    "{outputJson}".to_string(),
                ],
                notes: String::new(),
            },
            parameters: Vec::new(),
            data_outputs: Vec::new(),
        };
        let node = FlowNode {
            id: "dummy_step".to_string(),
            module_id: "custom.dummy".to_string(),
            label: "Dummy".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            config: json!({}),
        };

        let (step, output) = execute_module_step(
            None,
            &module,
            &node,
            1,
            "run_test",
            "测试任务",
            &resource_dir,
            &artifact_dir,
            &step_dir,
            &[],
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("execute module");
        assert_eq!(step.status, "completed");
        assert_eq!(step.verdict, "pass");
        assert_eq!(step.processed_files, 2);
        assert_eq!(
            output.get("status").and_then(Value::as_str),
            Some("completed")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn imports_folder_with_module_json_and_icon_file() {
        let root = test_root("import");
        let source = root.join("source-module");
        fs::create_dir_all(&source).expect("create source module");
        fs::write(
            source.join("icon.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"></svg>"#,
        )
        .expect("write icon");
        fs::create_dir_all(source.join("Model")).expect("create model folder");
        fs::write(
            source.join("Model").join("weights.bin"),
            "large model placeholder",
        )
        .expect("write model placeholder");
        fs::write(
            source.join("module.json"),
            r#"{
  "id": "custom.demo",
  "name": "Demo Module",
  "kind": "demo",
  "summary": "Demo module",
  "modelLabel": "Demo model",
  "icon": "icon.svg",
  "launch": {
    "launchType": "manual",
    "notes": "Manual"
  },
  "parameters": []
}"#,
        )
        .expect("write module definition");

        let module_id = import_module_folder_inner(&root, &source).expect("import module");
        assert_eq!(module_id, "custom.demo");

        let modules = load_modules(&root, &HashMap::new()).expect("load modules");
        let imported = modules
            .iter()
            .find(|module| module.id == "custom.demo")
            .expect("imported module exists");
        assert_eq!(imported.source, "custom");
        assert!(!imported.built_in);
        assert_eq!(
            PathBuf::from(&imported.definition_dir)
                .canonicalize()
                .expect("canonical imported dir"),
            source.canonicalize().expect("canonical source dir")
        );
        assert!(imported
            .icon_path
            .as_deref()
            .unwrap_or("")
            .ends_with("icon.svg"));
        assert!(imported
            .icon_data_url
            .as_deref()
            .unwrap_or("")
            .starts_with("data:image/svg+xml;base64,"));
        assert!(!root
            .join("modules")
            .join("custom.demo")
            .join("Model")
            .exists());
        assert!(source.join("Model").join("weights.bin").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hides_legacy_preset_but_allows_importing_same_module_id() {
        let root = test_root("legacy-preset");
        let legacy_folder = root.join("modules").join("preset.custom.paddleocr");
        fs::create_dir_all(&legacy_folder).expect("create legacy module");
        fs::write(
            legacy_folder.join("module.json"),
            r#"{
  "id": "preset.custom.paddleocr",
  "name": "Legacy OCR",
  "kind": "image_ocr",
  "summary": "Legacy preset",
  "icon": "scan-text",
  "builtIn": true,
  "source": "preset",
  "parameters": []
}"#,
        )
        .expect("write legacy module definition");

        let modules = load_modules(&root, &HashMap::new()).expect("load modules");
        assert!(!modules
            .iter()
            .any(|module| module.id == "preset.custom.paddleocr"));

        let source = root.join("source-module");
        fs::create_dir_all(&source).expect("create source module");
        fs::write(
            source.join("module.json"),
            r#"{
  "id": "preset.custom.paddleocr",
  "name": "Imported OCR",
  "kind": "image_ocr",
  "summary": "Imported custom module",
  "icon": "scan-text",
  "parameters": []
}"#,
        )
        .expect("write module definition");

        let module_id = import_module_folder_inner(&root, &source).expect("import module");
        assert_eq!(module_id, "preset.custom.paddleocr");

        let modules = load_modules(&root, &HashMap::new()).expect("load modules");
        let imported = modules
            .iter()
            .find(|module| module.id == "preset.custom.paddleocr")
            .expect("imported module exists");
        assert_eq!(imported.name, "Imported OCR");
        assert_eq!(imported.source, "custom");
        assert!(!imported.built_in);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_folder_without_module_json() {
        let root = test_root("reject");
        let source = root.join("not-a-module");
        fs::create_dir_all(&source).expect("create source folder");

        let error =
            import_module_folder_inner(&root, &source).expect_err("reject missing definition");
        assert!(error.contains("module.json"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unregisters_imported_module_without_deleting_source_folder() {
        let root = test_root("remove");
        let source = root.join("source-module");
        fs::create_dir_all(&source).expect("create source module");
        fs::write(source.join("icon.svg"), "<svg></svg>").expect("write icon");
        fs::write(
            source.join("module.json"),
            r#"{
  "id": "custom.remove",
  "name": "Remove Module",
  "kind": "remove",
  "summary": "Remove module",
  "icon": "icon.svg",
  "parameters": []
}"#,
        )
        .expect("write module definition");

        import_module_folder_inner(&root, &source).expect("import module");
        let imported_folder = root.join("modules").join("custom.remove");
        assert!(!imported_folder.exists());

        let removed_module_id = remove_module_inner(&root, "custom.remove").expect("remove module");
        assert_eq!(removed_module_id, "custom.remove");
        assert!(!imported_folder.exists());
        assert!(source.exists());
        let modules = load_modules(&root, &HashMap::new()).expect("load modules");
        assert!(!modules.iter().any(|module| module.id == "custom.remove"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_remove_module_used_by_flow() {
        let root = test_root("remove-used");
        let source = root.join("source-module");
        fs::create_dir_all(&source).expect("create source module");
        fs::write(source.join("icon.svg"), "<svg></svg>").expect("write icon");
        fs::write(
            source.join("module.json"),
            r#"{
  "id": "custom.used",
  "name": "Used Module",
  "kind": "used",
  "summary": "Used module",
  "icon": "icon.svg",
  "parameters": []
}"#,
        )
        .expect("write module definition");
        import_module_folder_inner(&root, &source).expect("import module");

        let flow = FlowDefinition {
            id: "flow.test".to_string(),
            name: "Test Flow".to_string(),
            version: 1,
            nodes: vec![FlowNode {
                id: "node_custom_used".to_string(),
                module_id: "custom.used".to_string(),
                label: "Used".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                config: json!({}),
            }],
            edges: vec![],
        };
        write_json(&default_flow_file(&root), &flow).expect("write flow");

        let error = remove_module_inner(&root, "custom.used").expect_err("reject used module");
        assert!(error.contains("当前流程"));

        let _ = fs::remove_dir_all(root);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(RunRegistry::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if let Some(window_config) = app.config().app.windows.first() {
                tauri::WebviewWindowBuilder::from_config(app.handle(), window_config)?.build()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_data_root,
            get_app_settings,
            save_app_settings,
            list_modules,
            get_model_paths,
            save_model_path,
            import_module_folder,
            remove_module,
            open_module_definition_folder,
            reveal_report_target,
            get_runtime_status,
            install_runtime_dependency,
            open_runtime_dependency_folder,
            open_runtime_python_folder,
            load_flow,
            save_flow,
            load_scheme_file,
            save_scheme_file,
            get_scheme_library_dir,
            list_scheme_files,
            delete_scheme_file,
            save_scheme_to_library,
            validate_flow,
            start_run,
            start_run_live,
            cancel_run,
            list_runs,
            delete_run,
            delete_all_runs,
            read_run_record,
            read_run_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
