use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "archive_task",
    description = "Archive a todo task by ID. Archived tasks are hidden from normal lists but can be restored.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ArchiveTask {
    /// Task ID (UUID)
    id: String,
}

impl ArchiveTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        operations::archive_task(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            "📦 Task archived. Use 'unarchive_task' to restore or 'list_archived' to view archived tasks.".to_string(),
        )]))
    }
}
