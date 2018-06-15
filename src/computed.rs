use atom::{Atom, EvalContext, EvaluateKey, Evaluateable};
use std::cell::Ref;
use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::rc::{Rc, Weak};

pub type Computation<T> = Fn(&mut EvalContext) -> T;

pub struct ComputedGuard<'a, T: Eq + 'a> {
  guard: Ref<'a, ComputedBody<T>>,
}

impl<'b, T: Eq> Deref for ComputedGuard<'b, T> {
  type Target = T;
  fn deref(&self) -> &T {
    &self.guard.value.as_ref().unwrap()
  }
}

pub struct Computed<T: Eq + 'static> {
  body: Rc<RefCell<ComputedBody<T>>>,
}

impl<T: Eq + 'static> Clone for Computed<T> {
  fn clone(&self) -> Self {
    return Computed {
      body: self.body.clone(),
    };
  }
}

impl<T: Eq + 'static> Computed<T> {
  pub fn new<F: Fn(&mut EvalContext) -> T + 'static>(computation: F) -> Self {
    let computed = Computed {
      body: Rc::new(RefCell::new(ComputedBody::new(Box::new(computation)))),
    };

    {
      let mut body = computed.body.borrow_mut();
      body
        .atom
        .ext_mut()
        .insert::<EvaluateKey>(Box::new(computed.weak()));
    }

    computed
  }

  fn weak(&self) -> WeakComputed<T> {
    return WeakComputed {
      body: Rc::downgrade(&self.body),
    };
  }

  pub fn on_become_observed<F: Fn() + 'static>(&mut self, cb: F) {
    self.body.borrow_mut().atom.on_become_observed(cb)
  }

  pub fn on_become_unobserved<F: Fn() + 'static>(&mut self, cb: F) {
    self.body.borrow_mut().atom.on_become_unobserved(cb)
  }

  fn get_inner(&self, ctx: Option<&mut EvalContext>) -> ComputedGuard<T> {
    {
      let atom = self.body.borrow().atom.clone();
      if ctx.is_some() {
        atom.report_used_in(ctx.unwrap());
      }
      atom.update();
    }

    let body = self.body.borrow();
    ComputedGuard { guard: body }
  }

  pub fn once(&self) -> ComputedGuard<T> {
    self.get_inner(None)
  }

  pub fn observe(&self, ctx: &mut EvalContext) -> ComputedGuard<T> {
    self.get_inner(Some(ctx))
  }
}

pub struct WeakComputed<T: Eq> {
  body: Weak<RefCell<ComputedBody<T>>>,
}

impl<T: Eq + 'static> Evaluateable for WeakComputed<T> {
  fn evaluate(&mut self, ctx: &mut EvalContext) -> bool {
    let body = self.body.upgrade().expect("Access to destroyed Computed");
    let mut mut_body = body.borrow_mut();
    mut_body.evaluate(ctx)
  }
}

impl<T: Eq + 'static> WeakComputed<T> {}

struct ComputedBody<T> {
  value: Option<T>,
  computation: Box<Computation<T>>,
  atom: Atom,
}

impl<T: Eq + 'static> ComputedBody<T> {
  fn new(computation: Box<Computation<T>>) -> Self {
    ComputedBody {
      value: None,
      computation,
      atom: Atom::new("Computed".to_string()),
    }
  }
}

impl<T: Eq + 'static> Evaluateable for ComputedBody<T> {
  fn evaluate(&mut self, ctx: &mut EvalContext) -> bool {
    let res = (self.computation)(ctx);
    if self.value.as_ref() != Some(&res) {
      mem::replace(&mut self.value, Some(res));
      return true;
    } else {
      return false;
    }
  }
}
