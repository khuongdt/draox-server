use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type JobId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for JobPriority {
    fn default() -> Self {
        JobPriority::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed { reason: String },
    DeadLettered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub kind: String,
    pub payload: serde_json::Value,
    pub priority: JobPriority,
    pub state: JobState,
    pub attempt: u32,
    pub max_attempts: u32,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub queue_name: String,
}

impl Job {
    pub fn new(kind: impl Into<String>, payload: serde_json::Value, queue_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: format!("job_{}", Uuid::new_v4().as_simple()),
            kind: kind.into(),
            payload,
            priority: JobPriority::Normal,
            state: JobState::Pending,
            attempt: 0,
            max_attempts: 3,
            created_at: now,
            scheduled_at: now,
            started_at: None,
            completed_at: None,
            queue_name: queue_name.into(),
        }
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    pub fn with_scheduled_at(mut self, at: DateTime<Utc>) -> Self {
        self.scheduled_at = at;
        self
    }

    pub fn is_retryable(&self) -> bool {
        self.attempt < self.max_attempts
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state, JobState::Completed | JobState::DeadLettered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_defaults() {
        let job = Job::new("send_email", serde_json::json!({"to": "x@y.com"}), "default");
        assert_eq!(job.state, JobState::Pending);
        assert_eq!(job.attempt, 0);
        assert!(job.is_retryable());
        assert!(!job.is_terminal());
    }
}
