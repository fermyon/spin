use super::bindings::exports::wasi::http::incoming_handler::Guest;
use super::bindings::wasi::http::types::{
    Error, Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use super::bindings::wasi::io::streams::OutputStream;

impl Guest for super::Component {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
        let response = |status| OutgoingResponse::new(status, &Headers::new(&[]));
        match super::main() {
            Ok(()) => ResponseOutparam::set(response_out, Ok(response(200))),
            Err(err) => {
                let resp = response(500);
                let body = resp.write().expect("response body was already taken");
                ResponseOutparam::set(response_out, Ok(resp));
                outgoing_body(body, err.into_bytes()).unwrap();
            }
        }
    }
}

fn outgoing_body(body: OutgoingBody, buffer: Vec<u8>) -> Result<(), Error> {
    struct Outgoing(Option<(OutputStream, OutgoingBody)>);

    impl Drop for Outgoing {
        fn drop(&mut self) {
            if let Some((stream, body)) = self.0.take() {
                drop(stream);
                OutgoingBody::finish(body, None);
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
                        Err(e) => return Err(Error::ProtocolError(format!("I/O error: {e}"))),
                    }
                }
            }
            Err(e) => return Err(Error::ProtocolError(format!("I/O error: {e}"))),
        }
    }
}

macro_rules! ensure {
    ($expr:expr) => {{
        if !$expr {
            let krate = module_path!().split("::").next().unwrap();
            let file = file!();
            let line = line!();
            return Err(format!(
                "{krate}: `{}` ({file}:{line}) unexpectedly returned false",
                stringify!($expr),
            ));
        }
    }};
}

macro_rules! r#try {
    ($expr:expr) => {
        match $expr {
            Ok(s) => s,
            Err(e) => {
                let krate = module_path!().split("::").next().unwrap();
                let file = file!();
                let line = line!();
                return Err(format!(
                    "{krate}: `{}` ({file}:{line}) errored: '{e}'",
                    stringify!($expr)
                ));
            }
        }
    };
}
