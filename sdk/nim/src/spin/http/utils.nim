import private/http_types
import std/options

proc `?`*(x: HttpHeaders | HttpParams, key: string): Option[string] =
  if x.hasKey(key):
    some(x["key"])
  else:
    none(string)
