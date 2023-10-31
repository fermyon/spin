package variables

// #cgo CFLAGS: -Wno-unused-parameter -Wno-switch-bool
// #include<spin-config.h>
// #include<stdlib.h>
import "C"
import (
	"errors"
	"unsafe"
)

func get(key string) (string, error) {
	var spinResponse C.spin_config_expected_string_error_t

	spinKey := C.spin_config_string_t{ptr: C.CString(key), len: C.size_t(len(key))}
	defer func() {
		C.spin_config_expected_string_error_free(&spinResponse)
		C.spin_config_string_free(&spinKey)
	}()

	C.spin_config_get_config(&spinKey, &spinResponse)

	if spinResponse.is_err { // error response from spin
		spinErr := (*C.spin_config_error_t)(unsafe.Pointer(&spinResponse.val))
		return "", toError(spinErr)
	}

	ok := (*spinString)(unsafe.Pointer(&spinResponse.val))
	return ok.String(), nil
}

func toError(err *C.spin_config_error_t) error {
	spinErr := (*spinString)(unsafe.Pointer(&err.val))
	return errors.New(spinErr.String())
}

type spinString C.spin_config_string_t

// String returns the spinString as a go string.
func (ss spinString) String() string {
	return C.GoStringN(ss.ptr, C.int(ss.len))
}
