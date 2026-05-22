# MCP Todo - Pending Features

## Status: 16 tools implemented

### Implemented Tools
1. `create_task` - Crear con fechas naturales + detección duplicados
2. `update_task` - Actualizar por ID
3. `delete_task` - Borrar por ID
4. `get_task` - Obtener una tarea
5. `list_tasks` - Listar con filtros + fechas naturales
6. `complete_task` - Marcar como done rápidamente
7. `overdue_tasks` - Tareas vencidas
8. `task_stats` - Estadísticas completas
9. `search_tasks` - FTS5 full-text search
10. `create_recurring_task` - Tareas recurrentes
11. `list_recurring_tasks` - Listar/generar recurrentes
12. `batch_complete` - Completar múltiples tareas
13. `batch_delete` - Borrar múltiples tareas
14. `export_tasks` - Exportar a JSON
15. `import_tasks` - Importar desde JSON
16. `archive_task` - Archivar tareas
17. `unarchive_task` - Restaurar desde archivo
18. `list_archived` - Listar tareas archivadas

---

## Pending Features (by priority)

### 1. Soft Delete + Undo
- Add `is_deleted` column to `todo_items` table
- Add `deleted_at` timestamp column
- Update all queries to filter `WHERE is_deleted = 0`
- New tool: `undo_delete` - restore soft-deleted tasks
- New tool: `list_deleted` - show trash/recycle bin
- New tool: `purge_deleted` - permanently delete soft-deleted tasks

### 2. Export/Import ✅
- New tool: `export_tasks` - export to JSON or CSV
  - Support filters (project, status, tags, date range)
  - Output format: JSON (default) or CSV
- New tool: `import_tasks` - import from JSON
  - Validate structure before import
  - Option to skip duplicates
  - Report import results (success/failure counts)

### 3. Archive Tasks ✅
- Add `is_archived` column to `todo_items` table
- `list_tasks` should exclude archived by default
- New parameter: `include_archived: true` for `list_tasks`
- New tool: `archive_task` - mark task as archived
- New tool: `unarchive_task` - restore from archive
- New tool: `list_archived` - show archived tasks

### 4. Task Templates
- New table: `task_templates` (id, summary, description, priority, project, tags, recurrence_pattern)
- New tool: `create_template` - save current task as template
- New tool: `list_templates` - show all templates
- New tool: `delete_template` - remove template
- New tool: `create_from_template` - create task from template

### 5. Subtasks
- Add `parent_id` column to `todo_items` (self-referencing FK)
- New tool: `add_subtask` - add subtask to parent
- New tool: `list_subtasks` - show subtasks of a task
- New tool: `remove_subtask` - detach subtask from parent
- Update `complete_task`: optionally complete all subtasks
- Update `list_tasks`: option to show/hide subtasks

### 6. Priority Auto-Escalation
- Add `priority_escalation_days` config (default: 3)
- Update `list_tasks` / `overdue_tasks` to show escalated tasks
- New tool: `escalate_priorities` - manually run escalation
- Logic: if overdue > N days, bump priority one level

### 7. Dependencies
- New table: `task_dependencies` (task_id, depends_on_id)
- New tool: `add_dependency` - task A depends on task B
- New tool: `remove_dependency` - remove dependency
- New tool: `list_dependencies` - show dependency graph
- Prevent completing task if dependencies not met

### 8. Time Tracking
- New table: `time_entries` (id, task_id, start_time, end_time, duration)
- New tool: `start_timer` - start tracking time on task
- New tool: `stop_timer` - stop and save time entry
- New tool: `time_report` - show time spent per task/project

### 9. Comments
- New table: `task_comments` (id, task_id, text, created_at)
- New tool: `add_comment` - add comment to task
- New tool: `list_comments` - show comments for task
- New tool: `delete_comment` - remove comment

---

## Implementation Notes
- Always update `updated_at` on any modification
- Keep FTS5 triggers in sync when adding new columns
- Consider migration strategy for existing databases
- Test each feature with both stdio and HTTP transports
