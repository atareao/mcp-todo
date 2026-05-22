use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "unarchive_task",
    description = "Restore an archived todo task by ID. The task returns to normal active lists.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct UnarchiveTask {
    /// Task ID (UUID)
    id: String,
}

impl UnarchiveTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        operations::unarchive_task(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            "♻️ Task restored from archive. It is now visible in normal task lists.".to_string(),
        )]))
    }
}
