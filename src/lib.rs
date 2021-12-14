mod gl;
mod linalg;
pub mod simulations;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}

#[macro_export]
macro_rules! console {
    // ($($arg:tt)*) => {{
    //     let res = std::fmt::format(format_args!($($arg)*));
    //     web_sys::console::log_1(&res.into());
    // }}
    ($($arg:tt)*) => {{}};
}
