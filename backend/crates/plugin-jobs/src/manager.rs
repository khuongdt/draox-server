use std::sync::Arc;
use crate::dlq::DeadLetterQueue;
use crate::job::Job;
use crate::queue::JobQueue;
use crate::scheduler::JobScheduler;
use crate::worker::{JobHandler, WorkerPool};

pub struct JobManager {
    default_queue: Arc<JobQueue>,
    dlq: Arc<DeadLetterQueue>,
    scheduler: Arc<JobScheduler>,
    worker_count: usize,
    handlers: Vec<Arc<dyn JobHandler>>,
}

impl JobManager {
    pub fn new(worker_count: usize) -> Self {
        let default_queue = Arc::new(JobQueue::new("default"));
        let scheduler = Arc::new(JobScheduler::new());
        scheduler.register_queue((*default_queue).clone());

        Self {
            default_queue,
            dlq: Arc::new(DeadLetterQueue::new()),
            scheduler,
            worker_count,
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(mut self, handler: Arc<dyn JobHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Enqueue a job to the default queue.
    pub async fn enqueue(&self, job: Job) {
        self.default_queue.push(job).await;
    }

    /// Start the worker pool and scheduler.
    pub fn start(self: Arc<Self>) {
        let pool = {
            let mut p = WorkerPool::new(
                self.default_queue.clone(),
                self.dlq.clone(),
                self.worker_count,
            );
            for h in &self.handlers {
                p = p.add_handler(h.clone());
            }
            p
        };

        let scheduler = self.scheduler.clone();

        tokio::spawn(async move { pool.start().await });
        tokio::spawn(async move { scheduler.run(60).await });
    }

    pub fn scheduler(&self) -> &JobScheduler {
        &self.scheduler
    }

    pub fn dlq(&self) -> &DeadLetterQueue {
        &self.dlq
    }

    pub async fn queue_depth(&self) -> usize {
        self.default_queue.len().await
    }
}
