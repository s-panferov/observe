use std::rc::{Rc, Weak};

use crate::value::Access;
use crate::{Evaluation, Observable, Version};

pub struct Const<T> {
	body: Rc<ConstBody<T>>,
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
	fn update(&self) -> crate::Version {
		self.version()
	}

	fn version(&self) -> Version {
		Version::Hash(0)
	}

	fn used_by(&self, _: Weak<dyn crate::Derived>) {}
	fn not_used_by(&self, _: &Weak<dyn crate::Derived>) {}
}

impl<T> Access<T> for ConstBody<T>
where
	T: 'static,
{
	fn get(&self, _: &Evaluation) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Ref(&self.value)
	}

	fn get_once(&self) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Ref(&self.value)
	}
}
