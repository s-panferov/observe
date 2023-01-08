use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

pub struct Hashed<T> {
	pub value: T,
	pub hash: u64,
}

impl<T> Hashed<T> {
	pub fn new(value: T) -> Self
	where
		T: Hash,
	{
		let hash = fxhash::hash64(&value);
		Self { value, hash }
	}
}

impl<T> Deref for Hashed<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T> Debug for Hashed<T>
where
	T: Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.value.fmt(f)
	}
}
