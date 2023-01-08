use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::dependencies::Dependencies;
use crate::{Derived, Observable, Version};

pub struct Evaluation {
	inner: RefCell<EvaluationInner>,
	parent: Weak<dyn Derived>,
}

impl AsRef<Evaluation> for Evaluation {
	fn as_ref(&self) -> &Evaluation {
		self
	}
}

struct EvaluationInner {
	dependencies: Dependencies,
}

impl Evaluation {
	pub fn new(parent: Weak<dyn Derived>) -> Self {
		Evaluation {
			parent,
			inner: RefCell::new({
				EvaluationInner {
					dependencies: Dependencies::new(),
				}
			}),
		}
	}

	pub(crate) fn parent(&self) -> Weak<dyn Derived> {
		self.parent.clone()
	}

	pub(crate) fn based_on(&self, observable: Rc<dyn Observable>, version: Version) {
		self.inner
			.borrow_mut()
			.dependencies
			.based_on(observable, version);
	}

	pub fn take(self) -> Dependencies {
		self.inner.into_inner().dependencies
	}
}
