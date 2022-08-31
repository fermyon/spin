# Generated with nimterop and edited manually

from os import splitPath

{.push hint[ConvFromXtoItselfNotNeeded]: off.}

{.compile: currentSourcePath().splitPath.head & "/c/spin-config.c".}

{.pragma: impspinconfigHdr, header: currentSourcePath().splitPath.head & "/c/spin-config.h".}
{.experimental: "codeReordering".}
const
  SPIN_CONFIG_ERROR_PROVIDER* = 0
  SPIN_CONFIG_ERROR_INVALID_KEY* = 1
  SPIN_CONFIG_ERROR_INVALID_SCHEMA* = 2
  SPIN_CONFIG_ERROR_OTHER* = 3
type
  spin_config_string_t* {.bycopy, importc, impspinconfigHdr.} = object
    `ptr`*: cstring
    len*: uint

  Union_spinconfigh1* {.union, bycopy, impspinconfigHdr,
                        importc: "union Union_spinconfigh1".} = object
    provider*: spin_config_string_t
    invalid_key*: spin_config_string_t
    invalid_schema*: spin_config_string_t
    other*: spin_config_string_t

  spin_config_error_t* {.bycopy, importc, impspinconfigHdr.} = object
    tag*: uint8
    val*: Union_spinconfigh1

  Union_spinconfigh2* {.union, bycopy, impspinconfigHdr,
                        importc: "union Union_spinconfigh2".} = object
    ok*: spin_config_string_t
    err*: spin_config_error_t

  spin_config_expected_string_error_t* {.bycopy, importc, impspinconfigHdr.} = object
    is_err*: bool
    val*: Union_spinconfigh2

proc spin_config_string_set*(ret: ptr spin_config_string_t; s: cstring) {.
    importc, cdecl, impspinconfigHdr.}
proc spin_config_string_dup*(ret: ptr spin_config_string_t; s: cstring) {.
    importc, cdecl, impspinconfigHdr.}
proc spin_config_string_free*(ret: ptr spin_config_string_t) {.importc, cdecl,
    impspinconfigHdr.}
proc spin_config_error_free*(`ptr`: ptr spin_config_error_t) {.importc, cdecl,
    impspinconfigHdr.}
proc spin_config_expected_string_error_free*(
    `ptr`: ptr spin_config_expected_string_error_t) {.importc, cdecl,
    impspinconfigHdr.}
proc spin_config_get_config*(key: ptr spin_config_string_t;
                             ret0: ptr spin_config_expected_string_error_t) {.
    importc, cdecl, impspinconfigHdr.}
{.pop.}
