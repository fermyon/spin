import * as wasi from 'as-wasi';
import { Method, Request, Response } from "./http";

export function requestFromCgi(): Request {
    let uri = requestUriFromEnv();
    let method = requestMethodFromEnv();
    let headers = requestHeadersFromEnv();
    // let body = requestBodyFromEnv();

    return new Request(uri, method, headers);
}

export function sendResponse(response: Response): void {
    printHeaders(response);
    // wasi.Descriptor.Stdout.write(changetype<u8[]>(response.body));
}

function requestHeadersFromEnv(): Map<string, string> {
    let res = new Map<string, string>();
    let env = new wasi.Environ().all();

    for (var i = 0; i < env.length; i++) {
        // only set as request headers the environment variables that start with HTTP_
        if (env[i].key.startsWith("HTTP_")) {
            res.set(env[i].key.replace("HTTP_", "").toLowerCase(), env[i].value.toLowerCase());
        }
    }

    return res;
}

/** Return the request body from the standard input. */
function requestBodyFromEnv(): ArrayBuffer {
    let bytes = wasi.Descriptor.Stdin.readAll() || [];
    if (bytes !== null) {
        wasi.Console.error("Body size: " + bytes.length.toString());
    }
    return changetype<ArrayBuffer>(bytes);
}

function requestUriFromEnv(): string {
    let url = new wasi.Environ().get("X_FULL_URL");
    if (url !== null) {
        return url;
    } else {
        return "";
    }
}

function requestMethodFromEnv(): Method {
    let method = new wasi.Environ().get("REQUEST_METHOD");
    if (method !== null) {
        return Method.parse(method);
    } else {
        return Method.GET;
    }
}


function printHeaders(response: Response): void {
    let location = searchCaseInsensitive("location", response.headers);
    if (location !== null) {
        wasi.Console.write("Location: " + location, true);
    }
    let contentType = searchCaseInsensitive("content-type", response.headers);
    if (contentType !== null) {
        wasi.Console.write("Content-Type: " + contentType, true);
    }

    for (var i = 0; i < response.headers.size; i++) {
        wasi.Console.write(response.headers.keys()[i] + ": " + response.headers.values()[i]);
    }
    wasi.Console.write("Status: " + response.status.toString());
    wasi.Console.write("\n");
}

function searchCaseInsensitive(key: string, map: Map<string, string>): string | null {
    for (var i = 0; i < map.size; i++) {
        if (map.keys()[i].toLowerCase() === key.toLowerCase()) {
            return map.values()[i].toLowerCase();
        }
    }

    return null;
}
