use std::any::Any;
use std::collections::BTreeSet;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use futures::Future;
use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};
use tokio::sync::Notify;
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::arc::addr::WeakAddr;
use crate::arc::dependencies::Dependencies;
use crate::arc::{Derived, Evaluation, Invalid, Observable, State, Version};
use crate::capture::Capture;
use crate::hashed::Hashed;

#[doc(hidden)]
pub struct Async<T>
where
	T: Send + Sync + Hash + 'static,
{
	body: Arc<AsyncBody<T>>,
}

pub struct AsyncBody<T>
where
	T: Send + Sync + Hash + 'static,
{
	value: RwLock<(Option<T>, u64)>,
	notify: Notify,
	inner: Mutex<AsyncInner<T>>,
}

struct AsyncEffect<T, H: Fn(AsyncContext, C) -> F, F: Future<Output = T>, C>
where
	H: Send + 'static,
	F: Send + 'static,
{
	capture: C,
	func: H,
}

impl<T, H: Fn(AsyncContext, C) -> F, F: Future<Output = T>, C> AsyncEffecty<T>
	for AsyncEffect<T, H, F, C>
where
	T: Hash + Send + Sync + 'static,
	H: Send + 'static,
	F: Send + 'static,
	C: Clone + Send + 'static,
{
	fn invoke(&mut self, ctx: Weak<AsyncBody<T>>) -> tokio::task::AbortHandle {
		let this = ctx.upgrade().unwrap();
		let future = (self.func)(
			AsyncContext {
				evaluation: this.inner.lock().eval.clone(),
			},
			self.capture.clone(),
		);

		tokio::spawn(async move {
			let result = future.await;
			let this = ctx.upgrade().unwrap();
			this.set(result);
		})
		.abort_handle()
	}
}

pub trait AsyncEffecty<T: Send + Sync + Hash>: Send {
	fn invoke(&mut self, ctx: Weak<AsyncBody<T>>) -> AbortHandle;
}

pub struct AsyncInner<T>
where
	T: Send + Hash + Sync + 'static,
{
	effect: Box<dyn AsyncEffecty<T>>,
	cancel: CancellationToken,
	eval: Arc<Evaluation>,
	revision: u64,
	handle: Option<AbortHandle>,
	state: State,
	used_by: BTreeSet<WeakAddr<dyn Derived>>,
	dependencies: Dependencies,
	this: Weak<AsyncBody<T>>,
}

impl<T> Async<T>
where
	T: Send + Sync + Hash + 'static,
{
	pub fn new<C: Capture, F: Future<Output = T> + Send + 'static>(
		capture: C,
		func: impl Fn(AsyncContext, C::Output) -> F + Send + 'static,
	) -> Self
	where
		C::Output: Clone + Send + 'static,
	{
		Async {
			body: Arc::new_cyclic(|this| AsyncBody {
				value: RwLock::new((None, 0)),
				notify: Notify::new(),
				inner: Mutex::new(AsyncInner {
					effect: Box::new(AsyncEffect {
						func,
						capture: capture.capture(),
					}) as Box<dyn AsyncEffecty<T>>,
					revision: 0,
					eval: Arc::new(Evaluation::new(this.clone() as Weak<dyn Derived>)),
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

	pub async fn ready_once(&self) -> MappedRwLockReadGuard<T>
	where
		T: std::fmt::Debug,
	{
		loop {
			{
				let value = self.body.get_once();
				if value.is_some() {
					return MappedRwLockReadGuard::try_map(value, |t| Some(t.as_ref().unwrap()))
						.expect("Unreachable");
				}
			}
			self.body.notify.notified().await;
		}
	}

	pub async fn ready<'a>(&self, cx: &'a impl AsRef<Evaluation>) -> MappedRwLockReadGuard<T>
	where
		T: std::fmt::Debug,
	{
		loop {
			{
				let value = self.body.get(cx.as_ref());
				if value.is_some() {
					return MappedRwLockReadGuard::try_map(value, |t| Some(t.as_ref().unwrap()))
						.expect("Unreachable");
				}
			}
			self.body.notify.notified().await;
		}
	}

	// #[inline]
	// pub fn get_once(&self) -> MappedRwLockReadGuard<'_, T> {
	// 	self.body.get_once()
	// }

	#[inline]
	pub fn get<'a>(&'a self, cx: &'a impl AsRef<Evaluation>) -> MappedRwLockReadGuard<Option<T>> {
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

	pub fn set(&self, value: T) {
		let Hashed { value, hash } = Hashed::new(value);
		let value = (Some(value), hash);

		let changed = { self.value.read().1 != value.1 };

		if changed {
			let mut inner = self.inner.lock();
			*self.value.write() = value;

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
	}

	pub fn get_once(&self) -> MappedRwLockReadGuard<Option<T>> {
		{
			let mut self_mut = self.inner.lock();
			self.inner_update(&mut self_mut);
			RwLockReadGuard::map(self.value.read(), |v| &v.0)
		}
	}

	pub fn get<'a>(&'a self, eval: &'_ Evaluation) -> MappedRwLockReadGuard<Option<T>> {
		{
			let mut self_mut = self.inner.lock();
			self.inner_update(&mut self_mut);
			eval.based_on(
				self_mut.this.upgrade().unwrap(),
				Version::Hash(self_mut.revision),
			);
			self_mut.used_by(eval.parent());

			RwLockReadGuard::map(self.value.read(), |v| &v.0)
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

		// if is_valid {
		// 	inner_mut.state = State::Valid;
		// 	return;
		// }

		let this = inner_mut.this.clone();

		// clear dependencies list before evaluating future again
		// inner_mut.eval.take();
		// inner_mut.future = Some(future);
		// inner_mut.state = State::Valid;

		inner_mut.effect.invoke(this);

		let parent = inner_mut.this.clone() as Weak<dyn Derived>;
		// inner_mut.dependencies.swap(evaluation.take(), &parent);
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

impl<T> Drop for AsyncInner<T>
where
	T: Send + Sync + Hash + 'static,
{
	fn drop(&mut self) {
		let refr = self.this.clone() as Weak<dyn Derived>;
		self.dependencies.drop(&refr);
	}
}

pub struct AsyncContext {
	evaluation: Arc<Evaluation>,
}

impl AsRef<Evaluation> for AsyncContext {
	fn as_ref(&self) -> &Evaluation {
		&self.evaluation
	}
}

#[cfg(test)]
mod tests {
	use tokio::sync::watch::channel;

	use super::*;
	use crate::arc::{Computed, Var};

	#[tokio::test]
	async fn test() {
		let a = Var::new(10);

		let (s, r) = channel(10);

		// let c = Async::new((&b,), |cx, (b,)| async move { *b.ready(&cx).await });

		// let d = Computed::new(Box::new(move |cx| c.get(&cx).clone()));

		let b = Async::new((&a,), |cx, (a,)| async move { a.get(&cx)? });

		// kabina.fileset()?

		let v = b.ready_once().await;
		assert_eq!(*v, 10);

		// let value = d.get_once();

		// println!("{:?}", &*value)
	}
}

// let value = a.changed?()
