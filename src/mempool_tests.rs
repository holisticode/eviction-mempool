use super::Mempool;
use crate::errors::MempoolError;
use crate::transaction::Transaction;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio_test::assert_ok;

fn mock_mempool(capacity: usize, min_fee: u64) -> Mempool {
    Mempool::new(
        capacity,
        min_fee,
        Arc::new(crate::state::MockStateProvider::new()),
    )
}

#[tokio::test]
async fn test_pop_best_empty() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    assert!(mempool.pop_best().is_none());
    Ok(())
}

#[tokio::test]
async fn test_pop_best_highest_fee() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            1,
            20,
            "test2".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Bob"),
            0,
            15,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    let best_tx = mempool.pop_best().unwrap();
    assert_eq!(best_tx.fee, 20);
    Ok(())
}

#[tokio::test]
async fn test_pop_best_multi_same_fee() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            50,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Bob"),
            0,
            50,
            "test2".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            0,
            10,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("David"),
            0,
            10,
            "test4".as_bytes().to_vec(),
        ))
        .await?;
    let mut best_tx = mempool.pop_best().unwrap();
    assert_eq!(best_tx.fee, 50);
    assert_eq!(mempool.size, 3);
    best_tx = mempool.pop_best().unwrap();
    assert_eq!(best_tx.fee, 50);
    assert_eq!(mempool.size, 2);
    best_tx = mempool.pop_best().unwrap();
    assert_eq!(best_tx.fee, 10);
    assert_eq!(mempool.size, 1);
    Ok(())
}

#[tokio::test]
async fn test_snapshot_descending() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Bob"),
            0,
            30,
            "test2".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            0,
            20,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    let snapshot = mempool.snapshot(3);
    assert_eq!(snapshot[0].fee, 30);
    assert_eq!(snapshot[1].fee, 20);
    assert_eq!(snapshot[2].fee, 10);
    Ok(())
}

#[tokio::test]
async fn test_snapshot_top_n() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Bob"),
            0,
            30,
            "test2".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            0,
            20,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            1,
            12,
            "test4".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            2,
            35,
            "test5".as_bytes().to_vec(),
        ))
        .await?;
    let snapshot = mempool.snapshot(3);
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot[0].fee, 35);
    assert_eq!(snapshot[1].fee, 30);
    assert_eq!(snapshot[2].fee, 20);
    Ok(())
}

#[tokio::test]
async fn test_snapsphot_no_mutate_pool() -> Result<(), Box<dyn Error>> {
    let mut mempool = mock_mempool(10, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Bob"),
            0,
            30,
            "test2".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Charlie"),
            0,
            20,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("David"),
            0,
            20,
            "test3".as_bytes().to_vec(),
        ))
        .await?;
    let snapshot = mempool.snapshot(10);
    assert_eq!(mempool.size, 4);
    assert_eq!(snapshot.len(), 4);
    Ok(())
}

#[tokio::test]
async fn test_reject_minfee() {
    let mut mempool = mock_mempool(10, 5);
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            4,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::FeeBelowMinimum { fee, min_fee }) => {
            assert_eq!(fee, 4);
            assert_eq!(min_fee, 5);
        }
        other => panic!("expected FeeBelowMinimum, got {:?}", other),
    }
}

#[tokio::test]
async fn test_eviction() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    for i in 0..capacity {
        mempool
            .insert(Transaction::new(
                String::from("Alice"),
                i as u64,
                6 + i as u64,
                format!("test{}", i).as_bytes().to_vec(),
            ))
            .await?;
    }
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            capacity as u64,
            8,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    assert_eq!(mempool.size, capacity);
    assert_eq!(mempool.transactions.keys().next(), Some(&7u64));
    Ok(())
}

#[tokio::test]
async fn test_no_eviction() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    for i in 0..capacity {
        mempool
            .insert(Transaction::new(
                String::from("Alice"),
                i as u64,
                6 + i as u64,
                format!("test{}", i).as_bytes().to_vec(),
            ))
            .await?;
    }
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            capacity as u64,
            6,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::Eviction { fee, worst_fee }) => {
            assert_eq!(fee, 6);
            assert_eq!(worst_fee, 6);
        }
        other => panic!("expected Eviction, got {:?}", other),
    }
    assert_eq!(mempool.size, capacity);
    assert_eq!(mempool.transactions.keys().next(), Some(&6u64));
    Ok(())
}

#[tokio::test]
async fn test_reject_lowerfee() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            20,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test2".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::Duplicate {
            sender,
            nonce,
            existing_fee,
            new_fee,
        }) => {
            assert_eq!(sender, "Alice");
            assert_eq!(nonce, 0);
            assert_eq!(existing_fee, 20);
            assert_eq!(new_fee, 10);
        }
        other => panic!("expected Duplicate, got {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn test_duplicate_higher_fee_replaces() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            20,
            "test2".as_bytes().to_vec(),
        ))
        .await;
    assert_ok!(res);
    assert_eq!(mempool.size, 1);
    assert_eq!(mempool.transactions.keys().next(), Some(&20u64));

    Ok(())
}

#[tokio::test]
async fn test_nonce_gap_rejected() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            3,
            20,
            "test2".as_bytes().to_vec(),
        ))
        .await;

    match res {
        Err(MempoolError::NonceGap {
            sender,
            expected,
            got,
        }) => {
            assert_eq!(sender, "Alice");
            assert_eq!(expected, 1);
            assert_eq!(got, 3);
        }
        other => panic!("expected NonceGap, got {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn test_nonce_sequential_accepted() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            1,
            20,
            "test2".as_bytes().to_vec(),
        ))
        .await;

    assert_ok!(res);
    assert_eq!(mempool.size, 2);
    Ok(())
}

#[tokio::test]
async fn test_eviction_notification() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    let mut rx = mempool.subscribe_evictions();
    for i in 0..capacity {
        mempool
            .insert(Transaction::new(
                String::from("Alice"),
                i as u64,
                6 + i as u64,
                format!("test{}", i).as_bytes().to_vec(),
            ))
            .await?;
    }
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            capacity as u64,
            8,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    assert_eq!(mempool.size, capacity);
    assert_eq!(mempool.transactions.keys().next(), Some(&7u64));
    assert_eq!(mempool.eviction_count.load(Ordering::Relaxed), 1);
    let recv = rx.recv().await.unwrap();
    assert_eq!(recv.fee, 6);
    assert_eq!(recv.sender, "Alice");
    assert_eq!(recv.get_nonce(), 0);
    Ok(())
}

#[tokio::test]
async fn test_stale_nonce_rejected() -> Result<(), Box<dyn Error>> {
    let balances = HashMap::from([(String::from("Alice"), 3)]);
    let nonces = HashMap::from([(String::from("Alice"), 3)]);
    let mut mempool = Mempool::new(
        10,
        5,
        Arc::new(crate::state::MockStateProvider::new_with_values(
            nonces, balances,
        )),
    );
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            2,
            8,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::UnexpectedNonce {
            sender,
            expected,
            got,
        }) => {
            assert_eq!(sender, "Alice");
            assert_eq!(expected, 3);
            assert_eq!(got, 2);
        }
        other => panic!("expected UnexpectedNonce, got {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn test_insufficient_balance_rejected() -> Result<(), Box<dyn Error>> {
    let balances = HashMap::from([(String::from("Alice"), 5)]);
    let nonces = HashMap::from([(String::from("Alice"), 3)]);
    let mut mempool = Mempool::new(
        10,
        5,
        Arc::new(crate::state::MockStateProvider::new_with_values(
            nonces, balances,
        )),
    );
    let res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            3,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::InsufficientBalance {
            sender,
            balance,
            fee,
        }) => {
            assert_eq!(sender, "Alice");
            assert_eq!(balance, 5);
            assert_eq!(fee, 10);
        }
        other => panic!("expected InsufficientBalance, got {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn test_balance_nonce_valid() -> Result<(), Box<dyn Error>> {
    let balances = HashMap::from([(String::from("Alice"), 100)]);
    let nonces = HashMap::from([(String::from("Alice"), 0)]);
    let mut mempool = Mempool::new(
        10,
        5,
        Arc::new(crate::state::MockStateProvider::new_with_values(
            nonces, balances,
        )),
    );
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_rejection_count() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    let mut res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            1,
            2,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::FeeBelowMinimum { fee, min_fee }) => {
            assert_eq!(min_fee, 5);
            assert_eq!(fee, 2);
        }
        other => panic!("expected InsufficientBalance, got {:?}", other),
    }
    res = mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await;
    match res {
        Err(MempoolError::Duplicate {
            sender,
            nonce,
            existing_fee,
            new_fee,
        }) => {
            assert_eq!(sender, "Alice");
            assert_eq!(nonce, 0);
            assert_eq!(existing_fee, 10);
            assert_eq!(new_fee, 10);
        }
        other => panic!("expected InsufficientBalance, got {:?}", other),
    }
    assert_eq!(mempool.rejection_count.load(Ordering::Relaxed), 2);
    Ok(())
}

#[tokio::test]
async fn test_avg_fee_correct() -> Result<(), Box<dyn Error>> {
    let capacity = 6;
    let mut mempool = mock_mempool(capacity, 5);
    assert_eq!(mempool.avg_fee(), 0.0);
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            0,
            10,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            1,
            20,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    mempool
        .insert(Transaction::new(
            String::from("Alice"),
            2,
            30,
            "test1".as_bytes().to_vec(),
        ))
        .await?;
    assert_eq!(mempool.avg_fee(), 20.0);
    Ok(())
}
