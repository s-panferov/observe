use std::any::Any;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};

use crate::arc::addr::WeakAddr;
use crate::arc::dependencies::Dependencies;
use crate::arc::value::Access;
use crate::arc::{Derived, Evaluation, Invalid, Observable, State, Value, Version};
use crate::hashed::Hashed;

pub struct Computed<T>
where
	T: Send + Sync + Hash + 'static,
{
	body: Arc<ComputedBody<T>>,
}

impl<T> Clone for Computed<T>
where
	T: Send + Sync + Hash,
{
	fn clone(&self) -> Self {
		Self {
			body: self.body.clone(),
		}
	}
}

impl<T: Send + Sync + Hash + 'static> From<Computed<T>> for Arc<dyn Any> {
	fn from(var: Computed<T>) -> Self {
		var.body
	}
}

impl<T: Send + Sync + Hash + 'static> TryFrom<Arc<dyn Any + Send + Sync>> for Computed<T> {
	type Error = Arc<dyn Any + Send + Sync>;
	fn try_from(value: Arc<dyn Any + Send + Sync>) -> Result<Self, Self::Error> {
		Arc::downcast::<ComputedBody<T>>(value).map(|body| Computed { body })
	}
}

pub struct ComputedBody<T>
where
	T: Send + Hash + Sync + 'static,
{
	value: RwLock<Option<Hashed<T>>>,
	inner: Mutex<ComputedInner<T>>,
}

pub struct ComputedInner<T>
where
	T: Send + Hash + Sync + 'static,
{
	func: Box<dyn Fn(&Evaluation) -> T + Send>,
	state: State,
	used_by: BTreeSet<WeakAddr<dyn Derived>>,
	dependencies: Dependencies,
	this: Weak<ComputedBody<T>>,
}

impl<T> Drop for ComputedInner<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn drop(&mut self) {
		let refr = self.this.clone() as Weak<dyn Derived>;
		self.dependencies.drop(&refr);
	}
}

impl<T> Computed<T>
where
	T: Send + Sync + Hash + 'static,
{
	pub fn new(func: Box<dyn Fn(&Evaluation) -> T + Send>) -> Self {
		Computed {
			body: Arc::new_cyclic(|this| ComputedBody {
				value: RwLock::new(None),
				inner: Mutex::new(ComputedInner {
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
	pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
		self.body.get_once()
	}

	#[inline]
	pub fn get<'a>(&'a self, cx: &'a impl AsRef<Evaluation>) -> MappedRwLockReadGuard<'a, T> {
		self.body.get(cx.as_ref())
	}
}

impl<T> ComputedBody<T>
where
	T: Send + Sync + Hash + 'static,
{
	pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
		self.update();
		MappedRwLockReadGuard::map(
			RwLockReadGuard::map(self.value.read(), |s| s.as_ref().unwrap()),
			|s| &s.value,
		)
	}

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> MappedRwLockReadGuard<'a, T> {
		{
			let mut self_mut = self.inner.lock();
			self.inner_update(&mut self_mut);
			eval.based_on(
				self_mut.this.upgrade().unwrap(),
				Version::Hash(self.value.read().as_ref().unwrap().hash),
			);
			self_mut.used_by(eval.parent());
		}
		MappedRwLockReadGuard::map(
			RwLockReadGuard::map(self.value.read(), |s| s.as_ref().unwrap()),
			|s| &s.value,
		)
	}

	pub(crate) fn used_by(&self, observable: Weak<dyn Derived>) {
		self.inner.lock().used_by(observable);
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		self.inner.lock().not_used_by(derived);
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

		*self.value.write() = Some(Hashed::new(value));
	}
}

impl<T> ComputedInner<T>
where
	T: Send + Sync + Hash + 'static,
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
	T: Send + Sync + Hash + 'static,
{
	fn update(&self) -> Version {
		self.inner_update(&mut self.inner.lock());
		self.version()
	}

	fn version(&self) -> Version {
		Version::Hash(self.value.read().as_ref().unwrap().hash)
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
	T: Send + Sync + Hash + 'static,
{
	fn get(&self, tracker: &Evaluation) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Guard(self.get(tracker))
	}

	fn get_once(&self) -> crate::arc::value::Ref<'_, T> {
		crate::arc::value::Ref::Guard(self.get_once())
	}
}

impl<T: 'static> Derived for ComputedBody<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn invalidate(self: Arc<Self>, invalid: crate::arc::Invalid) {
		let mut self_mut = self.inner.lock();
		if matches!(self_mut.state, State::Valid) {
			self_mut.state = State::Invalid(invalid);
			self_mut.used_by.retain(|item| {
				if let Some(item) = item.upgrade() {
					item.invalidate(Invalid::Maybe);
					true
				} else {
					false
				}
			});
		}
	}
}

impl<T> From<Computed<T>> for Value<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn from(computed: Computed<T>) -> Self {
		Value::new(computed.body)
	}
}

impl<T> Debug for Computed<T>
where
	T: Send + Sync + Hash + Debug + 'static,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.get_once().fmt(f)
	}
}
