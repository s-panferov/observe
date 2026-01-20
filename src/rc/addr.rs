use std::cmp::Ordering;
use std::ops::Deref;
use std::rc::{Rc, Weak};

pub struct RcAddr<T: ?Sized> {
	ptr: Rc<T>,
}

impl<T: ?Sized> RcAddr<T> {
	pub fn new(ptr: Rc<T>) -> Self {
		RcAddr { ptr }
	}
}

impl<T: ?Sized> Deref for RcAddr<T> {
	type Target = Rc<T>;
	fn deref(&self) -> &Self::Target {
		&self.ptr
	}
}

impl<T: ?Sized> PartialEq for RcAddr<T> {
	fn eq(&self, other: &Self) -> bool {
		Rc::as_ptr(&self.ptr).eq(&Rc::as_ptr(&other.ptr))
	}
}

impl<T: ?Sized> Eq for RcAddr<T> {}

impl<T: ?Sized> Ord for RcAddr<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		Rc::as_ptr(&self.ptr).cmp(&Rc::as_ptr(&other.ptr))
	}
}

impl<T: ?Sized> PartialOrd for RcAddr<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Rc::as_ptr(&self.ptr).cmp(&Rc::as_ptr(&other.ptr)))
	}
}

pub struct WeakAddr<T: ?Sized> {
	ptr: Weak<T>,
}

impl<T: ?Sized> WeakAddr<T> {
	pub fn new(ptr: Weak<T>) -> Self {
		WeakAddr { ptr }
	}
}

impl<T: ?Sized> Deref for WeakAddr<T> {
	type Target = Weak<T>;
	fn deref(&self) -> &Self::Target {
		&self.ptr
	}
}

impl<T: ?Sized> PartialEq for WeakAddr<T> {
	fn eq(&self, other: &Self) -> bool {
		Weak::as_ptr(&self.ptr).eq(&Weak::as_ptr(&other.ptr))
	}
}

impl<T: ?Sized> Eq for WeakAddr<T> {}

impl<T: ?Sized> Ord for WeakAddr<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		Weak::as_ptr(&self.ptr).cmp(&Weak::as_ptr(&other.ptr))
	}
}

impl<T: ?Sized> PartialOrd for WeakAddr<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Weak::as_ptr(&self.ptr).cmp(&Weak::as_ptr(&other.ptr)))
	}
}
