#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(js_name = queueMicrotask)]
	fn queue_microtask(closure: &JsValue);
}

pub fn queue<F: FnOnce() + 'static>(func: F) {
	queue_microtask(&Closure::once_into_js(func));
}
