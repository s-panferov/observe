use std::sync::{Arc, Mutex, MutexGuard};

use mockall::*;

#[automock]
pub trait Spy {
	fn trigger(&self, value: u64);
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
