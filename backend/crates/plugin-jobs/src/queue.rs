use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::job::{Job, JobPriority};

/// Priority queue wrapper for in-process job scheduling.
/// For durable queues, use Redis Streams (see RedisJobQueue).
#[derive(Clone)]
pub struct JobQueue {
    name: String,
    heap: Arc<Mutex<BinaryHeap<PriorityJob>>>,
}

#[derive(Debug)]
struct PriorityJob {
    priority: JobPriority,
    created_order: u64,
    job: Job,
}

impl PartialEq for PriorityJob {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.created_order == other.created_order
    }
}

impl Eq for PriorityJob {}

impl PartialOrd for PriorityJob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityJob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first; for equal priority, FIFO (lower created_order first)
        self.priority.cmp(&other.priority)
            .then(other.created_order.cmp(&self.created_order))
    }
}

impl JobQueue {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn push(&self, job: Job) {
        let priority = job.priority;
        let seq = job.created_at.timestamp_nanos_opt().unwrap_or(0) as u64;
        let mut heap = self.heap.lock().await;
        heap.push(PriorityJob { priority, created_order: seq, job });
    }

    pub async fn pop(&self) -> Option<Job> {
        let mut heap = self.heap.lock().await;
        heap.pop().map(|pj| pj.job)
    }

    pub async fn len(&self) -> usize {
        self.heap.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.heap.lock().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_priority_ordering() {
        let q = JobQueue::new("test");
        let low = Job::new("task", json!({}), "test").with_priority(JobPriority::Low);
        let high = Job::new("task", json!({}), "test").with_priority(JobPriority::High);
        let normal = Job::new("task", json!({}), "test").with_priority(JobPriority::Normal);

        q.push(low).await;
        q.push(normal).await;
        q.push(high).await;

        let first = q.pop().await.unwrap();
        assert_eq!(first.priority, JobPriority::High);
        let second = q.pop().await.unwrap();
        assert_eq!(second.priority, JobPriority::Normal);
        let third = q.pop().await.unwrap();
        assert_eq!(third.priority, JobPriority::Low);
    }
}
