use anyhow::Result;
use chrono::{Datelike, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::models::{DuplicateCandidate, FtsSearchResult, Priority, RecurrencePattern, TaskStatus, TodoItem};

pub async fn create_pool(db_path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .busy_timeout(std::time::Duration::from_secs(5))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
    )
    .await?;
    Ok(pool)
}

pub struct CreateTaskParams {
    pub summary: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
}

pub async fn create_task(pool: &SqlitePool, params: CreateTaskParams) -> Result<TodoItem> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let status = TaskStatus::Todo;
    let priority = params
        .priority
        .and_then(|p| Priority::from_str(&p))
        .unwrap_or(Priority::Medium);
    let due_date = params
        .due_date
        .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
        .transpose()?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO todo_items (id, summary, description, status, priority, project, due_date, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(id.to_string())
    .bind(&params.summary)
    .bind(&params.description)
    .bind(status.as_str().to_string())
    .bind(priority.as_str().to_string())
    .bind(&params.project)
    .bind(due_date.map(|d| d.to_rfc3339()))
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&mut *tx)
    .await?;

    if let Some(tags) = params.tags {
        for tag in tags {
            sqlx::query(
                r#"
                INSERT INTO tags (todo_id, name) VALUES ($1, $2)
                "#,
            )
            .bind(id.to_string())
            .bind(tag)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    get_task_by_id(pool, id.to_string()).await
}

pub struct UpdateTaskParams {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
}

pub async fn update_task(pool: &SqlitePool, params: UpdateTaskParams) -> Result<TodoItem> {
    let id = Uuid::parse_str(&params.id)?;
    let now = Utc::now();

    let existing = get_task_by_id(pool, params.id.clone()).await?;

    let summary = params.summary.unwrap_or(existing.summary.clone());
    let description = params.description.or(existing.description);
    let status = params
        .status
        .and_then(|s| TaskStatus::from_str(&s))
        .unwrap_or(existing.status.clone());
    let priority = params
        .priority
        .and_then(|p| Priority::from_str(&p))
        .unwrap_or(existing.priority.clone());
    let project = params.project.or(existing.project);
    let due_date = params
        .due_date
        .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
        .transpose()?
        .or(existing.due_date);

    let completed_at = if status == TaskStatus::Done && existing.status != TaskStatus::Done {
        Some(now)
    } else if status != TaskStatus::Done {
        None
    } else {
        existing.completed_at
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE todo_items 
        SET summary = $1, description = $2, status = $3, priority = $4, 
            project = $5, due_date = $6, updated_at = $7, completed_at = $8
        WHERE id = $9
        "#,
    )
    .bind(&summary)
    .bind(&description)
    .bind(status.as_str().to_string())
    .bind(priority.as_str().to_string())
    .bind(&project)
    .bind(due_date.map(|d| d.to_rfc3339()))
    .bind(now.to_rfc3339())
    .bind(completed_at.map(|d| d.to_rfc3339()))
    .bind(id.to_string())
    .execute(&mut *tx)
    .await?;

    if let Some(tags) = params.tags {
        sqlx::query("DELETE FROM tags WHERE todo_id = $1")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;

        for tag in tags {
            sqlx::query("INSERT INTO tags (todo_id, name) VALUES ($1, $2)")
                .bind(id.to_string())
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    get_task_by_id(pool, id.to_string()).await
}

pub async fn delete_task(pool: &SqlitePool, id: String) -> Result<()> {
    soft_delete_task(pool, id).await
}

pub async fn get_task(pool: &SqlitePool, id: String) -> Result<TodoItem> {
    get_task_by_id(pool, id).await
}

async fn get_task_by_id(pool: &SqlitePool, id: String) -> Result<TodoItem> {
    let row = sqlx::query(
        r#"
        SELECT id, summary, description, status, priority, project, due_date, created_at, updated_at, completed_at, recurrence_pattern, recurrence_end
        FROM todo_items
        WHERE id = $1
        "#,
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    let id: String = row.try_get("id")?;
    let summary: String = row.try_get("summary")?;
    let description: Option<String> = row.try_get("description")?;
    let status: String = row.try_get("status")?;
    let priority: String = row.try_get("priority")?;
    let project: Option<String> = row.try_get("project")?;
    let due_date: Option<String> = row.try_get("due_date")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    let completed_at: Option<String> = row.try_get("completed_at")?;
    let recurrence_pattern: Option<String> = row.try_get("recurrence_pattern")?;
    let recurrence_end: Option<String> = row.try_get("recurrence_end")?;

    let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
        .bind(&id)
        .fetch_all(pool)
        .await?;

    Ok(TodoItem {
        id: Uuid::parse_str(&id)?,
        summary,
        description,
        status: TaskStatus::from_str(&status).unwrap_or(TaskStatus::Todo),
        priority: Priority::from_str(&priority).unwrap_or(Priority::Medium),
        project,
        tags,
        due_date: due_date
            .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))?,
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))?,
        completed_at: completed_at
            .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?,
        recurrence_pattern: recurrence_pattern.and_then(|s| RecurrencePattern::from_str(&s)),
        recurrence_end: recurrence_end
            .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?,
    })
}

pub struct ListTasksParams {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_before: Option<String>,
    pub due_after: Option<String>,
    pub created_before: Option<String>,
    pub created_after: Option<String>,
    pub completed_before: Option<String>,
    pub completed_after: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub include_archived: bool,
}

pub async fn list_tasks(pool: &SqlitePool, params: ListTasksParams) -> Result<Vec<TodoItem>> {
    let mut query = r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at
        FROM todo_items t
        WHERE t.is_deleted = 0 AND t.is_archived = 0 AND 1=1
    "#
    .to_string();

    if params.include_archived {
        query = query.replace("AND t.is_archived = 0 AND 1=1", "AND 1=1");
    }

    if let Some(status) = &params.status {
        query.push_str(" AND t.status = ?");
    }
    if let Some(priority) = &params.priority {
        query.push_str(" AND t.priority = ?");
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
    if let Some(due_before) = &params.due_before {
        query.push_str(" AND t.due_date <= ?");
    }
    if let Some(due_after) = &params.due_after {
        query.push_str(" AND t.due_date >= ?");
    }
    if let Some(created_before) = &params.created_before {
        query.push_str(" AND t.created_at <= ?");
    }
    if let Some(created_after) = &params.created_after {
        query.push_str(" AND t.created_at >= ?");
    }
    if let Some(completed_before) = &params.completed_before {
        query.push_str(" AND t.completed_at <= ?");
    }
    if let Some(completed_after) = &params.completed_after {
        query.push_str(" AND t.completed_at >= ?");
    }
    if let Some(search) = &params.search {
        query.push_str(" AND (t.summary LIKE ? OR t.description LIKE ?)");
    }

    query.push_str(" ORDER BY t.created_at DESC");

    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }

    let mut db_query = sqlx::query(&query);

    if let Some(status) = &params.status {
        db_query = db_query.bind(status);
    }
    if let Some(priority) = &params.priority {
        db_query = db_query.bind(priority);
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
    if let Some(due_before) = &params.due_before {
        let dt = chrono::DateTime::parse_from_rfc3339(due_before)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(due_after) = &params.due_after {
        let dt = chrono::DateTime::parse_from_rfc3339(due_after)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(created_before) = &params.created_before {
        let dt = chrono::DateTime::parse_from_rfc3339(created_before)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(created_after) = &params.created_after {
        let dt = chrono::DateTime::parse_from_rfc3339(created_after)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(completed_before) = &params.completed_before {
        let dt = chrono::DateTime::parse_from_rfc3339(completed_before)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(completed_after) = &params.completed_after {
        let dt = chrono::DateTime::parse_from_rfc3339(completed_after)?.with_timezone(&Utc);
        db_query = db_query.bind(dt.to_rfc3339());
    }
    if let Some(search) = &params.search {
        let like_pattern = format!("%{}%", search);
        db_query = db_query.bind(like_pattern.clone()).bind(like_pattern);
    }

    let rows = db_query.fetch_all(pool).await?;

    let mut results = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let created_at: String = row.try_get("created_at")?;
        let updated_at: String = row.try_get("updated_at")?;
        let completed_at: Option<String> = row.try_get("completed_at")?;

        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        results.push(TodoItem {
            id: Uuid::parse_str(&id)?,
            summary,
            description,
            status: TaskStatus::from_str(&status).unwrap_or(TaskStatus::Todo),
            priority: Priority::from_str(&priority).unwrap_or(Priority::Medium),
            project,
            tags,
            due_date: due_date
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))?,
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))?,
            completed_at: completed_at
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?,
            recurrence_pattern: None,
            recurrence_end: None,
        });
    }

    Ok(results)
}

pub async fn find_similar_tasks(
    pool: &SqlitePool,
    summary: &str,
    threshold: f64,
) -> Result<Vec<DuplicateCandidate>> {
    let all_tasks: Vec<(String, String)> =
        sqlx::query_as("SELECT id, summary FROM todo_items WHERE status != 'done' AND is_deleted = 0")
            .fetch_all(pool)
            .await?;

    let candidates = all_tasks
        .into_iter()
        .filter_map(|(id, existing_summary)| {
            let similarity = crate::utils::similarity::calculate_similarity(summary, &existing_summary);
            if similarity >= threshold {
                Some(DuplicateCandidate {
                    id: Uuid::parse_str(&id).ok()?,
                    summary: existing_summary,
                    similarity,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(candidates)
}

pub async fn complete_task(pool: &SqlitePool, id: String) -> Result<TodoItem> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE todo_items 
        SET status = 'done', updated_at = ?, completed_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(&id)
    .execute(pool)
    .await?;

    get_task_by_id(pool, id).await
}

pub struct BatchCompleteParams {
    pub ids: Option<Vec<String>>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub struct BatchResult {
    pub affected_count: i64,
    pub affected_ids: Vec<String>,
}

pub async fn batch_complete_tasks(pool: &SqlitePool, params: BatchCompleteParams) -> Result<BatchResult> {
    let now = Utc::now();

    if let Some(ids) = &params.ids {
        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!(
            "UPDATE todo_items SET status = 'done', updated_at = ?, completed_at = ? WHERE id IN ({})",
            placeholders
        );

        let mut db_query = sqlx::query(&query)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339());

        for id in ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids.clone(),
        })
    } else {
        let mut query = "SELECT id FROM todo_items WHERE status != 'done' AND is_deleted = 0".to_string();

        if let Some(status) = &params.status {
            query.push_str(&format!(" AND status = '{}'", status.replace('\'', "''")));
        }
        if let Some(project) = &params.project {
            query.push_str(&format!(" AND project = '{}'", project.replace('\'', "''")));
        }
        if let Some(tags) = &params.tags {
            if !tags.is_empty() {
                let tag_list = tags.iter().map(|t| format!("'{}'", t.replace('\'', "''"))).collect::<Vec<_>>().join(", ");
                query.push_str(&format!(" AND id IN (SELECT todo_id FROM tags WHERE name IN ({}))", tag_list));
            }
        }

        let rows: Vec<(String,)> = sqlx::query_as(&query).fetch_all(pool).await?;
        let ids: Vec<String> = rows.into_iter().map(|r| r.0).collect();

        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let update_query = format!(
            "UPDATE todo_items SET status = 'done', updated_at = ?, completed_at = ? WHERE id IN ({})",
            placeholders
        );

        let mut db_query = sqlx::query(&update_query)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339());

        for id in &ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids,
        })
    }
}

pub struct BatchDeleteParams {
    pub ids: Option<Vec<String>>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub async fn batch_delete_tasks(pool: &SqlitePool, params: BatchDeleteParams) -> Result<BatchResult> {
    let now = Utc::now();
    if let Some(ids) = &params.ids {
        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!(
            "UPDATE todo_items SET is_deleted = 1, deleted_at = ?, updated_at = ? WHERE id IN ({}) AND is_deleted = 0",
            placeholders
        );

        let mut db_query = sqlx::query(&query)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339());
        for id in ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids.clone(),
        })
    } else {
        let mut query = "SELECT id FROM todo_items WHERE is_deleted = 0".to_string();

        if let Some(status) = &params.status {
            query.push_str(&format!(" AND status = '{}'", status.replace('\'', "''")));
        }
        if let Some(project) = &params.project {
            query.push_str(&format!(" AND project = '{}'", project.replace('\'', "''")));
        }
        if let Some(tags) = &params.tags {
            if !tags.is_empty() {
                let tag_list = tags.iter().map(|t| format!("'{}'", t.replace('\'', "''"))).collect::<Vec<_>>().join(", ");
                query.push_str(&format!(" AND id IN (SELECT todo_id FROM tags WHERE name IN ({}))", tag_list));
            }
        }

        let rows: Vec<(String,)> = sqlx::query_as(&query).fetch_all(pool).await?;
        let ids: Vec<String> = rows.into_iter().map(|r| r.0).collect();

        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let update_query = format!(
            "UPDATE todo_items SET is_deleted = 1, deleted_at = ?, updated_at = ? WHERE id IN ({})",
            placeholders
        );

        let mut db_query = sqlx::query(&update_query)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339());
        for id in &ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids,
        })
    }
}

pub async fn get_overdue_tasks(pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let now = Utc::now().to_rfc3339();

    let rows = sqlx::query(
        r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at
        FROM todo_items t
        WHERE t.is_deleted = 0 AND t.due_date IS NOT NULL AND t.due_date < ? AND t.status != 'done'
        ORDER BY t.due_date ASC
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    rows_to_tasks(rows, pool).await
}

pub struct TaskStats {
    pub total: i64,
    pub todo_count: i64,
    pub in_progress_count: i64,
    pub done_count: i64,
    pub overdue_count: i64,
    pub due_today_count: i64,
    pub due_this_week_count: i64,
    pub completed_today: i64,
    pub completed_this_week: i64,
    pub high_priority_count: i64,
    pub projects: Vec<(String, i64)>,
}

pub async fn get_task_stats(pool: &SqlitePool) -> Result<TaskStats> {
    let now = Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339();
    let week_start = (now.date_naive() - chrono::Duration::days(now.weekday().num_days_from_monday() as i64))
        .and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339();
    let now_str = now.to_rfc3339();
    let week_end = (now.date_naive() + chrono::Duration::days(7 - now.weekday().num_days_from_monday() as i64))
        .and_hms_opt(23, 59, 59).unwrap().and_utc().to_rfc3339();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todo_items WHERE is_deleted = 0")
        .fetch_one(pool).await?;

    let todo_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todo_items WHERE status = 'todo' AND is_deleted = 0")
        .fetch_one(pool).await?;

    let in_progress_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todo_items WHERE status = 'in_progress' AND is_deleted = 0")
        .fetch_one(pool).await?;

    let done_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todo_items WHERE status = 'done' AND is_deleted = 0")
        .fetch_one(pool).await?;

    let overdue_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE due_date IS NOT NULL AND due_date < ? AND status != 'done'"
    )
    .bind(&now_str)
    .fetch_one(pool).await?;

    let today_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc().to_rfc3339();
    let due_today_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE due_date >= ? AND due_date <= ? AND status != 'done'"
    )
    .bind(&today_start)
    .bind(&today_end)
    .fetch_one(pool).await?;

    let due_this_week_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE due_date >= ? AND due_date <= ? AND status != 'done'"
    )
    .bind(&week_start)
    .bind(&week_end)
    .fetch_one(pool).await?;

    let completed_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE completed_at >= ? AND completed_at <= ?"
    )
    .bind(&today_start)
    .bind(&today_end)
    .fetch_one(pool).await?;

    let completed_this_week: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE completed_at >= ? AND completed_at <= ?"
    )
    .bind(&week_start)
    .bind(&week_end)
    .fetch_one(pool).await?;

    let high_priority_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todo_items WHERE priority = 'high' AND status != 'done'"
    )
    .fetch_one(pool).await?;

    let projects: Vec<(String, i64)> = sqlx::query_as(
        "SELECT project, COUNT(*) as count FROM todo_items WHERE project IS NOT NULL AND is_deleted = 0 AND status != 'done' GROUP BY project ORDER BY count DESC"
    )
    .fetch_all(pool).await?;

    Ok(TaskStats {
        total,
        todo_count,
        in_progress_count,
        done_count,
        overdue_count,
        due_today_count,
        due_this_week_count,
        completed_today,
        completed_this_week,
        high_priority_count,
        projects,
    })
}

async fn rows_to_tasks(rows: Vec<sqlx::sqlite::SqliteRow>, pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let mut results = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let created_at: String = row.try_get("created_at")?;
        let updated_at: String = row.try_get("updated_at")?;
        let completed_at: Option<String> = row.try_get("completed_at")?;

        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        use crate::models::{Priority, TaskStatus, TodoItem};
        results.push(TodoItem {
            id: Uuid::parse_str(&id)?,
            summary,
            description,
            status: TaskStatus::from_str(&status).unwrap_or(TaskStatus::Todo),
            priority: Priority::from_str(&priority).unwrap_or(Priority::Medium),
            project,
            tags,
            due_date: due_date
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            completed_at: completed_at
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            recurrence_pattern: None,
            recurrence_end: None,
        });
    }
    Ok(results)
}

pub async fn search_tasks_fts(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<FtsSearchResult>> {
    let rows = sqlx::query(
        r#"
        SELECT f.todo_id, f.summary, f.project, rank
        FROM todo_fts f
        WHERE todo_fts MATCH ?
        ORDER BY rank
        LIMIT ?
        "#,
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut results = Vec::new();
    for row in rows {
        let id: String = row.try_get("todo_id")?;
        let summary: String = row.try_get("summary")?;
        let project: Option<String> = row.try_get("project")?;
        let rank: f64 = row.try_get("rank")?;

        let snippet_row = sqlx::query(
            r#"
            SELECT snippet(todo_fts, 1, '<b>', '</b>', '...', 20) as snippet
            FROM todo_fts
            WHERE todo_fts MATCH ? AND todo_id = ?
            "#,
        )
        .bind(query)
        .bind(&id)
        .fetch_optional(pool)
        .await?;

        let snippet = snippet_row
            .and_then(|r| r.try_get("snippet").ok())
            .unwrap_or_else(|| summary.clone());

        let score = (1.0 / (1.0 + rank.abs())).min(1.0);

        results.push(FtsSearchResult {
            id: Uuid::parse_str(&id)?,
            summary,
            project,
            score,
            snippet,
        });
    }

    Ok(results)
}

pub struct CreateRecurringTaskParams {
    pub summary: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
    pub recurrence_pattern: String,
    pub recurrence_end: Option<String>,
}

pub async fn create_recurring_task(pool: &SqlitePool, params: CreateRecurringTaskParams) -> Result<TodoItem> {
    let id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let status = TaskStatus::Todo;
    let priority = params
        .priority
        .and_then(|p| Priority::from_str(&p))
        .unwrap_or(Priority::Medium);
    let due_date = params
        .due_date
        .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
        .transpose()?;
    let recurrence_end = params
        .recurrence_end
        .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)))
        .transpose()?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO todo_items (id, summary, description, status, priority, project, due_date, created_at, updated_at, recurrence_pattern, recurrence_end)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(id.to_string())
    .bind(&params.summary)
    .bind(&params.description)
    .bind(status.as_str().to_string())
    .bind(priority.as_str().to_string())
    .bind(&params.project)
    .bind(due_date.map(|d| d.to_rfc3339()))
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(&params.recurrence_pattern)
    .bind(recurrence_end.map(|d| d.to_rfc3339()))
    .execute(&mut *tx)
    .await?;

    if let Some(tags) = params.tags {
        for tag in tags {
            sqlx::query("INSERT INTO tags (todo_id, name) VALUES ($1, $2)")
                .bind(id.to_string())
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    get_task_by_id(pool, id.to_string()).await
}

pub async fn get_recurring_tasks(pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at, t.recurrence_pattern, t.recurrence_end
        FROM todo_items t
        WHERE t.is_deleted = 0 AND t.recurrence_pattern IS NOT NULL
        ORDER BY t.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows_to_recurring_tasks(rows, pool).await
}

pub async fn delete_recurring_task(pool: &SqlitePool, id: String) -> Result<()> {
    uuid::Uuid::parse_str(&id)?;
    sqlx::query("DELETE FROM todo_items WHERE id = ? AND recurrence_pattern IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn process_recurring_tasks(pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let now = Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();

    let rows = sqlx::query(
        r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at, t.recurrence_pattern, t.recurrence_end
        FROM todo_items t
        WHERE t.is_deleted = 0 AND t.recurrence_pattern IS NOT NULL
        AND t.recurrence_pattern != ''
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut created = Vec::new();

    for row in &rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let recurrence_pattern: String = row.try_get("recurrence_pattern")?;
        let recurrence_end: Option<String> = row.try_get("recurrence_end")?;

        if let Some(end_str) = &recurrence_end {
            if let Ok(end_dt) = chrono::DateTime::parse_from_rfc3339(end_str) {
                if end_dt.with_timezone(&Utc) < now {
                    continue;
                }
            }
        }

        let pattern = RecurrencePattern::from_str(&recurrence_pattern).unwrap_or(RecurrencePattern::Daily);

        let last_instance = sqlx::query(
            r#"
            SELECT MAX(created_at) as last_created
            FROM todo_items
            WHERE summary = ? AND id != ? AND created_at >= ?
            "#,
        )
        .bind(&summary)
        .bind(&id)
        .bind(today_start.to_rfc3339())
        .fetch_optional(pool)
        .await?;

        if last_instance.is_some() {
            continue;
        }

        let new_due_date = match pattern {
            RecurrencePattern::Daily => due_date.map(|d| {
                chrono::DateTime::parse_from_rfc3339(&d)
                    .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::days(1))
            }).transpose()?,
            RecurrencePattern::Weekly => due_date.map(|d| {
                chrono::DateTime::parse_from_rfc3339(&d)
                    .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::weeks(1))
            }).transpose()?,
            RecurrencePattern::Biweekly => due_date.map(|d| {
                chrono::DateTime::parse_from_rfc3339(&d)
                    .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::weeks(2))
            }).transpose()?,
            RecurrencePattern::Monthly => due_date.map(|d| {
                chrono::DateTime::parse_from_rfc3339(&d)
                    .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::days(30))
            }).transpose()?,
            RecurrencePattern::Yearly => due_date.map(|d| {
                chrono::DateTime::parse_from_rfc3339(&d)
                    .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::days(365))
            }).transpose()?,
        };

        let new_id = uuid::Uuid::new_v4();
        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO todo_items (id, summary, description, status, priority, project, due_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(new_id.to_string())
        .bind(&summary)
        .bind(&description)
        .bind(TaskStatus::Todo.as_str().to_string())
        .bind(priority)
        .bind(&project)
        .bind(new_due_date.map(|d| d.to_rfc3339()))
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        for tag in &tags {
            sqlx::query("INSERT INTO tags (todo_id, name) VALUES ($1, $2)")
                .bind(new_id.to_string())
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        if let Ok(task) = get_task_by_id(pool, new_id.to_string()).await {
            created.push(task);
        }
    }

    Ok(created)
}

async fn rows_to_recurring_tasks(rows: Vec<sqlx::sqlite::SqliteRow>, pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let mut results = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let created_at: String = row.try_get("created_at")?;
        let updated_at: String = row.try_get("updated_at")?;
        let completed_at: Option<String> = row.try_get("completed_at")?;
        let recurrence_pattern: Option<String> = row.try_get("recurrence_pattern")?;
        let recurrence_end: Option<String> = row.try_get("recurrence_end")?;

        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        results.push(TodoItem {
            id: Uuid::parse_str(&id)?,
            summary,
            description,
            status: TaskStatus::from_str(&status).unwrap_or(TaskStatus::Todo),
            priority: Priority::from_str(&priority).unwrap_or(Priority::Medium),
            project,
            tags,
            due_date: due_date
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            completed_at: completed_at
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            recurrence_pattern: recurrence_pattern.and_then(|s| RecurrencePattern::from_str(&s)),
            recurrence_end: recurrence_end
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
        });
    }
    Ok(results)
}

pub async fn soft_delete_task(pool: &SqlitePool, id: String) -> Result<()> {
    let now = Utc::now();
    let mut conn = pool.acquire().await?;
    sqlx::query(
        r#"
        UPDATE todo_items SET is_deleted = 1, deleted_at = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(&id)
    .execute(&mut *conn)
    .await?;
    drop(conn);
    Ok(())
}

pub async fn undo_delete(pool: &SqlitePool, id: String) -> Result<TodoItem> {
    let now = Utc::now();
    let mut conn = pool.acquire().await?;
    sqlx::query(
        r#"
        UPDATE todo_items SET is_deleted = 0, deleted_at = NULL, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(&id)
    .execute(&mut *conn)
    .await?;
    drop(conn);
    get_task_by_id(pool, id).await
}

pub async fn list_deleted(pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at, t.recurrence_pattern, t.recurrence_end, t.deleted_at
        FROM todo_items t
        WHERE t.is_deleted = 1
        ORDER BY t.deleted_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows_to_deleted_tasks(rows, pool).await
}

pub async fn purge_deleted(pool: &SqlitePool, ids: Option<Vec<String>>) -> Result<BatchResult> {
    if let Some(ids) = ids {
        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("DELETE FROM todo_items WHERE id IN ({}) AND is_deleted = 1", placeholders);

        let mut db_query = sqlx::query(&query);
        for id in &ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids,
        })
    } else {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM todo_items WHERE is_deleted = 1")
            .fetch_all(pool)
            .await?;
        let ids: Vec<String> = rows.into_iter().map(|r| r.0).collect();

        if ids.is_empty() {
            return Ok(BatchResult { affected_count: 0, affected_ids: vec![] });
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("DELETE FROM todo_items WHERE id IN ({})", placeholders);

        let mut db_query = sqlx::query(&query);
        for id in &ids {
            db_query = db_query.bind(id);
        }

        let result = db_query.execute(pool).await?;
        let affected = result.rows_affected() as i64;

        Ok(BatchResult {
            affected_count: affected,
            affected_ids: ids,
        })
    }
}

async fn rows_to_deleted_tasks(rows: Vec<sqlx::sqlite::SqliteRow>, pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let mut results = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let summary: String = row.try_get("summary")?;
        let description: Option<String> = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let priority: String = row.try_get("priority")?;
        let project: Option<String> = row.try_get("project")?;
        let due_date: Option<String> = row.try_get("due_date")?;
        let created_at: String = row.try_get("created_at")?;
        let updated_at: String = row.try_get("updated_at")?;
        let completed_at: Option<String> = row.try_get("completed_at")?;
        let recurrence_pattern: Option<String> = row.try_get("recurrence_pattern")?;
        let recurrence_end: Option<String> = row.try_get("recurrence_end")?;

        let tags: Vec<String> = sqlx::query_scalar("SELECT name FROM tags WHERE todo_id = ?")
            .bind(&id)
            .fetch_all(pool)
            .await?;

        results.push(TodoItem {
            id: Uuid::parse_str(&id)?,
            summary,
            description,
            status: TaskStatus::from_str(&status).unwrap_or(TaskStatus::Todo),
            priority: Priority::from_str(&priority).unwrap_or(Priority::Medium),
            project,
            tags,
            due_date: due_date
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            completed_at: completed_at
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
            recurrence_pattern: recurrence_pattern.and_then(|s| RecurrencePattern::from_str(&s)),
            recurrence_end: recurrence_end
                .map(|d| chrono::DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&chrono::Utc)))
                .transpose()?,
        });
    }
    Ok(results)
}

pub async fn archive_task(pool: &SqlitePool, id: String) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE todo_items SET is_archived = 1, archived_at = ?, updated_at = ? WHERE id = ? AND is_deleted = 0 AND is_archived = 0"
    )
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Task not found or already archived"));
    }

    Ok(())
}

pub async fn unarchive_task(pool: &SqlitePool, id: String) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE todo_items SET is_archived = 0, archived_at = NULL, updated_at = ? WHERE id = ? AND is_archived = 1"
    )
    .bind(&now)
    .bind(&id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Task not found or not archived"));
    }

    Ok(())
}

pub async fn list_archived(pool: &SqlitePool) -> Result<Vec<TodoItem>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.summary, t.description, t.status, t.priority, t.project, t.due_date, t.created_at, t.updated_at, t.completed_at
        FROM todo_items t
        WHERE t.is_archived = 1
        ORDER BY t.archived_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows_to_tasks(rows, pool).await
}
