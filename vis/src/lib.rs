#![allow(non_snake_case)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hello_world() -> String {
    "Hello, world!".to_string()
}
