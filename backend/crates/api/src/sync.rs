use std::sync::{Mutex, MutexGuard};

pub(crate) fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::lock_recover;
    use std::sync::{Arc, Mutex};

    #[test]
    fn poisoned_mutex_remains_available() {
        let value = Arc::new(Mutex::new(1));
        let poisoned = Arc::clone(&value);
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.lock();
            panic!("poison mutex for test");
        })
        .join();

        assert_eq!(*lock_recover(&value), 1);
    }
}
