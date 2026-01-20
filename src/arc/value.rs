use std::ops::Deref;
use std::sync::Arc;

use parking_lot::MappedRwLockReadGuard;

use crate::arc::{Evaluation, Observable};

pub struct Value<T> {
	value: Arc<dyn Access<T>>,
}

impl<T> Clone for Value<T> {
	fn clone(&self) -> Self {
		Value {
			value: self.value.clone(),
		}
	}
}

impl<T> Value<T>
where
	T: 'static,
{
	pub fn new(value: Arc<dyn Access<T>>) -> Self {
		Value { value }
	}

	#[inline]
	pub fn get(&self, eval: &impl AsRef<Evaluation>) -> Ref<'_, T> {
		self.value.get(eval.as_ref())
	}

	#[inline]
	pub fn get_once(&self) -> Ref<'_, T> {
		self.value.get_once()
	}
}

pub enum Ref<'a, T> {
	Ref(&'a T),
	Guard(MappedRwLockReadGuard<'a, T>),
}

impl<'a, T> Deref for Ref<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		match self {
			Ref::Guard(guard) => guard.deref(),
			Ref::Ref(t) => t,
		}
	}
}

pub trait Access<T>: Observable {
	fn get(&self, tracker: &Evaluation) -> Ref<'_, T>;
	fn get_once(&self) -> Ref<'_, T>;
}
