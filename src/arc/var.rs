use std::any::Any;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use parking_lot::{MappedMutexGuard, MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};

use crate::arc::addr::WeakAddr;
use crate::arc::evaluation::Evaluation;
use crate::arc::value::{Access, Value};
use crate::arc::{Computed, Derived, Invalid, Observable, Version};
use crate::hashed::Hashed;

type Ref<'a, T> = MappedMutexGuard<'a, T>;

pub struct Var<T> {
	body: Arc<VarBody<T>>,
}

impl<T: 'static> From<Var<T>> for Arc<dyn Any> {
	fn from(var: Var<T>) -> Self {
		var.body
	}
}

impl<T: 'static + Send + Sync> TryFrom<Arc<dyn Any + Send + Sync>> for Var<T> {
	type Error = Arc<dyn Any + Send + Sync>;
	fn try_from(value: Arc<dyn Any + Send + Sync>) -> Result<Self, Self::Error> {
		Arc::downcast::<VarBody<T>>(value).map(|body| Var { body })
	}
}

pub struct VarBody<T> {
	value: RwLock<Hashed<T>>,
	inner: Mutex<VarInner<T>>,
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
	T: Default + Hash + Send + Sync + 'static,
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
	T: Send + Sync + 'static,
{
	pub fn new(value: T) -> Self
	where
		T: Hash,
	{
		Var {
			body: Arc::new_cyclic(|this| VarBody {
				value: RwLock::new(Hashed::new(value)),
				inner: Mutex::new(VarInner {
					used_by: BTreeSet::new(),
					this: this.clone(),
				}),
			}),
		}
	}

	pub fn map<F, R>(&self, func: F) -> Computed<R>
	where
		F: Fn(&T) -> R + 'static + Send,
		R: Send + Sync + Hash + 'static,
	{
		let this = self.body.clone();
		Computed::new(Box::new(move |ev| {
			let value = this.get(ev);
			func(&*value)
		}))
	}

	#[inline]
	pub fn get_ref(&self, eval: &impl AsRef<Evaluation>) -> MappedRwLockReadGuard<'_, T> {
		self.body.get(eval.as_ref())
	}

	#[inline]
	pub fn get(&self, eval: &impl AsRef<Evaluation>) -> T
	where
		T: Clone,
	{
		self.body.get(eval.as_ref()).clone()
	}

	#[inline]
	pub fn get_ref_once(&self) -> MappedRwLockReadGuard<'_, T> {
		self.body.get_once()
	}

	#[inline]
	pub fn get_once(&self) -> T
	where
		T: Clone,
	{
		self.body.get_once().clone()
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
	pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
		RwLockReadGuard::map(self.value.read(), |s| &s.value)
	}

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> MappedRwLockReadGuard<'a, T>
	where
		T: Send + Sync + 'static,
	{
		let value = self.value.read();

		{
			let mut self_mut = self.inner.lock();
			eval.based_on(self_mut.this.upgrade().unwrap(), Version::Hash(value.hash));
			self_mut.used_by(eval.parent());
		}

		RwLockReadGuard::map(value, |v| &v.value)
	}

	pub fn update<'a>(&'a self, func: impl FnOnce(&mut T))
	where
		T: 'static + Hash,
	{
		let mut value = self.value.write();
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
		let mut current = self.value.write();
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
		let mut self_mut = self.inner.lock();
		self_mut.used_by.retain(|item| {
			if let Some(item) = item.upgrade() {
				item.invalidate(Invalid::Definitely);
				true
			} else {
				false
			}
		});
	}

	fn used_by(&self, derived: Weak<dyn Derived>) {
		self.inner.lock().used_by(derived);
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		self.inner.lock().not_used_by(derived);
	}
}

impl<T> VarInner<T> {
	pub fn used_by(&mut self, derived: Weak<dyn Derived + Send + Sync>) {
		self.used_by.insert(WeakAddr::new(derived));
	}

	pub fn not_used_by(&mut self, derived: &Weak<dyn Derived>) {
		self.used_by.remove(&WeakAddr::new(derived.clone()));
	}
}

impl<T: Send + Sync + 'static> Observable for VarBody<T> {
	fn version(&self) -> Version {
		Version::Hash(self.value.read().hash)
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
	T: Send + Sync + 'static,
{
	fn get(&self, eval: &Evaluation) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Guard(VarBody::get(self, eval))
	}

	fn get_once(&self) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Guard(VarBody::get_once(&self))
	}
}

impl<T> From<Var<T>> for Value<T>
where
	T: Send + Sync + 'static,
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
		state.write_u64(self.body.value.read().hash);
	}
}

impl<T> Debug for Var<T>
where
	T: Debug + Send + Sync + 'static,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.get_ref_once().fmt(f)
	}
}
