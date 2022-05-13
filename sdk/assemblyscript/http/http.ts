export { requestFromCgi, sendResponse } from "./cgi";

export type Handler = (request: Request) => Response;


/** An HTTP request. */
export class Request {
    /** The URL of the request. */
    public url: string;
    /** The HTTP method of the request. */
    public method: Method;
    /** The request headers. */
    public headers: Map<string, string>;
    /** The request body as bytes. */
    public body: ArrayBuffer;

    constructor(
        url: string,
        method: Method = Method.GET,
        headers: Map<string, string> = new Map<string, string>(),
        body: ArrayBuffer = new ArrayBuffer(0)
    ) {
        this.url = url;
        this.method = method;
        this.headers = headers;
        this.body = body;
    }
}

/** An HTTP request builder. */
export class RequestBuilder {
    private request: Request;

    constructor(url: string) {
        this.request = new Request(url);
    }

    /** Set the request's HTTP method. */
    public method(m: Method): RequestBuilder {
        this.request.method = m;
        return this;
    }

    /** Add a new pair of header key and header value to the request. */
    public header(key: string, value: string): RequestBuilder {
        this.request.headers.set(key, value);
        return this;
    }

    /** Set the request's body. */
    public body(b: ArrayBuffer): RequestBuilder {
        this.request.body = b;
        return this;
    }

    /** Send the request and return an HTTP response. */
    public send(): Response {
        return new Response(StatusCode.OK);
    }
}

/** An HTTP response. */
export class Response {
    /** The HTTP response status code. */
    public status: StatusCode;
    /** The response headers. */
    public headers: Map<string, string>;
    /** The response body */
    public body: ArrayBuffer;

    public constructor(
        status: StatusCode,
        headers: Map<string, string> = new Map<string, string>(),
        body: ArrayBuffer = new ArrayBuffer(0)
    ) {
        this.status = status;
        this.headers = headers;
        this.body = body;
    }
}

/** An HTTP response builder. */
export class ResponseBuilder {
    private response: Response;

    constructor(status: StatusCode) {
        this.response = new Response(status);
    }

    /** Add a new pair of header key and header value to the response. */
    public header(key: string, value: string): ResponseBuilder {
        this.response.headers.set(key, value);
        return this;
    }

    /** Set the response body and get the actual response. */
    public body(body: ArrayBuffer = new ArrayBuffer(0)): Response {
        this.response.body = changetype<ArrayBuffer>(body);
        return this.response;
    }
}

/** The standard HTTP methods. */
export enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

export namespace Method {
    /** Parse a string into an HTTP method. */
    export function parse(m: string): Method {
        if (m == "GET" || m == "get") {
            return Method.GET;
        } else if (m == "HEAD" || m == "head") {
            return Method.HEAD;
        } else if (m == "POST" || m == "post") {
            return Method.POST;
        } else if (m == "PUT" || m == "put") {
            return Method.PUT;
        } else if (m == "DELETE" || m == "delete") {
            return Method.DELETE;
        } else if (m == "CONNECT" || m == "connect") {
            return Method.CONNECT;
        } else if (m == "OPTIONS" || m == "options") {
            return Method.OPTIONS;
        } else if (m == "TRACE" || m == "trace") {
            return Method.TRACE;
        } else if (m == "PATCH" || m == "patch") {
            return Method.PATCH;
        } else {
            return Method.GET;
        }
    }

    /** Convert an HTTP method into a string. */
    export function from(m: Method): string {
        switch (m) {
            case Method.GET:
                return "GET";
            case Method.HEAD:
                return "HEAD";
            case Method.POST:
                return "POST";
            case Method.PUT:
                return "PUT";
            case Method.DELETE:
                return "DELET";
            case Method.CONNECT:
                return "CONNECT";
            case Method.OPTIONS:
                return "OPTIONS";
            case Method.TRACE:
                return "TRACE";
            case Method.PATCH:
                return "PATCH";
            default:
                return "";
        }
    }
}

/** The standard HTTP status codes. */
export enum StatusCode {
    CONTINUE = 100,
    SWITCHING_PROTOCOL = 101,
    PROCESSING = 102,
    EARLY_HINTS = 103,

    OK = 200,
    CREATED = 201,
    ACCEPTED = 202,
    NON_AUTHORITATIVE_INFORMATION = 203,
    NO_CONTENT = 204,
    RESET_CONTENT = 205,
    PARTIAL_CONTENT = 206,
    MULTI_STATUS = 207,
    ALREADY_REPORTED = 208,
    IM_USED = 226,

    MULTIPLE_CHOICE = 300,
    MOVED_PERMANENTLY = 301,
    FOUND = 302,
    SEE_OTHER = 303,
    NOT_MODIFIED = 304,
    USE_PROXY = 305,
    UNUSED = 306,
    TEMPORARY_REDIRECT = 307,
    PERMANENT_REDIRECT = 308,

    BAD_REQUEST = 400,
    UNAUTHORIZED = 401,
    PAYMENT_REQUIRED = 402,
    FORBIDDEN = 403,
    NOT_FOUND = 404,
    METHOD_NOT_ALLOWED = 405,
    NOT_ACCEPTABLE = 406,
    PROXY_AUTHENTICATION_REQUIRED = 407,
    REQUEST_TIMEOUT = 408,
    CONFLICT = 409,
    GONE = 410,
    LENGTH_REQUIRED = 411,
    PRECONDITION_FAILED = 412,
    PAYLOAD_TOO_LARGE = 413,
    URI_TOO_LONG = 414,
    UNSUPPORTED_MEDIA_TYPE = 415,
    RANGE_NOT_SATISFIABLE = 416,
    EXPECTATION_FAILED = 417,
    IM_A_TEAPOT = 418,
    MISDIRECTED_REQUEST = 421,
    UNPROCESSABLE_ENTITY = 422,
    LOCKED = 423,
    FAILED_DEPENDENCY = 424,
    TOO_EARLY = 425,
    UPGRADE_REQUIRED = 426,
    PRECONDITION_REQURIED = 428,
    TOO_MANY_REQUESTS = 429,
    REQUEST_HEADER_FIELDS_TOO_LARGE = 431,
    UNAVAILABLE_FOR_LEGAL_REASONS = 451,

    INTERNAL_SERVER_ERROR = 500,
    NOT_IMPLELENTED = 501,
    BAD_GATEWAY = 502,
    SERVICE_UNAVAILABLE = 503,
    GATEWAY_TIMEOUT = 504,
    HTTP_VERSION_NOT_SUPPORTED = 505,
    VARIANT_ALSO_NEGOTIATES = 506,
    INSUFFICIENT_STORAGE = 507,
    LOOP_DETECTED = 508,
    NOT_EXTENDED = 510,
    NETWORK_AUTHENTICATION_REQUIRED = 511,
}
