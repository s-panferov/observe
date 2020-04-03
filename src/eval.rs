use crate::{tracker::TrackerImpl, types::Apply, EvalContext};

pub enum Invalidate {
    SelfAndDeps,
    OnlyDeps,
}

pub trait Evaluation<Impl>
where
    Impl: TrackerImpl,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Impl>) -> u64;

    fn is_scheduled(&self) -> bool {
        false
    }

    fn on_reaction(&mut self) {}
    fn on_become_observed(&mut self) {}
    fn on_become_unobserved(&mut self) {}

    fn get(&self) -> <Impl::Ptr as Apply<Impl::Any>>::Result {
        unimplemented!()
    }

    fn set(&mut self, _value: <Impl::Ptr as Apply<Impl::Any>>::Result) -> (u64, Invalidate) {
        unimplemented!()
    }
}

impl<F: 'static, Impl: TrackerImpl> Evaluation<Impl> for F
where
    Impl: TrackerImpl,
    F: Fn(&mut EvalContext<Impl>) -> u64,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Impl>) -> u64 {
        self(ctx)
    }
}
