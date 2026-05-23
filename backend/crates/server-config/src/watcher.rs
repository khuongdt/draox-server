use crate::loader::ConfigLoader;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Watches the config file for changes and triggers reload.
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    cancel_tx: mpsc::Sender<()>,
}

impl ConfigWatcher {
    /// Start watching a config file. On changes, calls `loader.reload()`.
    pub fn start(loader: Arc<Mutex<ConfigLoader>>) -> server_core::Result<Self> {
        let path = {
            let l = loader.lock().unwrap();
            l.path().to_path_buf()
        };

        if path.as_os_str().is_empty() {
            return Err(server_core::Error::Config(
                "no config file path — cannot watch".to_string(),
            ));
        }

        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        let (event_tx, mut event_rx) = mpsc::channel::<()>(4);

        // File watcher
        let tx_clone = event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    ) {
                        let _ = tx_clone.try_send(());
                    }
                }
                Err(e) => warn!("file watch error: {e}"),
            }
        })
        .map_err(|e| server_core::Error::Config(format!("failed to create file watcher: {e}")))?;

        let watch_path = path.parent().unwrap_or(Path::new("."));
        watcher
            .watch(watch_path, RecursiveMode::NonRecursive)
            .map_err(|e| server_core::Error::Config(format!("failed to watch {}: {e}", watch_path.display())))?;

        info!(path = %path.display(), "config file watcher started");

        // Debounced reload task
        tokio::spawn(async move {
            let debounce = Duration::from_millis(500);
            let mut last_reload = Instant::now() - debounce;

            loop {
                tokio::select! {
                    Some(()) = event_rx.recv() => {
                        // Debounce: skip if reloaded recently
                        if last_reload.elapsed() < debounce {
                            continue;
                        }
                        last_reload = Instant::now();

                        info!("config file changed, reloading...");
                        match loader.lock() {
                            Ok(mut l) => {
                                if let Err(e) = l.reload() {
                                    error!("config reload failed: {e}");
                                } else {
                                    info!("config reloaded successfully");
                                }
                            }
                            Err(e) => error!("config lock poisoned: {e}"),
                        }
                    }
                    _ = cancel_rx.recv() => {
                        info!("config watcher stopped");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            cancel_tx,
        })
    }

    /// Stop watching.
    pub async fn stop(self) {
        let _ = self.cancel_tx.send(()).await;
    }
}
