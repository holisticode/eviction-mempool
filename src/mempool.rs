use crate::state::StateProvider;
use crate::transaction::Transaction;

use crate::errors::MempoolError;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tracing::{error, info};

pub struct Mempool {
    transactions: BTreeMap<u64, Vec<Transaction>>,
    capacity: usize,
    size: usize,
    min_fee: u64,

    eviction_ch: Option<tokio::sync::broadcast::Sender<Transaction>>,
    state_provider: Arc<dyn StateProvider>,

    eviction_count: AtomicU64,
    rejection_count: AtomicU64,
}

impl Mempool {
    pub fn new(capacity: usize, min_fee: u64, state_provider: Arc<dyn StateProvider>) -> Self {
        Mempool {
            transactions: BTreeMap::new(),
            capacity,
            min_fee,
            size: 0,

            eviction_ch: None,
            state_provider,

            eviction_count: AtomicU64::new(0),
            rejection_count: AtomicU64::new(0),
        }
    }
    pub fn subscribe_evictions(&mut self) -> broadcast::Receiver<Transaction> {
        let (sender, receiver) = broadcast::channel(10);
        self.eviction_ch = Some(sender);
        receiver
    }

    pub async fn insert(&mut self, tx: Transaction) -> Result<(), MempoolError> {
        let onchain_nonce: u64 = self
            .state_provider
            .get_nonce(&tx.sender)
            .await
            .unwrap_or_default();

        if tx.get_nonce() < onchain_nonce {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return Err(MempoolError::UnexpectedNonce {
                sender: tx.sender.clone(),
                expected: onchain_nonce,
                got: tx.get_nonce(),
            });
        }

        match self.state_provider.get_balance(&tx.sender).await {
            Some(onchain_balance) if tx.fee > onchain_balance => {
                self.rejection_count.fetch_add(1, Ordering::Relaxed);
                return Err(MempoolError::InsufficientBalance {
                    sender: tx.sender.clone(),
                    balance: onchain_balance,
                    fee: tx.fee,
                });
            }
            _ => {}
        }
        if tx.fee < self.min_fee {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return Err(MempoolError::FeeBelowMinimum {
                fee: tx.fee,
                min_fee: self.min_fee,
            });
        }

        self.duplicity_check(&tx)?;

        self.nonce_gap_check(&tx, onchain_nonce)?;

        if self.size >= self.capacity {
            let opt_worst_fee = { self.transactions.keys().next().copied() };
            if let Some(worst_fee) = opt_worst_fee {
                if tx.fee <= worst_fee {
                    return Err(MempoolError::Eviction {
                        fee: tx.fee,
                        worst_fee,
                    });
                }
                self.pop_worst();
            } else {
                return Err(MempoolError::PoolFull);
            }
        }
        let this_fee_txs = self.transactions.entry(tx.fee).or_insert_with(Vec::new);
        this_fee_txs.push(tx);
        self.size += 1;
        info!("Transaction added to mempool. Current size: {}", self.size);
        Ok(())
    }

    fn duplicity_check(&mut self, tx: &Transaction) -> Result<(), MempoolError> {
        // Phase 1: find — iterator lives and dies here
        let existing = self.transactions.iter().find_map(|(&fee, txs)| {
            txs.iter()
                .find(|t| t.sender == tx.sender && t.get_nonce() == tx.get_nonce())
                .map(|t| (fee, t.fee)) // copy out two u64s, no references escape
        });

        // Phase 2: mutate — no active borrows from phase 1
        if let Some((bucket_fee, old_fee)) = existing {
            if tx.fee <= old_fee {
                self.rejection_count.fetch_add(1, Ordering::Relaxed);
                return Err(MempoolError::Duplicate {
                    sender: tx.sender.clone(),
                    nonce: tx.get_nonce(),
                    existing_fee: old_fee,
                    new_fee: tx.fee,
                });
            }
            if let Some(txs) = self.transactions.get_mut(&bucket_fee) {
                txs.retain(|t| !(t.sender == tx.sender && t.get_nonce() == tx.get_nonce()));
                if txs.is_empty() {
                    self.transactions.remove(&bucket_fee);
                }
            }
            self.size -= 1;
        }

        Ok(())
    }

    fn nonce_gap_check(
        &mut self,
        tx: &Transaction,
        onchain_nonce: u64,
    ) -> Result<(), MempoolError> {
        let existing = self
            .transactions
            .values()
            .flatten()
            .filter(|t| t.sender == tx.sender)
            .max_by_key(|t| t.get_nonce());

        let expected = match existing {
            Some(v) => v.get_nonce() + 1,
            None => onchain_nonce,
        };

        if tx.get_nonce() != expected {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return Err(MempoolError::NonceGap {
                sender: tx.sender.clone(),
                expected,
                got: tx.get_nonce(),
            });
        }
        Ok(())
    }

    pub fn pop_worst(&mut self) -> Option<Transaction> {
        if self.size == 0 {
            return None;
        }
        let (&fee, txs) = self.transactions.iter_mut().next()?;
        let tx = txs.pop()?;
        if txs.is_empty() {
            self.transactions.remove(&fee);
        }
        self.size -= 1;
        self.eviction_count.fetch_add(1, Ordering::Relaxed);
        info!(
            "Worst transaction popped from mempool. Current size: {}",
            self.size
        );
        if let Some(eviction_ch) = &self.eviction_ch {
            match eviction_ch.send(tx.clone()) {
                Ok(_) => info!(
                    "Eviction notification sent for transaction with fee {}",
                    fee
                ),
                Err(e) => error!("Failed to send eviction notification: {}", e),
            }
        }
        Some(tx)
    }

    pub fn pop_best(&mut self) -> Option<Transaction> {
        if self.size == 0 {
            return None;
        }
        let (&fee, txs) = self.transactions.iter_mut().next_back()?;
        let tx = txs.pop()?;
        if txs.is_empty() {
            self.transactions.remove(&fee);
        }
        self.size -= 1;
        info!(
            "Best transaction popped from mempool. Current size: {}",
            self.size
        );
        Some(tx)
    }

    pub fn snapshot(&self, top_n: usize) -> Vec<Transaction> {
        self.transactions
            .values()
            .rev()
            .flatten()
            .take(top_n)
            .cloned()
            .collect()
    }

    pub async fn get_transactions(&self) -> Vec<Transaction> {
        self.transactions.values().flatten().cloned().collect()
    }

    pub fn avg_fee(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        let total_fee: u64 = self.transactions.values().flatten().map(|t| t.fee).sum();

        total_fee as f64 / self.size as f64
    }

    pub fn metrics(&self) -> (u64, u64, f64) {
        (
            self.eviction_count.load(Ordering::Relaxed),
            self.rejection_count.load(Ordering::Relaxed),
            self.avg_fee(),
        )
    }
}

#[cfg(test)]
#[path = "mempool_tests.rs"]
mod tests;
