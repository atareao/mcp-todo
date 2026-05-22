use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "purge_deleted",
    description = "Permanently delete soft-deleted tasks from the trash. This action cannot be undone! Provide specific IDs or omit to purge all deleted tasks.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct PurgeDeleted {
    /// List of deleted task IDs to permanently delete. If omitted, all deleted tasks are purged.
    ids: Option<Vec<String>>,
}

impl PurgeDeleted {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let result = operations::purge_deleted(pool, self.ids.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = if result.affected_count == 0 {
            "No deleted tasks to purge.".to_string()
        } else {
            let scope = if self.ids.is_some() {
                format!("{} task(s) permanently deleted by ID", result.affected_count)
            } else {
                format!("{} deleted task(s) permanently purged", result.affected_count)
            };

            let mut lines = vec![format!("🗑️ {}", scope)];

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

            lines.push(String::new());
            lines.push("⚠️ This action cannot be undone.".to_string());

            lines.join("\n")
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}
