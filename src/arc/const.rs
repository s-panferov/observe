use std::fmt::Debug;
use std::sync::{Arc, Weak};

use crate::arc::value::Access;
use crate::arc::{Evaluation, Observable, Version};

pub struct Const<T> {
	body: Arc<ConstBody<T>>,
}

impl<T> Clone for Const<T> {
	fn clone(&self) -> Self {
		Self {
			body: self.body.clone(),
		}
	}
}

struct ConstBody<T> {
	value: T,
}

#[allow(unused)]
impl<T> Const<T> {
	pub fn new(value: T) -> Self {
		Const {
			body: Arc::new(ConstBody { value }),
		}
	}

	pub fn get(&self) -> &T {
		&self.body.value
	}
}

impl<T> Observable for ConstBody<T>
where
	T: Send + Sync + 'static,
{
	fn update(&self) -> crate::arc::Version {
		self.version()
	}

	fn version(&self) -> Version {
		Version::Hash(0)
	}

	fn used_by(&self, _: Weak<dyn crate::arc::Derived>) {}
	fn not_used_by(&self, _: &Weak<dyn crate::arc::Derived>) {}
}

impl<T> Access<T> for ConstBody<T>
where
	T: Send + Sync + 'static,
{
	fn get(&self, _: &Evaluation) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Ref(&self.value)
	}

	fn get_once(&self) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Ref(&self.value)
	}
}

impl<T> Debug for Const<T>
where
	T: Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.get().fmt(f)
	}
}
