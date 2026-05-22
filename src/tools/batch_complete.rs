use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations::{self, BatchCompleteParams};
use crate::AppState;

#[mcp_tool(
    name = "batch_complete",
    description = "Complete multiple tasks at once. Can complete by specific IDs or by filter (status, project, tags).",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct BatchComplete {
    /// List of task IDs to complete. If provided, filters are ignored.
    ids: Option<Vec<String>>,
    /// Filter by status to complete (e.g., 'todo', 'in_progress')
    status: Option<String>,
    /// Filter by project to complete all tasks in that project
    project: Option<String>,
    /// Filter by tags to complete tasks with any of these tags
    tags: Option<Vec<String>>,
}

impl BatchComplete {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        if self.ids.is_none() && self.status.is_none() && self.project.is_none() && self.tags.is_none() {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "❌ Must provide either 'ids' or at least one filter (status, project, tags).".to_string(),
            )]));
        }

        let params = BatchCompleteParams {
            ids: self.ids.clone(),
            status: self.status.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
        };

        let result = operations::batch_complete_tasks(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = if result.affected_count == 0 {
            "No tasks matched the criteria.".to_string()
        } else {
            let target = if self.ids.is_some() {
                format!("{} task(s) completed by ID", result.affected_count)
            } else {
                format!("{} task(s) completed by filter", result.affected_count)
            };

            let mut lines = vec![format!("✅ {}", target), String::new()];

            if result.affected_count <= 10 {
                for id in &result.affected_ids {
                    lines.push(format!("  - {}", id));
                }
            } else {
                lines.push(format!("(Showing first 10 of {} tasks)", result.affected_count));
                for id in result.affected_ids.iter().take(10) {
                    lines.push(format!("  - {}", id));
                }
            }

            lines.join("\n")
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}
