template nalloc*(n: int, t: typedesc): untyped =
  cast[ptr t](alloc(n * sizeof(t)))

proc newUnmanagedStr*(str: string | cstring): cstring =
  let data = alloc(str.len + 1)
  copyMem(data, unsafeAddr str[0], str.len)
  cast[ptr UncheckedArray[byte]](data)[str.len] = 0'u8
  result = cast[cstring](data)
