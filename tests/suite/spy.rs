use mockers_derive::mock;

pub trait Spy {
  fn trigger(&self);
  fn u32(&self, test: u32);
}

// This mock shares expectations between clones.
mock!{
    SharedSpy,
    self,
    trait Spy {
      fn trigger(&self);
      fn u32(&self, test: u32);
    }
}

mock_clone!(SharedSpy, share_expectations);
