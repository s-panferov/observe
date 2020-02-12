use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use mockall::*;

#[automock]
pub trait Spy {
    fn trigger(&self);
    fn u32(&self, test: u32);
}

#[derive(Clone)]
pub struct SharedMock(Arc<Mutex<MockSpy>>);

impl SharedMock {
    pub fn new() -> SharedMock {
        SharedMock(Arc::new(Mutex::new(MockSpy::new())))
    }

    pub fn get<'a>(&'a self) -> MutexGuard<'a, MockSpy> {
        return self.0.lock().unwrap();
    }
}
