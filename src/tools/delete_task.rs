use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "delete_task",
    description = "Soft-delete a todo task by ID. The task moves to trash and can be restored with undo_delete.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct DeleteTask {
    /// Task ID (UUID)
    id: String,
}

impl DeleteTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        operations::delete_task(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            "🗑️ Task moved to trash. Use 'undo_delete' to restore or 'purge_deleted' to permanently delete.".to_string(),
        )]))
    }
}
