# AGENTS.md

## Project

MCP Todo server — an MCP (Model Context Protocol) tool server for managing TODO items, backed by SQLite.

## Commands

```
just lint        # cargo clippy --all-targets --all-features
just fmt         # cargo fmt -- --check
just fmt-fix     # cargo fmt
just build       # podman build (image: atareao/mcp-todo:<version>)
just push        # push image to registry
```

No `cargo test` target exists — no test suite is wired up yet.

## Run

```bash
cargo run                    # stdio transport, DB at ./todo.db
TODO_DB_PATH=/tmp/test.db cargo run   # custom DB path
MCP_TRANSPORT=http cargo run          # HTTP transport on 127.0.0.1:3003
MCP_TRANSPORT=http MCP_PORT=8080 MCP_HOST=0.0.0.0 cargo run  # custom HTTP bind
RUST_LOG=debug cargo run     # enable debug tracing
```

## Architecture

Single binary, single crate (`src/main.rs`). Modules:

- `src/main.rs` — entry point, `TodoServerHandler` dispatches tool calls. Supports **stdio** (default) and **HTTP** transports via `MCP_TRANSPORT` env var.
- `src/db/schema.rs` — DDL run at startup (`init_db`). Creates `todo_items`, `tags`, `todo_fts` (FTS5 virtual table) with sync triggers. No migration framework — schema is imperative `CREATE TABLE IF NOT EXISTS` / `ALTER TABLE` in code.
- `src/db/operations.rs` — all SQL queries.
- `src/db/export_import.rs` — JSON export/import logic.
- `src/tools/mod.rs` — `TodoTools` enum mapping tool names to handler structs.
- `src/tools/*.rs` — one file per MCP tool (22 tools total).
- `src/utils/natural_date.rs` — parses natural date strings like "tomorrow", "next week".
- `src/utils/similarity.rs` — string similarity for duplicate detection.
- `src/models/mod.rs` — data structs.

## Database

- **SQLite** via `sqlx`, file-based. Path from `TODO_DB_PATH` env var (default: `todo.db`).
- Tables: `todo_items` (UUID PK), `tags`, `todo_fts` (FTS5 virtual table).
- Soft delete: `is_deleted` / `deleted_at` columns (already implemented despite PENDING_FEATURES listing it as pending).
- Archive: `is_archived` / `archived_at` columns.
- FTS5 triggers keep search index in sync on insert/update/delete of `todo_items` and `tags`.
- Adding new columns to `todo_items` requires updating FTS5 triggers in `schema.rs`.

## Conventions

- Edition 2024, `rust-mcp-sdk` 0.9 for MCP protocol.
- Tools use `async-trait`, state passed as `Arc<Mutex<AppState>>` with `SqlitePool`.
- Each tool file follows the same pattern: struct with `call_tool(&self, state)` method.
- New tool: add struct to `src/tools/<name>.rs`, add variant to `TodoTools` enum in `tools/mod.rs`, add match arm in `main.rs` handler.

## Docker

Multi-stage Alpine build. Binary exposed on port **3003** (Dockerfile default; runtime HTTP uses 3003 by default). User: `atareao`.

## Pending Features

See `PENDING_FEATURES.md` for roadmap (subtasks, dependencies, time tracking, comments, templates, priority escalation).
