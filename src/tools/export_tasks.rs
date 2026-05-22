use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::export_import::{self, ExportParams};
use crate::AppState;

#[mcp_tool(
    name = "export_tasks",
    description = "Export tasks to JSON format for backup or migration. Supports filters to export specific subsets of tasks.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ExportTasks {
    /// Include soft-deleted tasks in export (default: false)
    include_deleted: Option<bool>,
    /// Filter by status (todo, in_progress, done)
    status: Option<String>,
    /// Filter by project name
    project: Option<String>,
    /// Filter by tags (tasks with any of these tags)
    tags: Option<Vec<String>>,
}

impl ExportTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let params = ExportParams {
            include_deleted: self.include_deleted.unwrap_or(false),
            status: self.status.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
        };

        let data = export_import::export_tasks(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| CallToolError::from_message(format!("Failed to serialize: {}", e)))?;

        let task_count = data.tasks.len();
        let deleted_info = if self.include_deleted.unwrap_or(false) {
            " (including deleted)"
        } else {
            ""
        };

        let response = format!(
            "📦 Exported {} task(s){}:\n\n```json\n{}\n```",
            task_count, deleted_info, json
        );

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}
