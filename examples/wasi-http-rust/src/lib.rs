wit_bindgen::generate!("proxy" in "../../wit/preview2");

use self::http::{IncomingRequest, ResponseOutparam, Http};

struct Component;

impl Http for Component {
    fn handle(request: IncomingRequest, _response_out: ResponseOutparam) {
        let method = types2::incoming_request_method(request);
        todo!("http-component-handle {:?}", method)
    }
}

export_proxy!(Component);