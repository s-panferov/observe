use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::addr::WeakAddr;
use crate::evaluation::Evaluation;
use crate::value::{Access, Value};
use crate::{Computed, Derived, Hashed, Invalid, Observable, Version};

pub struct Var<T> {
	body: Rc<VarBody<T>>,
}

impl<T: 'static> From<Var<T>> for Rc<dyn Any> {
	fn from(var: Var<T>) -> Self {
		var.body
	}
}

impl<T: 'static> TryFrom<Rc<dyn Any>> for Var<T> {
	type Error = Rc<dyn Any>;
	fn try_from(value: Rc<dyn Any>) -> Result<Self, Self::Error> {
		Rc::downcast::<VarBody<T>>(value).map(|body| Var { body })
	}
}

pub struct VarBody<T> {
	value: RefCell<Hashed<T>>,
	inner: RefCell<VarInner<T>>,
}

struct VarInner<T> {
	used_by: BTreeSet<WeakAddr<dyn Derived>>,
	this: Weak<VarBody<T>>,
}

impl<T> Clone for Var<T> {
	fn clone(&self) -> Self {
		Self {
			body: self.body.clone(),
		}
	}
}

impl<T> Default for Var<T>
where
	T: Default + Hash + 'static,
{
	fn default() -> Self {
		Var::new(Default::default())
	}
}

pub trait Toggle {
	fn toggle(&mut self);
}

impl Toggle for bool {
	fn toggle(&mut self) {
		*self = !*self
	}
}

impl<T> Var<T>
where
	T: 'static,
{
	pub fn new(value: T) -> Self
	where
		T: Hash,
	{
		Var {
			body: Rc::new_cyclic(|this| VarBody {
				value: RefCell::new(Hashed::new(value)),
				inner: RefCell::new(VarInner {
					used_by: BTreeSet::new(),
					this: this.clone(),
				}),
			}),
		}
	}

	pub fn map<F, R>(&self, func: F) -> Computed<R>
	where
		F: Fn(&T) -> R + 'static,
		R: Hash + 'static,
	{
		let this = self.body.clone();
		Computed::new(Box::new(move |ev| {
			let value = this.get(ev);
			func(&*value)
		}))
	}

	#[inline]
	pub fn get(&self, eval: &impl AsRef<Evaluation>) -> Ref<'_, T> {
		self.body.get(eval.as_ref())
	}

	#[inline]
	pub fn get_once(&self) -> Ref<'_, T> {
		self.body.get_once()
	}

	#[inline]
	pub fn set(&self, value: T)
	where
		T: Hash,
	{
		self.body.set(value)
	}

	#[inline]
	pub fn toggle(&self)
	where
		T: Toggle + Hash,
	{
		self.update(T::toggle)
	}

	#[inline]
	pub fn replace(&self, value: T) -> T
	where
		T: Hash,
	{
		self.body.replace(value)
	}

	#[inline]
	pub fn update(&self, func: impl FnOnce(&mut T))
	where
		T: Hash,
	{
		self.body.update(func)
	}
}

impl<T> VarBody<T> {
	pub fn get_once(&self) -> Ref<'_, T> {
		Ref::map(self.value.borrow(), |s| &s.value)
	}

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> Ref<'a, T>
	where
		T: 'static,
	{
		let value = self.value.borrow();

		{
			let mut self_mut = self.inner.borrow_mut();
			eval.based_on(self_mut.this.upgrade().unwrap(), Version::Hash(value.hash));
			self_mut.used_by(eval.parent());
		}

		Ref::map(value, |v| &v.value)
	}

	pub fn update<'a>(&'a self, func: impl FnOnce(&mut T))
	where
		T: 'static + Hash,
	{
		let mut value = self.value.borrow_mut();
		func(&mut value.value);
		let hash = fxhash::hash64(&value.value);
		if value.hash != hash {
			value.hash = hash;
			self.invalidate()
		}
	}

	pub fn replace(&self, value: T) -> T
	where
		T: Hash,
	{
		let mut current = self.value.borrow_mut();
		let new = Hashed::new(value);
		let hash = new.hash;

		let old = std::mem::replace(&mut *current, new);
		if old.hash != hash {
			std::mem::drop(current);
			self.invalidate();
		}

		return old.value;
	}

	pub fn set(&self, value: T)
	where
		T: Hash,
	{
		let _ = self.replace(value);
	}

	fn invalidate(&self) {
		let self_mut = self.inner.borrow();
		for item in &self_mut.used_by {
			if let Some(item) = item.upgrade() {
				item.invalidate(Invalid::Definitely)
			}
		}
	}

	fn used_by(&self, derived: Weak<dyn Derived>) {
		self.inner.borrow_mut().used_by(derived);
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		self.inner.borrow_mut().not_used_by(derived);
	}
}

impl<T> VarInner<T> {
	pub fn used_by(&mut self, derived: Weak<dyn Derived>) {
		self.used_by.insert(WeakAddr::new(derived));
	}

	pub fn not_used_by(&mut self, derived: &Weak<dyn Derived>) {
		self.used_by.remove(&WeakAddr::new(derived.clone()));
	}
}

impl<T: 'static> Observable for VarBody<T> {
	fn version(&self) -> Version {
		Version::Hash(self.value.borrow().hash)
	}

	fn update(&self) -> Version {
		self.version()
	}

	fn used_by(&self, derived: Weak<dyn Derived>) {
		VarBody::used_by(&self, derived)
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		VarBody::not_used_by(&self, derived)
	}
}

impl<T> Access<T> for VarBody<T>
where
	T: 'static,
{
	fn get(&self, eval: &Evaluation) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Cell(VarBody::get(self, eval))
	}

	fn get_once(&self) -> crate::value::Ref<'_, T> {
		crate::value::Ref::Cell(VarBody::get_once(&self))
	}
}

impl<T> From<Var<T>> for Value<T>
where
	T: 'static,
{
	fn from(var: Var<T>) -> Self {
		Value::new(var.body)
	}
}

impl<T> Hash for Var<T>
where
	T: Hash,
{
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write_u64(self.body.value.borrow().hash);
	}
}

impl<T> Debug for Var<T>
where
	T: 'static + Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.get_once().fmt(f)
	}
}
