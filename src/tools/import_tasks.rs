use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::export_import;
use crate::AppState;

#[mcp_tool(
    name = "import_tasks",
    description = "Import tasks from JSON export data. Validates structure, detects duplicates, and reports import results.",
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ImportTasks {
    /// JSON string from export_tasks output
    data: String,
    /// Skip tasks that are similar to existing ones (default: true)
    skip_duplicates: Option<bool>,
}

impl ImportTasks {
    pub async fn call_tool(&self, state: Arc<Mutex<AppState>>) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let pool = &state.pool;

        let skip_duplicates = self.skip_duplicates.unwrap_or(true);

        let result = export_import::import_tasks(pool, &self.data, skip_duplicates)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let mut lines = vec![format!("📥 Import complete:"), String::new()];
        lines.push(format!("✅ Imported: {}", result.imported));
        lines.push(format!("⏭️ Skipped (duplicates): {}", result.skipped));

        if !result.errors.is_empty() {
            lines.push(String::new());
            lines.push(format!("❌ Errors ({}):", result.errors.len()));
            for error in &result.errors {
                lines.push(format!("  - {}", error));
            }
        }

        if result.imported > 0 || result.skipped > 0 {
            lines.push(String::new());
            lines.push(format!("Total processed: {}", result.imported + result.skipped));
        }

        Ok(CallToolResult::text_content(vec![TextContent::from(lines.join("\n"))]))
    }
}
