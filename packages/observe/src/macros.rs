pub use enclose::*;

#[macro_export]
macro_rules! computed {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::Computed::new($crate::macros::enclose!(($( $d_tt )*) move |$ctx: &observe::EvalContext| { $($b)* }))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::Computed::new(move |$ctx: &observe::EvalContext| { $($b)* })
    };
}

#[macro_export]
macro_rules! tx {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::transaction(None, $crate::macros::enclose!(($( $d_tt )*) move |$ctx: &mut observe::Transaction| { $($b)* }))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::transaction(None, move |$ctx: &mut observe::Transaction| { $($b)* })
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
