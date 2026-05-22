use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::models::Priority;
use crate::utils::natural_date;
use crate::AppState;

#[mcp_tool(
    name = "list_recurring_tasks",
    description = "List all recurring task templates and optionally generate new instances for today.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListRecurringTasks {
    /// Generate new instances for today based on recurrence patterns
    generate_instances: Option<bool>,
}

impl ListRecurringTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let mut new_instances = Vec::new();
        if self.generate_instances.unwrap_or(false) {
            new_instances = operations::process_recurring_tasks(pool)
                .await
                .map_err(|e| CallToolError::from_message(e.to_string()))?;
        }

        let recurring = operations::get_recurring_tasks(pool)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let mut response = Vec::new();

        if !new_instances.is_empty() {
            response.push(format!("✅ Generated {} new instance(s):\n", new_instances.len()));
            for task in &new_instances {
                response.push(format!("  - {}", task.summary));
                if let Some(due) = &task.due_date {
                    let relative = natural_date::format_relative_time(*due);
                    response.push(format!("    Due: {} ({})", due.format("%Y-%m-%d"), relative));
                }
            }
            response.push(String::new());
        }

        if recurring.is_empty() {
            response.push("No recurring tasks configured.".to_string());
        } else {
            response.push(format!("🔄 {} recurring task(s):\n", recurring.len()));
            for task in &recurring {
                let pattern = task.recurrence_pattern
                    .as_ref()
                    .map(|p| format!("{:?}", p))
                    .unwrap_or_else(|| "unknown".to_string());

                let priority_icon = match task.priority {
                    Priority::High => "🔴",
                    Priority::Medium => "🟡",
                    Priority::Low => "🟢",
                };

                response.push(format!("{} {} ({})", priority_icon, task.summary, pattern));

                if let Some(project) = &task.project {
                    response.push(format!("   Project: {}", project));
                }

                if let Some(due) = &task.due_date {
                    let relative = natural_date::format_relative_time(*due);
                    response.push(format!("   Next due: {} ({})", due.format("%Y-%m-%d"), relative));
                }

                if let Some(end) = &task.recurrence_end {
                    response.push(format!("   Ends: {}", end.format("%Y-%m-%d")));
                }

                response.push(String::new());
            }
        }

        Ok(CallToolResult::text_content(vec![TextContent::from(response.join("\n"))]))
    }
}
