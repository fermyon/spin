package postgres

// #include "outbound-pg.h"
// #include<stdlib.h>
import "C"
import (
	"errors"
	"unsafe"
)

type ParameterValueKind C.uint8_t
const (
	ParameterValueKindBool = iota
	ParameterValueKindInt8
	ParameterValueKindInt16
	ParameterValueKindInt32
	ParameterValueKindInt64
	ParameterValueKindUint8
	ParameterValueKindUint16
	ParameterValueKindUint32
	ParameterValueKindUint64
	ParameterValueKindFloat32
	ParameterValueKindFloat64
	ParameterValueKindString
	ParameterValueKindBinary
)

/*
typedef struct {
uint8_t tag;
union {
	bool boolean;
	int8_t int8;
	int16_t int16;
	int32_t int32;
	int64_t int64;
	uint8_t uint8;
	uint16_t uint16;
	uint32_t uint32;
	uint64_t uint64;
	float floating32;
	double floating64;
	outbound_pg_string_t str;
	outbound_pg_list_u8_t binary;
} val;
} outbound_pg_parameter_value_t;
*/
type ParameterValue {
	Kind ParameterValueKind
	Val interface{}
}

// TODO: templatize?
func pgStr(x string) C.outbound_pg_string_t {
	return C.outbound_pg_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func pgParameter(x ParameterValue) C.outbound_pg_parameter_value_t {
	var val C._Ctype_union___9
	switch x.Kind {
	//ParameterValueKindInt64
	// case ParameterValueKindInt64: *(*C.int64_t)(unsafe.Pointer(&val)) = x.Val.(int64)
	case ParameterValueKindBool: *(*C.bool)(unsafe.Pointer(&val)) = x.Val.(bool)
	case ParameterValueKindInt8: *(*C.int8_t)(unsafe.Pointer(&val)) = x.Val.(int8)
	case ParameterValueKindInt16: *(*C.int16_t)(unsafe.Pointer(&val)) = x.Val.(int16)
	case ParameterValueKindInt32: *(*C.int32_t)(unsafe.Pointer(&val)) = x.Val.(int32)
	case ParameterValueKindInt64: *(*C.int64_t)(unsafe.Pointer(&val)) = x.Val.(int64)
	case ParameterValueKindUint8: *(*C.uint8_t)(unsafe.Pointer(&val)) = x.Val.(uint8)
	case ParameterValueKindUint16: *(*C.uint16_t)(unsafe.Pointer(&val)) = x.Val.(uint16)
	case ParameterValueKindUint32: *(*C.uint32_t)(unsafe.Pointer(&val)) = x.Val.(uint32)
	case ParameterValueKindUint64: *(*C.uint64_t)(unsafe.Pointer(&val)) = x.Val.(uint64)
	case ParameterValueKindFloat32: *(*C.float32_t)(unsafe.Pointer(&val)) = x.Val.(float32)
	case ParameterValueKindFloat64: *(*C.float64_t)(unsafe.Pointer(&val)) = x.Val.(float64)
	case ParameterValueKindString: *(*C.outbound_pg_string_t)(unsafe.Pointer(&val)) = pgStr(x)
	case ParameterValueKindBinary: {
		value := x.Val.([]uint8)
		list_u8 := C.outbound_pg_list_u8_t{ptr: &value[0], len: C.size_t(len(value))}
		*(*C.outbound_pg_list_u8_t)(unsafe.Pointer(&val)) = list_u8
	}
	}
	return C.outbound_pg_parameter_value_t{tag: C.uint8_t(x.Kind), val: val}
}

func pgListParameter(p []ParameterValue) C.outbound_pg_list_parameter_value_t {
	var cp []C.outbound_pg_parameter_value_t
	for i := 0; i < len(p); i++ {
		cp = append(cp, pgParameter(p[i]))
	}
	return C.outbound_pg_list_parameter_value_t{ptr: &cp[0], len: C.size_t(len(cp))}
}

/*
void outbound_pg_execute(

	outbound_pg_string_t *address,
	outbound_pg_string_t *statement,
	outbound_pg_list_parameter_value_t *params,
	outbound_pg_expected_u64_pg_error_t *ret0);
*/
func execute(addr string, statement string, params []ParameterValue) (RowSet, error) {
	caddr := pgStr(addr)
	cstatement := pgStr(statement)
	cparameters := pgListParameter(params)

	var cpayload C.outbound_pg_list_pg_result_t
	err := C.outbound_postgres_execute(&caddr, &cstatement, &cparams, &cpayload)
	return fromPgListResult(&cpayload), toErr(err)
}
