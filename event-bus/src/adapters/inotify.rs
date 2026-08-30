//! inotify adapter for filesystem change events.

use super::publish_event;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::path::PathBuf;
use std::sync::mpsc;
use tracing::{info, warn};

pub async fn run() {
    let watches = default_watches();
    if watches.is_empty() {
        info!("inotify: no watch paths configured");
        return;
    }
    tokio::task::spawn_blocking(move || run_watcher(watches));
}

fn default_watches() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(v) = std::env::var("THE_MACHINE_FS_WATCHES") {
        for p in v.split(':') {
            if !p.is_empty() {
                paths.push(PathBuf::from(p));
            }
        }
    }
    for p in ["/tmp/the-machine/downloads", "/tmp/Downloads"] {
        let pb = PathBuf::from(p);
        if pb.exists() {
            paths.push(pb);
        }
    }
    paths
}

fn run_watcher(paths: Vec<PathBuf>) {
    let (tx, rx) = mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            warn!("inotify watcher failed: {}", e);
            return;
        }
    };
    for path in &paths {
        if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
            warn!("cannot watch {}: {}", path.display(), e);
        } else {
            info!("inotify watching {}", path.display());
        }
    }
    while let Ok(res) = rx.recv() {
        match res {
            Ok(event) => {
                let pattern = match event.kind {
                    EventKind::Create(_) => "create",
                    EventKind::Modify(_) => "modify",
                    EventKind::Remove(_) => "remove",
                    _ => "change",
                };
                let path = event
                    .paths
                    .first()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let payload = json!({ "path": path, "kind": format!("{:?}", event.kind) });
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(publish_event(
                        "filesystem",
                        &format!("fs.change.{}", pattern),
                        payload,
                    ));
                }
            }
            Err(e) => warn!("inotify error: {}", e),
        }
    }
}
