use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait StateProvider: Send + Sync {
    async fn get_nonce(&self, sender: &str) -> Option<u64>;
    async fn get_balance(&self, sender: &str) -> Option<u64>;
}

pub struct MockStateProvider {
    nonces: std::collections::HashMap<String, u64>,
    balances: std::collections::HashMap<String, u64>,
}

impl MockStateProvider {
    pub fn new() -> Self {
        Self {
            nonces: std::collections::HashMap::new(),
            balances: std::collections::HashMap::new(),
        }
    }

    pub fn new_with_values(nonces: HashMap<String, u64>, balances: HashMap<String, u64>) -> Self {
        Self { nonces, balances }
    }
}

#[async_trait]
impl StateProvider for MockStateProvider {
    async fn get_nonce(&self, sender: &str) -> Option<u64> {
        self.nonces.get(sender).copied()
    }

    async fn get_balance(&self, sender: &str) -> Option<u64> {
        self.balances.get(sender).copied()
    }
}
