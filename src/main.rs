mod builder;
mod errors;
mod mempool;
mod state;
mod transaction;

use mempool::Mempool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

const TX_CREATION_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout)
        .init();

    let state_provider = Arc::new(state::MockStateProvider::new());
    let mpool = Arc::new(RwLock::new(Mempool::new(100, 5, state_provider)));
    let builder_handle = mpool.clone();
    let inject_handle = mpool.clone();

    let builder_thread = tokio::spawn(async {
        builder::block_builder(builder_handle).await;
    });

    let tx_injection_thread = tokio::spawn(async {
        inject_transactions(inject_handle).await;
    });

    tx_injection_thread.await.unwrap();
    builder_thread.abort();
}

async fn inject_transactions(mempool: Arc<RwLock<Mempool>>) {
    const NUM_TXS: usize = 10;
    for i in 1..NUM_TXS + 1 {
        let data = format!("test tx {}", i).as_bytes().to_vec();
        let tx =
            transaction::Transaction::new(format!("User{}", i), i as u64, (i * 10) as u64, data);
        match mempool.write().await.insert(tx.clone()).await {
            Ok(_) => info!("Transaction {} added successfully", tx.get_nonce()),
            Err(e_) => error!("Failed to add transaction {}: {}", tx.get_nonce(), e_),
        }
        tokio::time::sleep(TX_CREATION_INTERVAL).await;
    }
}
