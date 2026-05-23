use async_trait::async_trait;
use std::sync::Arc;
use tokio::time;
use tracing::{error, info, warn};
use crate::dlq::DeadLetterQueue;
use crate::job::{Job, JobState};
use crate::queue::JobQueue;
use crate::retry::next_delay;

/// Implement this trait to handle a specific job kind.
#[async_trait]
pub trait JobHandler: Send + Sync + 'static {
    fn job_kind(&self) -> &str;
    async fn handle(&self, job: &Job) -> Result<(), String>;
}

pub struct WorkerPool {
    queue: Arc<JobQueue>,
    dlq: Arc<DeadLetterQueue>,
    handlers: Arc<Vec<Arc<dyn JobHandler>>>,
    worker_count: usize,
}

impl WorkerPool {
    pub fn new(queue: Arc<JobQueue>, dlq: Arc<DeadLetterQueue>, worker_count: usize) -> Self {
        Self { queue, dlq, handlers: Arc::new(Vec::new()), worker_count }
    }

    pub fn add_handler(mut self, handler: Arc<dyn JobHandler>) -> Self {
        Arc::get_mut(&mut self.handlers)
            .expect("handlers arc must be unique at build time")
            .push(handler);
        self
    }

    /// Start the worker pool.
    pub async fn start(self) {
        let queue = self.queue;
        let dlq = self.dlq;
        let handlers = self.handlers;

        for worker_id in 0..self.worker_count {
            let q = queue.clone();
            let d = dlq.clone();
            let h = handlers.clone();

            tokio::spawn(async move {
                info!("worker {} started", worker_id);
                loop {
                    if let Some(job) = q.pop().await {
                        Self::process_job(worker_id, job, &q, &d, &h).await;
                    } else {
                        time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            });
        }
    }

    async fn process_job(
        worker_id: usize,
        mut job: Job,
        queue: &JobQueue,
        dlq: &DeadLetterQueue,
        handlers: &[Arc<dyn JobHandler>],
    ) {
        let handler = handlers.iter().find(|h| h.job_kind() == job.kind);
        if handler.is_none() {
            warn!(worker = worker_id, job_id = %job.id, kind = %job.kind, "no handler for job kind");
            return;
        }
        let handler = handler.unwrap();

        job.state = JobState::Running;
        job.attempt += 1;
        job.started_at = Some(chrono::Utc::now());

        info!(worker = worker_id, job_id = %job.id, kind = %job.kind, attempt = job.attempt, "processing job");

        match handler.handle(&job).await {
            Ok(_) => {
                info!(worker = worker_id, job_id = %job.id, "job completed");
                job.state = JobState::Completed;
                job.completed_at = Some(chrono::Utc::now());
            }
            Err(reason) => {
                if job.is_retryable() {
                    let delay = next_delay(job.attempt, 5, 300);
                    warn!(
                        worker = worker_id,
                        job_id = %job.id,
                        attempt = job.attempt,
                        delay_secs = delay.as_secs(),
                        reason = %reason,
                        "job failed, will retry"
                    );
                    job.state = JobState::Pending;
                    job.scheduled_at = chrono::Utc::now() + chrono::Duration::seconds(delay.as_secs() as i64);
                    time::sleep(delay).await;
                    queue.push(job).await;
                } else {
                    error!(worker = worker_id, job_id = %job.id, reason = %reason, "job exhausted retries");
                    dlq.send_to_dlq(job, reason);
                }
            }
        }
    }
}
