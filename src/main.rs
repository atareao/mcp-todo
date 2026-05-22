mod db;
mod models;
mod tools;
mod utils;

use async_trait::async_trait;
use rust_mcp_schema::{
    CompleteResult, ListResourceTemplatesResult, ListResourcesResult, ReadResourceRequestParams,
    ReadResourceResult,
};
use rust_mcp_sdk::mcp_server::{
    hyper_server, server_runtime, HyperServerOptions, ServerHandler, ToMcpServerHandler,
    McpServerOptions, ServerRuntime,
};
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequestParams, CallToolResult, Implementation,
    InitializeResult, ListToolsResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
    RpcError, PaginatedRequestParams, CompleteRequestParams,
};
use rust_mcp_sdk::{StdioTransport, TransportOptions, McpServer};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub pool: SqlitePool,
}

pub struct TodoServerHandler {
    pub state: Arc<Mutex<AppState>>,
}

#[async_trait]
impl ServerHandler for TodoServerHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: tools::TodoTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params = tools::TodoTools::try_from(params).map_err(CallToolError::new)?;

        let state = self.state.clone();

        match tool_params {
            tools::TodoTools::CreateTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::UpdateTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::DeleteTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::GetTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::ListTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::CompleteTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::OverdueTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::TaskStats(tool) => tool.call_tool(state).await,
            tools::TodoTools::SearchTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::CreateRecurringTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::ListRecurringTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::BatchComplete(tool) => tool.call_tool(state).await,
            tools::TodoTools::BatchDelete(tool) => tool.call_tool(state).await,
            tools::TodoTools::UndoDelete(tool) => tool.call_tool(state).await,
            tools::TodoTools::ListDeleted(tool) => tool.call_tool(state).await,
            tools::TodoTools::PurgeDeleted(tool) => tool.call_tool(state).await,
            tools::TodoTools::ExportTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::ImportTasks(tool) => tool.call_tool(state).await,
            tools::TodoTools::ArchiveTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::UnarchiveTask(tool) => tool.call_tool(state).await,
            tools::TodoTools::ListArchived(tool) => tool.call_tool(state).await,
        }
    }

    async fn handle_list_resources_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            meta: None,
            next_cursor: None,
            resources: vec![],
        })
    }

    async fn handle_list_resource_templates_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourceTemplatesResult, RpcError> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            next_cursor: None,
            resource_templates: vec![],
        })
    }

    async fn handle_read_resource_request(
        &self,
        _params: ReadResourceRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        Err(RpcError::method_not_found())
    }

    async fn handle_complete_request(
        &self,
        _params: CompleteRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CompleteResult, RpcError> {
        Err(RpcError::method_not_found())
    }
}

fn build_server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "MCP Todo Server".into(),
            version: "0.1.0".into(),
            title: Some("MCP Todo Server".into()),
            description: Some("A TODO list MCP server with SQLite storage and duplicate detection.".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            resources: None,
            completions: None,
            tasks: None,
            ..Default::default()
        },
        meta: None,
        instructions: Some("Use this server to manage TODO items. You can create, update, delete, and list tasks with various filters.".into()),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let db_path = std::env::var("TODO_DB_PATH").unwrap_or_else(|_| "todo.db".to_string());

    let pool = match db::operations::create_pool(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to create database pool: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = db::schema::init_db(&pool).await {
        tracing::error!("Failed to initialize database: {}", e);
        std::process::exit(1);
    }

    let state = Arc::new(Mutex::new(AppState { pool }));

    let transport = std::env::var("MCP_TRANSPORT").unwrap_or_else(|_| "stdio".to_string());

    match transport.as_str() {
        "stdio" => run_stdio(state).await,
        "http" => run_http(state).await,
        _ => {
            eprintln!("Unknown transport: {}. Use 'stdio' or 'http'", transport);
            std::process::exit(1);
        }
    }
}

async fn run_stdio(state: Arc<Mutex<AppState>>) {
    tracing::info!("Starting MCP Todo server (stdio transport)");

    let server_details = build_server_details();
    let transport = match StdioTransport::new(TransportOptions::default()) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create transport: {}", e);
            std::process::exit(1);
        }
    };

    let handler = TodoServerHandler { state };

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
        message_observer: None,
    });

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

async fn run_http(state: Arc<Mutex<AppState>>) {
    let port: u16 = std::env::var("MCP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3003);

    let host = std::env::var("MCP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    tracing::info!(
        "Starting MCP Todo server (HTTP transport) on {}:{}",
        host,
        port
    );

    let server_details = build_server_details();
    let handler = TodoServerHandler { state };

    let server = hyper_server::create_server(
        server_details,
        handler.to_mcp_server_handler(),
        HyperServerOptions {
            host,
            port,
            ..Default::default()
        },
    );

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
