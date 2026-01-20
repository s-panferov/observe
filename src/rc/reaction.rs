use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::rc::batch::in_batch;
use crate::rc::dependencies::Dependencies;
use crate::rc::{Derived, Evaluation, Invalid, State};

pub trait Reactive {
	fn update(&self);
}

pub static mut CHANGED: RefCell<Vec<Weak<dyn Reactive>>> = RefCell::new(vec![]);

#[derive(Default, Clone)]
pub struct Reactions<const N: usize> {
	vec: smallvec::SmallVec<[Reaction; N]>,
}

impl<const N: usize> Reactions<N> {
	pub fn add(&mut self, reaction: Reaction) {
		self.vec.push(reaction);

		#[cfg(debug_assertions)]
		if self.vec.len() > N {
			// tracing::error!("Please increase limit here")
		}
	}

	pub fn update(&self) {
		for reaction in &self.vec {
			reaction.update()
		}
	}
}

#[derive(Clone)]
pub struct Reaction {
	pub(crate) body: Rc<ReactionBody>,
}

pub struct ReactionBody {
	pub(crate) inner: RefCell<ReactionInner>,
}

pub struct ReactionInner {
	state: State,
	#[allow(unused)]
	pub(crate) name: &'static str,
	func: Box<dyn Fn(&Evaluation)>,
	dependencies: Dependencies,
	this: Weak<ReactionBody>,
}

impl Drop for ReactionInner {
	fn drop(&mut self) {
		let refr = self.this.clone() as Weak<dyn Derived>;
		self.dependencies.drop(&refr)
	}
}

impl Reaction {
	#[must_use]
	pub fn new(func: Box<dyn Fn(&Evaluation)>) -> Self {
		Self::new_with_name("<unnamed>", func)
	}

	#[must_use]
	pub fn new_with_name(name: &'static str, func: Box<dyn Fn(&Evaluation)>) -> Self {
		Reaction {
			body: Rc::new_cyclic(|this| ReactionBody {
				inner: RefCell::new(ReactionInner {
					func,
					name,
					state: State::Invalid(Invalid::Definitely),
					dependencies: Dependencies::new(),
					this: this.clone(),
				}),
			}),
		}
	}

	pub fn update_unchecked(&self) {
		// NOTE: this logic is shared with the Self::update

		let mut self_mut = self.body.inner.borrow_mut();

		let this = Rc::downgrade(&self.body) as Weak<dyn Derived>;
		let tracker = Evaluation::new(this.clone());
		(self_mut.func)(&tracker);

		self_mut.dependencies.swap(tracker.take(), &this);
		self_mut.state = State::Valid;
	}

	pub fn update(&self) {
		self.body.update();
	}
}

impl Reactive for ReactionBody {
	fn update(&self) {
		let mut self_mut = self.inner.borrow_mut();

		let is_valid = match self_mut.state {
			State::Valid => true,
			State::Invalid(Invalid::Definitely) => false,
			State::Invalid(Invalid::Maybe) => self_mut.dependencies.are_valid(),
		};

		if is_valid {
			self_mut.state = State::Valid;
			return;
		}

		// NOTE: this logic is shared with the Self::update_unchecked

		let this = self_mut.this.clone() as Weak<dyn Derived>;
		let tracker = Evaluation::new(this.clone());
		(self_mut.func)(&tracker);

		self_mut.dependencies.swap(tracker.take(), &this);
		self_mut.state = State::Valid;
	}
}

impl Derived for ReactionBody {
	fn invalidate(self: Rc<Self>, invalid: crate::rc::Invalid) {
		let mut self_mut = self.inner.borrow_mut();
		if matches!(self_mut.state, State::Valid) {
			if !in_batch() {
				panic!("Reaction was updated outside of the `batch` function");
			}

			self_mut.state = State::Invalid(invalid);
			std::mem::drop(self_mut);

			unsafe {
				CHANGED
					.borrow_mut()
					.push(Rc::downgrade(&self) as Weak<dyn Reactive>)
			}
		}
	}
}

impl std::fmt::Debug for Reaction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Reaction")
			.field("name", &self.body.inner.borrow().name)
			.finish()
	}
}
