use std::cell::Cell;

use crate::reaction::CHANGED;

pub static mut STARTED: Cell<bool> = Cell::new(false);

// TODO: implement microtask planner
pub fn batch(func: impl FnOnce()) {
	func();

	unsafe {
		if !STARTED.get() {
			STARTED.set(true);
			loop {
				let changed = {
					let mut borrow = CHANGED.borrow_mut();
					let items = std::mem::replace(&mut *borrow, Vec::new());
					std::mem::drop(borrow);

					items
				};

				if changed.len() == 0 {
					break;
				}

				// if let Ok(mut changed) = changed {
				for reaction in changed {
					if let Some(reactive) = reaction.upgrade() {
						reactive.update();
					}
				}
			}

			STARTED.set(false);
		}
	}
}
