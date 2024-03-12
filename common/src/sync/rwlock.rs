use tokio::sync::RwLockReadGuard;

pub fn unwrap_lock<T>(lock: RwLockReadGuard<'_, Option<T>>) -> RwLockReadGuard<'_, T> {
    RwLockReadGuard::map(lock, |lock| lock.as_ref().unwrap())
}
