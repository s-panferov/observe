use super::{EmptyBody, RawTracker};
use crate::{
    types::{Apply, Forall, Higher},
    Evaluation,
};

use std::{
    any::Any,
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{Arc, RwLock},
};

pub trait TrackerImpl: Sized {
    type Any: ?Sized;
    type Ptr: Higher + Apply<Self::Any>;
    type PtrWeak: Higher + Apply<Self::Any>;

    type Body;
    type WeakBody;

    type Eval: Evaluation<Self> + ?Sized;

    fn clone_body(value: &Self::Body) -> Self::Body;
    fn clone_weak_body(value: &Self::WeakBody) -> Self::WeakBody;

    fn empty_body() -> Box<Self::Eval>;

    fn ptr_clone<T>(value: &<Self::Ptr as Apply<T>>::Result) -> <Self::Ptr as Apply<T>>::Result
    where
        Self::Ptr: Apply<T>;

    fn ptr_downgrade<T>(
        value: &<Self::Ptr as Apply<T>>::Result,
    ) -> <Self::PtrWeak as Apply<T>>::Result
    where
        Self::Ptr: Apply<T>,
        Self::PtrWeak: Apply<T>;

    fn ptr_upgrade<T>(
        value: &<Self::PtrWeak as Apply<T>>::Result,
    ) -> Option<<Self::Ptr as Apply<T>>::Result>
    where
        Self::Ptr: Apply<T>,
        Self::PtrWeak: Apply<T>;

    fn ptr_wrap<T>(value: T) -> <Self::Ptr as Apply<T>>::Result
    where
        Self::Ptr: Apply<T>;

    fn wrap(value: RawTracker<Self>) -> Self::Body;
    fn downgrade(value: &Self::Body) -> Self::WeakBody;
    fn upgrade(value: &Self::WeakBody) -> Option<Self::Body>;

    fn read<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&RawTracker<Self>) -> R;

    fn write<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&mut RawTracker<Self>) -> R;
}

pub struct Local;

impl TrackerImpl for Local {
    type Any = dyn Any + 'static;
    type Ptr = Rc<Forall>;
    type PtrWeak = std::rc::Weak<Forall>;

    type Body = Rc<RefCell<RawTracker<Self>>>;
    type WeakBody = Weak<RefCell<RawTracker<Self>>>;

    type Eval = dyn Evaluation<Self>;

    fn empty_body() -> Box<Self::Eval> {
        Box::new(EmptyBody {})
    }

    fn ptr_clone<T>(value: &<Self::Ptr as Apply<T>>::Result) -> <Self::Ptr as Apply<T>>::Result {
        value.clone()
    }

    fn ptr_wrap<T>(value: T) -> <Self::Ptr as Apply<T>>::Result {
        Rc::<T>::new(value)
    }

    fn ptr_downgrade<T>(
        value: &<Self::Ptr as Apply<T>>::Result,
    ) -> <Self::PtrWeak as Apply<T>>::Result {
        Rc::downgrade(&value)
    }

    fn ptr_upgrade<T>(
        value: &<Self::PtrWeak as Apply<T>>::Result,
    ) -> Option<<Self::Ptr as Apply<T>>::Result> {
        value.upgrade()
    }

    fn clone_body(value: &Self::Body) -> Self::Body {
        value.clone()
    }

    fn clone_weak_body(value: &Self::WeakBody) -> Self::WeakBody {
        value.clone()
    }

    fn upgrade(value: &Self::WeakBody) -> Option<Self::Body> {
        value.upgrade()
    }

    fn downgrade(value: &Self::Body) -> Self::WeakBody {
        Rc::downgrade(&value)
    }

    fn wrap(value: RawTracker<Self>) -> Self::Body {
        Rc::new(RefCell::new(value))
    }

    fn read<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&RawTracker<Self>) -> R,
    {
        let body = value.borrow();
        handler(&body)
    }

    fn write<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&mut RawTracker<Self>) -> R,
    {
        let mut body = value.borrow_mut();
        handler(&mut body)
    }
}

pub struct Shared;

impl TrackerImpl for Shared {
    type Any = dyn Any + Send + Sync + 'static;

    type Ptr = Arc<Forall>;
    type PtrWeak = std::sync::Weak<Forall>;

    type Body = Arc<RwLock<RawTracker<Self>>>;
    type WeakBody = std::sync::Weak<RwLock<RawTracker<Self>>>;

    type Eval = dyn Evaluation<Self> + Send + Sync;

    fn empty_body() -> Box<Self::Eval> {
        Box::new(EmptyBody {})
    }

    fn ptr_clone<T>(value: &<Self::Ptr as Apply<T>>::Result) -> <Self::Ptr as Apply<T>>::Result {
        value.clone()
    }

    fn ptr_wrap<T>(value: T) -> <Self::Ptr as Apply<T>>::Result {
        Arc::<T>::new(value)
    }

    fn ptr_downgrade<T>(
        value: &<Self::Ptr as Apply<T>>::Result,
    ) -> <Self::PtrWeak as Apply<T>>::Result {
        Arc::downgrade(&value)
    }

    fn ptr_upgrade<T>(
        value: &<Self::PtrWeak as Apply<T>>::Result,
    ) -> Option<<Self::Ptr as Apply<T>>::Result> {
        value.upgrade()
    }

    fn clone_body(value: &Self::Body) -> Self::Body {
        value.clone()
    }

    fn clone_weak_body(value: &Self::WeakBody) -> Self::WeakBody {
        value.clone()
    }

    fn upgrade(value: &Self::WeakBody) -> Option<Self::Body> {
        value.upgrade()
    }

    fn downgrade(value: &Self::Body) -> Self::WeakBody {
        Arc::downgrade(&value)
    }

    fn wrap(value: RawTracker<Self>) -> Self::Body {
        Arc::new(RwLock::new(value))
    }

    fn read<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&RawTracker<Self>) -> R,
    {
        let body = value.read().unwrap();
        handler(&body)
    }

    fn write<F, R>(value: &Self::Body, handler: F) -> R
    where
        F: FnOnce(&mut RawTracker<Self>) -> R,
    {
        let mut body = value.write().unwrap();
        handler(&mut body)
    }
}
