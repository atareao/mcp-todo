use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: Uuid,
    pub summary: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recurrence_pattern: Option<RecurrencePattern>,
    pub recurrence_end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecurrencePattern {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Yearly,
}

impl RecurrencePattern {
    pub fn as_str(&self) -> &str {
        match self {
            RecurrencePattern::Daily => "daily",
            RecurrencePattern::Weekly => "weekly",
            RecurrencePattern::Biweekly => "biweekly",
            RecurrencePattern::Monthly => "monthly",
            RecurrencePattern::Yearly => "yearly",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(RecurrencePattern::Daily),
            "weekly" => Some(RecurrencePattern::Weekly),
            "biweekly" => Some(RecurrencePattern::Biweekly),
            "monthly" => Some(RecurrencePattern::Monthly),
            "yearly" => Some(RecurrencePattern::Yearly),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "todo" => Some(TaskStatus::Todo),
            "in_progress" => Some(TaskStatus::InProgress),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(&self) -> &str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Priority::Low),
            "medium" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            _ => None,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidate {
    pub id: Uuid,
    pub summary: String,
    pub similarity: f64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchResult {
    pub id: Uuid,
    pub summary: String,
    pub project: Option<String>,
    pub score: f64,
    pub snippet: String,
}
