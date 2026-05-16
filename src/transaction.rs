#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub sender: String,
    nonce: u64,
    pub fee: u64,
    data: Vec<u8>,
}

impl Transaction {
    pub fn new(sender: String, nonce: u64, fee: u64, data: Vec<u8>) -> Self {
        Transaction {
            sender,
            nonce,
            fee,
            data,
        }
    }

    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }
}
