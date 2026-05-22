use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "task_stats",
    description = "Get statistics about your tasks: totals, completion rates, overdue counts, and project breakdown.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct TaskStats {}

impl TaskStats {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let stats = operations::get_task_stats(pool)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = format_stats(&stats);

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}

fn format_stats(stats: &operations::TaskStats) -> String {
    let mut lines = vec!["📊 Task Statistics\n".to_string()];

    lines.push("**Overview**".to_string());
    lines.push(format!("Total tasks: {}", stats.total));
    lines.push(format!("Todo: {} | In Progress: {} | Done: {}", stats.todo_count, stats.in_progress_count, stats.done_count));
    lines.push(String::new());

    if stats.total > 0 {
        let completion_rate = (stats.done_count as f64 / stats.total as f64) * 100.0;
        lines.push(format!("Completion rate: {:.1}%", completion_rate));
        lines.push(String::new());
    }

    lines.push("**Urgent**".to_string());
    if stats.overdue_count > 0 {
        lines.push(format!("⚠️ Overdue: {}", stats.overdue_count));
    } else {
        lines.push("✅ No overdue tasks".to_string());
    }
    lines.push(format!("Due today: {}", stats.due_today_count));
    lines.push(format!("Due this week: {}", stats.due_this_week_count));
    if stats.high_priority_count > 0 {
        lines.push(format!("High priority (active): {}", stats.high_priority_count));
    }
    lines.push(String::new());

    lines.push("**Completed**".to_string());
    lines.push(format!("Completed today: {}", stats.completed_today));
    lines.push(format!("Completed this week: {}", stats.completed_this_week));
    lines.push(String::new());

    if !stats.projects.is_empty() {
        lines.push("**By Project**".to_string());
        for (project, count) in &stats.projects {
            lines.push(format!("  {}: {} active tasks", project, count));
        }
    }

    lines.join("\n")
}
