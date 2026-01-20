use std::fmt::Debug;
use std::rc::{Rc, Weak};

use crate::rc::value::Access;
use crate::rc::{Evaluation, Observable, Version};

pub struct Const<T> {
	body: Rc<ConstBody<T>>,
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
			body: Rc::new(ConstBody { value }),
		}
	}

	pub fn get(&self) -> &T {
		&self.body.value
	}
}

impl<T> Observable for ConstBody<T>
where
	T: 'static,
{
	fn update(&self) -> crate::rc::Version {
		self.version()
	}

	fn version(&self) -> Version {
		Version::Hash(0)
	}

	fn used_by(&self, _: Weak<dyn crate::rc::Derived>) {}
	fn not_used_by(&self, _: &Weak<dyn crate::rc::Derived>) {}
}

impl<T> Access<T> for ConstBody<T>
where
	T: 'static,
{
	fn get(&self, _: &Evaluation) -> crate::rc::value::Ref<'_, T> {
		crate::rc::value::Ref::Ref(&self.value)
	}

	fn get_once(&self) -> crate::rc::value::Ref<'_, T> {
		crate::rc::value::Ref::Ref(&self.value)
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
