pub mod archive_task;
pub mod batch_complete;
pub mod batch_delete;
pub mod complete_task;
pub mod create_recurring_task;
pub mod create_task;
pub mod delete_task;
pub mod export_tasks;
pub mod get_task;
pub mod import_tasks;
pub mod list_archived;
pub mod list_deleted;
pub mod list_recurring_tasks;
pub mod list_tasks;
pub mod overdue_tasks;
pub mod purge_deleted;
pub mod search_tasks;
pub mod task_stats;
pub mod unarchive_task;
pub mod undo_delete;
pub mod update_task;

pub use archive_task::ArchiveTask;
pub use batch_complete::BatchComplete;
pub use batch_delete::BatchDelete;
pub use complete_task::CompleteTask;
pub use create_recurring_task::CreateRecurringTask;
pub use create_task::CreateTask;
pub use delete_task::DeleteTask;
pub use export_tasks::ExportTasks;
pub use get_task::GetTask;
pub use import_tasks::ImportTasks;
pub use list_archived::ListArchived;
pub use list_deleted::ListDeleted;
pub use list_recurring_tasks::ListRecurringTasks;
pub use list_tasks::ListTasks;
pub use overdue_tasks::OverdueTasks;
pub use purge_deleted::PurgeDeleted;
pub use search_tasks::SearchTasks;
pub use task_stats::TaskStats;
pub use unarchive_task::UnarchiveTask;
pub use undo_delete::UndoDelete;
pub use update_task::UpdateTask;

use rust_mcp_sdk::tool_box;

tool_box!(
    TodoTools,
    [
        CreateTask,
        UpdateTask,
        DeleteTask,
        GetTask,
        ListTasks,
        CompleteTask,
        OverdueTasks,
        TaskStats,
        SearchTasks,
        CreateRecurringTask,
        ListRecurringTasks,
        BatchComplete,
        BatchDelete,
        UndoDelete,
        ListDeleted,
        PurgeDeleted,
        ExportTasks,
        ImportTasks,
        ArchiveTask,
        UnarchiveTask,
        ListArchived
    ]
);
