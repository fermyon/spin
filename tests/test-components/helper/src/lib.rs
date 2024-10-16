#[cfg(feature = "define-component")]
pub mod bindings {
    wit_bindgen::generate!({
        world: "fermyon:spin/platform@3.0.0",
        path: "../../../wit",
        runtime_path: "::wit_bindgen::rt"
    });
}

#[cfg(feature = "define-component")]
use bindings::wasi::http0_2_0::types::{
    ErrorCode, Fields, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
#[cfg(feature = "define-component")]
use bindings::wasi::io0_2_0::streams::OutputStream;

#[cfg(feature = "define-component")]
#[macro_export]
macro_rules! define_component {
    ($name:ident) => {
        // Unfortunately wit-bindgen currently requires us to generate bindings
        // in the same crate as the component which implements the export.
        // For now, this assumes the crate using this macro has `wit-bindgen` as a dependency
        mod bindings {
            $crate::wit_bindgen::generate!({
                world: "fermyon:spin/http-trigger@3.0.0",
                path: "../../../../wit",
                exports: {
                    "wasi:http/incoming-handler@0.2.0": super::Component
                },
            });
        }

        use bindings::exports::wasi::http0_2_0::incoming_handler::{Guest, IncomingRequest, ResponseOutparam};
        struct $name;

        impl Guest for $name {
            fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
                $crate::handle(response_out.into(), $name::main())
            }
        }

        impl From<ResponseOutparam> for $crate::bindings::wasi::http0_2_0::types::ResponseOutparam {
            fn from(value: ResponseOutparam) -> Self {
                unsafe { Self::from_handle(value.into_handle()) }
            }
        }
    };
}

#[cfg(feature = "define-component")]
pub fn handle(response_out: ResponseOutparam, result: Result<(), String>) {
    let response = |status| {
        let resp = OutgoingResponse::new(Fields::new());
        resp.set_status_code(status).expect("should have set status code");
        resp
    };
    match result {
        Ok(()) => ResponseOutparam::set(response_out, Ok(response(200))),
        Err(err) => {
            let resp = response(500);
            let body = resp.body().unwrap();
            ResponseOutparam::set(response_out, Ok(resp));
            outgoing_body(body, err.into_bytes()).unwrap();
        }
    }
}

#[cfg(feature = "define-component")]
pub fn outgoing_body(body: OutgoingBody, buffer: Vec<u8>) -> Result<(), ErrorCode> {
    struct Outgoing(Option<(OutputStream, OutgoingBody)>);

    impl Drop for Outgoing {
        fn drop(&mut self) {
            if let Some((stream, body)) = self.0.take() {
                drop(stream);
                OutgoingBody::finish(body, None).expect("should have finished OutgoingBody");
            }
        }
    }

    let stream = body.write().expect("response body should be writable");
    let pair = Outgoing(Some((stream, body)));

    let mut offset = 0;
    let mut flushing = false;
    loop {
        let stream = &pair.0.as_ref().unwrap().0;
        match stream.check_write() {
            Ok(0) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(count) => {
                if offset == buffer.len() {
                    if flushing {
                        return Ok(());
                    } else {
                        stream.flush().expect("stream should be flushable");
                        flushing = true;
                    }
                } else {
                    let count = usize::try_from(count).unwrap().min(buffer.len() - offset);

                    match stream.write(&buffer[offset..][..count]) {
                        Ok(()) => {
                            offset += count;
                        }
                        Err(e) => return Err(ErrorCode::InternalError(Some(format!("I/O error: {e}")))),
                    }
                }
            }
            Err(e) => return Err(ErrorCode::InternalError(Some(format!("I/O error: {e}")))),
        }
    }
}

#[macro_export]
macro_rules! ensure {
    ($expr:expr) => {{
        if !$expr {
            $crate::bail!("`{}` unexpectedly returned false", stringify!($expr))
        }
    }};
}

#[macro_export]
macro_rules! ensure_ok {
    ($expr:expr) => {
        match $expr {
            Ok(s) => s,
            Err(e) => $crate::bail!("`{}` errored: '{e}'", stringify!($expr)),
        }
    };
}

#[macro_export]
macro_rules! ensure_some {
    ($expr:expr) => {
        match $expr {
            Some(e) => e,
            None => $crate::bail!("`{}` was None", stringify!($expr)),
        }
    };
}

#[macro_export]
macro_rules! ensure_matches {
    ($expr:expr, $($arg:tt)*) => {
        if !matches!($expr, $($arg)*) {
            $crate::bail!("`{:?}` did not match `{}`", $expr, stringify!($($arg)*))
        }
    };
}

#[macro_export]
macro_rules! ensure_eq {
    ($expr1:expr, $expr2:expr) => {
        if $expr1 != $expr2 {
            $crate::bail!("`{}` != `{}`", stringify!($expr1), stringify!($expr2));
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($fmt:expr, $($arg:tt)*) => {{
        let krate = module_path!().split("::").next().unwrap();
        let file = file!();
        let line = line!();
        return Err(format!(
            "{krate}#({file}:{line}) {}", format_args!($fmt, $($arg)*)
        ));
    }};
}

#[cfg(feature = "define-component")]
pub use wit_bindgen;
