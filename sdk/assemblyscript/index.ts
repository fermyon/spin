import { Console } from 'as-wasi';
import * as http from './http/http';

export * from './http/http';

export function handleRequest(handler: http.Handler): void {
  let response = handler(http.requestFromCgi());
  http.sendResponse(response);
}
