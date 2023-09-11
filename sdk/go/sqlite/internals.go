package sqlite

// #include "sqlite.h"
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

func open(name string) (*conn, error) {
	var dbname C.sqlite_string_t
	var ret C.sqlite_expected_connection_error_t

	dbname = sqliteStr(name)
	C.sqlite_open(&dbname, &ret)

	if ret.is_err {
		return nil, toErr((*C.sqlite_error_t)(unsafe.Pointer(&ret.val)))
	}

	sqliteConn := *((*C.sqlite_connection_t)(unsafe.Pointer(&ret.val)))
	return &conn{_ptr: sqliteConn}, nil
}

func (db *conn) close() {
	C.sqlite_close(db._ptr)
}

func (db *conn) execute(statement string, args []any) (*rows, error) {
	var ret C.sqlite_expected_query_result_error_t
	defer C.sqlite_expected_query_result_error_free(&ret)

	sqliteStatement := sqliteStr(statement)
	params := toSqliteListValue(args)

	C.sqlite_execute(db._ptr, &sqliteStatement, &params, &ret)

	if ret.is_err {
		spinErr := (*C.sqlite_error_t)(unsafe.Pointer(&ret.val))
		return nil, toErr(spinErr)
	}

	qr := (*C.sqlite_query_result_t)(unsafe.Pointer(&ret.val))

	result := &rows{
		columns: fromSqliteListString(qr.columns),
		rows:    fromSqliteListRowResult(qr.rows),
		len:     int(qr.rows.len),
	}

	return result, nil
}

func fromSqliteListRowResult(list C.sqlite_list_row_result_t) [][]any {
	listLen := int(list.len)
	ret := make([][]any, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		row := *((*C.sqlite_list_value_t)(unsafe.Pointer(&slice[i])))
		ret[i] = fromSqliteListValue(row)
	}
	return ret

}

func fromSqliteListString(list C.sqlite_list_string_t) []string {
	listLen := int(list.len)
	ret := make([]string, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		str := slice[i]
		ret[i] = C.GoStringN(str.ptr, C.int(str.len))
	}
	return ret
}

func fromSqliteListValue(list C.sqlite_list_value_t) []any {
	listLen := int(list.len)
	ret := make([]any, listLen)
	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		ret[i] = fromSqliteValue(slice[i])
	}
	return ret
}

func toSqliteListValue(xv []any) C.sqlite_list_value_t {
	if len(xv) == 0 {
		return C.sqlite_list_value_t{}
	}
	cxv := make([]C.sqlite_value_t, len(xv))
	for i := 0; i < len(xv); i++ {
		cxv[i] = toSqliteValue(xv[i])
	}
	return C.sqlite_list_value_t{ptr: &cxv[0], len: C.size_t(len(cxv))}
}

const (
	valueInt uint8 = iota
	valueReal
	valueText
	valueBlob
	valueNull
)

func toSqliteValue(x any) C.sqlite_value_t {
	var ret C.sqlite_value_t
	switch v := x.(type) {
	case int:
		*(*C.int64_t)(unsafe.Pointer(&ret.val)) = int64(v)
		ret.tag = valueInt
	case int64:
		*(*C.int64_t)(unsafe.Pointer(&ret.val)) = v
		ret.tag = valueInt
	case float64:
		*(*C.double)(unsafe.Pointer(&ret.val)) = v
		ret.tag = valueReal
	case string:
		str := sqliteStr(v)
		*(*C.sqlite_string_t)(unsafe.Pointer(&ret.val)) = str
		ret.tag = valueText
	case []byte:
		blob := C.sqlite_list_u8_t{ptr: &v[0], len: C.size_t(len(v))}
		*(*C.sqlite_list_u8_t)(unsafe.Pointer(&ret.val)) = blob
		ret.tag = valueBlob
	default:
		ret.tag = valueNull
	}
	return ret
}

func fromSqliteValue(x C.sqlite_value_t) any {
	switch x.tag {
	case valueInt:
		return int64(*(*C.int64_t)(unsafe.Pointer(&x.val)))
	case valueReal:
		return float64(*(*C.double)(unsafe.Pointer(&x.val)))
	case valueBlob:
		blob := (*C.sqlite_list_u8_t)(unsafe.Pointer(&x.val))
		return C.GoBytes(unsafe.Pointer(blob.ptr), C.int(blob.len))
	case valueText:
		str := (*C.sqlite_string_t)(unsafe.Pointer(&x.val))
		return C.GoStringN(str.ptr, C.int(str.len))
	}
	return nil
}

func sqliteStr(x string) C.sqlite_string_t {
	return C.sqlite_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func toErr(err *C.sqlite_error_t) error {
	switch err.tag {
	case 0:
		return errors.New("no such database")
	case 1:
		return errors.New("access denied")
	case 2:
		return errors.New("invalid connection")
	case 3:
		return errors.New("database full")
	case 4:
		str := (*C.sqlite_string_t)(unsafe.Pointer(&err.val))
		return errors.New(fmt.Sprintf("io error: %s", C.GoStringN(str.ptr, C.int(str.len))))
	default:
		return errors.New(fmt.Sprintf("unrecognized error: %v", err.tag))
	}
}
