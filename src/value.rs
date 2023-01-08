use std::ops::Deref;
use std::rc::Rc;

use crate::{Evaluation, Observable};

pub struct Value<T> {
	value: Rc<dyn Access<T>>,
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
	pub fn new(value: Rc<dyn Access<T>>) -> Self {
		Value { value }
	}

	pub fn get(&self, eval: &Evaluation) -> Ref<T> {
		self.value.get(eval)
	}

	pub fn get_once(&self) -> Ref<T> {
		self.value.get_once()
	}
}

pub enum Ref<'a, T> {
	Ref(&'a T),
	Cell(std::cell::Ref<'a, T>),
}

impl<'a, T> Deref for Ref<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		match self {
			Ref::Cell(guard) => guard.deref(),
			Ref::Ref(t) => t,
		}
	}
}

pub trait Access<T>: Observable {
	fn get(&self, tracker: &Evaluation) -> Ref<'_, T>;
	fn get_once(&self) -> Ref<'_, T>;
}
