import std/[options, tables]
from std/httpclient import HttpMethod

type
  HttpParams* = TableRef[string, string]
  HttpHeaders* = TableRef[string, string]
  Request* = object
    `method`*: HttpMethod
    uri*: string
    headers*: HttpHeaders
    params*: HttpParams
    body*: Option[string]
  Response* = object
    status*: uint16
    headers*: Option[HttpHeaders]
    body*: Option[string]