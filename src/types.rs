use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug)]
pub struct Forall;

pub trait Higher {
    type Type;
    type Arg;
}

impl<A> Higher for Rc<A> {
    type Type = Rc<Forall>;
    type Arg = A;
}

impl<A> Higher for std::rc::Weak<A> {
    type Type = std::rc::Weak<Forall>;
    type Arg = A;
}

impl<A> Higher for Arc<A> {
    type Type = Arc<Forall>;
    type Arg = A;
}

impl<A> Higher for std::sync::Weak<A> {
    type Type = std::sync::Weak<Forall>;
    type Arg = A;
}

impl<A> Higher for RwLock<A> {
    type Type = RwLock<Forall>;
    type Arg = A;
}

impl<A> Higher for RefCell<A> {
    type Type = RefCell<Forall>;
    type Arg = A;
}

impl<A: ?Sized> Apply<A> for Rc<Forall> {
    type Result = Rc<A>;
}

impl<A: ?Sized> Apply<A> for std::rc::Weak<Forall> {
    type Result = std::rc::Weak<A>;
}

impl<A: ?Sized> Apply<A> for Arc<Forall> {
    type Result = Arc<A>;
}

impl<A: ?Sized> Apply<A> for std::sync::Weak<Forall> {
    type Result = std::sync::Weak<A>;
}

impl<A, B> Apply<A> for RwLock<B> {
    type Result = RwLock<A>;
}

impl<A, B> Apply<A> for RefCell<B> {
    type Result = RefCell<A>;
}

pub trait Apply<A: ?Sized> {
    type Result;
}

pub type Type<H, T> = <H as Apply<T>>::Result;
