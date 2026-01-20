use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use crate::arc::addr::ArcAddr;
use crate::arc::{Derived, Observable, Version};

pub struct Dependencies {
	based_on: BTreeMap<ArcAddr<dyn Observable>, Version>,
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

	pub fn based_on(&mut self, observable: Arc<dyn Observable>, version: Version) {
		self.based_on.insert(ArcAddr::new(observable), version);
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
