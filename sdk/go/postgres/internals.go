package postgres

// #include "outbound-pg.h"
import "C"

import "fmt"

// pg-types
type PgErrorKind int

const (
PgErrorKindSuccess PgErrorKind = iota
PgErrorKindConnectionFailed
PgErrorKindBadParameter
PgErrorKindQueryFailed
PgErrorKindValueConversionFailed
PgErrorKindOtherError
)

type PgError struct {
  kind PgErrorKind
  val any
}

func (n PgError) Kind() PgErrorKind {
  return n.kind
}

func PgErrorSuccess() PgError{
  return PgError{kind: PgErrorKindSuccess}
}

func PgErrorConnectionFailed(v string) PgError{
  return PgError{kind: PgErrorKindConnectionFailed, val: v}
}

func (n PgError) GetConnectionFailed() string{
  if g, w := n.Kind(), PgErrorKindConnectionFailed; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *PgError) SetConnectionFailed(v string) {
  n.val = v
  n.kind = PgErrorKindConnectionFailed
}

func PgErrorBadParameter(v string) PgError{
  return PgError{kind: PgErrorKindBadParameter, val: v}
}

func (n PgError) GetBadParameter() string{
  if g, w := n.Kind(), PgErrorKindBadParameter; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *PgError) SetBadParameter(v string) {
  n.val = v
  n.kind = PgErrorKindBadParameter
}

func PgErrorQueryFailed(v string) PgError{
  return PgError{kind: PgErrorKindQueryFailed, val: v}
}

func (n PgError) GetQueryFailed() string{
  if g, w := n.Kind(), PgErrorKindQueryFailed; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *PgError) SetQueryFailed(v string) {
  n.val = v
  n.kind = PgErrorKindQueryFailed
}

func PgErrorValueConversionFailed(v string) PgError{
  return PgError{kind: PgErrorKindValueConversionFailed, val: v}
}

func (n PgError) GetValueConversionFailed() string{
  if g, w := n.Kind(), PgErrorKindValueConversionFailed; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *PgError) SetValueConversionFailed(v string) {
  n.val = v
  n.kind = PgErrorKindValueConversionFailed
}

func PgErrorOtherError(v string) PgError{
  return PgError{kind: PgErrorKindOtherError, val: v}
}

func (n PgError) GetOtherError() string{
  if g, w := n.Kind(), PgErrorKindOtherError; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *PgError) SetOtherError(v string) {
  n.val = v
  n.kind = PgErrorKindOtherError
}

// rdbms-types
type ParameterValueKind int

const (
ParameterValueKindBoolean ParameterValueKind = iota
ParameterValueKindInt8
ParameterValueKindInt16
ParameterValueKindInt32
ParameterValueKindInt64
ParameterValueKindUint8
ParameterValueKindUint16
ParameterValueKindUint32
ParameterValueKindUint64
ParameterValueKindFloating32
ParameterValueKindFloating64
ParameterValueKindStr
ParameterValueKindBinary
ParameterValueKindDbNull
)

type ParameterValue struct {
  kind ParameterValueKind
  val any
}

func (n ParameterValue) Kind() ParameterValueKind {
  return n.kind
}

func ParameterValueBoolean(v bool) ParameterValue{
  return ParameterValue{kind: ParameterValueKindBoolean, val: v}
}

func (n ParameterValue) GetBoolean() bool{
  if g, w := n.Kind(), ParameterValueKindBoolean; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(bool)
}

func (n *ParameterValue) SetBoolean(v bool) {
  n.val = v
  n.kind = ParameterValueKindBoolean
}

func ParameterValueInt8(v int8) ParameterValue{
  return ParameterValue{kind: ParameterValueKindInt8, val: v}
}

func (n ParameterValue) GetInt8() int8{
  if g, w := n.Kind(), ParameterValueKindInt8; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int8)
}

func (n *ParameterValue) SetInt8(v int8) {
  n.val = v
  n.kind = ParameterValueKindInt8
}

func ParameterValueInt16(v int16) ParameterValue{
  return ParameterValue{kind: ParameterValueKindInt16, val: v}
}

func (n ParameterValue) GetInt16() int16{
  if g, w := n.Kind(), ParameterValueKindInt16; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int16)
}

func (n *ParameterValue) SetInt16(v int16) {
  n.val = v
  n.kind = ParameterValueKindInt16
}

func ParameterValueInt32(v int32) ParameterValue{
  return ParameterValue{kind: ParameterValueKindInt32, val: v}
}

func (n ParameterValue) GetInt32() int32{
  if g, w := n.Kind(), ParameterValueKindInt32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int32)
}

func (n *ParameterValue) SetInt32(v int32) {
  n.val = v
  n.kind = ParameterValueKindInt32
}

func ParameterValueInt64(v int64) ParameterValue{
  return ParameterValue{kind: ParameterValueKindInt64, val: v}
}

func (n ParameterValue) GetInt64() int64{
  if g, w := n.Kind(), ParameterValueKindInt64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int64)
}

func (n *ParameterValue) SetInt64(v int64) {
  n.val = v
  n.kind = ParameterValueKindInt64
}

func ParameterValueUint8(v uint8) ParameterValue{
  return ParameterValue{kind: ParameterValueKindUint8, val: v}
}

func (n ParameterValue) GetUint8() uint8{
  if g, w := n.Kind(), ParameterValueKindUint8; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint8)
}

func (n *ParameterValue) SetUint8(v uint8) {
  n.val = v
  n.kind = ParameterValueKindUint8
}

func ParameterValueUint16(v uint16) ParameterValue{
  return ParameterValue{kind: ParameterValueKindUint16, val: v}
}

func (n ParameterValue) GetUint16() uint16{
  if g, w := n.Kind(), ParameterValueKindUint16; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint16)
}

func (n *ParameterValue) SetUint16(v uint16) {
  n.val = v
  n.kind = ParameterValueKindUint16
}

func ParameterValueUint32(v uint32) ParameterValue{
  return ParameterValue{kind: ParameterValueKindUint32, val: v}
}

func (n ParameterValue) GetUint32() uint32{
  if g, w := n.Kind(), ParameterValueKindUint32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint32)
}

func (n *ParameterValue) SetUint32(v uint32) {
  n.val = v
  n.kind = ParameterValueKindUint32
}

func ParameterValueUint64(v uint64) ParameterValue{
  return ParameterValue{kind: ParameterValueKindUint64, val: v}
}

func (n ParameterValue) GetUint64() uint64{
  if g, w := n.Kind(), ParameterValueKindUint64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint64)
}

func (n *ParameterValue) SetUint64(v uint64) {
  n.val = v
  n.kind = ParameterValueKindUint64
}

func ParameterValueFloating32(v float32) ParameterValue{
  return ParameterValue{kind: ParameterValueKindFloating32, val: v}
}

func (n ParameterValue) GetFloating32() float32{
  if g, w := n.Kind(), ParameterValueKindFloating32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(float32)
}

func (n *ParameterValue) SetFloating32(v float32) {
  n.val = v
  n.kind = ParameterValueKindFloating32
}

func ParameterValueFloating64(v float64) ParameterValue{
  return ParameterValue{kind: ParameterValueKindFloating64, val: v}
}

func (n ParameterValue) GetFloating64() float64{
  if g, w := n.Kind(), ParameterValueKindFloating64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(float64)
}

func (n *ParameterValue) SetFloating64(v float64) {
  n.val = v
  n.kind = ParameterValueKindFloating64
}

func ParameterValueStr(v string) ParameterValue{
  return ParameterValue{kind: ParameterValueKindStr, val: v}
}

func (n ParameterValue) GetStr() string{
  if g, w := n.Kind(), ParameterValueKindStr; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *ParameterValue) SetStr(v string) {
  n.val = v
  n.kind = ParameterValueKindStr
}

func ParameterValueBinary(v []uint8) ParameterValue{
  return ParameterValue{kind: ParameterValueKindBinary, val: v}
}

func (n ParameterValue) GetBinary() []uint8{
  if g, w := n.Kind(), ParameterValueKindBinary; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.([]uint8)
}

func (n *ParameterValue) SetBinary(v []uint8) {
  n.val = v
  n.kind = ParameterValueKindBinary
}

func ParameterValueDbNull() ParameterValue{
  return ParameterValue{kind: ParameterValueKindDbNull}
}

type DbValueKind int

const (
DbValueKindBoolean DbValueKind = iota
DbValueKindInt8
DbValueKindInt16
DbValueKindInt32
DbValueKindInt64
DbValueKindUint8
DbValueKindUint16
DbValueKindUint32
DbValueKindUint64
DbValueKindFloating32
DbValueKindFloating64
DbValueKindStr
DbValueKindBinary
DbValueKindDbNull
DbValueKindUnsupported
)

type DbValue struct {
  kind DbValueKind
  val any
}

func (n DbValue) Kind() DbValueKind {
  return n.kind
}

func DbValueBoolean(v bool) DbValue{
  return DbValue{kind: DbValueKindBoolean, val: v}
}

func (n DbValue) GetBoolean() bool{
  if g, w := n.Kind(), DbValueKindBoolean; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(bool)
}

func (n *DbValue) SetBoolean(v bool) {
  n.val = v
  n.kind = DbValueKindBoolean
}

func DbValueInt8(v int8) DbValue{
  return DbValue{kind: DbValueKindInt8, val: v}
}

func (n DbValue) GetInt8() int8{
  if g, w := n.Kind(), DbValueKindInt8; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int8)
}

func (n *DbValue) SetInt8(v int8) {
  n.val = v
  n.kind = DbValueKindInt8
}

func DbValueInt16(v int16) DbValue{
  return DbValue{kind: DbValueKindInt16, val: v}
}

func (n DbValue) GetInt16() int16{
  if g, w := n.Kind(), DbValueKindInt16; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int16)
}

func (n *DbValue) SetInt16(v int16) {
  n.val = v
  n.kind = DbValueKindInt16
}

func DbValueInt32(v int32) DbValue{
  return DbValue{kind: DbValueKindInt32, val: v}
}

func (n DbValue) GetInt32() int32{
  if g, w := n.Kind(), DbValueKindInt32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int32)
}

func (n *DbValue) SetInt32(v int32) {
  n.val = v
  n.kind = DbValueKindInt32
}

func DbValueInt64(v int64) DbValue{
  return DbValue{kind: DbValueKindInt64, val: v}
}

func (n DbValue) GetInt64() int64{
  if g, w := n.Kind(), DbValueKindInt64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(int64)
}

func (n *DbValue) SetInt64(v int64) {
  n.val = v
  n.kind = DbValueKindInt64
}

func DbValueUint8(v uint8) DbValue{
  return DbValue{kind: DbValueKindUint8, val: v}
}

func (n DbValue) GetUint8() uint8{
  if g, w := n.Kind(), DbValueKindUint8; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint8)
}

func (n *DbValue) SetUint8(v uint8) {
  n.val = v
  n.kind = DbValueKindUint8
}

func DbValueUint16(v uint16) DbValue{
  return DbValue{kind: DbValueKindUint16, val: v}
}

func (n DbValue) GetUint16() uint16{
  if g, w := n.Kind(), DbValueKindUint16; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint16)
}

func (n *DbValue) SetUint16(v uint16) {
  n.val = v
  n.kind = DbValueKindUint16
}

func DbValueUint32(v uint32) DbValue{
  return DbValue{kind: DbValueKindUint32, val: v}
}

func (n DbValue) GetUint32() uint32{
  if g, w := n.Kind(), DbValueKindUint32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint32)
}

func (n *DbValue) SetUint32(v uint32) {
  n.val = v
  n.kind = DbValueKindUint32
}

func DbValueUint64(v uint64) DbValue{
  return DbValue{kind: DbValueKindUint64, val: v}
}

func (n DbValue) GetUint64() uint64{
  if g, w := n.Kind(), DbValueKindUint64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(uint64)
}

func (n *DbValue) SetUint64(v uint64) {
  n.val = v
  n.kind = DbValueKindUint64
}

func DbValueFloating32(v float32) DbValue{
  return DbValue{kind: DbValueKindFloating32, val: v}
}

func (n DbValue) GetFloating32() float32{
  if g, w := n.Kind(), DbValueKindFloating32; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(float32)
}

func (n *DbValue) SetFloating32(v float32) {
  n.val = v
  n.kind = DbValueKindFloating32
}

func DbValueFloating64(v float64) DbValue{
  return DbValue{kind: DbValueKindFloating64, val: v}
}

func (n DbValue) GetFloating64() float64{
  if g, w := n.Kind(), DbValueKindFloating64; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(float64)
}

func (n *DbValue) SetFloating64(v float64) {
  n.val = v
  n.kind = DbValueKindFloating64
}

func DbValueStr(v string) DbValue{
  return DbValue{kind: DbValueKindStr, val: v}
}

func (n DbValue) GetStr() string{
  if g, w := n.Kind(), DbValueKindStr; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.(string)
}

func (n *DbValue) SetStr(v string) {
  n.val = v
  n.kind = DbValueKindStr
}

func DbValueBinary(v []uint8) DbValue{
  return DbValue{kind: DbValueKindBinary, val: v}
}

func (n DbValue) GetBinary() []uint8{
  if g, w := n.Kind(), DbValueKindBinary; g != w {
    panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
  }
  return n.val.([]uint8)
}

func (n *DbValue) SetBinary(v []uint8) {
  n.val = v
  n.kind = DbValueKindBinary
}

func DbValueDbNull() DbValue{
  return DbValue{kind: DbValueKindDbNull}
}

func DbValueUnsupported() DbValue{
  return DbValue{kind: DbValueKindUnsupported}
}

type Row = DbValue
type DbDataTypeKind int

const (
DbDataTypeKindBoolean DbDataTypeKind = iota
DbDataTypeKindInt8
DbDataTypeKindInt16
DbDataTypeKindInt32
DbDataTypeKindInt64
DbDataTypeKindUint8
DbDataTypeKindUint16
DbDataTypeKindUint32
DbDataTypeKindUint64
DbDataTypeKindFloating32
DbDataTypeKindFloating64
DbDataTypeKindStr
DbDataTypeKindBinary
DbDataTypeKindOther
)

type DbDataType struct {
  kind DbDataTypeKind
}

func (n DbDataType) Kind() DbDataTypeKind {
  return n.kind
}

func DbDataTypeBoolean() DbDataType{
  return DbDataType{kind: DbDataTypeKindBoolean}
}

func DbDataTypeInt8() DbDataType{
  return DbDataType{kind: DbDataTypeKindInt8}
}

func DbDataTypeInt16() DbDataType{
  return DbDataType{kind: DbDataTypeKindInt16}
}

func DbDataTypeInt32() DbDataType{
  return DbDataType{kind: DbDataTypeKindInt32}
}

func DbDataTypeInt64() DbDataType{
  return DbDataType{kind: DbDataTypeKindInt64}
}

func DbDataTypeUint8() DbDataType{
  return DbDataType{kind: DbDataTypeKindUint8}
}

func DbDataTypeUint16() DbDataType{
  return DbDataType{kind: DbDataTypeKindUint16}
}

func DbDataTypeUint32() DbDataType{
  return DbDataType{kind: DbDataTypeKindUint32}
}

func DbDataTypeUint64() DbDataType{
  return DbDataType{kind: DbDataTypeKindUint64}
}

func DbDataTypeFloating32() DbDataType{
  return DbDataType{kind: DbDataTypeKindFloating32}
}

func DbDataTypeFloating64() DbDataType{
  return DbDataType{kind: DbDataTypeKindFloating64}
}

func DbDataTypeStr() DbDataType{
  return DbDataType{kind: DbDataTypeKindStr}
}

func DbDataTypeBinary() DbDataType{
  return DbDataType{kind: DbDataTypeKindBinary}
}

func DbDataTypeOther() DbDataType{
  return DbDataType{kind: DbDataTypeKindOther}
}

type Column struct {
  Name string
  DataType DbDataType
}

type RowSet struct {
  Columns []Column
  Rows [][]DbValue
}

