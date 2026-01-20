use std::any::Any;
use std::collections::BTreeSet;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use arc_swap::ArcSwap;
use futures::future::BoxFuture;
use parking_lot::Mutex;
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::arc::addr::WeakAddr;
use crate::arc::dependencies::Dependencies;
use crate::arc::{Derived, Evaluation, Invalid, Observable, State, Version};
use crate::hashed::Hashed;

#[doc(hidden)]
pub struct Async<T>
where
	T: Send + Sync + Hash + 'static,
{
	body: Arc<AsyncBody<T>>,
}

impl<T> Clone for Async<T>
where
	T: Send + Sync + Hash,
{
	fn clone(&self) -> Self {
		Self {
			body: self.body.clone(),
		}
	}
}

impl<T: Send + Sync + Hash + 'static> From<Async<T>> for Arc<dyn Any> {
	fn from(var: Async<T>) -> Self {
		var.body
	}
}

impl<T: Send + Sync + Hash + 'static> TryFrom<Arc<dyn Any + Send + Sync>> for Async<T> {
	type Error = Arc<dyn Any + Send + Sync>;
	fn try_from(value: Arc<dyn Any + Send + Sync>) -> Result<Self, Self::Error> {
		Arc::downcast::<AsyncBody<T>>(value).map(|body| Async { body })
	}
}

pub struct AsyncBody<T>
where
	T: Send + Hash + Sync + 'static,
{
	value: ArcSwap<Option<Hashed<T>>>,
	inner: Mutex<AsyncInner<T>>,
}

struct AsyncEffect<
	K: Hash + Send,
	T,
	H: Fn(&Evaluation) -> K + Send + 'static,
	F: Fn(K, CancellationToken) -> BoxFuture<'static, T> + Send + 'static,
> {
	handler: H,
	func: F,
	value: Option<Hashed<K>>,
}

pub trait AsyncEffecty<T>: Send {
	fn compute(&mut self, cx: &Evaluation) -> u64;
	fn invoke(&mut self, cancel: CancellationToken) -> BoxFuture<'static, T>;
}

impl<K, T, H, F> AsyncEffecty<T> for AsyncEffect<K, T, H, F>
where
	K: Hash + Send,
	H: Fn(&Evaluation) -> K + 'static + Send,
	F: Fn(K, CancellationToken) -> BoxFuture<'static, T> + 'static + Send,
{
	fn compute(&mut self, cx: &Evaluation) -> u64 {
		let value = Hashed::new((self.handler)(cx));
		self.value = Some(value);
		return self.value.as_ref().unwrap().hash;
	}

	fn invoke(&mut self, cancel: CancellationToken) -> BoxFuture<'static, T> {
		(self.func)(self.value.take().unwrap().value, cancel)
	}
}

pub struct AsyncInner<T>
where
	T: Send + Hash + Sync + 'static,
{
	effect: Box<dyn AsyncEffecty<T>>,
	future: Option<BoxFuture<'static, T>>,
	revision: u64,
	cancel: CancellationToken,
	handle: Option<AbortHandle>,
	state: State,
	used_by: BTreeSet<WeakAddr<dyn Derived>>,
	dependencies: Dependencies,
	this: Weak<AsyncBody<T>>,
}

impl<T> Drop for AsyncInner<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn drop(&mut self) {
		let refr = self.this.clone() as Weak<dyn Derived>;
		self.dependencies.drop(&refr);
	}
}

impl<T> Async<T>
where
	T: Send + Sync + Hash + 'static,
{
	pub fn new<K: Hash + Send + 'static>(
		handler: impl Fn(&Evaluation) -> K + 'static + Send,
		func: impl Fn(K, CancellationToken) -> BoxFuture<'static, T> + 'static + Send,
	) -> Self {
		Async {
			body: Arc::new_cyclic(|this| AsyncBody {
				value: ArcSwap::new(Arc::new(None)),
				inner: Mutex::new(AsyncInner {
					effect: Box::new(AsyncEffect {
						func,
						handler,
						value: None,
					}) as Box<dyn AsyncEffecty<T>>,
					revision: 0,
					future: None,
					handle: None,
					cancel: CancellationToken::new(),
					state: State::Invalid(Invalid::Definitely),
					used_by: BTreeSet::new(),
					dependencies: Dependencies::new(),
					this: this.clone(),
				}),
			}),
		}
	}

	// #[inline]
	// pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
	// 	self.body.get_once()
	// }

	#[inline]
	pub fn get<'a>(&'a self, cx: &'a impl AsRef<Evaluation>) -> u64 {
		self.body.get(cx.as_ref())
	}
}

impl<T> AsyncBody<T>
where
	T: Send + Sync + Hash + 'static,
{
	// pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
	// 	self.update();
	// 	MappedRwLockReadGuard::map(
	// 		RwLockReadGuard::map(self.value.read(), |s| s.as_ref().unwrap()),
	// 		|s| &s.value,
	// 	)
	// }

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> u64 {
		{
			let mut self_mut = self.inner.lock();
			self.inner_update(&mut self_mut);
			eval.based_on(
				self_mut.this.upgrade().unwrap(),
				Version::Hash(self_mut.revision),
			);
			self_mut.used_by(eval.parent());
			self_mut.revision
		}
	}

	pub(crate) fn used_by(&self, observable: Weak<dyn Derived>) {
		self.inner.lock().used_by(observable);
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		self.inner.lock().not_used_by(derived);
	}

	pub fn inner_update(&self, inner_mut: &mut AsyncInner<T>) {
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

		let this = inner_mut.this.clone();
		let evaluation = Evaluation::new(this.clone() as Weak<dyn Derived>);
		let revision = inner_mut.effect.compute(&evaluation);

		if revision != inner_mut.revision {
			inner_mut.revision = revision;

			let future = inner_mut.effect.invoke(inner_mut.cancel.clone());

			// respawn future
			inner_mut.handle = Some(
				tokio::spawn(async move {
					let value = future.await;
					let Some(this) = this.upgrade() else {
						return;
					};

					let value = Some(Hashed::new(value));
					if **this.value.load() != value {
						this.value.swap(Arc::new(value));

						let mut inner = this.inner.lock();

						// only invalidating deps, not the value itself
						inner.used_by.retain(|item| {
							if let Some(item) = item.upgrade() {
								item.invalidate(Invalid::Maybe);
								true
							} else {
								false
							}
						});
					}
				})
				.abort_handle(),
			);
		}

		// inner_mut.future = Some(future);
		inner_mut.state = State::Valid;

		let parent = inner_mut.this.clone() as Weak<dyn Derived>;
		inner_mut.dependencies.swap(evaluation.take(), &parent);
	}
}

impl<T> AsyncInner<T>
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

impl<T> Observable for AsyncBody<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn update(&self) -> Version {
		self.inner_update(&mut self.inner.lock());
		self.version()
	}

	fn version(&self) -> Version {
		// FIXME
		Version::Hash(0)
	}

	fn used_by(&self, derived: Weak<dyn Derived>) {
		AsyncBody::used_by(self, derived)
	}

	fn not_used_by(&self, derived: &Weak<dyn Derived>) {
		AsyncBody::not_used_by(self, derived)
	}
}

// impl<T> Access<T> for AsyncBody<T>
// where
// 	T: Send + Sync + Hash + 'static,
// {
// 	fn get(&self, tracker: &Evaluation) -> crate::value::Ref<'_, T> {
// 		crate::value::Ref::Guard(self.get(tracker))
// 	}

// 	fn get_once(&self) -> crate::value::Ref<'_, T> {
// 		crate::value::Ref::Guard(self.get_once())
// 	}
// }

impl<T: 'static> Derived for AsyncBody<T>
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

// impl<T> From<Async<T>> for Value<T>
// where
// 	T: Send + Sync + Hash + 'static,
// {
// 	fn from(computed: Async<T>) -> Self {
// 		Value::new(computed.body)
// 	}
// }
