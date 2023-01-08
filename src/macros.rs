pub use enclose::*;

#[macro_export]
macro_rules! computed {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::Computed::new($crate::macros::enclose!(($( $d_tt )*) Box::new(move |$ctx: &observe::Evaluation| { $($b)* })))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::Computed::new(Box::new(move |$ctx: &observe::Evaluation| { $($b)* }))
    };
}

#[macro_export]
macro_rules! batch {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::batch(None, $crate::macros::enclose!(($( $d_tt )*) move |$ctx: &mut observe::Batch| { $($b)* }))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::batch(None, move |$ctx: &mut observe::Batch| { $($b)* })
    };
}

#[macro_export]
macro_rules! autorun {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {{
        let computed = observe::Computed::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &observe::EvalContext| { $($b)* }));
        computed.autorun();
        computed.update();
        computed
    }};
    ($ctx:ident => $($b:tt)*) => {{
        let computed = observe::Computed::new(move |$ctx: &observe::EvalContext| { $($b)* });
        computed.autorun();
        computed.update();
        computed
    }};
}

#[macro_export]
macro_rules! future {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::future::ComputedFuture::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &observe::EvalContext| { $($b)* }))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::future::ComputedFuture::new(move |$ctx: &observe::EvalContext| { $($b)* })
    };
}
