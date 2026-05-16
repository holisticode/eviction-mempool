use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tracing::info;

use crate::mempool::Mempool;

const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub async fn block_builder(mempool: Arc<RwLock<Mempool>>) {
    let mut interval = time::interval(SNAPSHOT_POLL_INTERVAL);

    loop {
        interval.tick().await;
        // Your async code here
        let snapshot = {
            let mpool = mempool.read().await;
            mpool.snapshot(5)
        };
        info!("Block Builder: Snapshot of top transactions:");
        for tx in snapshot {
            info!(
                "sender: {} nonce: {} fee: {}",
                tx.sender,
                tx.get_nonce(),
                tx.fee,
            );
        }
    }
}
