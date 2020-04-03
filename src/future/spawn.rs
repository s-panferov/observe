use futures::Future;

pub trait FutureRuntime {
    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static + Send;

    fn spawn_local<F>(future: F)
    where
        F: Future<Output = ()> + 'static;
}

#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
pub struct TokioRuntime {}

#[cfg(feature = "tokio-0")]
#[cfg(not(target_arch = "wasm32"))]
impl FutureRuntime for TokioRuntime {
    fn spawn<F>(f: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        tokio::task::spawn(f);
    }

    fn spawn_local<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        tokio::task::spawn_local(f);
    }
}

#[cfg(feature = "wasm-bindgen-futures-0")]
pub struct WasmRuntime {}

#[cfg(feature = "wasm-bindgen-futures-0")]
impl FutureRuntime for WasmRuntime {
    fn spawn<F>(_f: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        unreachable!()
    }

    fn spawn_local<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        wasm_bindgen_futures::spawn_local(f)
    }
}
