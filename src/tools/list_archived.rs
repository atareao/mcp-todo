use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "list_archived",
    description = "List all archived tasks. Archived tasks are hidden from normal lists.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListArchived {}

impl ListArchived {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let tasks = operations::list_archived(pool)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        if tasks.is_empty() {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "📭 No archived tasks.".to_string(),
            )]));
        }

        let mut lines = vec![format!("📦 Archived tasks ({}):", tasks.len()), String::new()];

        for task in &tasks {
            let status_icon = match task.status {
                crate::models::TaskStatus::Todo => "📋",
                crate::models::TaskStatus::InProgress => "🔄",
                crate::models::TaskStatus::Done => "✅",
            };

            let priority_icon = match task.priority {
                crate::models::Priority::High => "🔴",
                crate::models::Priority::Medium => "🟡",
                crate::models::Priority::Low => "🟢",
            };

            let mut line = format!("{} {} {} ({})", status_icon, priority_icon, task.summary, task.id);

            if let Some(project) = &task.project {
                line.push_str(&format!(" | 📁 {}", project));
            }

            if !task.tags.is_empty() {
                line.push_str(&format!(" | 🏷️ {}", task.tags.join(", ")));
            }

            if let Some(due) = &task.due_date {
                line.push_str(&format!(" | 📅 {}", due.format("%Y-%m-%d")));
            }

            lines.push(line);
        }

        Ok(CallToolResult::text_content(vec![TextContent::from(lines.join("\n"))]))
    }
}
