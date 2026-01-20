use std::cell::Cell;

use crate::rc::reaction::CHANGED;

thread_local! {
	pub(crate) static STARTED: Cell<bool> = Cell::new(false);
	pub(crate) static MICROTASK: Cell<bool> = Cell::new(false);
}

pub fn in_batch() -> bool {
	STARTED.with(|s| s.get())
}

pub fn batch(func: impl FnOnce()) {
	let is_root = batch_start();
	func();
	if is_root {
		batch_stop();
		if !is_microtask_scheduled() {
			batch_run();
		}
	}
}

fn batch_start() -> bool {
	STARTED.with(|s| {
		if !s.get() {
			s.set(true);
			true
		} else {
			false
		}
	})
}

#[allow(unused)]
fn batch_start_microtask() -> bool {
	MICROTASK.with(|s| {
		if !s.get() {
			s.set(true);
			true
		} else {
			false
		}
	})
}

fn is_microtask_scheduled() -> bool {
	MICROTASK.with(|s| s.get())
}

fn batch_stop() {
	STARTED.with(|s| s.set(false));
}

pub fn batch_run() {
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
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused)]
pub fn batch_microtask(func: impl FnOnce()) {
	panic!("Not implemented")
}

#[cfg(target_arch = "wasm32")]
pub fn batch_microtask(func: impl FnOnce()) {
	let is_root = batch_start();
	let is_first_microtask = batch_start_microtask();
	func();
	if is_root {
		batch_stop();
	}

	if is_first_microtask {
		crate::microtask::queue(|| {
			batch_run();
			MICROTASK.with(|s| s.set(false));
		});
	}
}
