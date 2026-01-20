use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::{Arc, Weak};

pub struct ArcAddr<T: ?Sized> {
	ptr: Arc<T>,
}

impl<T: ?Sized> ArcAddr<T> {
	pub fn new(ptr: Arc<T>) -> Self {
		ArcAddr { ptr }
	}
}

impl<T: ?Sized> Deref for ArcAddr<T> {
	type Target = Arc<T>;
	fn deref(&self) -> &Self::Target {
		&self.ptr
	}
}

impl<T: ?Sized> PartialEq for ArcAddr<T> {
	fn eq(&self, other: &Self) -> bool {
		std::ptr::addr_eq(&self.ptr, &other.ptr)
	}
}

impl<T: ?Sized> Eq for ArcAddr<T> {}

impl<T: ?Sized> Ord for ArcAddr<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		Arc::as_ptr(&self.ptr).cmp(&Arc::as_ptr(&other.ptr))
	}
}

impl<T: ?Sized> PartialOrd for ArcAddr<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Arc::as_ptr(&self.ptr).cmp(&Arc::as_ptr(&other.ptr)))
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
		std::ptr::addr_eq(&self.ptr, &other.ptr)
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
