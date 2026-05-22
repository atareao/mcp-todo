use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::operations;
use crate::AppState;

#[mcp_tool(
    name = "search_tasks",
    description = "Full-text search across all tasks using SQLite FTS5. Searches summary, description, project, and tags. Supports boolean operators (AND, OR, NOT) and phrase matching with quotes.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct SearchTasks {
    /// Search query. Supports: plain words, phrases in quotes, AND/OR/NOT operators. Examples: "groceries milk", "work AND urgent", "\"meeting notes\""
    query: String,
    /// Maximum number of results (default: 20)
    limit: Option<i64>,
}

impl SearchTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let limit = self.limit.unwrap_or(20);
        let results = operations::search_tasks_fts(pool, &self.query, limit)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let response = if results.is_empty() {
            format!("No tasks found for query: \"{}\"", self.query)
        } else {
            format_search_results(&results, &self.query)
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(response)]))
    }
}

fn format_search_results(results: &[crate::models::FtsSearchResult], query: &str) -> String {
    let mut lines = vec![format!("🔍 Found {} result(s) for \"{}\":\n", results.len(), query)];

    for (i, r) in results.iter().enumerate() {
        let relevance = if r.score > 0.8 {
            "🟢"
        } else if r.score > 0.5 {
            "🟡"
        } else {
            "🔴"
        };

        lines.push(format!(
            "{} {}. {} (ID: {})",
            relevance,
            i + 1,
            r.summary,
            r.id
        ));

        if let Some(project) = &r.project {
            lines.push(format!("   Project: {}", project));
        }

        lines.push(format!("   Relevance: {:.0}%", r.score * 100.0));
        lines.push(format!("   Context: {}", r.snippet));
        lines.push(String::new());
    }

    lines.join("\n")
}
