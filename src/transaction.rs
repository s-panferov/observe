use atom::Atom;
use reaction::{Reaction, ReactionKey};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub struct Transaction {
  changed: HashSet<Atom>,
}

impl Transaction {
  pub fn new() -> Self {
    Transaction {
      changed: HashSet::new(),
    }
  }

  pub fn mark_changed(&mut self, atom: Atom) -> bool {
    self.changed.insert(atom)
  }

  pub fn fire_reactions<'a, I>(changed: I)
  where
    I: Iterator<Item = &'a Atom>,
  {
    let affected: Rc<RefCell<HashSet<Reaction>>> = Rc::new(RefCell::new(HashSet::new()));

    let mut walker = {
      let mut affected = affected.clone();
      move |atom: &Atom| {
        let ext = atom.ext();
        let reaction = ext.get::<ReactionKey>();
        if reaction.is_some() {
          let reaction = reaction.unwrap().0.upgrade();
          if reaction.is_some() {
            affected.borrow_mut().insert(reaction.unwrap());
          }
          false
        } else {
          atom.is_observed()
        }
      }
    };

    // find all affected reactions
    for changed in changed {
      changed.walk(&mut walker)
    }

    for mut reaction in &mut affected.borrow().iter() {
      reaction.clone().maybe_run();
    }
  }

  fn commit(&mut self) {
    Transaction::fire_reactions(self.changed.iter())
  }
}

pub fn transaction<F: FnOnce(&mut Transaction)>(outer: Option<&mut Transaction>, func: F) {
  if outer.is_some() {
    let tx = outer.unwrap();
    func(tx);
  } else {
    let mut tx = Transaction::new();
    func(&mut tx);
    tx.commit();
  };
}
