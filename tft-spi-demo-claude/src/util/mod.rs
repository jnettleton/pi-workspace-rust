use std::sync::{Mutex, MutexGuard};

pub trait MutexExt<T: ?Sized> {
    fn xlock(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> MutexExt<T> for Mutex<T> {
    fn xlock(&self) -> MutexGuard<'_, T> {
        self.lock().expect("Unexpected lock poisoning.")
    }
}
