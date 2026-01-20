use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use crate::rc::addr::RcAddr;
use crate::rc::{Derived, Observable, Version};

pub struct Dependencies {
	based_on: BTreeMap<RcAddr<dyn Observable>, Version>,
}

impl Default for Dependencies {
	fn default() -> Self {
		Dependencies {
			based_on: BTreeMap::new(),
		}
	}
}

impl Dependencies {
	pub fn new() -> Self {
		Self {
			based_on: BTreeMap::new(),
		}
	}

	pub fn drop(&mut self, parent: &Weak<dyn Derived>) {
		for (item, _) in &self.based_on {
			item.not_used_by(&parent)
		}
	}

	pub fn based_on(&mut self, observable: Rc<dyn Observable>, version: Version) {
		self.based_on.insert(RcAddr::new(observable), version);
	}

	pub fn are_valid(&self) -> bool {
		for (base, version) in self.based_on.iter() {
			if base.update() != *version {
				return false;
			}
		}

		true
	}

	pub fn swap(&mut self, next: Dependencies, parent: &Weak<dyn Derived>) {
		let prev = std::mem::replace(&mut self.based_on, next.based_on);

		// Diff the keys
		prev.keys()
			.filter(|k| !self.based_on.contains_key(k))
			.for_each(|k| k.not_used_by(&parent));
	}
}
