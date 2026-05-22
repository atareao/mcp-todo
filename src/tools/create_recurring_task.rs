use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations::{self, CreateRecurringTaskParams};
use crate::utils::natural_date;
use crate::AppState;

#[mcp_tool(
    name = "create_recurring_task",
    description = "Create a recurring task that auto-generates new instances. Patterns: daily, weekly, biweekly, monthly, yearly. When marked done, a new instance is created automatically.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct CreateRecurringTask {
    /// Short summary of the recurring task
    summary: String,
    /// Recurrence pattern: daily, weekly, biweekly, monthly, yearly
    recurrence: String,
    /// Optional detailed description
    description: Option<String>,
    /// Task priority (default: medium)
    priority: Option<String>,
    /// Optional project name
    project: Option<String>,
    /// List of tags
    tags: Option<Vec<String>>,
    /// Due date for first instance: RFC3339 or natural (tomorrow, next monday)
    due_date: Option<String>,
    /// Optional end date for recurrence: RFC3339 or natural (in 3 months)
    recurrence_end: Option<String>,
}

impl CreateRecurringTask {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let pattern = self.recurrence.to_lowercase();
        let valid_patterns = ["daily", "weekly", "biweekly", "monthly", "yearly"];
        if !valid_patterns.contains(&pattern.as_str()) {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Invalid recurrence pattern: '{}'. Valid options: {}", self.recurrence, valid_patterns.join(", ")),
            )]));
        }

        let due_date = self.due_date.as_ref().map(|d| {
            if let Some(dt) = natural_date::parse_natural_date(d) {
                dt.to_rfc3339()
            } else {
                d.clone()
            }
        });

        let recurrence_end = self.recurrence_end.as_ref().map(|d| {
            if let Some(dt) = natural_date::parse_natural_date(d) {
                dt.to_rfc3339()
            } else {
                d.clone()
            }
        });

        let params = CreateRecurringTaskParams {
            summary: self.summary.clone(),
            description: self.description.clone(),
            priority: self.priority.clone(),
            project: self.project.clone(),
            tags: self.tags.clone(),
            due_date,
            recurrence_pattern: pattern,
            recurrence_end,
        };

        let task = operations::create_recurring_task(pool, params)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let recurrence_label = task.recurrence_pattern
            .as_ref()
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "unknown".to_string());

        let mut response = vec![
            format!("🔄 Recurring task created: {}", task.summary),
            String::new(),
            format!("ID: {}", task.id),
            format!("Pattern: {}", recurrence_label),
            format!("Priority: {:?}", task.priority),
        ];

        if let Some(project) = &task.project {
            response.push(format!("Project: {}", project));
        }

        if let Some(due) = &task.due_date {
            let relative = natural_date::format_relative_time(*due);
            response.push(format!("Next due: {} ({})", due.format("%Y-%m-%d %H:%M"), relative));
        }

        if let Some(end) = &task.recurrence_end {
            response.push(format!("Ends: {}", end.format("%Y-%m-%d %H:%M")));
        }

        response.push(String::new());
        response.push("💡 When you complete this task, a new instance will be created automatically.".to_string());

        Ok(CallToolResult::text_content(vec![TextContent::from(response.join("\n"))]))
    }
}
