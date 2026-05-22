use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "get_task",
    description = "Get a single todo task by ID.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct GetTask {
    /// Task ID (UUID)
    id: String,
}

impl GetTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let task = operations::get_task(pool, self.id.clone())
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format_task(&task),
        )]))
    }
}

fn format_task(task: &crate::models::TodoItem) -> String {
    let mut lines = vec![format!("📋 {}", task.summary), String::new()];
    lines.push(format!("ID: {}", task.id));
    lines.push(format!("Status: {:?}", task.status));
    lines.push(format!("Priority: {:?}", task.priority));

    if let Some(desc) = &task.description {
        lines.push(format!("Description: {}", desc));
    }

    if let Some(project) = &task.project {
        lines.push(format!("Project: {}", project));
    }

    if !task.tags.is_empty() {
        lines.push(format!("Tags: {}", task.tags.join(", ")));
    }

    if let Some(due) = &task.due_date {
        lines.push(format!("Due: {}", due.format("%Y-%m-%d %H:%M")));
    }

    lines.push(format!("Created: {}", task.created_at.format("%Y-%m-%d %H:%M")));

    if let Some(completed) = &task.completed_at {
        lines.push(format!("Completed: {}", completed.format("%Y-%m-%d %H:%M")));
    }

    lines.join("\n")
}
