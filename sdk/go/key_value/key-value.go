// Package key_value provides access to key value stores within Spin
// components.
package key_value

// #include "key-value.h"
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

type Store C.key_value_store_t

const (
	errorKindStoreTableFull = iota
	errorKindNoSuchStore
	errorKindAccessDenied
	errorKindInvalidStore
	errorKindNoSuchKey
	errorKindIo
)

func Open(name string) (Store, error) {
	cname := toCStr(name)
	var ret C.key_value_expected_store_error_t
	C.key_value_open(&cname, &ret)
	if ret.is_err {
		return 0xFFFF_FFFF, toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	return *(*Store)(unsafe.Pointer(&ret.val)), nil
}

func Get(store Store, key string) ([]byte, error) {
	ckey := toCStr(key)
	var ret C.key_value_expected_list_u8_error_t
	C.key_value_get(C.uint32_t(store), &ckey, &ret)
	if ret.is_err {
		return []byte{}, toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	list := (*C.key_value_list_u8_t)(unsafe.Pointer(&ret.val))
	return C.GoBytes(unsafe.Pointer(list.ptr), C.int(list.len)), nil
}

func Set(store Store, key string, value []byte) error {
	ckey := toCStr(key)
	cbytes := toCBytes(value)
	var ret C.key_value_expected_unit_error_t
	C.key_value_set(C.uint32_t(store), &ckey, &cbytes, &ret)
	if ret.is_err {
		return toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	return nil
}

func Delete(store Store, key string) error {
	ckey := toCStr(key)
	var ret C.key_value_expected_unit_error_t
	C.key_value_delete(C.uint32_t(store), &ckey, &ret)
	if ret.is_err {
		return toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	return nil
}

func Exists(store Store, key string) (bool, error) {
	ckey := toCStr(key)
	var ret C.key_value_expected_bool_error_t
	C.key_value_exists(C.uint32_t(store), &ckey, &ret)
	if ret.is_err {
		return false, toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	return *(*bool)(unsafe.Pointer(&ret.val)), nil
}

func GetKeys(store Store) ([]string, error) {
	var ret C.key_value_expected_list_string_error_t
	C.key_value_get_keys(C.uint32_t(store), &ret)
	if ret.is_err {
		return []string{}, toErr((*C.key_value_error_t)(unsafe.Pointer(&ret.val)))
	}
	return fromCStrList((*C.key_value_list_string_t)(unsafe.Pointer(&ret.val))), nil
}

func Close(store Store) {
	C.key_value_close(C.uint32_t(store))
}

func toCBytes(x []byte) C.key_value_list_u8_t {
	return C.key_value_list_u8_t{ptr: (*C.uint8_t)(unsafe.Pointer(&x[0])), len: C.size_t(len(x))}
}

func toCStr(x string) C.key_value_string_t {
	return C.key_value_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func fromCStrList(list *C.key_value_list_string_t) []string {
	listLen := int(list.len)
	var result []string

	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		str := slice[i]
		result = append(result, C.GoStringN(str.ptr, C.int(str.len)))
	}

	return result
}

func toErr(error *C.key_value_error_t) error {
	switch error.tag {
	case errorKindStoreTableFull:
		return errors.New("store table full")
	case errorKindNoSuchStore:
		return errors.New("no such store")
	case errorKindAccessDenied:
		return errors.New("access denied")
	case errorKindInvalidStore:
		return errors.New("invalid store")
	case errorKindNoSuchKey:
		return errors.New("no such key")
	case errorKindIo:
		str := (*C.key_value_string_t)(unsafe.Pointer(&error.val))
		return errors.New(fmt.Sprintf("io error: %s", C.GoStringN(str.ptr, C.int(str.len))))
	default:
		return errors.New(fmt.Sprintf("unrecognized error: %v", error.tag))
	}
}
