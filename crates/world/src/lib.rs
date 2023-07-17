#![allow(missing_docs)]

wasmtime::component::bindgen!({
    path: "../../wit",
    world: "reactor",
    async: true
});

pub use fermyon::spin::*;
