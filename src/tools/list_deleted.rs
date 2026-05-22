use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::models::{Priority, TodoItem};
use crate::AppState;

#[mcp_tool(
    name = "list_deleted",
    description = "List all soft-deleted tasks in the trash/recycle bin. These tasks can be restored with undo_delete or permanently deleted with purge_deleted.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListDeleted {}

impl ListDeleted {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let tasks = operations::list_deleted(pool)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = if tasks.is_empty() {
            "🗑️ Trash is empty.".to_string()
        } else {
            format_deleted_tasks(&tasks)
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}

fn format_deleted_tasks(tasks: &[TodoItem]) -> String {
    let mut lines = vec![format!("🗑️ {} deleted task(s) in trash:\n", tasks.len())];

    for task in tasks {
        let priority_icon = match task.priority {
            Priority::High => "🔴",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        };

        let deleted_time = task.updated_at.format("%Y-%m-%d %H:%M");

        let mut task_lines = vec![format!(
            "{} {} (ID: {})",
            priority_icon, task.summary, task.id
        )];

        task_lines.push(format!("   Deleted at: {}", deleted_time));

        if let Some(project) = &task.project {
            task_lines.push(format!("   Project: {}", project));
        }

        task_lines.push(String::new());
        lines.extend(task_lines);
    }

    lines.push("Use 'undo_delete' to restore or 'purge_deleted' to permanently delete.".to_string());

    lines.join("\n")
}
