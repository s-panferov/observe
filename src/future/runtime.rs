#![cfg_attr(nightly, feature(coerce_unsized))]

use futures::Future;

/// https://github.com/rust-lang/rust/issues/27732
pub trait FutureRuntime: Send + Sync {
    type Bounds: ?Sized;

    #[cfg(nightly)]
    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static + std::ops::CoerceUnsized<Self::Bounds>;

    #[cfg(not(nightly))]
    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static;
}

#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
pub struct TokioLocal {}

#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
impl FutureRuntime for TokioLocal {
    type Bounds = dyn Empty;

    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        tokio::task::spawn_local(f);
    }
}

#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
pub struct Tokio {}

#[cfg(nightly)]
#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
impl FutureRuntime for Tokio {
    type Bounds = dyn Send;

    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static + std::ops::CoerceUnsized<Self::Bounds>,
    {
        tokio::task::spawn(f);
    }
}

#[cfg(feature = "wasm-bindgen-futures-0")]
pub struct WasmBindgen {}

pub trait Empty {}
impl<T> Empty for T {}

#[cfg(feature = "wasm-bindgen-futures-0")]
impl FutureRuntime for WasmBindgen {
    type Bounds = dyn Empty;

    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        wasm_bindgen_futures::spawn_local(f)
    }
}
