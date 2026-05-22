use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations::{self, UpdateTaskParams};
use crate::AppState;

#[mcp_tool(
    name = "update_task",
    description = "Update an existing todo task by ID.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct UpdateTask {
    /// Task ID (UUID)
    id: String,
    /// New summary
    summary: Option<String>,
    /// New description
    description: Option<String>,
    /// New status
    status: Option<String>,
    /// New priority
    priority: Option<String>,
    /// New project
    project: Option<String>,
    /// New list of tags (replaces existing)
    tags: Option<Vec<String>>,
    /// New due date in RFC3339 format
    due_date: Option<String>,
}

impl UpdateTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let params = UpdateTaskParams {
            id: self.id.clone(),
            summary: self.summary.clone(),
            description: self.description.clone(),
            status: self.status.clone(),
            priority: self.priority.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
            due_date: self.due_date.clone(),
        };

        let task = operations::update_task(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format_task_updated(&task),
        )]))
    }
}

fn format_task_updated(task: &crate::models::TodoItem) -> String {
    let mut lines = vec![format!("✅ Task updated: {}", task.summary), String::new()];
    lines.push(format!("ID: {}", task.id));
    lines.push(format!("Status: {:?}", task.status));
    lines.push(format!("Priority: {:?}", task.priority));

    if let Some(project) = &task.project {
        lines.push(format!("Project: {}", project));
    }

    if !task.tags.is_empty() {
        lines.push(format!("Tags: {}", task.tags.join(", ")));
    }

    if let Some(due) = &task.due_date {
        lines.push(format!("Due: {}", due.format("%Y-%m-%d %H:%M")));
    }

    if let Some(completed) = &task.completed_at {
        lines.push(format!("Completed: {}", completed.format("%Y-%m-%d %H:%M")));
    }

    lines.join("\n")
}
