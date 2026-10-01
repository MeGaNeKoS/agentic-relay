use super::*;
use std::time::Duration;

#[tokio::test]
async fn same_key_is_exclusive_and_ordered() {
    let locks = Arc::new(KeyedLocks::default());
    let first = locks.acquire("k".into()).await;
    let order = Arc::new(StdMutex::new(Vec::new()));
    let mut tasks = Vec::new();
    for n in 0..3 {
        let locks = locks.clone();
        let order = order.clone();
        tasks.push(tokio::spawn(async move {
            let _guard = locks.acquire("k".into()).await;
            order.lock().unwrap().push(n);
        }));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(order.lock().unwrap().is_empty());
    drop(first);
    for task in tasks {
        task.await.unwrap();
    }
    assert_eq!(*order.lock().unwrap(), vec![0, 1, 2]);
}

#[tokio::test]
async fn different_keys_do_not_block_each_other() {
    let locks = KeyedLocks::default();
    let _a = locks.acquire("a".into()).await;
    tokio::time::timeout(Duration::from_secs(1), locks.acquire("b".into())).await.unwrap();
}

#[tokio::test]
async fn idle_entries_are_pruned() {
    let locks = KeyedLocks::default();
    drop(locks.acquire("a".into()).await);
    drop(locks.acquire("b".into()).await);
    let _c = locks.acquire("c".into()).await;
    assert_eq!(locks.len(), 1);
}
