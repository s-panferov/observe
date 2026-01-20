use observe::rc::{batch, Computed, Reaction, Var};

mod mock;

use mock::Spy;

#[test]
fn computed() {
	let a = Var::new(10);
	assert_eq!(a.get_once(), 10);

	let b = Computed::new(Box::new({
		let a = a.clone();
		move |cx| a.get(cx) + 10
	}));

	assert_eq!(*b.get_once(), 20);

	let mock = mock::SharedMock::new();

	mock.get().expect_trigger().times(1).return_const(());

	let r = Reaction::new(Box::new({
		let a = a.clone();
		let b = b.clone();
		let mock = mock.clone();
		move |cx| {
			mock.get().trigger(a.get(cx) + *b.get(cx));
		}
	}));

	r.update();

	mock.get().checkpoint();

	mock.get().expect_trigger().times(1).return_const(());

	batch(|| {
		a.set(20);
		a.set(20);
		a.set(20);
		a.set(20);
	});

	assert_eq!(*b.get_once(), 30);

	mock.get().checkpoint();
}

#[test]
fn check_invalidation() {
	let a = Var::new(1);

	let mock = mock::SharedMock::new();

	let reaction = Reaction::new(Box::new({
		let a = a.clone();
		let mock = mock.clone();
		move |cx| {
			mock.get().trigger(a.get(cx));
		}
	}));

	mock.get().expect_trigger().once().return_const(());

	reaction.update();

	mock.get().checkpoint();

	mock.get().expect_trigger().times(0).return_const(());

	batch(|| {
		a.set(1);
	});

	mock.get().checkpoint();
}
