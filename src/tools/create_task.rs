use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations::{self, CreateTaskParams};
use crate::utils::natural_date;
use crate::AppState;

#[mcp_tool(
    name = "create_task",
    description = "Create a new todo task. Supports natural language dates like 'tomorrow', 'next week', 'in 3 days', 'monday'. Returns potential duplicates with similarity scores for confirmation.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct CreateTask {
    /// Short summary of the task
    summary: String,
    /// Optional detailed description
    description: Option<String>,
    /// Task priority (default: medium)
    priority: Option<String>,
    /// Optional project name
    project: Option<String>,
    /// List of tags
    tags: Option<Vec<String>>,
    /// Due date: RFC3339 format or natural language (tomorrow, next week, monday, in 3 days, etc.)
    due_date: Option<String>,
}

impl CreateTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let duplicates = operations::find_similar_tasks(pool, &self.summary, 0.7)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        if !duplicates.is_empty() {
            let dup_text = format_duplicates(&duplicates);
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                format!(
                    "⚠️ Potential duplicates found:\n{}\n\nDo you want to create this task anyway? Reply with the same parameters to confirm.",
                    dup_text
                ),
            )]));
        }

        let due_date = self.due_date.as_ref().map(|d| {
            if let Some(dt) = natural_date::parse_natural_date(d) {
                dt.to_rfc3339()
            } else {
                d.clone()
            }
        });

        let params = CreateTaskParams {
            summary: self.summary.clone(),
            description: self.description.clone(),
            priority: self.priority.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
            due_date,
        };

        let task = operations::create_task(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format_task_created(&task),
        )]))
    }
}

fn format_duplicates(duplicates: &[crate::models::DuplicateCandidate]) -> String {
    duplicates
        .iter()
        .map(|d| {
            format!(
                "- {} (similarity: {:.0}%, id: {})",
                d.summary,
                d.similarity * 100.0,
                d.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_task_created(task: &crate::models::TodoItem) -> String {
    let mut lines = vec![format!("✅ Task created: {}", task.summary), String::new()];
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
        let relative = natural_date::format_relative_time(*due);
        lines.push(format!("Due: {} ({})", due.format("%Y-%m-%d %H:%M"), relative));
    }

    lines.join("\n")
}
