use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::BTreeSet;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::addr::WeakAddr;
use crate::dependencies::Dependencies;
use crate::value::Access;
use crate::{Derived, Evaluation, Hashed, Invalid, Observable, State, Value, Version};

pub struct Computed<T>
where
	T: Hash + 'static,
{
	body: Rc<ComputedBody<T>>,
}

impl<T> Clone for Computed<T>
where
	T: Hash,
{
	fn clone(&self) -> Self {
		Self {
			body: self.body.clone(),
		}
	}
}

impl<T: Hash + 'static> From<Computed<T>> for Rc<dyn Any> {
	fn from(var: Computed<T>) -> Self {
		var.body
	}
}

impl<T: Hash + 'static> TryFrom<Rc<dyn Any>> for Computed<T> {
	type Error = Rc<dyn Any>;
	fn try_from(value: Rc<dyn Any>) -> Result<Self, Self::Error> {
		Rc::downcast::<ComputedBody<T>>(value).map(|body| Computed { body })
	}
}

pub struct ComputedBody<T>
where
	T: Hash + 'static,
{
	value: RefCell<Option<Hashed<T>>>,
	inner: RefCell<ComputedInner<T>>,
}

pub struct ComputedInner<T>
where
	T: Hash + 'static,
{
	func: Box<dyn Fn(&Evaluation) -> T>,
	state: State,
	used_by: BTreeSet<WeakAddr<dyn Derived>>,
	dependencies: Dependencies,
	this: Weak<ComputedBody<T>>,
}

impl<T> Drop for ComputedInner<T>
where
	T: Hash + 'static,
{
	fn drop(&mut self) {
		let refr = self.this.clone() as Weak<dyn Derived>;
		self.dependencies.drop(&refr);
	}
}

impl<T> Computed<T>
where
	T: Hash + 'static,
{
	pub fn new(func: Box<dyn Fn(&Evaluation) -> T>) -> Self {
		Computed {
			body: Rc::new_cyclic(|this| ComputedBody {
				value: RefCell::new(None),
				inner: RefCell::new(ComputedInner {
					func,
					state: State::Invalid(Invalid::Definitely),
					used_by: BTreeSet::new(),
					dependencies: Dependencies::new(),
					this: this.clone(),
				}),
			}),
		}
	}

	#[inline]
	pub fn get_once(&self) -> Ref<'_, T> {
		self.body.get_once()
	}

	#[inline]
	pub fn get<'a>(&'a self, cx: &'a impl AsRef<Evaluation>) -> Ref<'a, T> {
		self.body.get(cx.as_ref())
	}
}

impl<T> ComputedBody<T>
where
	T: Hash + 'static,
{
	pub fn get_once(&self) -> Ref<'_, T> {
		self.update();
		Ref::map(
			Ref::map(self.value.borrow(), |s| s.as_ref().unwrap()),
			|s| &s.value,
		)
	}

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> Ref<'a, T> {
		{
			let mut self_mut = self.inner.borrow_mut();
			self.inner_update(&mut self_mut);
			eval.based_on(
				self_mut.this.upgrade().unwrap(),
				Version::Hash(self.value.borrow().as_ref().unwrap().hash),
			);
			self_mut.used_by(eval.parent());
		}
		Ref::map(
			Ref::map(self.value.borrow(), |s| s.as_ref().unwrap()),
			|s| &s.value,
		)
	}

	pub(crate) fn used_by(&self, observable: Weak<dyn Derived>) {
		self.inner.borrow_mut().used_by(observable);
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		self.inner.borrow_mut().not_used_by(derived);
	}

	pub fn inner_update(&self, inner_mut: &mut ComputedInner<T>) {
		if inner_mut.state == State::Valid {
			return;
		}

		let is_valid = match inner_mut.state {
			State::Valid => true,
			State::Invalid(Invalid::Definitely) => false,
			State::Invalid(Invalid::Maybe) => inner_mut.dependencies.are_valid(),
		};

		if is_valid {
			inner_mut.state = State::Valid;
			return;
		}

		let this = inner_mut.this.clone() as Weak<dyn Derived>;
		let evaluation = Evaluation::new(this);
		let value = (inner_mut.func)(&evaluation);
		inner_mut.state = State::Valid;

		let parent = inner_mut.this.clone() as Weak<dyn Derived>;
		inner_mut.dependencies.swap(evaluation.take(), &parent);

		*self.value.borrow_mut() = Some(Hashed::new(value));
	}
}

impl<T> ComputedInner<T>
where
	T: Hash + 'static,
{
	pub(crate) fn used_by(&mut self, observable: Weak<dyn Derived>) {
		self.used_by.insert(WeakAddr::new(observable));
	}

	fn not_used_by(&mut self, derived: &Weak<dyn Derived>) {
		self.used_by.remove(&WeakAddr::new(derived.clone()));
	}
}

impl<T> Observable for ComputedBody<T>
where
	T: Hash + 'static,
{
	fn update(&self) -> Version {
		self.inner_update(&mut self.inner.borrow_mut());
		self.version()
	}

	fn version(&self) -> Version {
		Version::Hash(self.value.borrow().as_ref().unwrap().hash)
	}

	fn used_by(&self, derived: Weak<dyn Derived>) {
		ComputedBody::used_by(self, derived)
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		ComputedBody::not_used_by(self, derived)
	}
}

impl<T> Access<T> for ComputedBody<T>
where
	T: Hash + 'static,
{
	fn get(&self, tracker: &Evaluation) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Cell(self.get(tracker))
	}

	fn get_once(&self) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Cell(self.get_once())
	}
}

impl<T: 'static> Derived for ComputedBody<T>
where
	T: Hash + 'static,
{
	fn invalidate(self: Rc<Self>, invalid: crate::Invalid) {
		let mut self_mut = self.inner.borrow_mut();
		if matches!(self_mut.state, State::Valid) {
			self_mut.state = State::Invalid(invalid);
			for item in &self_mut.used_by {
				if let Some(item) = item.upgrade() {
					item.invalidate(Invalid::Maybe);
				}
			}
		}
	}
}

impl<T> From<Computed<T>> for Value<T>
where
	T: Hash + 'static,
{
	fn from(computed: Computed<T>) -> Self {
		Value::new(computed.body)
	}
}
