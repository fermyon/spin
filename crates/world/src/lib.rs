#![allow(missing_docs)]

wasmtime::component::bindgen!({
    inline: r#"
    package fermyon:runtime;
    world host {
        include fermyon:spin/host;
        include fermyon:spin/platform@2.0.0;
    }
    "#,
    path: "../../wit-2023-10-18",
    async: true
});

pub use fermyon::spin as v1;
pub use fermyon::spin2_0_0 as v2;

mod conversions;
