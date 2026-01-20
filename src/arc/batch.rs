use std::sync::atomic::{AtomicBool, Ordering};

use crate::arc::reaction::CHANGED;

static STARTED: AtomicBool = AtomicBool::new(false);
static MICROTASK: AtomicBool = AtomicBool::new(false);

pub fn in_batch() -> bool {
	STARTED.load(Ordering::Acquire)
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
	STARTED
		.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
		.is_ok()
}

#[allow(unused)]
fn batch_start_microtask() -> bool {
	MICROTASK
		.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
		.is_ok()
}

fn is_microtask_scheduled() -> bool {
	MICROTASK.load(Ordering::Acquire)
}

fn batch_stop() {
	STARTED.store(false, Ordering::Release);
}

pub fn batch_run() {
	loop {
		let changed = {
			let mut borrow = unsafe { CHANGED.lock() };
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
			MICROTASK.store(false, Ordering::Release);
		});
	}
}
