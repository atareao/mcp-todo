use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "undo_delete",
    description = "Restore a soft-deleted task from the trash. The task will be marked as active again.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct UndoDelete {
    /// Task ID (UUID) of the deleted task to restore
    id: String,
}

impl UndoDelete {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let task = operations::undo_delete(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = format!(
            "♻️ Task restored: {}\n\nID: {}\nStatus: {:?}\nProject: {}",
            task.summary,
            task.id,
            task.status,
            task.project.as_deref().unwrap_or("none"),
        );

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}
