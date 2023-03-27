use std::cell::Cell;

use crate::reaction::CHANGED;

pub(crate) static mut STARTED: Cell<bool> = Cell::new(false);
pub(crate) static mut MICROTASK: Cell<bool> = Cell::new(false);

pub fn in_batch() -> bool {
	unsafe { STARTED.get() }
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
	let mut we_started = false;
	unsafe {
		if !STARTED.get() {
			STARTED.set(true);
			we_started = true;
		}
	}

	we_started
}

#[allow(unused)]
fn batch_start_microtask() -> bool {
	let mut we_started = false;
	unsafe {
		if !MICROTASK.get() {
			MICROTASK.set(true);
			we_started = true;
		}
	}

	we_started
}

fn is_microtask_scheduled() -> bool {
	unsafe { MICROTASK.get() }
}

fn batch_stop() {
	unsafe {
		STARTED.set(false);
	}
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
			unsafe {
				MICROTASK.set(false);
			}
		});
	}
}
