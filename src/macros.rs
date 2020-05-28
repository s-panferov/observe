pub use enclose::*;

#[macro_export]
macro_rules! computed {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::Computed::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &mut observe::EvalContext| { $($b)* }))
    };
}

#[macro_export]
macro_rules! autorun {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {{
        let computed = observe::Computed::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &mut observe::EvalContext| { $($b)* }));
        computed.autorun();
        computed.update();
        computed
    }};
}

#[macro_export]
macro_rules! future {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::future::ComputedFuture::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &mut observe::EvalContext| { $($b)* }))
    };
}
