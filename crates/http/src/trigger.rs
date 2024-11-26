use serde::{Deserialize, Serialize};
use wasmtime::component::Component;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    // The based url
    #[serde(default = "default_base")]
    pub base: String,
}

pub fn default_base() -> String {
    "/".into()
}

/// The type of http handler export used by a component.
#[derive(Clone, Copy)]
pub enum HandlerType {
    Spin,
    Wagi,
    Wasi0_2,
    Wasi2023_11_10,
    Wasi2023_10_18,
}

/// The `incoming-handler` export for `wasi:http` version rc-2023-10-18
pub const WASI_HTTP_EXPORT_2023_10_18: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-10-18";
/// The `incoming-handler` export for `wasi:http` version rc-2023-11-10
pub const WASI_HTTP_EXPORT_2023_11_10: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-11-10";
/// The `incoming-handler` export prefix for all `wasi:http` 0.2 versions
pub const WASI_HTTP_EXPORT_0_2_PREFIX: &str = "wasi:http/incoming-handler@0.2";
/// The `inbound-http` export for `fermyon:spin`
pub const SPIN_HTTP_EXPORT: &str = "fermyon:spin/inbound-http";

impl HandlerType {
    /// Determine the handler type from the exports of a component.
    pub fn from_component(
        engine: &wasmtime::Engine,
        component: &Component,
    ) -> anyhow::Result<HandlerType> {
        let mut handler_ty = None;

        let mut set = |ty: HandlerType| {
            if handler_ty.is_none() {
                handler_ty = Some(ty);
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "component exports multiple different handlers but \
                     it's expected to export only one"
                ))
            }
        };
        let ty = component.component_type();
        for (name, _) in ty.exports(engine) {
            match name {
                WASI_HTTP_EXPORT_2023_10_18 => set(HandlerType::Wasi2023_10_18)?,
                WASI_HTTP_EXPORT_2023_11_10 => set(HandlerType::Wasi2023_11_10)?,
                SPIN_HTTP_EXPORT => set(HandlerType::Spin)?,
                name if name.starts_with(WASI_HTTP_EXPORT_0_2_PREFIX) => set(HandlerType::Wasi0_2)?,
                _ => {}
            }
        }

        handler_ty.ok_or_else(|| {
            anyhow::anyhow!(
                "Expected component to export one of \
                `{WASI_HTTP_EXPORT_2023_10_18}`, \
                `{WASI_HTTP_EXPORT_2023_11_10}`, \
                `{WASI_HTTP_EXPORT_0_2_PREFIX}.*`, \
                 or `{SPIN_HTTP_EXPORT}` but it exported none of those"
            )
        })
    }
}
