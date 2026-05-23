use dashmap::DashMap;
use std::sync::Arc;
use tracing::error;
use crate::job::{Job, JobState};

/// Dead-letter queue for jobs that exceeded max retry attempts.
pub struct DeadLetterQueue {
    jobs: Arc<DashMap<String, Job>>,
}

impl DeadLetterQueue {
    pub fn new() -> Self {
        Self { jobs: Arc::new(DashMap::new()) }
    }

    /// Move a failed job to the DLQ.
    pub fn send_to_dlq(&self, mut job: Job, reason: String) {
        error!(
            job_id = %job.id,
            kind = %job.kind,
            attempts = job.attempt,
            reason = %reason,
            "job sent to dead-letter queue"
        );
        job.state = JobState::DeadLettered;
        self.jobs.insert(job.id.clone(), job);
    }

    pub fn list(&self) -> Vec<Job> {
        self.jobs.iter().map(|e| e.value().clone()).collect()
    }

    pub fn get(&self, job_id: &str) -> Option<Job> {
        self.jobs.get(job_id).map(|e| e.clone())
    }

    /// Re-enqueue a job from DLQ for manual retry.
    pub fn requeue(&self, job_id: &str) -> Option<Job> {
        self.jobs.remove(job_id).map(|(_, mut job)| {
            job.state = JobState::Pending;
            job.attempt = 0;
            job
        })
    }

    pub fn remove(&self, job_id: &str) {
        self.jobs.remove(job_id);
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }
}

impl Default for DeadLetterQueue {
    fn default() -> Self {
        Self::new()
    }
}
