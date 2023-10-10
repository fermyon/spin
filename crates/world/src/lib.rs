#![allow(missing_docs)]

wasmtime::component::bindgen!({
    path: "../../wit/preview2",
    world: "host",
    async: true
});

pub use fermyon::spin1_0_0 as v1;
pub use fermyon::spin2_0_0 as v2;
