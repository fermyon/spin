package postgres

// #include "outbound-pg.h"
// #include <stdlib.h>
import "C"

import "unsafe"

import "fmt"

func lowerPgStr(x string) C.outbound_pg_string_t {
	return C.outbound_pg_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func lowerParameterValues(params []ParameterValue) C.outbound_pg_list_parameter_value_t {
	var lower_params C.outbound_pg_list_parameter_value_t
	if len(params) == 0 {
		lower_params.ptr = nil
		lower_params.len = 0
	} else {
		var empty_lower_params C.outbound_pg_parameter_value_t
		lower_params.ptr = (*C.outbound_pg_parameter_value_t)(C.malloc(C.size_t(len(params)) * C.size_t(unsafe.Sizeof(empty_lower_params))))
		lower_params.len = C.size_t(len(params))
		for lower_params_i := range params {
			lower_params_ptr := (*C.outbound_pg_parameter_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params.ptr)) +
				uintptr(lower_params_i)*unsafe.Sizeof(empty_lower_params)))

			var lower_params_ptr_value C.outbound_pg_parameter_value_t
			var lower_params_ptr_value_val C.outbound_pg_parameter_value_t
			if params[lower_params_i].Kind() == ParameterValueKindBoolean {

				lower_params_ptr_value_val.tag = 0
				lower_params_ptr_value_val_ptr := (*bool)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := params[lower_params_i].GetBoolean()
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindInt8 {

				lower_params_ptr_value_val.tag = 1
				lower_params_ptr_value_val_ptr := (*C.int8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int8_t(params[lower_params_i].GetInt8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindInt16 {

				lower_params_ptr_value_val.tag = 2
				lower_params_ptr_value_val_ptr := (*C.int16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int16_t(params[lower_params_i].GetInt16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindInt32 {

				lower_params_ptr_value_val.tag = 3
				lower_params_ptr_value_val_ptr := (*C.int32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int32_t(params[lower_params_i].GetInt32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindInt64 {

				lower_params_ptr_value_val.tag = 4
				lower_params_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int64_t(params[lower_params_i].GetInt64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindUint8 {

				lower_params_ptr_value_val.tag = 5
				lower_params_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint8_t(params[lower_params_i].GetUint8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindUint16 {

				lower_params_ptr_value_val.tag = 6
				lower_params_ptr_value_val_ptr := (*C.uint16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint16_t(params[lower_params_i].GetUint16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindUint32 {

				lower_params_ptr_value_val.tag = 7
				lower_params_ptr_value_val_ptr := (*C.uint32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint32_t(params[lower_params_i].GetUint32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindUint64 {

				lower_params_ptr_value_val.tag = 8
				lower_params_ptr_value_val_ptr := (*C.uint64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint64_t(params[lower_params_i].GetUint64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindFloating32 {

				lower_params_ptr_value_val.tag = 9
				lower_params_ptr_value_val_ptr := (*C.float)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.float(params[lower_params_i].GetFloating32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindFloating64 {

				lower_params_ptr_value_val.tag = 10
				lower_params_ptr_value_val_ptr := (*C.double)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.double(params[lower_params_i].GetFloating64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindStr {

				lower_params_ptr_value_val.tag = 11
				lower_params_ptr_value_val_ptr := (*C.outbound_pg_string_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.outbound_pg_string_t

				lower_params_ptr_value_val_val.ptr = C.CString(params[lower_params_i].GetStr())
				lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetStr()))
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindBinary {

				lower_params_ptr_value_val.tag = 12
				lower_params_ptr_value_val_ptr := (*C.outbound_pg_list_u8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.outbound_pg_list_u8_t
				if len(params[lower_params_i].GetBinary()) == 0 {
					lower_params_ptr_value_val_val.ptr = nil
					lower_params_ptr_value_val_val.len = 0
				} else {
					var empty_lower_params_ptr_value_val_val C.uint8_t
					lower_params_ptr_value_val_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(params[lower_params_i].GetBinary())) * C.size_t(unsafe.Sizeof(empty_lower_params_ptr_value_val_val))))
					lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetBinary()))
					for lower_params_ptr_value_val_val_i := range params[lower_params_i].GetBinary() {
						lower_params_ptr_value_val_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params_ptr_value_val_val.ptr)) +
							uintptr(lower_params_ptr_value_val_val_i)*unsafe.Sizeof(empty_lower_params_ptr_value_val_val)))
						lower_params_ptr_value_val_val_ptr_value := C.uint8_t(params[lower_params_i].GetBinary()[lower_params_ptr_value_val_val_i])
						*lower_params_ptr_value_val_val_ptr = lower_params_ptr_value_val_val_ptr_value
					}
				}
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == ParameterValueKindDbNull {
				lower_params_ptr_value_val.tag = 13
			}
			lower_params_ptr_value = lower_params_ptr_value_val
			*lower_params_ptr = lower_params_ptr_value
		}
	}
	return lower_params
}

func liftPgError(err *C.outbound_pg_pg_error_t) error {
	var gstr string
	switch int(err.tag) {
	case int(PgErrorKindSuccess):
		gstr = "success"
	case int(PgErrorKindConnectionFailed):
	case int(PgErrorKindBadParameter):
	case int(PgErrorKindQueryFailed):
	case int(PgErrorKindValueConversionFailed):
	case int(PgErrorKindOtherError):
		cstr := (*C.outbound_pg_string_t)(unsafe.Pointer(&err.val))
		gstr = C.GoStringN(cstr.ptr, C.int(cstr.len))
	default:
		gstr = fmt.Sprintf("unrecognized error: %v", err.tag)
	}
	return fmt.Errorf(gstr)
}

func liftRowSet(rowset *C.outbound_pg_row_set_t) RowSet {
	var lift_rowset RowSet
	lift_rowset_Columns := make([]Column, rowset.columns.len)
	if rowset.columns.len > 0 {
		for lift_rowset_Columns_i := 0; lift_rowset_Columns_i < int(rowset.columns.len); lift_rowset_Columns_i++ {
			var empty_lift_rowset_Columns C.outbound_pg_column_t
			lift_rowset_Columns_ptr := *(*C.outbound_pg_column_t)(unsafe.Pointer(uintptr(unsafe.Pointer(rowset.columns.ptr)) +
				uintptr(lift_rowset_Columns_i)*unsafe.Sizeof(empty_lift_rowset_Columns)))
			var list_lift_rowset_Columns Column
			list_lift_rowset_Columns_Name := C.GoStringN(lift_rowset_Columns_ptr.name.ptr, C.int(lift_rowset_Columns_ptr.name.len))
			list_lift_rowset_Columns.Name = list_lift_rowset_Columns_Name
			var list_lift_rowset_Columns_DataType DbDataType
			if lift_rowset_Columns_ptr.data_type == 0 {
				list_lift_rowset_Columns_DataType = DbDataTypeBoolean()
			}
			if lift_rowset_Columns_ptr.data_type == 1 {
				list_lift_rowset_Columns_DataType = DbDataTypeInt8()
			}
			if lift_rowset_Columns_ptr.data_type == 2 {
				list_lift_rowset_Columns_DataType = DbDataTypeInt16()
			}
			if lift_rowset_Columns_ptr.data_type == 3 {
				list_lift_rowset_Columns_DataType = DbDataTypeInt32()
			}
			if lift_rowset_Columns_ptr.data_type == 4 {
				list_lift_rowset_Columns_DataType = DbDataTypeInt64()
			}
			if lift_rowset_Columns_ptr.data_type == 5 {
				list_lift_rowset_Columns_DataType = DbDataTypeUint8()
			}
			if lift_rowset_Columns_ptr.data_type == 6 {
				list_lift_rowset_Columns_DataType = DbDataTypeUint16()
			}
			if lift_rowset_Columns_ptr.data_type == 7 {
				list_lift_rowset_Columns_DataType = DbDataTypeUint32()
			}
			if lift_rowset_Columns_ptr.data_type == 8 {
				list_lift_rowset_Columns_DataType = DbDataTypeUint64()
			}
			if lift_rowset_Columns_ptr.data_type == 9 {
				list_lift_rowset_Columns_DataType = DbDataTypeFloating32()
			}
			if lift_rowset_Columns_ptr.data_type == 10 {
				list_lift_rowset_Columns_DataType = DbDataTypeFloating64()
			}
			if lift_rowset_Columns_ptr.data_type == 11 {
				list_lift_rowset_Columns_DataType = DbDataTypeStr()
			}
			if lift_rowset_Columns_ptr.data_type == 12 {
				list_lift_rowset_Columns_DataType = DbDataTypeBinary()
			}
			if lift_rowset_Columns_ptr.data_type == 13 {
				list_lift_rowset_Columns_DataType = DbDataTypeOther()
			}
			list_lift_rowset_Columns.DataType = list_lift_rowset_Columns_DataType
			lift_rowset_Columns[lift_rowset_Columns_i] = list_lift_rowset_Columns
		}
	}
	lift_rowset.Columns = lift_rowset_Columns
	lift_rowset_Rows := make([][]DbValue, rowset.rows.len)
	if rowset.rows.len > 0 {
		for lift_rowset_Rows_i := 0; lift_rowset_Rows_i < int(rowset.rows.len); lift_rowset_Rows_i++ {
			var empty_lift_rowset_Rows C.outbound_pg_row_t
			lift_rowset_Rows_ptr := *(*C.outbound_pg_row_t)(unsafe.Pointer(uintptr(unsafe.Pointer(rowset.rows.ptr)) +
				uintptr(lift_rowset_Rows_i)*unsafe.Sizeof(empty_lift_rowset_Rows)))
			list_lift_rowset_Rows := make([]DbValue, lift_rowset_Rows_ptr.len)
			if lift_rowset_Rows_ptr.len > 0 {
				for list_lift_rowset_Rows_i := 0; list_lift_rowset_Rows_i < int(lift_rowset_Rows_ptr.len); list_lift_rowset_Rows_i++ {
					var empty_list_lift_rowset_Rows C.outbound_pg_db_value_t
					list_lift_rowset_Rows_ptr := *(*C.outbound_pg_db_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_rowset_Rows_ptr.ptr)) +
						uintptr(list_lift_rowset_Rows_i)*unsafe.Sizeof(empty_list_lift_rowset_Rows)))
					var list_list_lift_rowset_Rows DbValue
					if list_lift_rowset_Rows_ptr.tag == 0 {
						list_list_lift_rowset_Rows_ptr := *(*bool)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := list_list_lift_rowset_Rows_ptr
						list_list_lift_rowset_Rows = DbValueBoolean(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 1 {
						list_list_lift_rowset_Rows_ptr := *(*C.int8_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := int8(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueInt8(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 2 {
						list_list_lift_rowset_Rows_ptr := *(*C.int16_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := int16(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueInt16(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 3 {
						list_list_lift_rowset_Rows_ptr := *(*C.int32_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := int32(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueInt32(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 4 {
						list_list_lift_rowset_Rows_ptr := *(*C.int64_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := int64(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueInt64(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 5 {
						list_list_lift_rowset_Rows_ptr := *(*C.uint8_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := uint8(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueUint8(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 6 {
						list_list_lift_rowset_Rows_ptr := *(*C.uint16_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := uint16(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueUint16(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 7 {
						list_list_lift_rowset_Rows_ptr := *(*C.uint32_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := uint32(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueUint32(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 8 {
						list_list_lift_rowset_Rows_ptr := *(*C.uint64_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := uint64(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueUint64(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 9 {
						list_list_lift_rowset_Rows_ptr := *(*C.float)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := float32(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueFloating32(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 10 {
						list_list_lift_rowset_Rows_ptr := *(*C.double)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := float64(list_list_lift_rowset_Rows_ptr)
						list_list_lift_rowset_Rows = DbValueFloating64(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 11 {
						list_list_lift_rowset_Rows_ptr := *(*C.outbound_pg_string_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := C.GoStringN(list_list_lift_rowset_Rows_ptr.ptr, C.int(list_list_lift_rowset_Rows_ptr.len))
						list_list_lift_rowset_Rows = DbValueStr(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 12 {
						list_list_lift_rowset_Rows_ptr := *(*C.outbound_pg_list_u8_t)(unsafe.Pointer(&list_lift_rowset_Rows_ptr.val))
						list_list_lift_rowset_Rows_val := make([]uint8, list_list_lift_rowset_Rows_ptr.len)
						if list_list_lift_rowset_Rows_ptr.len > 0 {
							for list_list_lift_rowset_Rows_val_i := 0; list_list_lift_rowset_Rows_val_i < int(list_list_lift_rowset_Rows_ptr.len); list_list_lift_rowset_Rows_val_i++ {
								var empty_list_list_lift_rowset_Rows_val C.uint8_t
								list_list_lift_rowset_Rows_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(list_list_lift_rowset_Rows_ptr.ptr)) +
									uintptr(list_list_lift_rowset_Rows_val_i)*unsafe.Sizeof(empty_list_list_lift_rowset_Rows_val)))
								list_list_list_lift_rowset_Rows_val := uint8(list_list_lift_rowset_Rows_val_ptr)
								list_list_lift_rowset_Rows_val[list_list_lift_rowset_Rows_val_i] = list_list_list_lift_rowset_Rows_val
							}
						}
						list_list_lift_rowset_Rows = DbValueBinary(list_list_lift_rowset_Rows_val)
					}
					if list_lift_rowset_Rows_ptr.tag == 13 {
						list_list_lift_rowset_Rows = DbValueDbNull()
					}
					if list_lift_rowset_Rows_ptr.tag == 14 {
						list_list_lift_rowset_Rows = DbValueUnsupported()
					}
					list_lift_rowset_Rows[list_lift_rowset_Rows_i] = list_list_lift_rowset_Rows
				}
			}
			lift_rowset_Rows[lift_rowset_Rows_i] = list_lift_rowset_Rows
		}
	}
	lift_rowset.Rows = lift_rowset_Rows
	return lift_rowset
}

func Query(address string, statement string, params []ParameterValue) (RowSet, error) {
	lower_address := lowerPgStr(address)
	defer C.outbound_pg_string_free(&lower_address)

	lower_statement := lowerPgStr(statement)
	defer C.outbound_pg_string_free(&lower_statement)

	lower_params := lowerParameterValues(params)
	defer C.outbound_pg_list_parameter_value_free(&lower_params)

	var result C.outbound_pg_expected_row_set_pg_error_t
	C.outbound_pg_query(&lower_address, &lower_statement, &lower_params, &result)

	if result.is_err {
		err := liftPgError((*C.outbound_pg_pg_error_t)(unsafe.Pointer(&result.val)))
		return RowSet{}, err
	} else {
		rowset := liftRowSet((*C.outbound_pg_row_set_t)(unsafe.Pointer(&result.val)))
		return rowset, nil
	}
}

func Execute(address string, statement string, params []ParameterValue) (uint64, error) {
	lower_address := lowerPgStr(address)
	defer C.outbound_pg_string_free(&lower_address)

	lower_statement := lowerPgStr(statement)
	defer C.outbound_pg_string_free(&lower_statement)

	lower_params := lowerParameterValues(params)
	defer C.outbound_pg_list_parameter_value_free(&lower_params)

	var result C.outbound_pg_expected_u64_pg_error_t
	C.outbound_pg_execute(&lower_address, &lower_statement, &lower_params, &result)

	if result.is_err {
		err := liftPgError((*C.outbound_pg_pg_error_t)(unsafe.Pointer(&result.val)))
		return 0, err
	} else {
		//TODO: ask how to convert the value to uint64
		// return uint64(result.val), nil
		return 0, nil
	}
}
