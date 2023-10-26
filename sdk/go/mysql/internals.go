package mysql

// #include "outbound-mysql.h"
// #include<stdlib.h>
import "C"
import (
	"errors"
	"fmt"
	"reflect"
	"unsafe"
)

func execute(address string, statement string, args []any) error {
	var ret C.outbound_mysql_expected_unit_mysql_error_t
	defer C.outbound_mysql_expected_unit_mysql_error_free(&ret)

	mysqlAddress := outboundMysqlStr(address)
	mysqlStatement := outboundMysqlStr(statement)
	params := toOutboundMysqlParameterListValue(args)

	C.outbound_mysql_execute(&mysqlAddress, &mysqlStatement, &params, &ret)

	if ret.is_err {
		spinErr := (*C.outbound_mysql_mysql_error_t)(unsafe.Pointer(&ret.val))
		return toErr(spinErr)
	}

	return nil
}

func query(address string, statement string, args []any) (*rows, error) {
	var ret C.outbound_mysql_expected_row_set_mysql_error_t
	defer C.outbound_mysql_expected_row_set_mysql_error_free(&ret)

	mysqlAddress := outboundMysqlStr(address)
	mysqlStatement := outboundMysqlStr(statement)
	params := toOutboundMysqlParameterListValue(args)

	C.outbound_mysql_query(&mysqlAddress, &mysqlStatement, &params, &ret)

	if ret.is_err {
		spinErr := (*C.outbound_mysql_mysql_error_t)(unsafe.Pointer(&ret.val))
		return nil, toErr(spinErr)
	}

	qr := (*C.outbound_mysql_row_set_t)(unsafe.Pointer(&ret.val))

	columns, columnType := fromOutboundMysqlListColoum(qr.columns)

	rs, err := fromOutboundMysqlListRow(qr.rows)
	if err != nil {
		return nil, err
	}

	result := &rows{
		columns:    columns,
		columnType: columnType,
		rows:       rs,
		len:        int(qr.rows.len),
	}

	return result, nil
}

func fromOutboundMysqlListRow(list C.outbound_mysql_list_row_t) ([][]any, error) {
	var err error
	listLen := int(list.len)
	ret := make([][]any, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		row := *((*C.outbound_mysql_row_t)(unsafe.Pointer(&slice[i])))
		ret[i], err = fromOutboundMysqlRow(row)
		if err != nil {
			return nil, err
		}
	}
	return ret, nil

}

func fromOutboundMysqlRow(row C.outbound_mysql_row_t) ([]any, error) {
	var err error
	rowLen := int(row.len)
	ret := make([]any, rowLen)
	slice := unsafe.Slice(row.ptr, rowLen)
	for i := 0; i < rowLen; i++ {
		value := *((*C.outbound_mysql_db_value_t)(unsafe.Pointer(&slice[i])))
		ret[i], err = fromOutboundMysqlDbValue(value)
		if err != nil {
			return nil, err
		}
	}
	return ret, err
}

func fromOutboundMysqlListColoum(list C.outbound_mysql_list_column_t) ([]string, []uint8) {
	coloumLen := int(list.len)
	ret := make([]string, coloumLen)
	retType := make([]uint8, coloumLen)
	slice := unsafe.Slice(list.ptr, coloumLen)
	for i := 0; i < coloumLen; i++ {
		column := *((*C.outbound_mysql_column_t)(unsafe.Pointer(&slice[i])))
		ret[i], retType[i] = fromOutboundMysqlDbColumn(column)
	}
	return ret, retType
}

func fromOutboundMysqlDbColumn(c C.outbound_mysql_column_t) (string, uint8) {
	return C.GoStringN(c.name.ptr, C.int(c.name.len)), uint8(*(*C.uint8_t)(unsafe.Pointer(&c.data_type)))
}

func outboundMysqlStr(x string) C.outbound_mysql_string_t {
	return C.outbound_mysql_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func toOutboundMysqlParameterListValue(xv []any) C.outbound_mysql_list_parameter_value_t {
	if len(xv) == 0 {
		return C.outbound_mysql_list_parameter_value_t{}
	}
	cxv := make([]C.outbound_mysql_parameter_value_t, len(xv))
	for i := 0; i < len(xv); i++ {
		cxv[i] = toOutboundMysqlParameterValue(xv[i])
	}
	return C.outbound_mysql_list_parameter_value_t{ptr: &cxv[0], len: C.size_t(len(cxv))}
}

const (
	dbValueBoolean uint8 = iota
	dbValueInt8
	dbValueInt16
	dbValueInt32
	dbValueInt64
	dbValueUint8
	dbValueUint16
	dbValueUint32
	dbValueUint64
	dbValueFloat32
	dbValueFloat64
	dbValueStr
	dbValueBinary
	dbValueNull
	dbValueUnsupported
)

func fromOutboundMysqlDbValue(x C.outbound_mysql_db_value_t) (any, error) {
	switch x.tag {
	case dbValueBoolean:
		return *(*bool)(unsafe.Pointer(&x.val)), nil
	case dbValueInt8:
		return int8(*(*C.int8_t)(unsafe.Pointer(&x.val))), nil
	case dbValueInt16:
		return int16(*(*C.int16_t)(unsafe.Pointer(&x.val))), nil
	case dbValueInt32:
		return int32(*(*C.int32_t)(unsafe.Pointer(&x.val))), nil
	case dbValueInt64:
		return int64(*(*C.int64_t)(unsafe.Pointer(&x.val))), nil
	case dbValueUint8:
		return uint8(*(*C.uint8_t)(unsafe.Pointer(&x.val))), nil
	case dbValueUint16:
		return uint16(*(*C.uint16_t)(unsafe.Pointer(&x.val))), nil
	case dbValueUint32:
		return uint32(*(*C.uint32_t)(unsafe.Pointer(&x.val))), nil
	case dbValueUint64:
		return uint64(*(*C.uint64_t)(unsafe.Pointer(&x.val))), nil
	case dbValueFloat32:
		return float32(*(*C.float)(unsafe.Pointer(&x.val))), nil
	case dbValueFloat64:
		return float64(*(*C.double)(unsafe.Pointer(&x.val))), nil
	case dbValueBinary:
		blob := (*C.outbound_mysql_list_u8_t)(unsafe.Pointer(&x.val))
		return C.GoBytes(unsafe.Pointer(blob.ptr), C.int(blob.len)), nil
	case dbValueStr:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&x.val))
		return C.GoStringN(str.ptr, C.int(str.len)), nil
	case dbValueNull:
		return nil, nil
	case dbValueUnsupported:
		return nil, errors.New("db return value type unsupported")
	}
	return nil, errors.New("db return value unknown type")
}

const (
	paramValueBoolean uint8 = iota
	paramValueInt8
	paramValueInt16
	paramValueInt32
	paramValueInt64
	paramValueUint8
	paramValueUint16
	paramValueUint32
	paramValueUint64
	paramValueFloat32
	paramValueFloat64
	paramValueStr
	paramValueBinary
	paramValueNull
	paramValueUnspported
)

func toOutboundMysqlParameterValue(x any) C.outbound_mysql_parameter_value_t {
	var ret C.outbound_mysql_parameter_value_t
	switch v := x.(type) {
	case bool:
		*(*bool)(unsafe.Pointer(&ret.val)) = bool(v)
		ret.tag = paramValueBoolean
	case int8:
		*(*C.int8_t)(unsafe.Pointer(&ret.val)) = int8(v)
		ret.tag = paramValueInt8
	case int16:
		*(*C.int16_t)(unsafe.Pointer(&ret.val)) = int16(v)
		ret.tag = paramValueInt16
	case int32:
		*(*C.int32_t)(unsafe.Pointer(&ret.val)) = int32(v)
		ret.tag = paramValueInt32
	case int64:
		*(*C.int64_t)(unsafe.Pointer(&ret.val)) = int64(v)
		ret.tag = paramValueInt64
	case int:
		*(*C.int64_t)(unsafe.Pointer(&ret.val)) = int64(v)
		ret.tag = paramValueInt64
	case uint8:
		*(*C.uint8_t)(unsafe.Pointer(&ret.val)) = uint8(v)
		ret.tag = paramValueUint8
	case uint16:
		*(*C.uint16_t)(unsafe.Pointer(&ret.val)) = uint16(v)
		ret.tag = paramValueUint16
	case uint32:
		*(*C.uint32_t)(unsafe.Pointer(&ret.val)) = uint32(v)
		ret.tag = paramValueUint32
	case uint64:
		*(*C.uint64_t)(unsafe.Pointer(&ret.val)) = uint64(v)
		ret.tag = paramValueUint64
	case float32:
		*(*C.float)(unsafe.Pointer(&ret.val)) = float32(v)
		ret.tag = paramValueFloat32
	case float64:
		*(*C.double)(unsafe.Pointer(&ret.val)) = float64(v)
		ret.tag = paramValueFloat64
	case string:
		str := outboundMysqlStr(v)
		*(*C.outbound_mysql_string_t)(unsafe.Pointer(&ret.val)) = str
		ret.tag = paramValueStr
	case []byte:
		blob := C.outbound_mysql_list_u8_t{ptr: &v[0], len: C.size_t(len(v))}
		*(*C.outbound_mysql_list_u8_t)(unsafe.Pointer(&ret.val)) = blob
		ret.tag = paramValueBinary
	case nil:
		ret.tag = paramValueNull
	default:
		ret.tag = paramValueUnspported
	}
	return ret
}

func toErr(err *C.outbound_mysql_mysql_error_t) error {
	switch err.tag {
	case 0:
		return nil
	case 1:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf("connection failed: %s", C.GoStringN(str.ptr, C.int(str.len)))
	case 2:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf("bad parameter: %s", C.GoStringN(str.ptr, C.int(str.len)))
	case 3:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf("query failed: %s", C.GoStringN(str.ptr, C.int(str.len)))
	case 4:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf(fmt.Sprintf("value conversion failed: %s", C.GoStringN(str.ptr, C.int(str.len))))
	case 5:
		str := (*C.outbound_mysql_string_t)(unsafe.Pointer(&err.val))
		return fmt.Errorf(fmt.Sprintf("other error: %s", C.GoStringN(str.ptr, C.int(str.len))))
	default:
		return fmt.Errorf("unrecognized error: %v", err.tag)
	}
}

const (
	dbDataTypeBoolean uint8 = iota
	dbDataTypeInt8
	dbDataTypeInt16
	dbDataTypeInt32
	dbDataTypeInt64
	dbDataTypeUint8
	dbDataTypeUint16
	dbDataTypeUint32
	dbDataTypeUint64
	dbDataTypeFloating32
	dbDataTypeFloating64
	dbDataTypeStr
	dbDataTypeBinary
	dbDataTypeOther
)

func colTypeToReflectType(typ uint8) reflect.Type {
	switch typ {
	case dbDataTypeBoolean:
		return reflect.TypeOf(false)
	case dbDataTypeInt8:
		return reflect.TypeOf(int8(0))
	case dbDataTypeInt16:
		return reflect.TypeOf(int16(0))
	case dbDataTypeInt32:
		return reflect.TypeOf(int32(0))
	case dbDataTypeInt64:
		return reflect.TypeOf(int64(0))
	case dbDataTypeUint8:
		return reflect.TypeOf(uint8(0))
	case dbDataTypeUint16:
		return reflect.TypeOf(uint16(0))
	case dbDataTypeUint32:
		return reflect.TypeOf(uint32(0))
	case dbDataTypeUint64:
		return reflect.TypeOf(uint64(0))
	case dbDataTypeStr:
		return reflect.TypeOf("")
	case dbDataTypeBinary:
		return reflect.TypeOf(new([]byte))
	case dbDataTypeOther:
		return reflect.TypeOf(new(any)).Elem()
	}
	panic("invalid db column type of " + string(typ))
}
