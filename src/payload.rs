use std::hash::{Hash, Hasher};

pub enum Payload<T> {
    Nothing,
    Loading,
    Value(T),
}

impl<T> Hash for Payload<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Payload::Nothing => 0.hash(state),
            Payload::Loading => 1.hash(state),
            Payload::Value(v) => v.hash(state),
        }
    }
}
