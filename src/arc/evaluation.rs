use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::arc::dependencies::Dependencies;
use crate::arc::{Derived, Observable, Version};

pub struct Evaluation {
	inner: Mutex<EvaluationInner>,
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
			inner: Mutex::new({
				EvaluationInner {
					dependencies: Dependencies::new(),
				}
			}),
		}
	}

	pub(crate) fn parent(&self) -> Weak<dyn Derived> {
		self.parent.clone()
	}

	pub(crate) fn based_on(&self, observable: Arc<dyn Observable>, version: Version) {
		self.inner.lock().dependencies.based_on(observable, version);
	}

	pub fn take(self) -> Dependencies {
		self.inner.into_inner().dependencies
	}
}
