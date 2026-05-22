use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "complete_task",
    description = "Quickly mark a task as done by ID. Sets status to 'done' and records completion time.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct CompleteTask {
    /// Task ID (UUID)
    id: String,
}

impl CompleteTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let task = operations::complete_task(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = format!(
            "✅ Task completed: {}\n\nID: {}\nCompleted at: {}",
            task.summary,
            task.id,
            task.completed_at
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        );

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}
