use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
pub struct KeyedLocks {
    locks: StdMutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl KeyedLocks {
    pub async fn acquire(&self, key: String) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.locks.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            locks.retain(|_, lock| Arc::strong_count(lock) > 1);
            locks.entry(key).or_default().clone()
        };
        lock.lock_owned().await
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.locks.lock().unwrap().len()
    }
}

#[cfg(test)]
#[path = "tests/lock/mod.rs"]
mod tests;
