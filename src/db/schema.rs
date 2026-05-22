use anyhow::Result;
use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS todo_items (
            id TEXT PRIMARY KEY,
            summary TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'todo',
            priority TEXT NOT NULL DEFAULT 'medium',
            project TEXT,
            due_date TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            recurrence_pattern TEXT,
            recurrence_end TEXT,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            deleted_at TEXT,
            is_archived INTEGER NOT NULL DEFAULT 0,
            archived_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE todo_items ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        ALTER TABLE todo_items ADD COLUMN archived_at TEXT
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            todo_id TEXT NOT NULL,
            name TEXT NOT NULL,
            FOREIGN KEY (todo_id) REFERENCES todo_items(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS todo_fts USING fts5(
            todo_id UNINDEXED,
            summary,
            description,
            project,
            tags
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS todo_items_ai AFTER INSERT ON todo_items BEGIN
            INSERT INTO todo_fts(todo_id, summary, description, project, tags)
            VALUES (
                new.id,
                new.summary,
                new.description,
                new.project,
                COALESCE((SELECT GROUP_CONCAT(name, ' ') FROM tags WHERE todo_id = new.id), '')
            );
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS todo_items_ad AFTER DELETE ON todo_items BEGIN
            DELETE FROM todo_fts WHERE todo_id = old.id;
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS todo_items_au AFTER UPDATE ON todo_items BEGIN
            DELETE FROM todo_fts WHERE todo_id = old.id;
            INSERT INTO todo_fts(todo_id, summary, description, project, tags)
            VALUES (
                new.id,
                new.summary,
                new.description,
                new.project,
                COALESCE((SELECT GROUP_CONCAT(name, ' ') FROM tags WHERE todo_id = new.id), '')
            );
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tags_ai AFTER INSERT ON tags BEGIN
            UPDATE todo_fts SET tags = COALESCE(
                (SELECT GROUP_CONCAT(name, ' ') FROM tags WHERE todo_id = new.todo_id), ''
            ) WHERE todo_id = new.todo_id;
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tags_ad AFTER DELETE ON tags BEGIN
            UPDATE todo_fts SET tags = COALESCE(
                (SELECT GROUP_CONCAT(name, ' ') FROM tags WHERE todo_id = old.todo_id), ''
            ) WHERE todo_id = old.todo_id;
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_status ON todo_items(status)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_priority ON todo_items(priority)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_project ON todo_items(project)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_due_date ON todo_items(due_date)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_recurrence ON todo_items(recurrence_pattern)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_deleted ON todo_items(is_deleted)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_todo_archived ON todo_items(is_archived)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tags_todo_id ON tags(todo_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
