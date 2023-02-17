use std::cell::Cell;

use crate::reaction::CHANGED;

pub(crate) static mut STARTED: Cell<bool> = Cell::new(false);

pub fn in_batch() -> bool {
	unsafe { STARTED.get() }
}

// TODO: implement microtask planner
pub fn batch(func: impl FnOnce()) {
	let mut we_started = false;
	unsafe {
		if !STARTED.get() {
			STARTED.set(true);
			we_started = true;
		}
	}

	func();

	if we_started {
		loop {
			let changed = {
				let mut borrow = unsafe { CHANGED.borrow_mut() };
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

		unsafe {
			STARTED.set(false);
		}
	}
}
