use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowType {
    RunPlayModeTests,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowState {
    Queued,
    Running,
    WaitingReconnect,
    Succeeded,
    Failed,
    Cancelled,
}

impl FlowState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            FlowState::Succeeded | FlowState::Failed | FlowState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTaskRecord {
    pub task_id: String,
    pub flow_type: FlowType,
    pub project_path: String,
    pub state: FlowState,
    pub step_index: u32,
    pub updated_at_ms: u64,
    pub created_at_ms: u64,
    pub last_error: Option<String>,
    pub detail: Option<String>,
    #[serde(default)]
    pub stats: Option<FlowTaskStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTaskStats {
    pub passed: i64,
    pub failed: i64,
    pub skipped: i64,
}

#[derive(Debug, Clone)]
pub struct FlowStartResult {
    pub task_id: String,
    pub state: FlowState,
}

#[derive(Debug)]
pub struct FlowStore {
    project_path: String,
    storage_dir: PathBuf,
    tasks: Mutex<HashMap<String, FlowTaskRecord>>,
}

static FLOW_STORES: OnceLock<Mutex<HashMap<String, Arc<FlowStore>>>> = OnceLock::new();

fn stores() -> &'static Mutex<HashMap<String, Arc<FlowStore>>> {
    FLOW_STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_project_path(project_path: &str) -> String {
    project_path.trim().replace('\\', "/")
}

fn task_file_path(storage_dir: &Path, task_id: &str) -> PathBuf {
    storage_dir.join(format!("{task_id}.json"))
}

fn parse_named_count(summary: &str, key: &str) -> Option<i64> {
    let needle = format!("{key}=");
    for segment in summary.split(';') {
        let trimmed = segment.trim();
        if let Some(value) = trimmed.strip_prefix(&needle) {
            if let Ok(parsed) = value.trim().parse::<i64>() {
                return Some(parsed);
            }
        }
    }
    None
}

fn parse_flow_stats(summary: &str) -> Option<FlowTaskStats> {
    let passed = parse_named_count(summary, "passed")?;
    let failed = parse_named_count(summary, "failed")?;
    let skipped = parse_named_count(summary, "skipped").unwrap_or(0);
    Some(FlowTaskStats {
        passed,
        failed,
        skipped,
    })
}

impl FlowStore {
    fn new(project_path: &str) -> Result<Self, String> {
        let normalized = normalize_project_path(project_path);
        let storage_dir = Path::new(project_path)
            .join("Library")
            .join("Locus")
            .join("Flows");
        std::fs::create_dir_all(&storage_dir).map_err(|error| {
            format!(
                "Failed to create Unity flow storage dir '{}': {}",
                storage_dir.display(),
                error
            )
        })?;

        let mut tasks = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|v| v.to_str()) != Some("json") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(record) = serde_json::from_str::<FlowTaskRecord>(&content) else {
                    continue;
                };
                tasks.insert(record.task_id.clone(), record);
            }
        }

        Ok(Self {
            project_path: normalized,
            storage_dir,
            tasks: Mutex::new(tasks),
        })
    }

    fn persist_record(&self, record: &FlowTaskRecord) -> Result<(), String> {
        let path = task_file_path(&self.storage_dir, &record.task_id);
        let content = serde_json::to_string_pretty(record).map_err(|error| {
            format!(
                "Failed to serialize Unity flow task '{}': {}",
                record.task_id, error
            )
        })?;
        std::fs::write(&path, content).map_err(|error| {
            format!(
                "Failed to write Unity flow task '{}' to '{}': {}",
                record.task_id,
                path.display(),
                error
            )
        })
    }

    async fn upsert_record(&self, record: FlowTaskRecord) -> Result<(), String> {
        self.persist_record(&record)?;
        let mut tasks = self.tasks.lock().await;
        tasks.insert(record.task_id.clone(), record);
        Ok(())
    }

    async fn get_record(&self, task_id: &str) -> Option<FlowTaskRecord> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).cloned()
    }
}

pub async fn get_or_create_store(project_path: &str) -> Result<Arc<FlowStore>, String> {
    let normalized = normalize_project_path(project_path);
    {
        let map = stores().lock().await;
        if let Some(existing) = map.get(&normalized) {
            return Ok(existing.clone());
        }
    }

    let store = Arc::new(FlowStore::new(project_path)?);
    let mut map = stores().lock().await;
    if let Some(existing) = map.get(&normalized) {
        return Ok(existing.clone());
    }
    map.insert(normalized, store.clone());
    Ok(store)
}

async fn update_task_state(
    store: &Arc<FlowStore>,
    task_id: &str,
    state: FlowState,
    step_index: u32,
    detail: Option<String>,
    last_error: Option<String>,
) -> Result<(), String> {
    let mut record = match store.get_record(task_id).await {
        Some(value) => value,
        None => return Err(format!("Unity flow task '{}' not found", task_id)),
    };
    record.state = state;
    record.step_index = step_index;
    record.updated_at_ms = now_ms();
    record.detail = detail;
    record.last_error = last_error;
    record.stats = None;
    store.upsert_record(record).await
}

async fn is_task_cancelled(store: &Arc<FlowStore>, task_id: &str) -> bool {
    match store.get_record(task_id).await {
        Some(record) => record.state == FlowState::Cancelled,
        None => false,
    }
}

async fn run_playmode_tests_flow(store: Arc<FlowStore>, task_id: String) {
    if is_task_cancelled(&store, &task_id).await {
        return;
    }

    if let Err(error) = update_task_state(
        &store,
        &task_id,
        FlowState::Running,
        1,
        Some("Checking Unity connection".to_string()),
        None,
    )
    .await
    {
        eprintln!("[Locus] unity_flow failed to mark running: {}", error);
        return;
    }

    let (connected, _status, _) = crate::unity_bridge::query_unity_status(&store.project_path).await;
    if !connected {
        let _ = update_task_state(
            &store,
            &task_id,
            FlowState::WaitingReconnect,
            1,
            Some("Unity disconnected, waiting for reconnect".to_string()),
            None,
        )
        .await;

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(45);
        loop {
            if is_task_cancelled(&store, &task_id).await {
                return;
            }
            if start.elapsed() > timeout {
                let _ = update_task_state(
                    &store,
                    &task_id,
                    FlowState::Failed,
                    1,
                    Some("Reconnect timeout".to_string()),
                    Some("Unity did not reconnect within 45s".to_string()),
                )
                .await;
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let (ok, _, _) = crate::unity_bridge::query_unity_status(&store.project_path).await;
            if ok {
                break;
            }
        }
    }

    let _ = update_task_state(
        &store,
        &task_id,
        FlowState::Running,
        2,
        Some("Pre-compile before test run".to_string()),
        None,
    )
    .await;

    if is_task_cancelled(&store, &task_id).await {
        return;
    }

    if let Err(error) = crate::unity_bridge::recompile_and_wait(&store.project_path).await {
        let _ = update_task_state(
            &store,
            &task_id,
            FlowState::Failed,
            2,
            Some("Recompile failed".to_string()),
            Some(error),
        )
        .await;
        return;
    }

    let _ = update_task_state(
        &store,
        &task_id,
        FlowState::Running,
        3,
        Some("Starting Unity PlayMode tests".to_string()),
        None,
    )
    .await;

    if is_task_cancelled(&store, &task_id).await {
        let _ = crate::unity_bridge::playmode_tests_cancel(&store.project_path).await;
        return;
    }

    if let Err(error) = crate::unity_bridge::playmode_tests_start(&store.project_path).await {
        let _ = update_task_state(
            &store,
            &task_id,
            FlowState::Failed,
            3,
            Some("Failed to start PlayMode tests".to_string()),
            Some(error),
        )
        .await;
        return;
    }

    let _ = update_task_state(
        &store,
        &task_id,
        FlowState::Running,
        4,
        Some("Waiting PlayMode test result".to_string()),
        None,
    )
    .await;

    let started_at = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(1800);
    loop {
        if is_task_cancelled(&store, &task_id).await {
            let _ = crate::unity_bridge::playmode_tests_cancel(&store.project_path).await;
            return;
        }
        if started_at.elapsed() > timeout {
            let _ = update_task_state(
                &store,
                &task_id,
                FlowState::Failed,
                4,
                Some("PlayMode test timeout".to_string()),
                Some("PlayMode tests did not finish within 1800s".to_string()),
            )
            .await;
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        match crate::unity_bridge::playmode_tests_status(&store.project_path).await {
            Ok(status) => {
                let status_state = status.state.trim().to_ascii_lowercase();
                if status_state == "running" {
                    continue;
                }
                if status_state == "finished" {
                    let parsed_stats = parse_flow_stats(&status.summary);
                    let has_failure = parsed_stats.as_ref().map(|value| value.failed > 0).unwrap_or(false);
                    if has_failure {
                        let mut record = match store.get_record(&task_id).await {
                            Some(value) => value,
                            None => return,
                        };
                        record.state = FlowState::Failed;
                        record.step_index = 5;
                        record.updated_at_ms = now_ms();
                        record.detail = Some("PlayMode tests completed with failures".to_string());
                        record.last_error = Some(status.summary);
                        record.stats = parsed_stats;
                        let _ = store.upsert_record(record).await;
                        return;
                    }
                    let detail = if status.summary.trim().is_empty() {
                        "PlayMode tests finished".to_string()
                    } else {
                        format!("PlayMode tests finished: {}", status.summary.trim())
                    };
                    let mut record = match store.get_record(&task_id).await {
                        Some(value) => value,
                        None => return,
                    };
                    record.state = FlowState::Succeeded;
                    record.step_index = 5;
                    record.updated_at_ms = now_ms();
                    record.detail = Some(detail);
                    record.last_error = None;
                    record.stats = parsed_stats;
                    let _ = store.upsert_record(record).await;
                    return;
                }
                if status_state == "failed" {
                    let mut record = match store.get_record(&task_id).await {
                        Some(value) => value,
                        None => return,
                    };
                    record.state = FlowState::Failed;
                    record.step_index = 5;
                    record.updated_at_ms = now_ms();
                    record.detail = Some("PlayMode tests failed".to_string());
                    record.last_error = Some(if status.error.trim().is_empty() {
                        status.summary
                    } else {
                        status.error
                    });
                    record.stats = parse_flow_stats(&record.last_error.clone().unwrap_or_default());
                    let _ = store.upsert_record(record).await;
                    return;
                }
                if status_state == "cancelled" {
                    let _ = update_task_state(
                        &store,
                        &task_id,
                        FlowState::Cancelled,
                        5,
                        Some("PlayMode tests cancelled".to_string()),
                        None,
                    )
                    .await;
                    return;
                }
            }
            Err(error) => {
                let (connected, _, _) = crate::unity_bridge::query_unity_status(&store.project_path).await;
                if !connected {
                    let _ = update_task_state(
                        &store,
                        &task_id,
                        FlowState::WaitingReconnect,
                        4,
                        Some("Unity disconnected while polling test status".to_string()),
                        None,
                    )
                    .await;
                    continue;
                }
                let _ = update_task_state(
                    &store,
                    &task_id,
                    FlowState::Failed,
                    4,
                    Some("Failed to poll PlayMode test status".to_string()),
                    Some(error),
                )
                .await;
                return;
            }
        }
    }

}

pub async fn start_run_playmode_tests(project_path: &str) -> Result<FlowStartResult, String> {
    let store = get_or_create_store(project_path).await?;
    let task_id = format!("flow-{}", uuid::Uuid::new_v4());
    let created_at = now_ms();
    let record = FlowTaskRecord {
        task_id: task_id.clone(),
        flow_type: FlowType::RunPlayModeTests,
        project_path: store.project_path.clone(),
        state: FlowState::Queued,
        step_index: 0,
        updated_at_ms: created_at,
        created_at_ms: created_at,
        last_error: None,
        detail: Some("Task created".to_string()),
        stats: None,
    };
    store.upsert_record(record).await?;

    let store_for_task = store.clone();
    let task_id_for_task = task_id.clone();
    tokio::spawn(async move {
        run_playmode_tests_flow(store_for_task, task_id_for_task).await;
    });

    Ok(FlowStartResult {
        task_id,
        state: FlowState::Queued,
    })
}

pub async fn get_task(project_path: &str, task_id: &str) -> Result<FlowTaskRecord, String> {
    let store = get_or_create_store(project_path).await?;
    store
        .get_record(task_id)
        .await
        .ok_or_else(|| format!("Unity flow task '{}' not found", task_id))
}

pub async fn cancel_task(project_path: &str, task_id: &str) -> Result<FlowTaskRecord, String> {
    let store = get_or_create_store(project_path).await?;
    let mut record = store
        .get_record(task_id)
        .await
        .ok_or_else(|| format!("Unity flow task '{}' not found", task_id))?;
    if record.state.is_terminal() {
        return Ok(record);
    }
    record.state = FlowState::Cancelled;
    record.updated_at_ms = now_ms();
    record.detail = Some("Task cancelled by user".to_string());
    record.last_error = None;
    record.stats = None;
    store.upsert_record(record.clone()).await?;
    let _ = crate::unity_bridge::playmode_tests_cancel(project_path).await;
    Ok(record)
}

pub async fn wait_task(
    project_path: &str,
    task_id: &str,
    timeout: std::time::Duration,
) -> Result<FlowTaskRecord, String> {
    let store = get_or_create_store(project_path).await?;
    let start = std::time::Instant::now();
    loop {
        let record = store
            .get_record(task_id)
            .await
            .ok_or_else(|| format!("Unity flow task '{}' not found", task_id))?;
        if record.state.is_terminal() {
            return Ok(record);
        }
        if start.elapsed() > timeout {
            return Err(format!(
                "Timed out waiting for Unity flow task '{}' after {}s",
                task_id,
                timeout.as_secs()
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_state_serializes_as_snake_case() {
        let value = serde_json::to_string(&FlowState::WaitingReconnect).expect("serialize");
        assert_eq!(value, "\"waiting_reconnect\"");
    }

    #[test]
    fn terminal_state_detection_matches_contract() {
        assert!(FlowState::Succeeded.is_terminal());
        assert!(FlowState::Failed.is_terminal());
        assert!(FlowState::Cancelled.is_terminal());
        assert!(!FlowState::Queued.is_terminal());
        assert!(!FlowState::Running.is_terminal());
        assert!(!FlowState::WaitingReconnect.is_terminal());
    }

    #[test]
    fn task_file_path_uses_json_suffix() {
        let root = Path::new("C:/Project/Library/Locus/Flows");
        let path = task_file_path(root, "flow-123");
        assert!(path.ends_with("flow-123.json"));
    }

    #[test]
    fn parse_flow_stats_from_summary_line() {
        let stats = parse_flow_stats("state=Passed; passed=7; failed=1; skipped=2").unwrap();
        assert_eq!(stats.passed, 7);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.skipped, 2);
    }
}
