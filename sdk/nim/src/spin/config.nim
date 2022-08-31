import private/utils
import config/private/wit/spin_config
import std/[options, strutils]

proc getConfig*(key: string): Option[string] =
  let
    key = newUnmanagedStr(key.toLowerAscii)
    srch = nalloc(1, spin_config_string_t)
    res = nalloc(1, spin_config_expected_string_error_t)
  defer:
    dealloc(key)
    spinConfigStringFree(srch)
    spinConfigExpectedStringErrorFree(res)
  spinConfigStringSet(srch, key)
  spinConfigGetConfig(srch, res)
  if not res.isErr:
    result = some($res.val.ok.ptr)
