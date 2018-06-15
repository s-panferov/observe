use atom::{Atom, EvalContext, EvaluateKey, Evaluateable};

use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::rc::{Rc, Weak};

use typemap::Key;

#[derive(Debug)]
pub struct Reaction {
  body: Rc<RefCell<ReactionBody>>,
}

impl Reaction {
  fn new(design: Box<ReactionRun>) -> Self {
    let reaction = Reaction {
      body: Rc::new(RefCell::new(ReactionBody::new(design))),
    };

    {
      let mut body = reaction.body.borrow_mut();

      body
        .atom
        .ext_mut()
        .insert::<ReactionKey>(ReactionValue(reaction.weak()));
    }

    {
      let mut body = reaction.body.borrow_mut();

      body
        .atom
        .ext_mut()
        .insert::<EvaluateKey>(Box::new(reaction.weak()));
    }

    reaction
  }

  fn weak(&self) -> WeakReaction {
    return WeakReaction {
      body: Rc::downgrade(&self.body),
    };
  }

  pub fn run(&self) {
    let atom = self.body.borrow().atom.clone();
    atom.evaluate();
  }

  pub fn maybe_run(&self) {
    let atom = self.body.borrow().atom.clone();
    atom.update();
  }
}

impl Drop for Reaction {
  fn drop(&mut self) {
    // will drop body
    if Rc::strong_count(&self.body) == 1 {
      let body = self.body.borrow_mut();
      for atom in body.atom.based_on().iter() {
        atom.clone().become_unobserved()
      }
    }
  }
}

impl PartialEq for Reaction {
  fn eq(&self, other: &Reaction) -> bool {
    self.body.borrow().eq(&*other.body.borrow())
  }
}

impl Hash for Reaction {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.body.borrow().hash(state);
  }
}

impl Eq for Reaction {}

impl Clone for Reaction {
  fn clone(&self) -> Self {
    Reaction {
      body: self.body.clone(),
    }
  }
}

trait ReactionRun {
  fn run_reaction(&mut self, &mut EvalContext);
}

struct Autorun {
  cb: Box<Fn(&mut EvalContext)>,
}

impl ReactionRun for Autorun {
  fn run_reaction(&mut self, context: &mut EvalContext) {
    (self.cb)(context)
  }
}

struct Expression<T: Eq> {
  expr: Box<Fn(&mut EvalContext) -> T>,
  eff: Box<Fn(&mut T, &mut EvalContext)>,
  cached: Option<T>,
}

impl<T: Eq> ReactionRun for Expression<T> {
  fn run_reaction(&mut self, context: &mut EvalContext) {
    let mut value = (self.expr)(context);
    if self.cached.as_ref() != Some(&value) {
      (self.eff)(&mut value, context);
      let _old = mem::replace(&mut self.cached, Some(value));
    }
  }
}

pub fn autorun<F: Fn(&mut EvalContext) + 'static>(func: F) -> Reaction {
  Reaction::new(Box::new(Autorun { cb: Box::new(func) }))
}

pub fn reaction<
  T: Eq + 'static,
  Expr: Fn(&mut EvalContext) -> T + 'static,
  Eff: Fn(&mut T, &mut EvalContext) + 'static,
>(
  expr: Expr,
  eff: Eff,
) -> Reaction {
  Reaction::new(Box::new(Expression {
    expr: Box::new(expr),
    eff: Box::new(eff),
    cached: None,
  }))
}

struct ReactionBody {
  calc: Box<ReactionRun>,
  atom: Atom,
}

impl fmt::Debug for ReactionBody {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Reaction[atom: {:?}]", self.atom)
  }
}

pub struct ReactionKey;
pub struct ReactionValue(pub WeakReaction);

impl Key for ReactionKey {
  type Value = ReactionValue;
}

pub struct WeakReaction {
  body: Weak<RefCell<ReactionBody>>,
}

impl WeakReaction {
  pub fn upgrade(&self) -> Option<Reaction> {
    let body = self.body.upgrade();
    body.map(|body| Reaction { body: body })
  }
}

impl Evaluateable for WeakReaction {
  fn evaluate(&mut self, ctx: &mut EvalContext) -> bool {
    let body = self.body.upgrade().expect("Access to destroyed Reaction");
    let mut mut_body = body.borrow_mut();
    mut_body.evaluate(ctx)
  }
}

impl ReactionBody {
  pub fn new(calc: Box<ReactionRun>) -> Self {
    ReactionBody {
      calc: calc,
      atom: Atom::new("Reaction".to_string()),
    }
  }

  pub fn evaluate(&mut self, ctx: &mut EvalContext) -> bool {
    self.calc.run_reaction(ctx);

    for atom in &mut ctx.diff_added() {
      atom.become_observed()
    }

    for atom in &mut ctx.diff_removed() {
      atom.become_unobserved()
    }

    true
  }
}

impl PartialEq for ReactionBody {
  fn eq(&self, other: &ReactionBody) -> bool {
    *self.atom.id() == *other.atom.id()
  }
}

impl Hash for ReactionBody {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.atom.id().hash(state);
  }
}

impl Eq for ReactionBody {}
