mod gl;
mod linalg;
pub mod simulations;

use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console::log_1(&"hello".into());
    Ok(())
}
