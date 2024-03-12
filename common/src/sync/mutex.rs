use tokio::sync::{MappedMutexGuard, MutexGuard};

pub fn unwrap_lock<T>(lock: MutexGuard<'_, Option<T>>) -> MappedMutexGuard<'_, T> {
    MutexGuard::map(lock, |lock| lock.as_mut().unwrap())
}
