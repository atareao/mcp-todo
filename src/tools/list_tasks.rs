use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations::{self, ListTasksParams};
use crate::models::{Priority, TaskStatus, TodoItem};
use crate::utils::natural_date;
use crate::AppState;

#[mcp_tool(
    name = "list_tasks",
    description = "List todo tasks with optional filters. Supports natural language dates (tomorrow, next week, monday, in 3 days). Filter by status, priority, project, tags, date ranges, and text search.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListTasks {
    /// Filter by status
    status: Option<String>,
    /// Filter by priority
    priority: Option<String>,
    /// Filter by project name
    project: Option<String>,
    /// Filter by tags (tasks must have at least one of these tags)
    tags: Option<Vec<String>>,
    /// Filter tasks due before this date (RFC3339 or natural: tomorrow, next week)
    due_before: Option<String>,
    /// Filter tasks due after this date (RFC3339 or natural: today, monday)
    due_after: Option<String>,
    /// Filter tasks created before this date (RFC3339 or natural: yesterday)
    created_before: Option<String>,
    /// Filter tasks created after this date (RFC3339 or natural: today)
    created_after: Option<String>,
    /// Filter tasks completed before this date (RFC3339 or natural: today)
    completed_before: Option<String>,
    /// Filter tasks completed after this date (RFC3339 or natural: today)
    completed_after: Option<String>,
    /// Search in summary and description
    search: Option<String>,
    /// Maximum number of results (default: 50)
    limit: Option<i64>,
    /// Include archived tasks in results (default: false)
    include_archived: Option<bool>,
}

impl ListTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let resolve_date = |d: &Option<String>| -> Option<String> {
            d.as_ref().map(|date_str| {
                if let Some(dt) = natural_date::parse_natural_date(date_str) {
                    dt.to_rfc3339()
                } else {
                    date_str.clone()
                }
            })
        };

        let params = ListTasksParams {
            status: self.status.clone(),
            priority: self.priority.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
            due_before: resolve_date(&self.due_before),
            due_after: resolve_date(&self.due_after),
            created_before: resolve_date(&self.created_before),
            created_after: resolve_date(&self.created_after),
            completed_before: resolve_date(&self.completed_before),
            completed_after: resolve_date(&self.completed_after),
            search: self.search.clone(),
            limit: self.limit.or(Some(50)),
            include_archived: self.include_archived.unwrap_or(false),
        };

        let tasks = operations::list_tasks(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response_text = if tasks.is_empty() {
            "No tasks found.".to_string()
        } else {
            format_tasks(&tasks)
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(
            response_text,
        )]))
    }
}

fn format_tasks(tasks: &[TodoItem]) -> String {
    let mut lines = vec![format!("📋 Found {} task(s):\n", tasks.len())];

    for task in tasks {
        let status_icon = match task.status {
            TaskStatus::Todo => "⬜",
            TaskStatus::InProgress => "🔄",
            TaskStatus::Done => "✅",
        };

        let priority_icon = match task.priority {
            Priority::High => "🔴",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        };

        let mut task_lines = vec![format!(
            "{} {} {} (ID: {})",
            status_icon, priority_icon, task.summary, task.id
        )];

        if let Some(project) = &task.project {
            task_lines.push(format!("   Project: {}", project));
        }

        if !task.tags.is_empty() {
            task_lines.push(format!("   Tags: {}", task.tags.join(", ")));
        }

        if let Some(due) = &task.due_date {
            let relative = natural_date::format_relative_time(*due);
            task_lines.push(format!("   Due: {} ({})", due.format("%Y-%m-%d %H:%M"), relative));
        }

        if let Some(completed) = &task.completed_at {
            task_lines.push(format!("   Completed: {}", completed.format("%Y-%m-%d %H:%M")));
        }

        task_lines.push(String::new());
        lines.extend(task_lines);
    }

    lines.join("\n")
}
