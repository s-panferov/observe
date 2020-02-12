use std::cell::Ref;
use std::cell::RefCell;
use std::mem;
use std::rc::{Rc, Weak};

use crate::atom::{Atom, EvalContext, EvaluateKey, Evaluateable};
use crate::transaction::Transaction;

pub struct Value<T: Eq + 'static> {
    body: Rc<RefCell<ValueBody<T>>>,
}

impl<T: Eq + 'static> Clone for Value<T> {
    fn clone(&self) -> Self {
        return Value {
            body: self.body.clone(),
        };
    }
}

impl<T: Eq + 'static> Value<T> {
    pub fn new(value: T) -> Value<T> {
        let value = Value {
            body: Rc::new(RefCell::new(ValueBody::new(value))),
        };

        {
            let mut body = value.body.borrow_mut();
            body.atom
                .ext_mut()
                .insert::<EvaluateKey>(Box::new(value.weak()));
        }

        value
    }

    pub fn weak(&self) -> WeakValue<T> {
        return WeakValue {
            body: Rc::downgrade(&self.body),
        };
    }

    pub fn observe(&self, ctx: &mut EvalContext) -> Ref<T> {
        self.get_inner(Some(ctx))
    }

    pub fn once(&self) -> Ref<T> {
        self.get_inner(None)
    }

    pub fn set(&mut self, next: T, tx: &mut Transaction) {
        self.set_inner(next, Some(tx))
    }

    pub fn set_now(&mut self, next: T) {
        self.set_inner(next, None)
    }

    pub fn on_become_observed<F: Fn() + 'static>(&mut self, cb: F) {
        self.body.borrow_mut().atom.on_become_observed(cb)
    }

    pub fn on_become_unobserved<F: Fn() + 'static>(&mut self, cb: F) {
        self.body.borrow_mut().atom.on_become_unobserved(cb)
    }

    fn get_inner(&self, ctx: Option<&mut EvalContext>) -> Ref<T> {
        {
            let atom = self.body.borrow().atom.clone();
            if ctx.is_some() {
                atom.report_used_in(ctx.unwrap());
            }
            atom.update();
        }

        let body = self.body.borrow();
        Ref::map(body, |b| &b.value)
    }

    fn set_inner(&mut self, next: T, tx: Option<&mut Transaction>) {
        let changed = {
            let mut body = self.body.borrow_mut();
            if body.value != next {
                body.next_value = Some(next);
                true
            } else {
                false
            }
        };

        if changed {
            let mut atom = self.body.borrow().atom.clone();
            atom.report_changed(tx);
        }
    }
}

pub struct WeakValue<T: Eq + 'static> {
    body: Weak<RefCell<ValueBody<T>>>,
}

impl<T: Eq + 'static> Evaluateable for WeakValue<T> {
    fn evaluate(&mut self, _ctx: &mut EvalContext) -> bool {
        let body = self.body.upgrade().expect("Access to destroyed value");
        let mut mut_body = body.borrow_mut();
        mut_body.evaluate()
    }
}

pub struct ValueBody<T: Eq + 'static> {
    value: T,
    next_value: Option<T>,
    atom: Atom,
}

impl<T: Eq + 'static> ValueBody<T> {
    pub fn new(value: T) -> ValueBody<T> {
        ValueBody {
            value,
            next_value: None,
            atom: Atom::new("Value".to_string()),
        }
    }

    fn evaluate(&mut self) -> bool {
        if self.next_value.is_some() {
            let next = mem::replace(&mut self.next_value, None);
            let value = next.unwrap();
            self.value = value;
            true
        } else {
            false
        }
    }
}
