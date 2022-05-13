import { Console } from 'as-wasi';
import { handleRequest, Request, Response, ResponseBuilder, StatusCode } from '../../sdk/assemblyscript';

export function _start(): void {
    handleRequest((request: Request): Response => {
        for (var i = 0; i < request.headers.size; i++) {
            Console.error("Key: " + request.headers.keys()[i]);
            Console.error("Value: " + request.headers.values()[i] + "\n");
        }
        return new ResponseBuilder(StatusCode.FORBIDDEN)
            .header("content-type", "text/plain")
            .header("foo", "bar")
            .body(String.UTF8.encode("Hello, Fermyon!\n"));
    });
}
