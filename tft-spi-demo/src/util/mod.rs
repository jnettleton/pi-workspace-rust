use std::sync::{Mutex, MutexGuard};

pub trait MutexExt<T: ?Sized> {
    fn xlock(&self) -> MutexGuard<T>;
}

impl<T: ?Sized> MutexExt<T> for Mutex<T> {
    fn xlock(&self) -> MutexGuard<T> {
        self.lock().expect("Unexpected lock poisoning.")
    }
}
