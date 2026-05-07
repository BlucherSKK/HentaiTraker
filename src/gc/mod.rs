use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::admin::metric::ServerState;
use crate::db::Store;
use crate::lfs::UPLOADS_DIR;

// ----- intervals -----

const INTERVAL_SUCCESS: Duration = Duration::from_secs(24 * 3600);
const INTERVAL_SKIP:    Duration = Duration::from_secs(3600);

// ----- public api -----

pub fn start(store: Arc<Store>, srv_state: ServerState, threshold: u64) {
    tokio::spawn(async move {
        let mut delay = INTERVAL_SUCCESS;
        loop {
            sleep(delay).await;

            let online = srv_state.get_users_online().await;
            if online >= threshold {
                info!("gc: skipped ({online} online, threshold {threshold})");
                delay = INTERVAL_SKIP;
                continue;
            }

            match collect(&store).await {
                Ok(n) => {
                    info!("gc: collected {n} orphan file(s)");
                    delay = INTERVAL_SUCCESS;
                }
                Err(e) => {
                    error!("gc: failed — {e}");
                    delay = INTERVAL_SKIP;
                }
            }
        }
    });
}

// ----- internals -----

async fn collect(store: &Store) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let referenced = store.get_all_referenced_filenames().await?;

    let mut dir     = tokio::fs::read_dir(UPLOADS_DIR).await?;
    let mut deleted = 0usize;

    while let Some(entry) = dir.next_entry().await? {
        let raw  = entry.file_name();
        let name = raw.to_string_lossy();

        if !referenced.contains(name.as_ref()) {
            match tokio::fs::remove_file(entry.path()).await {
                Ok(_)  => deleted += 1,
                Err(e) => warn!("gc: cannot remove {name}: {e}"),
            }
        }
    }

    Ok(deleted)
}
