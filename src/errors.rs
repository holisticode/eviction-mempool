use std::fmt;

#[derive(Debug)]
pub enum MempoolError {
    FeeBelowMinimum {
        fee: u64,
        min_fee: u64,
    },
    Eviction {
        fee: u64,
        worst_fee: u64,
    },
    PoolFull,
    NonceGap {
        sender: String,
        expected: u64,
        got: u64,
    },
    Duplicate {
        sender: String,
        nonce: u64,
        existing_fee: u64,
        new_fee: u64,
    },
    UnexpectedNonce {
        sender: String,
        expected: u64,
        got: u64,
    },
    InsufficientBalance {
        sender: String,
        balance: u64,
        fee: u64,
    },
    UnknownSender {
        sender: String,
    },
}

impl fmt::Display for MempoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MempoolError::FeeBelowMinimum { fee, min_fee } => {
                write!(f, "transaction fee {} is below minimum {}", fee, min_fee)
            }
            MempoolError::Eviction { fee, worst_fee } => write!(
                f,
                "pool full: fee {} does not outbid worst fee {}",
                fee, worst_fee
            ),
            MempoolError::PoolFull => write!(f, "pool full"),
            MempoolError::NonceGap {
                sender,
                expected,
                got,
            } => write!(
                f,
                "nonce gap for {}: expected {}, got {}",
                sender, expected, got
            ),
            MempoolError::Duplicate {
                sender,
                nonce,
                existing_fee,
                new_fee,
            } => write!(
                f,
                "duplicate (sender={}, nonce={}): new fee {} must exceed existing fee {}",
                sender, nonce, new_fee, existing_fee
            ),
            MempoolError::UnexpectedNonce {
                sender,
                expected,
                got,
            } => write!(
                f,
                "Unexpected nonce for {}: expected {}, got {}",
                sender, expected, got
            ),
            MempoolError::InsufficientBalance {
                sender,
                balance,
                fee,
            } => write!(
                f,
                "Insufficient balance for {}: balance {}, fee {}",
                sender, balance, fee
            ),
            MempoolError::UnknownSender { sender } => write!(f, "Unknown sender: {}", sender),
        }
    }
}

impl std::error::Error for MempoolError {}
