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
macro_rules! reaction {
    (( $($d_tt:tt)* ) $ctx:ident => $($b:tt)*) => {
        observe::Reaction::new($crate::macros::enclose!(($( $d_tt )*) Box::new(move |$ctx: &observe::Evaluation| { $($b)* })))
    };
    ($ctx:ident => $($b:tt)*) => {
        observe::Reaction::new(Box::new(move |$ctx: &observe::Evaluation| { $($b)* }))
    };
}
