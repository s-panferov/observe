use std::hash::Hash;

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
