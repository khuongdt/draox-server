use chrono::Utc;
use cron::Schedule;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};
use uuid::Uuid;
use crate::job::Job;
use crate::queue::JobQueue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobDefinition {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub job_kind: String,
    pub payload: serde_json::Value,
    pub queue_name: String,
    pub enabled: bool,
}

impl CronJobDefinition {
    pub fn new(
        name: String,
        cron_expr: String,
        job_kind: String,
        payload: serde_json::Value,
        queue_name: String,
    ) -> Self {
        Self {
            id: format!("cron_{}", Uuid::new_v4().as_simple()),
            name,
            cron_expr,
            job_kind,
            payload,
            queue_name,
            enabled: true,
        }
    }

    pub fn next_run(&self) -> Option<chrono::DateTime<Utc>> {
        Schedule::from_str(&self.cron_expr)
            .ok()?
            .upcoming(Utc)
            .next()
    }
}

/// Cron-style job scheduler that enqueues jobs based on expressions.
pub struct JobScheduler {
    definitions: Arc<DashMap<String, CronJobDefinition>>,
    queues: Arc<DashMap<String, JobQueue>>,
}

impl JobScheduler {
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(DashMap::new()),
            queues: Arc::new(DashMap::new()),
        }
    }

    pub fn register_queue(&self, queue: JobQueue) {
        self.queues.insert(queue.name().to_string(), queue);
    }

    pub fn add_job(&self, def: CronJobDefinition) {
        info!(id = %def.id, name = %def.name, cron = %def.cron_expr, "cron job registered");
        self.definitions.insert(def.id.clone(), def);
    }

    pub fn remove_job(&self, id: &str) {
        self.definitions.remove(id);
    }

    pub fn list_jobs(&self) -> Vec<CronJobDefinition> {
        self.definitions.iter().map(|e| e.value().clone()).collect()
    }

    /// Run the scheduler loop. Check every `tick_secs` for due jobs.
    pub async fn run(self: Arc<Self>, tick_secs: u64) {
        let mut interval = time::interval(Duration::from_secs(tick_secs));
        info!("job scheduler started");
        loop {
            interval.tick().await;
            let now = Utc::now();

            let due: Vec<CronJobDefinition> = self
                .definitions
                .iter()
                .filter(|e| e.enabled)
                .filter(|e| {
                    e.next_run()
                        .map(|next| {
                            let diff = (next - now).num_seconds().abs();
                            diff <= tick_secs as i64
                        })
                        .unwrap_or(false)
                })
                .map(|e| e.value().clone())
                .collect();

            for def in due {
                if let Some(queue) = self.queues.get(&def.queue_name) {
                    let job = Job::new(def.job_kind.clone(), def.payload.clone(), &def.queue_name);
                    queue.push(job).await;
                    info!(cron_id = %def.id, name = %def.name, "cron job enqueued");
                } else {
                    warn!(queue = %def.queue_name, "cron job queue not found");
                }
            }
        }
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}
