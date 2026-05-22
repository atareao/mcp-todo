use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportData {
    pub version: String,
    #[serde(rename = "exportedAt")]
    pub exported_at: String,
    pub tasks: Vec<ExportTask>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportTask {
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub project: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_end: Option<String>,
}

pub struct ExportParams {
    pub include_deleted: bool,
    pub status: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub async fn export_tasks(pool: &SqlitePool, params: ExportParams) -> Result<ExportData> {
    let mut query = r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at, t.recurrence_pattern, t.recurrence_end, t.is_deleted
        FROM todo_items t
        WHERE 1=1
    "#.to_string();

    if !params.include_deleted {
        query.push_str(" AND t.is_deleted = 0");
    }
    if let Some(status) = &params.status {
        query.push_str(" AND t.status = ?");
    }
    if let Some(project) = &params.project {
        query.push_str(" AND t.project = ?");
    }
    if let Some(tags) = &params.tags {
        if !tags.is_empty() {
            let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            query.push_str(&format!(
                " AND t.id IN (SELECT todo_id FROM tags WHERE name IN ({}))",
                placeholders
            ));
        }
    }

    query.push_str(" ORDER BY t.created_at ASC");

    let mut db_query = sqlx::query(&query);

    if let Some(status) = &params.status {
        db_query = db_query.bind(status);
    }
    if let Some(project) = &params.project {
        db_query = db_query.bind(project);
    }
    if let Some(tags) = &params.tags {
        if !tags.is_empty() {
            for tag in tags {
                db_query = db_query.bind(tag);
            }
        }
    }

    let rows = db_query.fetch_all(pool).await?;

    let mut tasks = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let completed_at: Option<String> = row.try_get("completed_at")?;
        let recurrence_pattern: Option<String> = row.try_get("recurrence_pattern")?;
        let recurrence_end: Option<String> = row.try_get("recurrence_end")?;

        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        tasks.push(ExportTask {
            summary,
            description,
            status,
            priority,
            project,
            tags,
            due_date,
            completed_at,
            recurrence_pattern,
            recurrence_end,
        });
    }

    Ok(ExportData {
        version: "1.0".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        tasks,
    })
}

pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

pub async fn import_tasks(pool: &SqlitePool, data: &str, skip_duplicates: bool) -> Result<ImportResult> {
    let export_data: ExportData = serde_json::from_str(data)
        .map_err(|e| anyhow::anyhow!("Invalid JSON format: {}", e))?;

    if export_data.version != "1.0" {
        return Err(anyhow::anyhow!("Unsupported export version: {}. Expected 1.0", export_data.version));
    }

    let mut result = ImportResult {
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    for export_task in &export_data.tasks {
        let similarity_threshold = 0.85;
        let duplicates = crate::db::operations::find_similar_tasks(pool, &export_task.summary, similarity_threshold)
            .await
            .unwrap_or_default();

        if skip_duplicates && !duplicates.is_empty() {
            result.skipped += 1;
            continue;
        }

        if let Err(e) = import_single_task(pool, export_task).await {
            result.errors.push(format!("Failed to import '{}': {}", export_task.summary, e));
        } else {
            result.imported += 1;
        }
    }

    Ok(result)
}

async fn import_single_task(pool: &SqlitePool, task: &ExportTask) -> Result<()> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO todo_items (id, summary, description, status, priority, project, due_date, created_at, updated_at, completed_at, recurrence_pattern, recurrence_end)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(id.to_string())
    .bind(&task.summary)
    .bind(&task.description)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(&task.project)
    .bind(&task.due_date)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(&task.completed_at)
    .bind(&task.recurrence_pattern)
    .bind(&task.recurrence_end)
    .execute(&mut *tx)
    .await?;

    for tag in &task.tags {
        sqlx::query("INSERT INTO tags (todo_id, name) VALUES ($1, $2)")
            .bind(id.to_string())
            .bind(tag)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(())
}
