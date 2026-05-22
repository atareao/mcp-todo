<div align="center">

# MCP Todo Server

[![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-0.9-6b5ce7?style=flat-square)](https://modelcontextprotocol.io)
[![SQLite](https://img.shields.io/badge/SQLite-FTS5-003b57?style=flat-square&logo=sqlite&logoColor=white)](https://www.sqlite.org/fts5.html)

A Model Context Protocol (MCP) tool server for managing TODO items, backed by SQLite with full-text search.

[Features](#features) • [Getting Started](#getting-started) • [Available Tools](#available-tools) • [Configuration](#configuration) • [Docker](#docker)

</div>

## Overview

MCP Todo is a lightweight, self-contained task management server that implements the [Model Context Protocol](https://modelcontextprotocol.io/). It provides AI assistants and MCP clients with a rich set of tools to create, organize, search, and manage tasks — all backed by a SQLite database with FTS5 full-text search.

The server runs as a single binary with zero external dependencies beyond the SQLite file, and supports both **stdio** and **HTTP** transports.

## Features

- **21 MCP Tools** — full CRUD, batch operations, search, statistics, export/import, archive, and recurring tasks
- **Full-Text Search** — FTS5-powered search with ranking and snippets
- **Natural Language Dates** — parse expressions like `tomorrow`, `next week`, `in 3 days`, `monday`, `end of month`
- **Duplicate Detection** — Levenshtein-based similarity scoring when creating tasks
- **Soft Delete & Archive** — trash bin with undo, separate archive for completed items
- **Recurring Tasks** — daily, weekly, biweekly, monthly, yearly patterns
- **Batch Operations** — complete or delete multiple tasks by ID, status, project, or tags
- **Export/Import** — JSON export with filters, import with duplicate skipping
- **Dual Transport** — stdio (default for MCP clients) or HTTP (for web/API access)
- **Zero Config** — runs out of the box with a local SQLite file

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Edition 2024)
- [Cargo](https://doc.rust-lang.org/cargo/)
- [just](https://github.com/casey/just) (optional, for project commands)

### Build and Run

```bash
# Clone the repository
git clone https://github.com/atareao/mcp-todo.git
cd mcp-todo

# Build
cargo build --release

# Run with stdio transport (default)
cargo run

# Run with HTTP transport
MCP_TRANSPORT=http cargo run
```

### Quick Test

Once running with HTTP transport, the server listens on `127.0.0.1:3003`. You can connect any MCP-compatible client to it.

## Available Tools

### Task Management

| Tool | Description |
|---|---|
| `create_task` | Create a new task with optional duplicate detection |
| `update_task` | Update task fields by ID |
| `get_task` | Retrieve a single task by ID |
| `delete_task` | Soft-delete a task (movable to trash) |
| `list_tasks` | List tasks with filters (status, priority, project, tags, date ranges, search) |
| `complete_task` | Mark a task as done |

### Batch Operations

| Tool | Description |
|---|---|
| `batch_complete` | Complete multiple tasks by ID, status, project, or tags |
| `batch_delete` | Soft-delete multiple tasks by ID, status, project, or tags |

### Trash & Archive

| Tool | Description |
|---|---|
| `list_deleted` | Show soft-deleted tasks (trash bin) |
| `undo_delete` | Restore a soft-deleted task |
| `purge_deleted` | Permanently delete soft-deleted tasks |
| `archive_task` | Move a task to the archive |
| `unarchive_task` | Restore a task from the archive |
| `list_archived` | Show archived tasks |

### Search & Statistics

| Tool | Description |
|---|---|
| `search_tasks` | Full-text search using FTS5 with ranking |
| `overdue_tasks` | List tasks past their due date |
| `task_stats` | Comprehensive statistics (counts by status, priority, project, overdue, completed) |

### Recurring Tasks

| Tool | Description |
|---|---|
| `create_recurring_task` | Create a task with a recurrence pattern |
| `list_recurring_tasks` | List recurring task definitions and generated instances |

### Export & Import

| Tool | Description |
|---|---|
| `export_tasks` | Export tasks to JSON with optional filters |
| `import_tasks` | Import tasks from JSON with duplicate detection |

## Configuration

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `TODO_DB_PATH` | `todo.db` | Path to the SQLite database file |
| `MCP_TRANSPORT` | `stdio` | Transport mode: `stdio` or `http` |
| `MCP_PORT` | `3003` | HTTP port (when using `http` transport) |
| `MCP_HOST` | `127.0.0.1` | HTTP bind address |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

### Examples

```bash
# Custom database path
TODO_DB_PATH=/tmp/my-todos.db cargo run

# HTTP on all interfaces, port 8080
MCP_TRANSPORT=http MCP_HOST=0.0.0.0 MCP_PORT=8080 cargo run

# Debug logging
RUST_LOG=debug cargo run
```

### Natural Language Dates

The `create_task` and `list_tasks` tools accept natural language date strings:

| Expression | Resolves To |
|---|---|
| `today` | Current date/time |
| `tomorrow` | Tomorrow at 09:00 |
| `yesterday` | Yesterday at 09:00 |
| `next week` | 7 days from now at 09:00 |
| `next month` | First of next month at 09:00 |
| `end of week` | Sunday at 18:00 |
| `end of month` | Last day of current month at 18:00 |
| `in 3 days` | 3 days from now at 09:00 |
| `in 2 weeks` | 14 days from now at 09:00 |
| `monday` / `lunes` | Next Monday at 09:00 |
| `next friday` | Friday of next week at 09:00 |

Standard RFC3339 and common date formats (`YYYY-MM-DD`, `DD/MM/YYYY`) are also supported.

## Docker

A multi-stage Dockerfile is provided for containerized deployment:

```bash
# Build the image
just build

# Run with stdio transport
podman run -it atareao/mcp-todo:latest

# Run with HTTP transport
podman run -p 3003:3003 -e MCP_TRANSPORT=http atareao/mcp-todo:latest

# Persist database to host
podman run -v $(pwd)/data:/data -e TODO_DB_PATH=/data/todo.db atareao/mcp-todo:latest
```

> [!NOTE]
> The container runs as non-root user `mcp` (UID 1000) for security.

## Project Structure

```
src/
├── main.rs              # Entry point, transport dispatch, MCP handler
├── db/
│   ├── schema.rs        # Database initialization (tables, triggers, indexes)
│   ├── operations.rs    # All SQL queries and business logic
│   └── export_import.rs # JSON export/import with versioning
├── models/
│   └── mod.rs           # Data structs (TodoItem, enums, search results)
├── tools/
│   ├── mod.rs           # TodoTools enum and tool_box! macro
│   └── *.rs             # One file per MCP tool (21 tools)
└── utils/
    ├── natural_date.rs  # Natural language date parser
    └── similarity.rs    # Levenshtein similarity for duplicate detection
```

## Database Schema

The server manages three SQLite structures:

- **`todo_items`** — core task data with soft delete (`is_deleted`) and archive (`is_archived`) flags
- **`tags`** — many-to-many tag association table
- **`todo_fts`** — FTS5 virtual table for full-text search, kept in sync via triggers

> [!TIP]
> The schema is created automatically on first run. No manual migration or setup is required.

## Development

```bash
# Run linter (clippy)
just lint

# Check formatting
just fmt

# Fix formatting
just fmt-fix
```
