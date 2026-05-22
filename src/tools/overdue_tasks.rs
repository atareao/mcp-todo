use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::models::{Priority, TodoItem};
use crate::utils::natural_date;
use crate::AppState;

#[mcp_tool(
    name = "overdue_tasks",
    description = "List all tasks that are past their due date and not yet completed.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct OverdueTasks {}

impl OverdueTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let tasks = operations::get_overdue_tasks(pool)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response_text = if tasks.is_empty() {
            "🎉 No overdue tasks! You're all caught up.".to_string()
        } else {
            format_overdue_tasks(&tasks)
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(response_text)]))
    }
}

fn format_overdue_tasks(tasks: &[TodoItem]) -> String {
    let mut lines = vec![format!("⚠️ {} overdue task(s):\n", tasks.len())];

    for task in tasks {
        let priority_icon = match task.priority {
            Priority::High => "🔴",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        };

        let overdue_text = task.due_date
            .map(|d| natural_date::format_relative_time(d))
            .unwrap_or_else(|| "unknown".to_string());

        let mut task_lines = vec![format!(
            "{} {} (was due: {})",
            priority_icon, task.summary, overdue_text
        )];

        if let Some(project) = &task.project {
            task_lines.push(format!("   Project: {}", project));
        }

        if let Some(due) = &task.due_date {
            task_lines.push(format!("   Due date: {}", due.format("%Y-%m-%d %H:%M")));
        }

        task_lines.push(String::new());
        lines.extend(task_lines);
    }

    lines.join("\n")
}
