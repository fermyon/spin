package http_trigger

// #include "http_trigger.h"
import "C"

import "unsafe"

import "fmt"

type FermyonSpinSqliteValueKind int

const (
	FermyonSpinSqliteValueKindInteger FermyonSpinSqliteValueKind = iota
	FermyonSpinSqliteValueKindReal
	FermyonSpinSqliteValueKindText
	FermyonSpinSqliteValueKindBlob
	FermyonSpinSqliteValueKindNull
)

type FermyonSpinSqliteValue struct {
	kind FermyonSpinSqliteValueKind
	val  any
}

func (n FermyonSpinSqliteValue) Kind() FermyonSpinSqliteValueKind {
	return n.kind
}

func FermyonSpinSqliteValueInteger(v int64) FermyonSpinSqliteValue {
	return FermyonSpinSqliteValue{kind: FermyonSpinSqliteValueKindInteger, val: v}
}

func (n FermyonSpinSqliteValue) GetInteger() int64 {
	if g, w := n.Kind(), FermyonSpinSqliteValueKindInteger; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int64)
}

func (n *FermyonSpinSqliteValue) SetInteger(v int64) {
	n.val = v
	n.kind = FermyonSpinSqliteValueKindInteger
}

func FermyonSpinSqliteValueReal(v float64) FermyonSpinSqliteValue {
	return FermyonSpinSqliteValue{kind: FermyonSpinSqliteValueKindReal, val: v}
}

func (n FermyonSpinSqliteValue) GetReal() float64 {
	if g, w := n.Kind(), FermyonSpinSqliteValueKindReal; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(float64)
}

func (n *FermyonSpinSqliteValue) SetReal(v float64) {
	n.val = v
	n.kind = FermyonSpinSqliteValueKindReal
}

func FermyonSpinSqliteValueText(v string) FermyonSpinSqliteValue {
	return FermyonSpinSqliteValue{kind: FermyonSpinSqliteValueKindText, val: v}
}

func (n FermyonSpinSqliteValue) GetText() string {
	if g, w := n.Kind(), FermyonSpinSqliteValueKindText; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinSqliteValue) SetText(v string) {
	n.val = v
	n.kind = FermyonSpinSqliteValueKindText
}

func FermyonSpinSqliteValueBlob(v []uint8) FermyonSpinSqliteValue {
	return FermyonSpinSqliteValue{kind: FermyonSpinSqliteValueKindBlob, val: v}
}

func (n FermyonSpinSqliteValue) GetBlob() []uint8 {
	if g, w := n.Kind(), FermyonSpinSqliteValueKindBlob; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.([]uint8)
}

func (n *FermyonSpinSqliteValue) SetBlob(v []uint8) {
	n.val = v
	n.kind = FermyonSpinSqliteValueKindBlob
}

func FermyonSpinSqliteValueNull() FermyonSpinSqliteValue {
	return FermyonSpinSqliteValue{kind: FermyonSpinSqliteValueKindNull}
}

type FermyonSpinSqliteRowResult struct {
	Values []FermyonSpinSqliteValue
}

type FermyonSpinSqliteQueryResult struct {
	Columns []string
	Rows    []FermyonSpinSqliteRowResult
}

type FermyonSpinSqliteErrorKind int

const (
	FermyonSpinSqliteErrorKindNoSuchDatabase FermyonSpinSqliteErrorKind = iota
	FermyonSpinSqliteErrorKindAccessDenied
	FermyonSpinSqliteErrorKindInvalidConnection
	FermyonSpinSqliteErrorKindDatabaseFull
	FermyonSpinSqliteErrorKindIo
)

type FermyonSpinSqliteError struct {
	kind FermyonSpinSqliteErrorKind
	val  any
}

func (n FermyonSpinSqliteError) Kind() FermyonSpinSqliteErrorKind {
	return n.kind
}

func FermyonSpinSqliteErrorNoSuchDatabase() FermyonSpinSqliteError {
	return FermyonSpinSqliteError{kind: FermyonSpinSqliteErrorKindNoSuchDatabase}
}

func FermyonSpinSqliteErrorAccessDenied() FermyonSpinSqliteError {
	return FermyonSpinSqliteError{kind: FermyonSpinSqliteErrorKindAccessDenied}
}

func FermyonSpinSqliteErrorInvalidConnection() FermyonSpinSqliteError {
	return FermyonSpinSqliteError{kind: FermyonSpinSqliteErrorKindInvalidConnection}
}

func FermyonSpinSqliteErrorDatabaseFull() FermyonSpinSqliteError {
	return FermyonSpinSqliteError{kind: FermyonSpinSqliteErrorKindDatabaseFull}
}

func FermyonSpinSqliteErrorIo(v string) FermyonSpinSqliteError {
	return FermyonSpinSqliteError{kind: FermyonSpinSqliteErrorKindIo, val: v}
}

func (n FermyonSpinSqliteError) GetIo() string {
	if g, w := n.Kind(), FermyonSpinSqliteErrorKindIo; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinSqliteError) SetIo(v string) {
	n.val = v
	n.kind = FermyonSpinSqliteErrorKindIo
}

type FermyonSpinSqliteConnection = uint32
type FermyonSpinRedisTypesPayload = uint8
type FermyonSpinRedisTypesRedisResultKind int

const (
	FermyonSpinRedisTypesRedisResultKindNil FermyonSpinRedisTypesRedisResultKind = iota
	FermyonSpinRedisTypesRedisResultKindStatus
	FermyonSpinRedisTypesRedisResultKindInt64
	FermyonSpinRedisTypesRedisResultKindBinary
)

type FermyonSpinRedisTypesRedisResult struct {
	kind FermyonSpinRedisTypesRedisResultKind
	val  any
}

func (n FermyonSpinRedisTypesRedisResult) Kind() FermyonSpinRedisTypesRedisResultKind {
	return n.kind
}

func FermyonSpinRedisTypesRedisResultNil() FermyonSpinRedisTypesRedisResult {
	return FermyonSpinRedisTypesRedisResult{kind: FermyonSpinRedisTypesRedisResultKindNil}
}

func FermyonSpinRedisTypesRedisResultStatus(v string) FermyonSpinRedisTypesRedisResult {
	return FermyonSpinRedisTypesRedisResult{kind: FermyonSpinRedisTypesRedisResultKindStatus, val: v}
}

func (n FermyonSpinRedisTypesRedisResult) GetStatus() string {
	if g, w := n.Kind(), FermyonSpinRedisTypesRedisResultKindStatus; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinRedisTypesRedisResult) SetStatus(v string) {
	n.val = v
	n.kind = FermyonSpinRedisTypesRedisResultKindStatus
}

func FermyonSpinRedisTypesRedisResultInt64(v int64) FermyonSpinRedisTypesRedisResult {
	return FermyonSpinRedisTypesRedisResult{kind: FermyonSpinRedisTypesRedisResultKindInt64, val: v}
}

func (n FermyonSpinRedisTypesRedisResult) GetInt64() int64 {
	if g, w := n.Kind(), FermyonSpinRedisTypesRedisResultKindInt64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int64)
}

func (n *FermyonSpinRedisTypesRedisResult) SetInt64(v int64) {
	n.val = v
	n.kind = FermyonSpinRedisTypesRedisResultKindInt64
}

func FermyonSpinRedisTypesRedisResultBinary(v []uint8) FermyonSpinRedisTypesRedisResult {
	return FermyonSpinRedisTypesRedisResult{kind: FermyonSpinRedisTypesRedisResultKindBinary, val: v}
}

func (n FermyonSpinRedisTypesRedisResult) GetBinary() []uint8 {
	if g, w := n.Kind(), FermyonSpinRedisTypesRedisResultKindBinary; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.([]uint8)
}

func (n *FermyonSpinRedisTypesRedisResult) SetBinary(v []uint8) {
	n.val = v
	n.kind = FermyonSpinRedisTypesRedisResultKindBinary
}

type FermyonSpinRedisTypesRedisParameterKind int

const (
	FermyonSpinRedisTypesRedisParameterKindInt64 FermyonSpinRedisTypesRedisParameterKind = iota
	FermyonSpinRedisTypesRedisParameterKindBinary
)

type FermyonSpinRedisTypesRedisParameter struct {
	kind FermyonSpinRedisTypesRedisParameterKind
	val  any
}

func (n FermyonSpinRedisTypesRedisParameter) Kind() FermyonSpinRedisTypesRedisParameterKind {
	return n.kind
}

func FermyonSpinRedisTypesRedisParameterInt64(v int64) FermyonSpinRedisTypesRedisParameter {
	return FermyonSpinRedisTypesRedisParameter{kind: FermyonSpinRedisTypesRedisParameterKindInt64, val: v}
}

func (n FermyonSpinRedisTypesRedisParameter) GetInt64() int64 {
	if g, w := n.Kind(), FermyonSpinRedisTypesRedisParameterKindInt64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int64)
}

func (n *FermyonSpinRedisTypesRedisParameter) SetInt64(v int64) {
	n.val = v
	n.kind = FermyonSpinRedisTypesRedisParameterKindInt64
}

func FermyonSpinRedisTypesRedisParameterBinary(v []uint8) FermyonSpinRedisTypesRedisParameter {
	return FermyonSpinRedisTypesRedisParameter{kind: FermyonSpinRedisTypesRedisParameterKindBinary, val: v}
}

func (n FermyonSpinRedisTypesRedisParameter) GetBinary() []uint8 {
	if g, w := n.Kind(), FermyonSpinRedisTypesRedisParameterKindBinary; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.([]uint8)
}

func (n *FermyonSpinRedisTypesRedisParameter) SetBinary(v []uint8) {
	n.val = v
	n.kind = FermyonSpinRedisTypesRedisParameterKindBinary
}

type FermyonSpinRedisTypesErrorKind int

const (
	FermyonSpinRedisTypesErrorKindSuccess FermyonSpinRedisTypesErrorKind = iota
	FermyonSpinRedisTypesErrorKindError
)

type FermyonSpinRedisTypesError struct {
	kind FermyonSpinRedisTypesErrorKind
}

func (n FermyonSpinRedisTypesError) Kind() FermyonSpinRedisTypesErrorKind {
	return n.kind
}

func FermyonSpinRedisTypesErrorSuccess() FermyonSpinRedisTypesError {
	return FermyonSpinRedisTypesError{kind: FermyonSpinRedisTypesErrorKindSuccess}
}

func FermyonSpinRedisTypesErrorError() FermyonSpinRedisTypesError {
	return FermyonSpinRedisTypesError{kind: FermyonSpinRedisTypesErrorKindError}
}

type FermyonSpinRedisPayload = []uint8
type FermyonSpinRedisRedisParameter = FermyonSpinRedisTypesRedisParameter
type FermyonSpinRedisRedisResult = FermyonSpinRedisTypesRedisResult
type FermyonSpinRedisError = FermyonSpinRedisTypesError
type FermyonSpinRdbmsTypesParameterValueKind int

const (
	FermyonSpinRdbmsTypesParameterValueKindBoolean FermyonSpinRdbmsTypesParameterValueKind = iota
	FermyonSpinRdbmsTypesParameterValueKindInt8
	FermyonSpinRdbmsTypesParameterValueKindInt16
	FermyonSpinRdbmsTypesParameterValueKindInt32
	FermyonSpinRdbmsTypesParameterValueKindInt64
	FermyonSpinRdbmsTypesParameterValueKindUint8
	FermyonSpinRdbmsTypesParameterValueKindUint16
	FermyonSpinRdbmsTypesParameterValueKindUint32
	FermyonSpinRdbmsTypesParameterValueKindUint64
	FermyonSpinRdbmsTypesParameterValueKindFloating32
	FermyonSpinRdbmsTypesParameterValueKindFloating64
	FermyonSpinRdbmsTypesParameterValueKindStr
	FermyonSpinRdbmsTypesParameterValueKindBinary
	FermyonSpinRdbmsTypesParameterValueKindDbNull
)

type FermyonSpinRdbmsTypesParameterValue struct {
	kind FermyonSpinRdbmsTypesParameterValueKind
	val  any
}

func (n FermyonSpinRdbmsTypesParameterValue) Kind() FermyonSpinRdbmsTypesParameterValueKind {
	return n.kind
}

func FermyonSpinRdbmsTypesParameterValueBoolean(v bool) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindBoolean, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetBoolean() bool {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindBoolean; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(bool)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetBoolean(v bool) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindBoolean
}

func FermyonSpinRdbmsTypesParameterValueInt8(v int8) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindInt8, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetInt8() int8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindInt8; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int8)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetInt8(v int8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindInt8
}

func FermyonSpinRdbmsTypesParameterValueInt16(v int16) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindInt16, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetInt16() int16 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindInt16; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int16)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetInt16(v int16) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindInt16
}

func FermyonSpinRdbmsTypesParameterValueInt32(v int32) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindInt32, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetInt32() int32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindInt32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int32)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetInt32(v int32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindInt32
}

func FermyonSpinRdbmsTypesParameterValueInt64(v int64) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindInt64, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetInt64() int64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindInt64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int64)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetInt64(v int64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindInt64
}

func FermyonSpinRdbmsTypesParameterValueUint8(v uint8) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindUint8, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetUint8() uint8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindUint8; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint8)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetUint8(v uint8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindUint8
}

func FermyonSpinRdbmsTypesParameterValueUint16(v uint16) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindUint16, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetUint16() uint16 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindUint16; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint16)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetUint16(v uint16) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindUint16
}

func FermyonSpinRdbmsTypesParameterValueUint32(v uint32) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindUint32, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetUint32() uint32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindUint32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint32)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetUint32(v uint32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindUint32
}

func FermyonSpinRdbmsTypesParameterValueUint64(v uint64) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindUint64, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetUint64() uint64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindUint64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint64)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetUint64(v uint64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindUint64
}

func FermyonSpinRdbmsTypesParameterValueFloating32(v float32) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindFloating32, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetFloating32() float32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindFloating32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(float32)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetFloating32(v float32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindFloating32
}

func FermyonSpinRdbmsTypesParameterValueFloating64(v float64) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindFloating64, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetFloating64() float64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindFloating64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(float64)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetFloating64(v float64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindFloating64
}

func FermyonSpinRdbmsTypesParameterValueStr(v string) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindStr, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetStr() string {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindStr; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetStr(v string) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindStr
}

func FermyonSpinRdbmsTypesParameterValueBinary(v []uint8) FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindBinary, val: v}
}

func (n FermyonSpinRdbmsTypesParameterValue) GetBinary() []uint8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesParameterValueKindBinary; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.([]uint8)
}

func (n *FermyonSpinRdbmsTypesParameterValue) SetBinary(v []uint8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesParameterValueKindBinary
}

func FermyonSpinRdbmsTypesParameterValueDbNull() FermyonSpinRdbmsTypesParameterValue {
	return FermyonSpinRdbmsTypesParameterValue{kind: FermyonSpinRdbmsTypesParameterValueKindDbNull}
}

type FermyonSpinRdbmsTypesDbValueKind int

const (
	FermyonSpinRdbmsTypesDbValueKindBoolean FermyonSpinRdbmsTypesDbValueKind = iota
	FermyonSpinRdbmsTypesDbValueKindInt8
	FermyonSpinRdbmsTypesDbValueKindInt16
	FermyonSpinRdbmsTypesDbValueKindInt32
	FermyonSpinRdbmsTypesDbValueKindInt64
	FermyonSpinRdbmsTypesDbValueKindUint8
	FermyonSpinRdbmsTypesDbValueKindUint16
	FermyonSpinRdbmsTypesDbValueKindUint32
	FermyonSpinRdbmsTypesDbValueKindUint64
	FermyonSpinRdbmsTypesDbValueKindFloating32
	FermyonSpinRdbmsTypesDbValueKindFloating64
	FermyonSpinRdbmsTypesDbValueKindStr
	FermyonSpinRdbmsTypesDbValueKindBinary
	FermyonSpinRdbmsTypesDbValueKindDbNull
	FermyonSpinRdbmsTypesDbValueKindUnsupported
)

type FermyonSpinRdbmsTypesDbValue struct {
	kind FermyonSpinRdbmsTypesDbValueKind
	val  any
}

func (n FermyonSpinRdbmsTypesDbValue) Kind() FermyonSpinRdbmsTypesDbValueKind {
	return n.kind
}

func FermyonSpinRdbmsTypesDbValueBoolean(v bool) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindBoolean, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetBoolean() bool {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindBoolean; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(bool)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetBoolean(v bool) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindBoolean
}

func FermyonSpinRdbmsTypesDbValueInt8(v int8) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindInt8, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetInt8() int8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindInt8; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int8)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetInt8(v int8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindInt8
}

func FermyonSpinRdbmsTypesDbValueInt16(v int16) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindInt16, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetInt16() int16 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindInt16; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int16)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetInt16(v int16) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindInt16
}

func FermyonSpinRdbmsTypesDbValueInt32(v int32) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindInt32, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetInt32() int32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindInt32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int32)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetInt32(v int32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindInt32
}

func FermyonSpinRdbmsTypesDbValueInt64(v int64) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindInt64, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetInt64() int64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindInt64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(int64)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetInt64(v int64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindInt64
}

func FermyonSpinRdbmsTypesDbValueUint8(v uint8) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindUint8, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetUint8() uint8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindUint8; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint8)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetUint8(v uint8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindUint8
}

func FermyonSpinRdbmsTypesDbValueUint16(v uint16) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindUint16, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetUint16() uint16 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindUint16; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint16)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetUint16(v uint16) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindUint16
}

func FermyonSpinRdbmsTypesDbValueUint32(v uint32) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindUint32, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetUint32() uint32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindUint32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint32)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetUint32(v uint32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindUint32
}

func FermyonSpinRdbmsTypesDbValueUint64(v uint64) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindUint64, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetUint64() uint64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindUint64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(uint64)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetUint64(v uint64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindUint64
}

func FermyonSpinRdbmsTypesDbValueFloating32(v float32) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindFloating32, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetFloating32() float32 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindFloating32; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(float32)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetFloating32(v float32) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindFloating32
}

func FermyonSpinRdbmsTypesDbValueFloating64(v float64) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindFloating64, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetFloating64() float64 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindFloating64; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(float64)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetFloating64(v float64) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindFloating64
}

func FermyonSpinRdbmsTypesDbValueStr(v string) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindStr, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetStr() string {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindStr; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetStr(v string) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindStr
}

func FermyonSpinRdbmsTypesDbValueBinary(v []uint8) FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindBinary, val: v}
}

func (n FermyonSpinRdbmsTypesDbValue) GetBinary() []uint8 {
	if g, w := n.Kind(), FermyonSpinRdbmsTypesDbValueKindBinary; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.([]uint8)
}

func (n *FermyonSpinRdbmsTypesDbValue) SetBinary(v []uint8) {
	n.val = v
	n.kind = FermyonSpinRdbmsTypesDbValueKindBinary
}

func FermyonSpinRdbmsTypesDbValueDbNull() FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindDbNull}
}

func FermyonSpinRdbmsTypesDbValueUnsupported() FermyonSpinRdbmsTypesDbValue {
	return FermyonSpinRdbmsTypesDbValue{kind: FermyonSpinRdbmsTypesDbValueKindUnsupported}
}

type FermyonSpinRdbmsTypesRow = FermyonSpinRdbmsTypesDbValue
type FermyonSpinRdbmsTypesDbDataTypeKind int

const (
	FermyonSpinRdbmsTypesDbDataTypeKindBoolean FermyonSpinRdbmsTypesDbDataTypeKind = iota
	FermyonSpinRdbmsTypesDbDataTypeKindInt8
	FermyonSpinRdbmsTypesDbDataTypeKindInt16
	FermyonSpinRdbmsTypesDbDataTypeKindInt32
	FermyonSpinRdbmsTypesDbDataTypeKindInt64
	FermyonSpinRdbmsTypesDbDataTypeKindUint8
	FermyonSpinRdbmsTypesDbDataTypeKindUint16
	FermyonSpinRdbmsTypesDbDataTypeKindUint32
	FermyonSpinRdbmsTypesDbDataTypeKindUint64
	FermyonSpinRdbmsTypesDbDataTypeKindFloating32
	FermyonSpinRdbmsTypesDbDataTypeKindFloating64
	FermyonSpinRdbmsTypesDbDataTypeKindStr
	FermyonSpinRdbmsTypesDbDataTypeKindBinary
	FermyonSpinRdbmsTypesDbDataTypeKindOther
)

type FermyonSpinRdbmsTypesDbDataType struct {
	kind FermyonSpinRdbmsTypesDbDataTypeKind
}

func (n FermyonSpinRdbmsTypesDbDataType) Kind() FermyonSpinRdbmsTypesDbDataTypeKind {
	return n.kind
}

func FermyonSpinRdbmsTypesDbDataTypeBoolean() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindBoolean}
}

func FermyonSpinRdbmsTypesDbDataTypeInt8() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindInt8}
}

func FermyonSpinRdbmsTypesDbDataTypeInt16() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindInt16}
}

func FermyonSpinRdbmsTypesDbDataTypeInt32() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindInt32}
}

func FermyonSpinRdbmsTypesDbDataTypeInt64() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindInt64}
}

func FermyonSpinRdbmsTypesDbDataTypeUint8() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindUint8}
}

func FermyonSpinRdbmsTypesDbDataTypeUint16() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindUint16}
}

func FermyonSpinRdbmsTypesDbDataTypeUint32() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindUint32}
}

func FermyonSpinRdbmsTypesDbDataTypeUint64() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindUint64}
}

func FermyonSpinRdbmsTypesDbDataTypeFloating32() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindFloating32}
}

func FermyonSpinRdbmsTypesDbDataTypeFloating64() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindFloating64}
}

func FermyonSpinRdbmsTypesDbDataTypeStr() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindStr}
}

func FermyonSpinRdbmsTypesDbDataTypeBinary() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindBinary}
}

func FermyonSpinRdbmsTypesDbDataTypeOther() FermyonSpinRdbmsTypesDbDataType {
	return FermyonSpinRdbmsTypesDbDataType{kind: FermyonSpinRdbmsTypesDbDataTypeKindOther}
}

type FermyonSpinRdbmsTypesColumn struct {
	Name     string
	DataType FermyonSpinRdbmsTypesDbDataType
}

type FermyonSpinRdbmsTypesRowSet struct {
	Columns []FermyonSpinRdbmsTypesColumn
	Rows    [][]FermyonSpinRdbmsTypesDbValue
}

type FermyonSpinPostgresParameterValue = FermyonSpinRdbmsTypesParameterValue
type FermyonSpinPostgresRowSet = FermyonSpinRdbmsTypesRowSet
type FermyonSpinPostgresPgErrorKind int

const (
	FermyonSpinPostgresPgErrorKindSuccess FermyonSpinPostgresPgErrorKind = iota
	FermyonSpinPostgresPgErrorKindConnectionFailed
	FermyonSpinPostgresPgErrorKindBadParameter
	FermyonSpinPostgresPgErrorKindQueryFailed
	FermyonSpinPostgresPgErrorKindValueConversionFailed
	FermyonSpinPostgresPgErrorKindOtherError
)

type FermyonSpinPostgresPgError struct {
	kind FermyonSpinPostgresPgErrorKind
	val  any
}

func (n FermyonSpinPostgresPgError) Kind() FermyonSpinPostgresPgErrorKind {
	return n.kind
}

func FermyonSpinPostgresPgErrorSuccess() FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindSuccess}
}

func FermyonSpinPostgresPgErrorConnectionFailed(v string) FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindConnectionFailed, val: v}
}

func (n FermyonSpinPostgresPgError) GetConnectionFailed() string {
	if g, w := n.Kind(), FermyonSpinPostgresPgErrorKindConnectionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinPostgresPgError) SetConnectionFailed(v string) {
	n.val = v
	n.kind = FermyonSpinPostgresPgErrorKindConnectionFailed
}

func FermyonSpinPostgresPgErrorBadParameter(v string) FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindBadParameter, val: v}
}

func (n FermyonSpinPostgresPgError) GetBadParameter() string {
	if g, w := n.Kind(), FermyonSpinPostgresPgErrorKindBadParameter; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinPostgresPgError) SetBadParameter(v string) {
	n.val = v
	n.kind = FermyonSpinPostgresPgErrorKindBadParameter
}

func FermyonSpinPostgresPgErrorQueryFailed(v string) FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindQueryFailed, val: v}
}

func (n FermyonSpinPostgresPgError) GetQueryFailed() string {
	if g, w := n.Kind(), FermyonSpinPostgresPgErrorKindQueryFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinPostgresPgError) SetQueryFailed(v string) {
	n.val = v
	n.kind = FermyonSpinPostgresPgErrorKindQueryFailed
}

func FermyonSpinPostgresPgErrorValueConversionFailed(v string) FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindValueConversionFailed, val: v}
}

func (n FermyonSpinPostgresPgError) GetValueConversionFailed() string {
	if g, w := n.Kind(), FermyonSpinPostgresPgErrorKindValueConversionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinPostgresPgError) SetValueConversionFailed(v string) {
	n.val = v
	n.kind = FermyonSpinPostgresPgErrorKindValueConversionFailed
}

func FermyonSpinPostgresPgErrorOtherError(v string) FermyonSpinPostgresPgError {
	return FermyonSpinPostgresPgError{kind: FermyonSpinPostgresPgErrorKindOtherError, val: v}
}

func (n FermyonSpinPostgresPgError) GetOtherError() string {
	if g, w := n.Kind(), FermyonSpinPostgresPgErrorKindOtherError; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinPostgresPgError) SetOtherError(v string) {
	n.val = v
	n.kind = FermyonSpinPostgresPgErrorKindOtherError
}

type FermyonSpinMysqlParameterValue = FermyonSpinRdbmsTypesParameterValue
type FermyonSpinMysqlRowSet = FermyonSpinRdbmsTypesRowSet
type FermyonSpinMysqlMysqlErrorKind int

const (
	FermyonSpinMysqlMysqlErrorKindSuccess FermyonSpinMysqlMysqlErrorKind = iota
	FermyonSpinMysqlMysqlErrorKindConnectionFailed
	FermyonSpinMysqlMysqlErrorKindBadParameter
	FermyonSpinMysqlMysqlErrorKindQueryFailed
	FermyonSpinMysqlMysqlErrorKindValueConversionFailed
	FermyonSpinMysqlMysqlErrorKindOtherError
)

type FermyonSpinMysqlMysqlError struct {
	kind FermyonSpinMysqlMysqlErrorKind
	val  any
}

func (n FermyonSpinMysqlMysqlError) Kind() FermyonSpinMysqlMysqlErrorKind {
	return n.kind
}

func FermyonSpinMysqlMysqlErrorSuccess() FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindSuccess}
}

func FermyonSpinMysqlMysqlErrorConnectionFailed(v string) FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindConnectionFailed, val: v}
}

func (n FermyonSpinMysqlMysqlError) GetConnectionFailed() string {
	if g, w := n.Kind(), FermyonSpinMysqlMysqlErrorKindConnectionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinMysqlMysqlError) SetConnectionFailed(v string) {
	n.val = v
	n.kind = FermyonSpinMysqlMysqlErrorKindConnectionFailed
}

func FermyonSpinMysqlMysqlErrorBadParameter(v string) FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindBadParameter, val: v}
}

func (n FermyonSpinMysqlMysqlError) GetBadParameter() string {
	if g, w := n.Kind(), FermyonSpinMysqlMysqlErrorKindBadParameter; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinMysqlMysqlError) SetBadParameter(v string) {
	n.val = v
	n.kind = FermyonSpinMysqlMysqlErrorKindBadParameter
}

func FermyonSpinMysqlMysqlErrorQueryFailed(v string) FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindQueryFailed, val: v}
}

func (n FermyonSpinMysqlMysqlError) GetQueryFailed() string {
	if g, w := n.Kind(), FermyonSpinMysqlMysqlErrorKindQueryFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinMysqlMysqlError) SetQueryFailed(v string) {
	n.val = v
	n.kind = FermyonSpinMysqlMysqlErrorKindQueryFailed
}

func FermyonSpinMysqlMysqlErrorValueConversionFailed(v string) FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindValueConversionFailed, val: v}
}

func (n FermyonSpinMysqlMysqlError) GetValueConversionFailed() string {
	if g, w := n.Kind(), FermyonSpinMysqlMysqlErrorKindValueConversionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinMysqlMysqlError) SetValueConversionFailed(v string) {
	n.val = v
	n.kind = FermyonSpinMysqlMysqlErrorKindValueConversionFailed
}

func FermyonSpinMysqlMysqlErrorOtherError(v string) FermyonSpinMysqlMysqlError {
	return FermyonSpinMysqlMysqlError{kind: FermyonSpinMysqlMysqlErrorKindOtherError, val: v}
}

func (n FermyonSpinMysqlMysqlError) GetOtherError() string {
	if g, w := n.Kind(), FermyonSpinMysqlMysqlErrorKindOtherError; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinMysqlMysqlError) SetOtherError(v string) {
	n.val = v
	n.kind = FermyonSpinMysqlMysqlErrorKindOtherError
}

type FermyonSpinKeyValueStore = uint32
type FermyonSpinKeyValueErrorKind int

const (
	FermyonSpinKeyValueErrorKindStoreTableFull FermyonSpinKeyValueErrorKind = iota
	FermyonSpinKeyValueErrorKindNoSuchStore
	FermyonSpinKeyValueErrorKindAccessDenied
	FermyonSpinKeyValueErrorKindInvalidStore
	FermyonSpinKeyValueErrorKindNoSuchKey
	FermyonSpinKeyValueErrorKindIo
)

type FermyonSpinKeyValueError struct {
	kind FermyonSpinKeyValueErrorKind
	val  any
}

func (n FermyonSpinKeyValueError) Kind() FermyonSpinKeyValueErrorKind {
	return n.kind
}

func FermyonSpinKeyValueErrorStoreTableFull() FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindStoreTableFull}
}

func FermyonSpinKeyValueErrorNoSuchStore() FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindNoSuchStore}
}

func FermyonSpinKeyValueErrorAccessDenied() FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindAccessDenied}
}

func FermyonSpinKeyValueErrorInvalidStore() FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindInvalidStore}
}

func FermyonSpinKeyValueErrorNoSuchKey() FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindNoSuchKey}
}

func FermyonSpinKeyValueErrorIo(v string) FermyonSpinKeyValueError {
	return FermyonSpinKeyValueError{kind: FermyonSpinKeyValueErrorKindIo, val: v}
}

func (n FermyonSpinKeyValueError) GetIo() string {
	if g, w := n.Kind(), FermyonSpinKeyValueErrorKindIo; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinKeyValueError) SetIo(v string) {
	n.val = v
	n.kind = FermyonSpinKeyValueErrorKindIo
}

type FermyonSpinHttpTypesUri = string
type FermyonSpinInboundHttpTuple2StringStringT struct {
	F0 string
	F1 string
}

type FermyonSpinHttpTuple2StringStringT = FermyonSpinInboundHttpTuple2StringStringT
type FermyonSpinHttpTypesTuple2StringStringT = FermyonSpinInboundHttpTuple2StringStringT
type FermyonSpinHttpTypesParams = FermyonSpinHttpTypesTuple2StringStringT
type FermyonSpinHttpTypesMethodKind int

const (
	FermyonSpinHttpTypesMethodKindGet FermyonSpinHttpTypesMethodKind = iota
	FermyonSpinHttpTypesMethodKindPost
	FermyonSpinHttpTypesMethodKindPut
	FermyonSpinHttpTypesMethodKindDelete
	FermyonSpinHttpTypesMethodKindPatch
	FermyonSpinHttpTypesMethodKindHead
	FermyonSpinHttpTypesMethodKindOptions
)

type FermyonSpinHttpTypesMethod struct {
	kind FermyonSpinHttpTypesMethodKind
}

func (n FermyonSpinHttpTypesMethod) Kind() FermyonSpinHttpTypesMethodKind {
	return n.kind
}

func FermyonSpinHttpTypesMethodGet() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindGet}
}

func FermyonSpinHttpTypesMethodPost() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindPost}
}

func FermyonSpinHttpTypesMethodPut() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindPut}
}

func FermyonSpinHttpTypesMethodDelete() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindDelete}
}

func FermyonSpinHttpTypesMethodPatch() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindPatch}
}

func FermyonSpinHttpTypesMethodHead() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindHead}
}

func FermyonSpinHttpTypesMethodOptions() FermyonSpinHttpTypesMethod {
	return FermyonSpinHttpTypesMethod{kind: FermyonSpinHttpTypesMethodKindOptions}
}

type FermyonSpinHttpTypesHttpStatus = uint16
type FermyonSpinHttpTypesHttpErrorKind int

const (
	FermyonSpinHttpTypesHttpErrorKindSuccess FermyonSpinHttpTypesHttpErrorKind = iota
	FermyonSpinHttpTypesHttpErrorKindDestinationNotAllowed
	FermyonSpinHttpTypesHttpErrorKindInvalidUrl
	FermyonSpinHttpTypesHttpErrorKindRequestError
	FermyonSpinHttpTypesHttpErrorKindRuntimeError
	FermyonSpinHttpTypesHttpErrorKindTooManyRequests
)

type FermyonSpinHttpTypesHttpError struct {
	kind FermyonSpinHttpTypesHttpErrorKind
}

func (n FermyonSpinHttpTypesHttpError) Kind() FermyonSpinHttpTypesHttpErrorKind {
	return n.kind
}

func FermyonSpinHttpTypesHttpErrorSuccess() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindSuccess}
}

func FermyonSpinHttpTypesHttpErrorDestinationNotAllowed() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindDestinationNotAllowed}
}

func FermyonSpinHttpTypesHttpErrorInvalidUrl() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindInvalidUrl}
}

func FermyonSpinHttpTypesHttpErrorRequestError() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindRequestError}
}

func FermyonSpinHttpTypesHttpErrorRuntimeError() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindRuntimeError}
}

func FermyonSpinHttpTypesHttpErrorTooManyRequests() FermyonSpinHttpTypesHttpError {
	return FermyonSpinHttpTypesHttpError{kind: FermyonSpinHttpTypesHttpErrorKindTooManyRequests}
}

type FermyonSpinHttpTypesHeaders = FermyonSpinHttpTypesTuple2StringStringT
type FermyonSpinHttpTypesBody = uint8
type FermyonSpinHttpTypesResponse struct {
	Status  uint16
	Headers Option[[]FermyonSpinHttpTypesTuple2StringStringT]
	Body    Option[[]uint8]
}

type FermyonSpinHttpTypesRequest struct {
	Method  FermyonSpinHttpTypesMethod
	Uri     string
	Headers []FermyonSpinHttpTypesTuple2StringStringT
	Params  []FermyonSpinHttpTypesTuple2StringStringT
	Body    Option[[]uint8]
}

type FermyonSpinInboundHttpRequest = FermyonSpinHttpTypesRequest
type FermyonSpinInboundHttpResponse = FermyonSpinHttpTypesResponse
type FermyonSpinHttpRequest = FermyonSpinHttpTypesRequest
type FermyonSpinHttpResponse = FermyonSpinHttpTypesResponse
type FermyonSpinHttpHttpError = FermyonSpinHttpTypesHttpError
type FermyonSpinConfigErrorKind int

const (
	FermyonSpinConfigErrorKindProvider FermyonSpinConfigErrorKind = iota
	FermyonSpinConfigErrorKindInvalidKey
	FermyonSpinConfigErrorKindInvalidSchema
	FermyonSpinConfigErrorKindOther
)

type FermyonSpinConfigError struct {
	kind FermyonSpinConfigErrorKind
	val  any
}

func (n FermyonSpinConfigError) Kind() FermyonSpinConfigErrorKind {
	return n.kind
}

func FermyonSpinConfigErrorProvider(v string) FermyonSpinConfigError {
	return FermyonSpinConfigError{kind: FermyonSpinConfigErrorKindProvider, val: v}
}

func (n FermyonSpinConfigError) GetProvider() string {
	if g, w := n.Kind(), FermyonSpinConfigErrorKindProvider; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinConfigError) SetProvider(v string) {
	n.val = v
	n.kind = FermyonSpinConfigErrorKindProvider
}

func FermyonSpinConfigErrorInvalidKey(v string) FermyonSpinConfigError {
	return FermyonSpinConfigError{kind: FermyonSpinConfigErrorKindInvalidKey, val: v}
}

func (n FermyonSpinConfigError) GetInvalidKey() string {
	if g, w := n.Kind(), FermyonSpinConfigErrorKindInvalidKey; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinConfigError) SetInvalidKey(v string) {
	n.val = v
	n.kind = FermyonSpinConfigErrorKindInvalidKey
}

func FermyonSpinConfigErrorInvalidSchema(v string) FermyonSpinConfigError {
	return FermyonSpinConfigError{kind: FermyonSpinConfigErrorKindInvalidSchema, val: v}
}

func (n FermyonSpinConfigError) GetInvalidSchema() string {
	if g, w := n.Kind(), FermyonSpinConfigErrorKindInvalidSchema; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinConfigError) SetInvalidSchema(v string) {
	n.val = v
	n.kind = FermyonSpinConfigErrorKindInvalidSchema
}

func FermyonSpinConfigErrorOther(v string) FermyonSpinConfigError {
	return FermyonSpinConfigError{kind: FermyonSpinConfigErrorKindOther, val: v}
}

func (n FermyonSpinConfigError) GetOther() string {
	if g, w := n.Kind(), FermyonSpinConfigErrorKindOther; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *FermyonSpinConfigError) SetOther(v string) {
	n.val = v
	n.kind = FermyonSpinConfigErrorKindOther
}

// Import functions from fermyon:spin/config
func FermyonSpinConfigGetConfig(key string) Result[string, FermyonSpinConfigError] {
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_string_error_t
	C.fermyon_spin_config_get_config(&lower_key, &ret)
	var lift_ret Result[string, FermyonSpinConfigError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_config_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinConfigError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinConfigErrorProvider(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinConfigErrorInvalidKey(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinConfigErrorInvalidSchema(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinConfigErrorOther(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val string
		lift_ret_val = C.GoStringN(lift_ret_ptr.ptr, C.int(lift_ret_ptr.len))
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

// Import functions from fermyon:spin/rdbms-types
// Import functions from fermyon:spin/postgres
func FermyonSpinPostgresQuery(address string, statement string, params []FermyonSpinRdbmsTypesParameterValue) Result[FermyonSpinRdbmsTypesRowSet, FermyonSpinPostgresPgError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_statement C.http_trigger_string_t

	lower_statement.ptr = C.CString(statement)
	lower_statement.len = C.size_t(len(statement))
	defer C.http_trigger_string_free(&lower_statement)
	var lower_params C.http_trigger_list_parameter_value_t
	if len(params) == 0 {
		lower_params.ptr = nil
		lower_params.len = 0
	} else {
		var empty_lower_params C.fermyon_spin_postgres_parameter_value_t
		lower_params.ptr = (*C.fermyon_spin_postgres_parameter_value_t)(C.malloc(C.size_t(len(params)) * C.size_t(unsafe.Sizeof(empty_lower_params))))
		lower_params.len = C.size_t(len(params))
		for lower_params_i := range params {
			lower_params_ptr := (*C.fermyon_spin_postgres_parameter_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params.ptr)) +
				uintptr(lower_params_i)*unsafe.Sizeof(empty_lower_params)))
			var lower_params_ptr_value C.fermyon_spin_rdbms_types_parameter_value_t
			var lower_params_ptr_value_val C.fermyon_spin_rdbms_types_parameter_value_t
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBoolean {

				lower_params_ptr_value_val.tag = 0
				lower_params_ptr_value_val_ptr := (*bool)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := params[lower_params_i].GetBoolean()
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt8 {

				lower_params_ptr_value_val.tag = 1
				lower_params_ptr_value_val_ptr := (*C.int8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int8_t(params[lower_params_i].GetInt8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt16 {

				lower_params_ptr_value_val.tag = 2
				lower_params_ptr_value_val_ptr := (*C.int16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int16_t(params[lower_params_i].GetInt16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt32 {

				lower_params_ptr_value_val.tag = 3
				lower_params_ptr_value_val_ptr := (*C.int32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int32_t(params[lower_params_i].GetInt32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt64 {

				lower_params_ptr_value_val.tag = 4
				lower_params_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int64_t(params[lower_params_i].GetInt64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint8 {

				lower_params_ptr_value_val.tag = 5
				lower_params_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint8_t(params[lower_params_i].GetUint8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint16 {

				lower_params_ptr_value_val.tag = 6
				lower_params_ptr_value_val_ptr := (*C.uint16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint16_t(params[lower_params_i].GetUint16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint32 {

				lower_params_ptr_value_val.tag = 7
				lower_params_ptr_value_val_ptr := (*C.uint32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint32_t(params[lower_params_i].GetUint32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint64 {

				lower_params_ptr_value_val.tag = 8
				lower_params_ptr_value_val_ptr := (*C.uint64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint64_t(params[lower_params_i].GetUint64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating32 {

				lower_params_ptr_value_val.tag = 9
				lower_params_ptr_value_val_ptr := (*C.float)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.float(params[lower_params_i].GetFloating32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating64 {

				lower_params_ptr_value_val.tag = 10
				lower_params_ptr_value_val_ptr := (*C.double)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.double(params[lower_params_i].GetFloating64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindStr {

				lower_params_ptr_value_val.tag = 11
				lower_params_ptr_value_val_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_string_t

				lower_params_ptr_value_val_val.ptr = C.CString(params[lower_params_i].GetStr())
				lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetStr()))
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBinary {

				lower_params_ptr_value_val.tag = 12
				lower_params_ptr_value_val_ptr := (*C.http_trigger_list_u8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_list_u8_t
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
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindDbNull {
				lower_params_ptr_value_val.tag = 13
			}
			lower_params_ptr_value = lower_params_ptr_value_val
			*lower_params_ptr = lower_params_ptr_value
		}
	}
	defer C.http_trigger_list_parameter_value_free(&lower_params)
	var ret C.http_trigger_result_row_set_pg_error_t
	C.fermyon_spin_postgres_query(&lower_address, &lower_statement, &lower_params, &ret)
	var lift_ret Result[FermyonSpinRdbmsTypesRowSet, FermyonSpinPostgresPgError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_postgres_pg_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinPostgresPgError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinPostgresPgErrorSuccess()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorConnectionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorBadParameter(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorQueryFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorValueConversionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorOtherError(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_postgres_row_set_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRdbmsTypesRowSet
		var lift_ret_val_val FermyonSpinRdbmsTypesRowSet
		var lift_ret_val_val_Columns []FermyonSpinRdbmsTypesColumn
		lift_ret_val_val_Columns = make([]FermyonSpinRdbmsTypesColumn, lift_ret_ptr.columns.len)
		if lift_ret_ptr.columns.len > 0 {
			for lift_ret_val_val_Columns_i := 0; lift_ret_val_val_Columns_i < int(lift_ret_ptr.columns.len); lift_ret_val_val_Columns_i++ {
				var empty_lift_ret_val_val_Columns C.fermyon_spin_rdbms_types_column_t
				lift_ret_val_val_Columns_ptr := *(*C.fermyon_spin_rdbms_types_column_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.columns.ptr)) +
					uintptr(lift_ret_val_val_Columns_i)*unsafe.Sizeof(empty_lift_ret_val_val_Columns)))
				var list_lift_ret_val_val_Columns FermyonSpinRdbmsTypesColumn
				var list_lift_ret_val_val_Columns_Name string
				list_lift_ret_val_val_Columns_Name = C.GoStringN(lift_ret_val_val_Columns_ptr.name.ptr, C.int(lift_ret_val_val_Columns_ptr.name.len))
				list_lift_ret_val_val_Columns.Name = list_lift_ret_val_val_Columns_Name
				var list_lift_ret_val_val_Columns_DataType FermyonSpinRdbmsTypesDbDataType
				if lift_ret_val_val_Columns_ptr.data_type == 0 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeBoolean()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 1 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt8()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 2 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt16()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 3 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 4 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 5 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint8()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 6 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint16()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 7 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 8 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 9 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeFloating32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 10 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeFloating64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 11 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeStr()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 12 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeBinary()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 13 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeOther()
				}
				list_lift_ret_val_val_Columns.DataType = list_lift_ret_val_val_Columns_DataType
				lift_ret_val_val_Columns[lift_ret_val_val_Columns_i] = list_lift_ret_val_val_Columns
			}
		}
		lift_ret_val_val.Columns = lift_ret_val_val_Columns
		var lift_ret_val_val_Rows [][]FermyonSpinRdbmsTypesDbValue
		lift_ret_val_val_Rows = make([][]FermyonSpinRdbmsTypesDbValue, lift_ret_ptr.rows.len)
		if lift_ret_ptr.rows.len > 0 {
			for lift_ret_val_val_Rows_i := 0; lift_ret_val_val_Rows_i < int(lift_ret_ptr.rows.len); lift_ret_val_val_Rows_i++ {
				var empty_lift_ret_val_val_Rows C.fermyon_spin_rdbms_types_row_t
				lift_ret_val_val_Rows_ptr := *(*C.fermyon_spin_rdbms_types_row_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.rows.ptr)) +
					uintptr(lift_ret_val_val_Rows_i)*unsafe.Sizeof(empty_lift_ret_val_val_Rows)))
				var list_lift_ret_val_val_Rows []FermyonSpinRdbmsTypesDbValue
				list_lift_ret_val_val_Rows = make([]FermyonSpinRdbmsTypesDbValue, lift_ret_val_val_Rows_ptr.len)
				if lift_ret_val_val_Rows_ptr.len > 0 {
					for list_lift_ret_val_val_Rows_i := 0; list_lift_ret_val_val_Rows_i < int(lift_ret_val_val_Rows_ptr.len); list_lift_ret_val_val_Rows_i++ {
						var empty_list_lift_ret_val_val_Rows C.fermyon_spin_rdbms_types_db_value_t
						list_lift_ret_val_val_Rows_ptr := *(*C.fermyon_spin_rdbms_types_db_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_val_val_Rows_ptr.ptr)) +
							uintptr(list_lift_ret_val_val_Rows_i)*unsafe.Sizeof(empty_list_lift_ret_val_val_Rows)))
						var list_list_lift_ret_val_val_Rows FermyonSpinRdbmsTypesDbValue
						if list_lift_ret_val_val_Rows_ptr.tag == 0 {
							list_list_lift_ret_val_val_Rows_ptr := *(*bool)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							list_list_lift_ret_val_val_Rows_val := list_list_lift_ret_val_val_Rows_ptr
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueBoolean(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 1 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int8
							list_list_lift_ret_val_val_Rows_val = int8(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt8(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 2 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int16_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int16
							list_list_lift_ret_val_val_Rows_val = int16(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt16(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 3 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int32_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int32
							list_list_lift_ret_val_val_Rows_val = int32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 4 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int64_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int64
							list_list_lift_ret_val_val_Rows_val = int64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 5 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint8
							list_list_lift_ret_val_val_Rows_val = uint8(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint8(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 6 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint16_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint16
							list_list_lift_ret_val_val_Rows_val = uint16(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint16(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 7 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint32_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint32
							list_list_lift_ret_val_val_Rows_val = uint32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 8 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint64_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint64
							list_list_lift_ret_val_val_Rows_val = uint64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 9 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.float)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val float32
							list_list_lift_ret_val_val_Rows_val = float32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueFloating32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 10 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.double)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val float64
							list_list_lift_ret_val_val_Rows_val = float64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueFloating64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 11 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val string
							list_list_lift_ret_val_val_Rows_val = C.GoStringN(list_list_lift_ret_val_val_Rows_ptr.ptr, C.int(list_list_lift_ret_val_val_Rows_ptr.len))
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueStr(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 12 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.http_trigger_list_u8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val []uint8
							list_list_lift_ret_val_val_Rows_val = make([]uint8, list_list_lift_ret_val_val_Rows_ptr.len)
							if list_list_lift_ret_val_val_Rows_ptr.len > 0 {
								for list_list_lift_ret_val_val_Rows_val_i := 0; list_list_lift_ret_val_val_Rows_val_i < int(list_list_lift_ret_val_val_Rows_ptr.len); list_list_lift_ret_val_val_Rows_val_i++ {
									var empty_list_list_lift_ret_val_val_Rows_val C.uint8_t
									list_list_lift_ret_val_val_Rows_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(list_list_lift_ret_val_val_Rows_ptr.ptr)) +
										uintptr(list_list_lift_ret_val_val_Rows_val_i)*unsafe.Sizeof(empty_list_list_lift_ret_val_val_Rows_val)))
									var list_list_list_lift_ret_val_val_Rows_val uint8
									list_list_list_lift_ret_val_val_Rows_val = uint8(list_list_lift_ret_val_val_Rows_val_ptr)
									list_list_lift_ret_val_val_Rows_val[list_list_lift_ret_val_val_Rows_val_i] = list_list_list_lift_ret_val_val_Rows_val
								}
							}
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueBinary(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 13 {
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueDbNull()
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 14 {
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUnsupported()
						}
						list_lift_ret_val_val_Rows[list_lift_ret_val_val_Rows_i] = list_list_lift_ret_val_val_Rows
					}
				}
				lift_ret_val_val_Rows[lift_ret_val_val_Rows_i] = list_lift_ret_val_val_Rows
			}
		}
		lift_ret_val_val.Rows = lift_ret_val_val_Rows
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinPostgresExecute(address string, statement string, params []FermyonSpinRdbmsTypesParameterValue) Result[uint64, FermyonSpinPostgresPgError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_statement C.http_trigger_string_t

	lower_statement.ptr = C.CString(statement)
	lower_statement.len = C.size_t(len(statement))
	defer C.http_trigger_string_free(&lower_statement)
	var lower_params C.http_trigger_list_parameter_value_t
	if len(params) == 0 {
		lower_params.ptr = nil
		lower_params.len = 0
	} else {
		var empty_lower_params C.fermyon_spin_postgres_parameter_value_t
		lower_params.ptr = (*C.fermyon_spin_postgres_parameter_value_t)(C.malloc(C.size_t(len(params)) * C.size_t(unsafe.Sizeof(empty_lower_params))))
		lower_params.len = C.size_t(len(params))
		for lower_params_i := range params {
			lower_params_ptr := (*C.fermyon_spin_postgres_parameter_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params.ptr)) +
				uintptr(lower_params_i)*unsafe.Sizeof(empty_lower_params)))
			var lower_params_ptr_value C.fermyon_spin_rdbms_types_parameter_value_t
			var lower_params_ptr_value_val C.fermyon_spin_rdbms_types_parameter_value_t
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBoolean {

				lower_params_ptr_value_val.tag = 0
				lower_params_ptr_value_val_ptr := (*bool)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := params[lower_params_i].GetBoolean()
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt8 {

				lower_params_ptr_value_val.tag = 1
				lower_params_ptr_value_val_ptr := (*C.int8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int8_t(params[lower_params_i].GetInt8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt16 {

				lower_params_ptr_value_val.tag = 2
				lower_params_ptr_value_val_ptr := (*C.int16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int16_t(params[lower_params_i].GetInt16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt32 {

				lower_params_ptr_value_val.tag = 3
				lower_params_ptr_value_val_ptr := (*C.int32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int32_t(params[lower_params_i].GetInt32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt64 {

				lower_params_ptr_value_val.tag = 4
				lower_params_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int64_t(params[lower_params_i].GetInt64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint8 {

				lower_params_ptr_value_val.tag = 5
				lower_params_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint8_t(params[lower_params_i].GetUint8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint16 {

				lower_params_ptr_value_val.tag = 6
				lower_params_ptr_value_val_ptr := (*C.uint16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint16_t(params[lower_params_i].GetUint16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint32 {

				lower_params_ptr_value_val.tag = 7
				lower_params_ptr_value_val_ptr := (*C.uint32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint32_t(params[lower_params_i].GetUint32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint64 {

				lower_params_ptr_value_val.tag = 8
				lower_params_ptr_value_val_ptr := (*C.uint64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint64_t(params[lower_params_i].GetUint64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating32 {

				lower_params_ptr_value_val.tag = 9
				lower_params_ptr_value_val_ptr := (*C.float)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.float(params[lower_params_i].GetFloating32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating64 {

				lower_params_ptr_value_val.tag = 10
				lower_params_ptr_value_val_ptr := (*C.double)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.double(params[lower_params_i].GetFloating64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindStr {

				lower_params_ptr_value_val.tag = 11
				lower_params_ptr_value_val_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_string_t

				lower_params_ptr_value_val_val.ptr = C.CString(params[lower_params_i].GetStr())
				lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetStr()))
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBinary {

				lower_params_ptr_value_val.tag = 12
				lower_params_ptr_value_val_ptr := (*C.http_trigger_list_u8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_list_u8_t
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
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindDbNull {
				lower_params_ptr_value_val.tag = 13
			}
			lower_params_ptr_value = lower_params_ptr_value_val
			*lower_params_ptr = lower_params_ptr_value
		}
	}
	defer C.http_trigger_list_parameter_value_free(&lower_params)
	var ret C.http_trigger_result_u64_pg_error_t
	C.fermyon_spin_postgres_execute(&lower_address, &lower_statement, &lower_params, &ret)
	var lift_ret Result[uint64, FermyonSpinPostgresPgError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_postgres_pg_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinPostgresPgError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinPostgresPgErrorSuccess()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorConnectionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorBadParameter(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorQueryFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorValueConversionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinPostgresPgErrorOtherError(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.uint64_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val uint64
		lift_ret_val = uint64(lift_ret_ptr)
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

// Import functions from fermyon:spin/mysql
func FermyonSpinMysqlQuery(address string, statement string, params []FermyonSpinRdbmsTypesParameterValue) Result[FermyonSpinRdbmsTypesRowSet, FermyonSpinMysqlMysqlError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_statement C.http_trigger_string_t

	lower_statement.ptr = C.CString(statement)
	lower_statement.len = C.size_t(len(statement))
	defer C.http_trigger_string_free(&lower_statement)
	var lower_params C.http_trigger_list_parameter_value_t
	if len(params) == 0 {
		lower_params.ptr = nil
		lower_params.len = 0
	} else {
		var empty_lower_params C.fermyon_spin_mysql_parameter_value_t
		lower_params.ptr = (*C.fermyon_spin_mysql_parameter_value_t)(C.malloc(C.size_t(len(params)) * C.size_t(unsafe.Sizeof(empty_lower_params))))
		lower_params.len = C.size_t(len(params))
		for lower_params_i := range params {
			lower_params_ptr := (*C.fermyon_spin_mysql_parameter_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params.ptr)) +
				uintptr(lower_params_i)*unsafe.Sizeof(empty_lower_params)))
			var lower_params_ptr_value C.fermyon_spin_rdbms_types_parameter_value_t
			var lower_params_ptr_value_val C.fermyon_spin_rdbms_types_parameter_value_t
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBoolean {

				lower_params_ptr_value_val.tag = 0
				lower_params_ptr_value_val_ptr := (*bool)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := params[lower_params_i].GetBoolean()
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt8 {

				lower_params_ptr_value_val.tag = 1
				lower_params_ptr_value_val_ptr := (*C.int8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int8_t(params[lower_params_i].GetInt8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt16 {

				lower_params_ptr_value_val.tag = 2
				lower_params_ptr_value_val_ptr := (*C.int16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int16_t(params[lower_params_i].GetInt16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt32 {

				lower_params_ptr_value_val.tag = 3
				lower_params_ptr_value_val_ptr := (*C.int32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int32_t(params[lower_params_i].GetInt32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt64 {

				lower_params_ptr_value_val.tag = 4
				lower_params_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int64_t(params[lower_params_i].GetInt64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint8 {

				lower_params_ptr_value_val.tag = 5
				lower_params_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint8_t(params[lower_params_i].GetUint8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint16 {

				lower_params_ptr_value_val.tag = 6
				lower_params_ptr_value_val_ptr := (*C.uint16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint16_t(params[lower_params_i].GetUint16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint32 {

				lower_params_ptr_value_val.tag = 7
				lower_params_ptr_value_val_ptr := (*C.uint32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint32_t(params[lower_params_i].GetUint32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint64 {

				lower_params_ptr_value_val.tag = 8
				lower_params_ptr_value_val_ptr := (*C.uint64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint64_t(params[lower_params_i].GetUint64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating32 {

				lower_params_ptr_value_val.tag = 9
				lower_params_ptr_value_val_ptr := (*C.float)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.float(params[lower_params_i].GetFloating32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating64 {

				lower_params_ptr_value_val.tag = 10
				lower_params_ptr_value_val_ptr := (*C.double)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.double(params[lower_params_i].GetFloating64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindStr {

				lower_params_ptr_value_val.tag = 11
				lower_params_ptr_value_val_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_string_t

				lower_params_ptr_value_val_val.ptr = C.CString(params[lower_params_i].GetStr())
				lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetStr()))
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBinary {

				lower_params_ptr_value_val.tag = 12
				lower_params_ptr_value_val_ptr := (*C.http_trigger_list_u8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_list_u8_t
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
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindDbNull {
				lower_params_ptr_value_val.tag = 13
			}
			lower_params_ptr_value = lower_params_ptr_value_val
			*lower_params_ptr = lower_params_ptr_value
		}
	}
	defer C.http_trigger_list_parameter_value_free(&lower_params)
	var ret C.http_trigger_result_row_set_mysql_error_t
	C.fermyon_spin_mysql_query(&lower_address, &lower_statement, &lower_params, &ret)
	var lift_ret Result[FermyonSpinRdbmsTypesRowSet, FermyonSpinMysqlMysqlError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_mysql_mysql_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinMysqlMysqlError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinMysqlMysqlErrorSuccess()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorConnectionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorBadParameter(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorQueryFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorValueConversionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorOtherError(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_mysql_row_set_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRdbmsTypesRowSet
		var lift_ret_val_val FermyonSpinRdbmsTypesRowSet
		var lift_ret_val_val_Columns []FermyonSpinRdbmsTypesColumn
		lift_ret_val_val_Columns = make([]FermyonSpinRdbmsTypesColumn, lift_ret_ptr.columns.len)
		if lift_ret_ptr.columns.len > 0 {
			for lift_ret_val_val_Columns_i := 0; lift_ret_val_val_Columns_i < int(lift_ret_ptr.columns.len); lift_ret_val_val_Columns_i++ {
				var empty_lift_ret_val_val_Columns C.fermyon_spin_rdbms_types_column_t
				lift_ret_val_val_Columns_ptr := *(*C.fermyon_spin_rdbms_types_column_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.columns.ptr)) +
					uintptr(lift_ret_val_val_Columns_i)*unsafe.Sizeof(empty_lift_ret_val_val_Columns)))
				var list_lift_ret_val_val_Columns FermyonSpinRdbmsTypesColumn
				var list_lift_ret_val_val_Columns_Name string
				list_lift_ret_val_val_Columns_Name = C.GoStringN(lift_ret_val_val_Columns_ptr.name.ptr, C.int(lift_ret_val_val_Columns_ptr.name.len))
				list_lift_ret_val_val_Columns.Name = list_lift_ret_val_val_Columns_Name
				var list_lift_ret_val_val_Columns_DataType FermyonSpinRdbmsTypesDbDataType
				if lift_ret_val_val_Columns_ptr.data_type == 0 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeBoolean()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 1 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt8()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 2 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt16()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 3 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 4 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeInt64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 5 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint8()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 6 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint16()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 7 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 8 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeUint64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 9 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeFloating32()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 10 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeFloating64()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 11 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeStr()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 12 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeBinary()
				}
				if lift_ret_val_val_Columns_ptr.data_type == 13 {
					list_lift_ret_val_val_Columns_DataType = FermyonSpinRdbmsTypesDbDataTypeOther()
				}
				list_lift_ret_val_val_Columns.DataType = list_lift_ret_val_val_Columns_DataType
				lift_ret_val_val_Columns[lift_ret_val_val_Columns_i] = list_lift_ret_val_val_Columns
			}
		}
		lift_ret_val_val.Columns = lift_ret_val_val_Columns
		var lift_ret_val_val_Rows [][]FermyonSpinRdbmsTypesDbValue
		lift_ret_val_val_Rows = make([][]FermyonSpinRdbmsTypesDbValue, lift_ret_ptr.rows.len)
		if lift_ret_ptr.rows.len > 0 {
			for lift_ret_val_val_Rows_i := 0; lift_ret_val_val_Rows_i < int(lift_ret_ptr.rows.len); lift_ret_val_val_Rows_i++ {
				var empty_lift_ret_val_val_Rows C.fermyon_spin_rdbms_types_row_t
				lift_ret_val_val_Rows_ptr := *(*C.fermyon_spin_rdbms_types_row_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.rows.ptr)) +
					uintptr(lift_ret_val_val_Rows_i)*unsafe.Sizeof(empty_lift_ret_val_val_Rows)))
				var list_lift_ret_val_val_Rows []FermyonSpinRdbmsTypesDbValue
				list_lift_ret_val_val_Rows = make([]FermyonSpinRdbmsTypesDbValue, lift_ret_val_val_Rows_ptr.len)
				if lift_ret_val_val_Rows_ptr.len > 0 {
					for list_lift_ret_val_val_Rows_i := 0; list_lift_ret_val_val_Rows_i < int(lift_ret_val_val_Rows_ptr.len); list_lift_ret_val_val_Rows_i++ {
						var empty_list_lift_ret_val_val_Rows C.fermyon_spin_rdbms_types_db_value_t
						list_lift_ret_val_val_Rows_ptr := *(*C.fermyon_spin_rdbms_types_db_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_val_val_Rows_ptr.ptr)) +
							uintptr(list_lift_ret_val_val_Rows_i)*unsafe.Sizeof(empty_list_lift_ret_val_val_Rows)))
						var list_list_lift_ret_val_val_Rows FermyonSpinRdbmsTypesDbValue
						if list_lift_ret_val_val_Rows_ptr.tag == 0 {
							list_list_lift_ret_val_val_Rows_ptr := *(*bool)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							list_list_lift_ret_val_val_Rows_val := list_list_lift_ret_val_val_Rows_ptr
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueBoolean(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 1 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int8
							list_list_lift_ret_val_val_Rows_val = int8(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt8(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 2 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int16_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int16
							list_list_lift_ret_val_val_Rows_val = int16(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt16(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 3 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int32_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int32
							list_list_lift_ret_val_val_Rows_val = int32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 4 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.int64_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val int64
							list_list_lift_ret_val_val_Rows_val = int64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueInt64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 5 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint8
							list_list_lift_ret_val_val_Rows_val = uint8(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint8(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 6 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint16_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint16
							list_list_lift_ret_val_val_Rows_val = uint16(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint16(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 7 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint32_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint32
							list_list_lift_ret_val_val_Rows_val = uint32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 8 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.uint64_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val uint64
							list_list_lift_ret_val_val_Rows_val = uint64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUint64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 9 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.float)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val float32
							list_list_lift_ret_val_val_Rows_val = float32(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueFloating32(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 10 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.double)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val float64
							list_list_lift_ret_val_val_Rows_val = float64(list_list_lift_ret_val_val_Rows_ptr)
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueFloating64(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 11 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val string
							list_list_lift_ret_val_val_Rows_val = C.GoStringN(list_list_lift_ret_val_val_Rows_ptr.ptr, C.int(list_list_lift_ret_val_val_Rows_ptr.len))
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueStr(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 12 {
							list_list_lift_ret_val_val_Rows_ptr := *(*C.http_trigger_list_u8_t)(unsafe.Pointer(&list_lift_ret_val_val_Rows_ptr.val))
							var list_list_lift_ret_val_val_Rows_val []uint8
							list_list_lift_ret_val_val_Rows_val = make([]uint8, list_list_lift_ret_val_val_Rows_ptr.len)
							if list_list_lift_ret_val_val_Rows_ptr.len > 0 {
								for list_list_lift_ret_val_val_Rows_val_i := 0; list_list_lift_ret_val_val_Rows_val_i < int(list_list_lift_ret_val_val_Rows_ptr.len); list_list_lift_ret_val_val_Rows_val_i++ {
									var empty_list_list_lift_ret_val_val_Rows_val C.uint8_t
									list_list_lift_ret_val_val_Rows_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(list_list_lift_ret_val_val_Rows_ptr.ptr)) +
										uintptr(list_list_lift_ret_val_val_Rows_val_i)*unsafe.Sizeof(empty_list_list_lift_ret_val_val_Rows_val)))
									var list_list_list_lift_ret_val_val_Rows_val uint8
									list_list_list_lift_ret_val_val_Rows_val = uint8(list_list_lift_ret_val_val_Rows_val_ptr)
									list_list_lift_ret_val_val_Rows_val[list_list_lift_ret_val_val_Rows_val_i] = list_list_list_lift_ret_val_val_Rows_val
								}
							}
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueBinary(list_list_lift_ret_val_val_Rows_val)
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 13 {
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueDbNull()
						}
						if list_lift_ret_val_val_Rows_ptr.tag == 14 {
							list_list_lift_ret_val_val_Rows = FermyonSpinRdbmsTypesDbValueUnsupported()
						}
						list_lift_ret_val_val_Rows[list_lift_ret_val_val_Rows_i] = list_list_lift_ret_val_val_Rows
					}
				}
				lift_ret_val_val_Rows[lift_ret_val_val_Rows_i] = list_lift_ret_val_val_Rows
			}
		}
		lift_ret_val_val.Rows = lift_ret_val_val_Rows
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinMysqlExecute(address string, statement string, params []FermyonSpinRdbmsTypesParameterValue) Result[struct{}, FermyonSpinMysqlMysqlError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_statement C.http_trigger_string_t

	lower_statement.ptr = C.CString(statement)
	lower_statement.len = C.size_t(len(statement))
	defer C.http_trigger_string_free(&lower_statement)
	var lower_params C.http_trigger_list_parameter_value_t
	if len(params) == 0 {
		lower_params.ptr = nil
		lower_params.len = 0
	} else {
		var empty_lower_params C.fermyon_spin_mysql_parameter_value_t
		lower_params.ptr = (*C.fermyon_spin_mysql_parameter_value_t)(C.malloc(C.size_t(len(params)) * C.size_t(unsafe.Sizeof(empty_lower_params))))
		lower_params.len = C.size_t(len(params))
		for lower_params_i := range params {
			lower_params_ptr := (*C.fermyon_spin_mysql_parameter_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_params.ptr)) +
				uintptr(lower_params_i)*unsafe.Sizeof(empty_lower_params)))
			var lower_params_ptr_value C.fermyon_spin_rdbms_types_parameter_value_t
			var lower_params_ptr_value_val C.fermyon_spin_rdbms_types_parameter_value_t
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBoolean {

				lower_params_ptr_value_val.tag = 0
				lower_params_ptr_value_val_ptr := (*bool)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := params[lower_params_i].GetBoolean()
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt8 {

				lower_params_ptr_value_val.tag = 1
				lower_params_ptr_value_val_ptr := (*C.int8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int8_t(params[lower_params_i].GetInt8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt16 {

				lower_params_ptr_value_val.tag = 2
				lower_params_ptr_value_val_ptr := (*C.int16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int16_t(params[lower_params_i].GetInt16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt32 {

				lower_params_ptr_value_val.tag = 3
				lower_params_ptr_value_val_ptr := (*C.int32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int32_t(params[lower_params_i].GetInt32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindInt64 {

				lower_params_ptr_value_val.tag = 4
				lower_params_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.int64_t(params[lower_params_i].GetInt64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint8 {

				lower_params_ptr_value_val.tag = 5
				lower_params_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint8_t(params[lower_params_i].GetUint8())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint16 {

				lower_params_ptr_value_val.tag = 6
				lower_params_ptr_value_val_ptr := (*C.uint16_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint16_t(params[lower_params_i].GetUint16())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint32 {

				lower_params_ptr_value_val.tag = 7
				lower_params_ptr_value_val_ptr := (*C.uint32_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint32_t(params[lower_params_i].GetUint32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindUint64 {

				lower_params_ptr_value_val.tag = 8
				lower_params_ptr_value_val_ptr := (*C.uint64_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.uint64_t(params[lower_params_i].GetUint64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating32 {

				lower_params_ptr_value_val.tag = 9
				lower_params_ptr_value_val_ptr := (*C.float)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.float(params[lower_params_i].GetFloating32())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindFloating64 {

				lower_params_ptr_value_val.tag = 10
				lower_params_ptr_value_val_ptr := (*C.double)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				lower_params_ptr_value_val_val := C.double(params[lower_params_i].GetFloating64())
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindStr {

				lower_params_ptr_value_val.tag = 11
				lower_params_ptr_value_val_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_string_t

				lower_params_ptr_value_val_val.ptr = C.CString(params[lower_params_i].GetStr())
				lower_params_ptr_value_val_val.len = C.size_t(len(params[lower_params_i].GetStr()))
				*lower_params_ptr_value_val_ptr = lower_params_ptr_value_val_val
			}
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindBinary {

				lower_params_ptr_value_val.tag = 12
				lower_params_ptr_value_val_ptr := (*C.http_trigger_list_u8_t)(unsafe.Pointer(&lower_params_ptr_value_val.val))
				var lower_params_ptr_value_val_val C.http_trigger_list_u8_t
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
			if params[lower_params_i].Kind() == FermyonSpinRdbmsTypesParameterValueKindDbNull {
				lower_params_ptr_value_val.tag = 13
			}
			lower_params_ptr_value = lower_params_ptr_value_val
			*lower_params_ptr = lower_params_ptr_value
		}
	}
	defer C.http_trigger_list_parameter_value_free(&lower_params)
	var ret C.http_trigger_result_void_mysql_error_t
	C.fermyon_spin_mysql_execute(&lower_address, &lower_statement, &lower_params, &ret)
	var lift_ret Result[struct{}, FermyonSpinMysqlMysqlError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_mysql_mysql_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinMysqlMysqlError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinMysqlMysqlErrorSuccess()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorConnectionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorBadParameter(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorQueryFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorValueConversionFailed(lift_ret_val_val)
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinMysqlMysqlErrorOtherError(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
	}
	return lift_ret
}

// Import functions from fermyon:spin/sqlite
func FermyonSpinSqliteOpen(database string) Result[uint32, FermyonSpinSqliteError] {
	var lower_database C.http_trigger_string_t

	lower_database.ptr = C.CString(database)
	lower_database.len = C.size_t(len(database))
	defer C.http_trigger_string_free(&lower_database)
	var ret C.http_trigger_result_connection_error_t
	C.fermyon_spin_sqlite_open(&lower_database, &ret)
	var lift_ret Result[uint32, FermyonSpinSqliteError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_sqlite_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinSqliteError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinSqliteErrorNoSuchDatabase()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinSqliteErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinSqliteErrorInvalidConnection()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinSqliteErrorDatabaseFull()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinSqliteErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_sqlite_connection_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val uint32
		var lift_ret_val_val uint32
		lift_ret_val_val = uint32(lift_ret_ptr)
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinSqliteExecute(conn uint32, statement string, parameters []FermyonSpinSqliteValue) Result[FermyonSpinSqliteQueryResult, FermyonSpinSqliteError] {
	var lower_conn C.uint32_t
	lower_conn_val := C.uint32_t(conn)
	lower_conn = lower_conn_val
	var lower_statement C.http_trigger_string_t

	lower_statement.ptr = C.CString(statement)
	lower_statement.len = C.size_t(len(statement))
	defer C.http_trigger_string_free(&lower_statement)
	var lower_parameters C.http_trigger_list_value_t
	if len(parameters) == 0 {
		lower_parameters.ptr = nil
		lower_parameters.len = 0
	} else {
		var empty_lower_parameters C.fermyon_spin_sqlite_value_t
		lower_parameters.ptr = (*C.fermyon_spin_sqlite_value_t)(C.malloc(C.size_t(len(parameters)) * C.size_t(unsafe.Sizeof(empty_lower_parameters))))
		lower_parameters.len = C.size_t(len(parameters))
		for lower_parameters_i := range parameters {
			lower_parameters_ptr := (*C.fermyon_spin_sqlite_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_parameters.ptr)) +
				uintptr(lower_parameters_i)*unsafe.Sizeof(empty_lower_parameters)))
			var lower_parameters_ptr_value C.fermyon_spin_sqlite_value_t
			if parameters[lower_parameters_i].Kind() == FermyonSpinSqliteValueKindInteger {

				lower_parameters_ptr_value.tag = 0
				lower_parameters_ptr_value_ptr := (*C.int64_t)(unsafe.Pointer(&lower_parameters_ptr_value.val))
				lower_parameters_ptr_value_val := C.int64_t(parameters[lower_parameters_i].GetInteger())
				*lower_parameters_ptr_value_ptr = lower_parameters_ptr_value_val
			}
			if parameters[lower_parameters_i].Kind() == FermyonSpinSqliteValueKindReal {

				lower_parameters_ptr_value.tag = 1
				lower_parameters_ptr_value_ptr := (*C.double)(unsafe.Pointer(&lower_parameters_ptr_value.val))
				lower_parameters_ptr_value_val := C.double(parameters[lower_parameters_i].GetReal())
				*lower_parameters_ptr_value_ptr = lower_parameters_ptr_value_val
			}
			if parameters[lower_parameters_i].Kind() == FermyonSpinSqliteValueKindText {

				lower_parameters_ptr_value.tag = 2
				lower_parameters_ptr_value_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(&lower_parameters_ptr_value.val))
				var lower_parameters_ptr_value_val C.http_trigger_string_t

				lower_parameters_ptr_value_val.ptr = C.CString(parameters[lower_parameters_i].GetText())
				lower_parameters_ptr_value_val.len = C.size_t(len(parameters[lower_parameters_i].GetText()))
				*lower_parameters_ptr_value_ptr = lower_parameters_ptr_value_val
			}
			if parameters[lower_parameters_i].Kind() == FermyonSpinSqliteValueKindBlob {

				lower_parameters_ptr_value.tag = 3
				lower_parameters_ptr_value_ptr := (*C.http_trigger_list_u8_t)(unsafe.Pointer(&lower_parameters_ptr_value.val))
				var lower_parameters_ptr_value_val C.http_trigger_list_u8_t
				if len(parameters[lower_parameters_i].GetBlob()) == 0 {
					lower_parameters_ptr_value_val.ptr = nil
					lower_parameters_ptr_value_val.len = 0
				} else {
					var empty_lower_parameters_ptr_value_val C.uint8_t
					lower_parameters_ptr_value_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(parameters[lower_parameters_i].GetBlob())) * C.size_t(unsafe.Sizeof(empty_lower_parameters_ptr_value_val))))
					lower_parameters_ptr_value_val.len = C.size_t(len(parameters[lower_parameters_i].GetBlob()))
					for lower_parameters_ptr_value_val_i := range parameters[lower_parameters_i].GetBlob() {
						lower_parameters_ptr_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_parameters_ptr_value_val.ptr)) +
							uintptr(lower_parameters_ptr_value_val_i)*unsafe.Sizeof(empty_lower_parameters_ptr_value_val)))
						lower_parameters_ptr_value_val_ptr_value := C.uint8_t(parameters[lower_parameters_i].GetBlob()[lower_parameters_ptr_value_val_i])
						*lower_parameters_ptr_value_val_ptr = lower_parameters_ptr_value_val_ptr_value
					}
				}
				*lower_parameters_ptr_value_ptr = lower_parameters_ptr_value_val
			}
			if parameters[lower_parameters_i].Kind() == FermyonSpinSqliteValueKindNull {
				lower_parameters_ptr_value.tag = 4
			}
			*lower_parameters_ptr = lower_parameters_ptr_value
		}
	}
	defer C.http_trigger_list_value_free(&lower_parameters)
	var ret C.http_trigger_result_query_result_error_t
	C.fermyon_spin_sqlite_execute(lower_conn, &lower_statement, &lower_parameters, &ret)
	var lift_ret Result[FermyonSpinSqliteQueryResult, FermyonSpinSqliteError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_sqlite_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinSqliteError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinSqliteErrorNoSuchDatabase()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinSqliteErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinSqliteErrorInvalidConnection()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinSqliteErrorDatabaseFull()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinSqliteErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_sqlite_query_result_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinSqliteQueryResult
		var lift_ret_val_Columns []string
		lift_ret_val_Columns = make([]string, lift_ret_ptr.columns.len)
		if lift_ret_ptr.columns.len > 0 {
			for lift_ret_val_Columns_i := 0; lift_ret_val_Columns_i < int(lift_ret_ptr.columns.len); lift_ret_val_Columns_i++ {
				var empty_lift_ret_val_Columns C.http_trigger_string_t
				lift_ret_val_Columns_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.columns.ptr)) +
					uintptr(lift_ret_val_Columns_i)*unsafe.Sizeof(empty_lift_ret_val_Columns)))
				var list_lift_ret_val_Columns string
				list_lift_ret_val_Columns = C.GoStringN(lift_ret_val_Columns_ptr.ptr, C.int(lift_ret_val_Columns_ptr.len))
				lift_ret_val_Columns[lift_ret_val_Columns_i] = list_lift_ret_val_Columns
			}
		}
		lift_ret_val.Columns = lift_ret_val_Columns
		var lift_ret_val_Rows []FermyonSpinSqliteRowResult
		lift_ret_val_Rows = make([]FermyonSpinSqliteRowResult, lift_ret_ptr.rows.len)
		if lift_ret_ptr.rows.len > 0 {
			for lift_ret_val_Rows_i := 0; lift_ret_val_Rows_i < int(lift_ret_ptr.rows.len); lift_ret_val_Rows_i++ {
				var empty_lift_ret_val_Rows C.fermyon_spin_sqlite_row_result_t
				lift_ret_val_Rows_ptr := *(*C.fermyon_spin_sqlite_row_result_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.rows.ptr)) +
					uintptr(lift_ret_val_Rows_i)*unsafe.Sizeof(empty_lift_ret_val_Rows)))
				var list_lift_ret_val_Rows FermyonSpinSqliteRowResult
				var list_lift_ret_val_Rows_Values []FermyonSpinSqliteValue
				list_lift_ret_val_Rows_Values = make([]FermyonSpinSqliteValue, lift_ret_val_Rows_ptr.values.len)
				if lift_ret_val_Rows_ptr.values.len > 0 {
					for list_lift_ret_val_Rows_Values_i := 0; list_lift_ret_val_Rows_Values_i < int(lift_ret_val_Rows_ptr.values.len); list_lift_ret_val_Rows_Values_i++ {
						var empty_list_lift_ret_val_Rows_Values C.fermyon_spin_sqlite_value_t
						list_lift_ret_val_Rows_Values_ptr := *(*C.fermyon_spin_sqlite_value_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_val_Rows_ptr.values.ptr)) +
							uintptr(list_lift_ret_val_Rows_Values_i)*unsafe.Sizeof(empty_list_lift_ret_val_Rows_Values)))
						var list_list_lift_ret_val_Rows_Values FermyonSpinSqliteValue
						if list_lift_ret_val_Rows_Values_ptr.tag == 0 {
							list_list_lift_ret_val_Rows_Values_ptr := *(*C.int64_t)(unsafe.Pointer(&list_lift_ret_val_Rows_Values_ptr.val))
							var list_list_lift_ret_val_Rows_Values_val int64
							list_list_lift_ret_val_Rows_Values_val = int64(list_list_lift_ret_val_Rows_Values_ptr)
							list_list_lift_ret_val_Rows_Values = FermyonSpinSqliteValueInteger(list_list_lift_ret_val_Rows_Values_val)
						}
						if list_lift_ret_val_Rows_Values_ptr.tag == 1 {
							list_list_lift_ret_val_Rows_Values_ptr := *(*C.double)(unsafe.Pointer(&list_lift_ret_val_Rows_Values_ptr.val))
							var list_list_lift_ret_val_Rows_Values_val float64
							list_list_lift_ret_val_Rows_Values_val = float64(list_list_lift_ret_val_Rows_Values_ptr)
							list_list_lift_ret_val_Rows_Values = FermyonSpinSqliteValueReal(list_list_lift_ret_val_Rows_Values_val)
						}
						if list_lift_ret_val_Rows_Values_ptr.tag == 2 {
							list_list_lift_ret_val_Rows_Values_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&list_lift_ret_val_Rows_Values_ptr.val))
							var list_list_lift_ret_val_Rows_Values_val string
							list_list_lift_ret_val_Rows_Values_val = C.GoStringN(list_list_lift_ret_val_Rows_Values_ptr.ptr, C.int(list_list_lift_ret_val_Rows_Values_ptr.len))
							list_list_lift_ret_val_Rows_Values = FermyonSpinSqliteValueText(list_list_lift_ret_val_Rows_Values_val)
						}
						if list_lift_ret_val_Rows_Values_ptr.tag == 3 {
							list_list_lift_ret_val_Rows_Values_ptr := *(*C.http_trigger_list_u8_t)(unsafe.Pointer(&list_lift_ret_val_Rows_Values_ptr.val))
							var list_list_lift_ret_val_Rows_Values_val []uint8
							list_list_lift_ret_val_Rows_Values_val = make([]uint8, list_list_lift_ret_val_Rows_Values_ptr.len)
							if list_list_lift_ret_val_Rows_Values_ptr.len > 0 {
								for list_list_lift_ret_val_Rows_Values_val_i := 0; list_list_lift_ret_val_Rows_Values_val_i < int(list_list_lift_ret_val_Rows_Values_ptr.len); list_list_lift_ret_val_Rows_Values_val_i++ {
									var empty_list_list_lift_ret_val_Rows_Values_val C.uint8_t
									list_list_lift_ret_val_Rows_Values_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(list_list_lift_ret_val_Rows_Values_ptr.ptr)) +
										uintptr(list_list_lift_ret_val_Rows_Values_val_i)*unsafe.Sizeof(empty_list_list_lift_ret_val_Rows_Values_val)))
									var list_list_list_lift_ret_val_Rows_Values_val uint8
									list_list_list_lift_ret_val_Rows_Values_val = uint8(list_list_lift_ret_val_Rows_Values_val_ptr)
									list_list_lift_ret_val_Rows_Values_val[list_list_lift_ret_val_Rows_Values_val_i] = list_list_list_lift_ret_val_Rows_Values_val
								}
							}
							list_list_lift_ret_val_Rows_Values = FermyonSpinSqliteValueBlob(list_list_lift_ret_val_Rows_Values_val)
						}
						if list_lift_ret_val_Rows_Values_ptr.tag == 4 {
							list_list_lift_ret_val_Rows_Values = FermyonSpinSqliteValueNull()
						}
						list_lift_ret_val_Rows_Values[list_lift_ret_val_Rows_Values_i] = list_list_lift_ret_val_Rows_Values
					}
				}
				list_lift_ret_val_Rows.Values = list_lift_ret_val_Rows_Values
				lift_ret_val_Rows[lift_ret_val_Rows_i] = list_lift_ret_val_Rows
			}
		}
		lift_ret_val.Rows = lift_ret_val_Rows
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinSqliteClose(conn uint32) {
	var lower_conn C.uint32_t
	lower_conn_val := C.uint32_t(conn)
	lower_conn = lower_conn_val
	C.fermyon_spin_sqlite_close(lower_conn)
}

// Import functions from fermyon:spin/redis-types
// Import functions from fermyon:spin/redis
func FermyonSpinRedisPublish(address string, channel string, payload []uint8) Result[struct{}, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_channel C.http_trigger_string_t

	lower_channel.ptr = C.CString(channel)
	lower_channel.len = C.size_t(len(channel))
	defer C.http_trigger_string_free(&lower_channel)
	var lower_payload C.fermyon_spin_redis_types_payload_t
	var lower_payload_val C.fermyon_spin_redis_types_payload_t
	if len(payload) == 0 {
		lower_payload_val.ptr = nil
		lower_payload_val.len = 0
	} else {
		var empty_lower_payload_val C.uint8_t
		lower_payload_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(payload)) * C.size_t(unsafe.Sizeof(empty_lower_payload_val))))
		lower_payload_val.len = C.size_t(len(payload))
		for lower_payload_val_i := range payload {
			lower_payload_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_payload_val.ptr)) +
				uintptr(lower_payload_val_i)*unsafe.Sizeof(empty_lower_payload_val)))
			lower_payload_val_ptr_value := C.uint8_t(payload[lower_payload_val_i])
			*lower_payload_val_ptr = lower_payload_val_ptr_value
		}
	}
	lower_payload = lower_payload_val
	defer C.fermyon_spin_redis_payload_free(&lower_payload)
	var ret C.http_trigger_result_void_error_t
	C.fermyon_spin_redis_publish(&lower_address, &lower_channel, &lower_payload, &ret)
	var lift_ret Result[struct{}, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
	}
	return lift_ret
}

func FermyonSpinRedisGet(address string, key string) Result[[]uint8, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_payload_error_t
	C.fermyon_spin_redis_get(&lower_address, &lower_key, &ret)
	var lift_ret Result[[]uint8, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_redis_payload_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val []uint8
		var lift_ret_val_val []uint8
		lift_ret_val_val = make([]uint8, lift_ret_ptr.len)
		if lift_ret_ptr.len > 0 {
			for lift_ret_val_val_i := 0; lift_ret_val_val_i < int(lift_ret_ptr.len); lift_ret_val_val_i++ {
				var empty_lift_ret_val_val C.uint8_t
				lift_ret_val_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.ptr)) +
					uintptr(lift_ret_val_val_i)*unsafe.Sizeof(empty_lift_ret_val_val)))
				var list_lift_ret_val_val uint8
				list_lift_ret_val_val = uint8(lift_ret_val_val_ptr)
				lift_ret_val_val[lift_ret_val_val_i] = list_lift_ret_val_val
			}
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisSet(address string, key string, value []uint8) Result[struct{}, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var lower_value C.fermyon_spin_redis_types_payload_t
	var lower_value_val C.fermyon_spin_redis_types_payload_t
	if len(value) == 0 {
		lower_value_val.ptr = nil
		lower_value_val.len = 0
	} else {
		var empty_lower_value_val C.uint8_t
		lower_value_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(value)) * C.size_t(unsafe.Sizeof(empty_lower_value_val))))
		lower_value_val.len = C.size_t(len(value))
		for lower_value_val_i := range value {
			lower_value_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_value_val.ptr)) +
				uintptr(lower_value_val_i)*unsafe.Sizeof(empty_lower_value_val)))
			lower_value_val_ptr_value := C.uint8_t(value[lower_value_val_i])
			*lower_value_val_ptr = lower_value_val_ptr_value
		}
	}
	lower_value = lower_value_val
	defer C.fermyon_spin_redis_payload_free(&lower_value)
	var ret C.http_trigger_result_void_error_t
	C.fermyon_spin_redis_set(&lower_address, &lower_key, &lower_value, &ret)
	var lift_ret Result[struct{}, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
	}
	return lift_ret
}

func FermyonSpinRedisIncr(address string, key string) Result[int64, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_s64_error_t
	C.fermyon_spin_redis_incr(&lower_address, &lower_key, &ret)
	var lift_ret Result[int64, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.int64_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val int64
		lift_ret_val = int64(lift_ret_ptr)
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisDel(address string, keys []string) Result[int64, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_keys C.http_trigger_list_string_t
	if len(keys) == 0 {
		lower_keys.ptr = nil
		lower_keys.len = 0
	} else {
		var empty_lower_keys C.http_trigger_string_t
		lower_keys.ptr = (*C.http_trigger_string_t)(C.malloc(C.size_t(len(keys)) * C.size_t(unsafe.Sizeof(empty_lower_keys))))
		lower_keys.len = C.size_t(len(keys))
		for lower_keys_i := range keys {
			lower_keys_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_keys.ptr)) +
				uintptr(lower_keys_i)*unsafe.Sizeof(empty_lower_keys)))
			var lower_keys_ptr_value C.http_trigger_string_t

			lower_keys_ptr_value.ptr = C.CString(keys[lower_keys_i])
			lower_keys_ptr_value.len = C.size_t(len(keys[lower_keys_i]))
			*lower_keys_ptr = lower_keys_ptr_value
		}
	}
	defer C.http_trigger_list_string_free(&lower_keys)
	var ret C.http_trigger_result_s64_error_t
	C.fermyon_spin_redis_del(&lower_address, &lower_keys, &ret)
	var lift_ret Result[int64, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.int64_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val int64
		lift_ret_val = int64(lift_ret_ptr)
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisSadd(address string, key string, values []string) Result[int64, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var lower_values C.http_trigger_list_string_t
	if len(values) == 0 {
		lower_values.ptr = nil
		lower_values.len = 0
	} else {
		var empty_lower_values C.http_trigger_string_t
		lower_values.ptr = (*C.http_trigger_string_t)(C.malloc(C.size_t(len(values)) * C.size_t(unsafe.Sizeof(empty_lower_values))))
		lower_values.len = C.size_t(len(values))
		for lower_values_i := range values {
			lower_values_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_values.ptr)) +
				uintptr(lower_values_i)*unsafe.Sizeof(empty_lower_values)))
			var lower_values_ptr_value C.http_trigger_string_t

			lower_values_ptr_value.ptr = C.CString(values[lower_values_i])
			lower_values_ptr_value.len = C.size_t(len(values[lower_values_i]))
			*lower_values_ptr = lower_values_ptr_value
		}
	}
	defer C.http_trigger_list_string_free(&lower_values)
	var ret C.http_trigger_result_s64_error_t
	C.fermyon_spin_redis_sadd(&lower_address, &lower_key, &lower_values, &ret)
	var lift_ret Result[int64, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.int64_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val int64
		lift_ret_val = int64(lift_ret_ptr)
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisSmembers(address string, key string) Result[[]string, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_list_string_error_t
	C.fermyon_spin_redis_smembers(&lower_address, &lower_key, &ret)
	var lift_ret Result[[]string, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.http_trigger_list_string_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val []string
		lift_ret_val = make([]string, lift_ret_ptr.len)
		if lift_ret_ptr.len > 0 {
			for lift_ret_val_i := 0; lift_ret_val_i < int(lift_ret_ptr.len); lift_ret_val_i++ {
				var empty_lift_ret_val C.http_trigger_string_t
				lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.ptr)) +
					uintptr(lift_ret_val_i)*unsafe.Sizeof(empty_lift_ret_val)))
				var list_lift_ret_val string
				list_lift_ret_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
				lift_ret_val[lift_ret_val_i] = list_lift_ret_val
			}
		}
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisSrem(address string, key string, values []string) Result[int64, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var lower_values C.http_trigger_list_string_t
	if len(values) == 0 {
		lower_values.ptr = nil
		lower_values.len = 0
	} else {
		var empty_lower_values C.http_trigger_string_t
		lower_values.ptr = (*C.http_trigger_string_t)(C.malloc(C.size_t(len(values)) * C.size_t(unsafe.Sizeof(empty_lower_values))))
		lower_values.len = C.size_t(len(values))
		for lower_values_i := range values {
			lower_values_ptr := (*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_values.ptr)) +
				uintptr(lower_values_i)*unsafe.Sizeof(empty_lower_values)))
			var lower_values_ptr_value C.http_trigger_string_t

			lower_values_ptr_value.ptr = C.CString(values[lower_values_i])
			lower_values_ptr_value.len = C.size_t(len(values[lower_values_i]))
			*lower_values_ptr = lower_values_ptr_value
		}
	}
	defer C.http_trigger_list_string_free(&lower_values)
	var ret C.http_trigger_result_s64_error_t
	C.fermyon_spin_redis_srem(&lower_address, &lower_key, &lower_values, &ret)
	var lift_ret Result[int64, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.int64_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val int64
		lift_ret_val = int64(lift_ret_ptr)
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinRedisExecute(address string, command string, arguments []FermyonSpinRedisTypesRedisParameter) Result[[]FermyonSpinRedisTypesRedisResult, FermyonSpinRedisTypesError] {
	var lower_address C.http_trigger_string_t

	lower_address.ptr = C.CString(address)
	lower_address.len = C.size_t(len(address))
	defer C.http_trigger_string_free(&lower_address)
	var lower_command C.http_trigger_string_t

	lower_command.ptr = C.CString(command)
	lower_command.len = C.size_t(len(command))
	defer C.http_trigger_string_free(&lower_command)
	var lower_arguments C.http_trigger_list_redis_parameter_t
	if len(arguments) == 0 {
		lower_arguments.ptr = nil
		lower_arguments.len = 0
	} else {
		var empty_lower_arguments C.fermyon_spin_redis_redis_parameter_t
		lower_arguments.ptr = (*C.fermyon_spin_redis_redis_parameter_t)(C.malloc(C.size_t(len(arguments)) * C.size_t(unsafe.Sizeof(empty_lower_arguments))))
		lower_arguments.len = C.size_t(len(arguments))
		for lower_arguments_i := range arguments {
			lower_arguments_ptr := (*C.fermyon_spin_redis_redis_parameter_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_arguments.ptr)) +
				uintptr(lower_arguments_i)*unsafe.Sizeof(empty_lower_arguments)))
			var lower_arguments_ptr_value C.fermyon_spin_redis_types_redis_parameter_t
			var lower_arguments_ptr_value_val C.fermyon_spin_redis_types_redis_parameter_t
			if arguments[lower_arguments_i].Kind() == FermyonSpinRedisTypesRedisParameterKindInt64 {

				lower_arguments_ptr_value_val.tag = 0
				lower_arguments_ptr_value_val_ptr := (*C.int64_t)(unsafe.Pointer(&lower_arguments_ptr_value_val.val))
				lower_arguments_ptr_value_val_val := C.int64_t(arguments[lower_arguments_i].GetInt64())
				*lower_arguments_ptr_value_val_ptr = lower_arguments_ptr_value_val_val
			}
			if arguments[lower_arguments_i].Kind() == FermyonSpinRedisTypesRedisParameterKindBinary {

				lower_arguments_ptr_value_val.tag = 1
				lower_arguments_ptr_value_val_ptr := (*C.fermyon_spin_redis_types_payload_t)(unsafe.Pointer(&lower_arguments_ptr_value_val.val))
				var lower_arguments_ptr_value_val_val C.fermyon_spin_redis_types_payload_t
				if len(arguments[lower_arguments_i].GetBinary()) == 0 {
					lower_arguments_ptr_value_val_val.ptr = nil
					lower_arguments_ptr_value_val_val.len = 0
				} else {
					var empty_lower_arguments_ptr_value_val_val C.uint8_t
					lower_arguments_ptr_value_val_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(arguments[lower_arguments_i].GetBinary())) * C.size_t(unsafe.Sizeof(empty_lower_arguments_ptr_value_val_val))))
					lower_arguments_ptr_value_val_val.len = C.size_t(len(arguments[lower_arguments_i].GetBinary()))
					for lower_arguments_ptr_value_val_val_i := range arguments[lower_arguments_i].GetBinary() {
						lower_arguments_ptr_value_val_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_arguments_ptr_value_val_val.ptr)) +
							uintptr(lower_arguments_ptr_value_val_val_i)*unsafe.Sizeof(empty_lower_arguments_ptr_value_val_val)))
						lower_arguments_ptr_value_val_val_ptr_value := C.uint8_t(arguments[lower_arguments_i].GetBinary()[lower_arguments_ptr_value_val_val_i])
						*lower_arguments_ptr_value_val_val_ptr = lower_arguments_ptr_value_val_val_ptr_value
					}
				}
				*lower_arguments_ptr_value_val_ptr = lower_arguments_ptr_value_val_val
			}
			lower_arguments_ptr_value = lower_arguments_ptr_value_val
			*lower_arguments_ptr = lower_arguments_ptr_value
		}
	}
	defer C.http_trigger_list_redis_parameter_free(&lower_arguments)
	var ret C.http_trigger_result_list_redis_result_error_t
	C.fermyon_spin_redis_execute(&lower_address, &lower_command, &lower_arguments, &ret)
	var lift_ret Result[[]FermyonSpinRedisTypesRedisResult, FermyonSpinRedisTypesError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_redis_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinRedisTypesError
		var lift_ret_val_val FermyonSpinRedisTypesError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinRedisTypesErrorError()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.http_trigger_list_redis_result_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val []FermyonSpinRedisTypesRedisResult
		lift_ret_val = make([]FermyonSpinRedisTypesRedisResult, lift_ret_ptr.len)
		if lift_ret_ptr.len > 0 {
			for lift_ret_val_i := 0; lift_ret_val_i < int(lift_ret_ptr.len); lift_ret_val_i++ {
				var empty_lift_ret_val C.fermyon_spin_redis_redis_result_t
				lift_ret_val_ptr := *(*C.fermyon_spin_redis_redis_result_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.ptr)) +
					uintptr(lift_ret_val_i)*unsafe.Sizeof(empty_lift_ret_val)))
				var list_lift_ret_val FermyonSpinRedisTypesRedisResult
				var list_lift_ret_val_val FermyonSpinRedisTypesRedisResult
				if lift_ret_val_ptr.tag == 0 {
					list_lift_ret_val_val = FermyonSpinRedisTypesRedisResultNil()
				}
				if lift_ret_val_ptr.tag == 1 {
					list_lift_ret_val_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_val_ptr.val))
					var list_lift_ret_val_val_val string
					list_lift_ret_val_val_val = C.GoStringN(list_lift_ret_val_val_ptr.ptr, C.int(list_lift_ret_val_val_ptr.len))
					list_lift_ret_val_val = FermyonSpinRedisTypesRedisResultStatus(list_lift_ret_val_val_val)
				}
				if lift_ret_val_ptr.tag == 2 {
					list_lift_ret_val_val_ptr := *(*C.int64_t)(unsafe.Pointer(&lift_ret_val_ptr.val))
					var list_lift_ret_val_val_val int64
					list_lift_ret_val_val_val = int64(list_lift_ret_val_val_ptr)
					list_lift_ret_val_val = FermyonSpinRedisTypesRedisResultInt64(list_lift_ret_val_val_val)
				}
				if lift_ret_val_ptr.tag == 3 {
					list_lift_ret_val_val_ptr := *(*C.fermyon_spin_redis_types_payload_t)(unsafe.Pointer(&lift_ret_val_ptr.val))
					var list_lift_ret_val_val_val []uint8
					list_lift_ret_val_val_val = make([]uint8, list_lift_ret_val_val_ptr.len)
					if list_lift_ret_val_val_ptr.len > 0 {
						for list_lift_ret_val_val_val_i := 0; list_lift_ret_val_val_val_i < int(list_lift_ret_val_val_ptr.len); list_lift_ret_val_val_val_i++ {
							var empty_list_lift_ret_val_val_val C.uint8_t
							list_lift_ret_val_val_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(list_lift_ret_val_val_ptr.ptr)) +
								uintptr(list_lift_ret_val_val_val_i)*unsafe.Sizeof(empty_list_lift_ret_val_val_val)))
							var list_list_lift_ret_val_val_val uint8
							list_list_lift_ret_val_val_val = uint8(list_lift_ret_val_val_val_ptr)
							list_lift_ret_val_val_val[list_lift_ret_val_val_val_i] = list_list_lift_ret_val_val_val
						}
					}
					list_lift_ret_val_val = FermyonSpinRedisTypesRedisResultBinary(list_lift_ret_val_val_val)
				}
				list_lift_ret_val = list_lift_ret_val_val
				lift_ret_val[lift_ret_val_i] = list_lift_ret_val
			}
		}
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

// Import functions from fermyon:spin/key-value
func FermyonSpinKeyValueOpen(name string) Result[uint32, FermyonSpinKeyValueError] {
	var lower_name C.http_trigger_string_t

	lower_name.ptr = C.CString(name)
	lower_name.len = C.size_t(len(name))
	defer C.http_trigger_string_free(&lower_name)
	var ret C.http_trigger_result_store_error_t
	C.fermyon_spin_key_value_open(&lower_name, &ret)
	var lift_ret Result[uint32, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_store_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val uint32
		var lift_ret_val_val uint32
		lift_ret_val_val = uint32(lift_ret_ptr)
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinKeyValueGet(store uint32, key string) Result[[]uint8, FermyonSpinKeyValueError] {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_list_u8_error_t
	C.fermyon_spin_key_value_get(lower_store, &lower_key, &ret)
	var lift_ret Result[[]uint8, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.http_trigger_list_u8_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val []uint8
		lift_ret_val = make([]uint8, lift_ret_ptr.len)
		if lift_ret_ptr.len > 0 {
			for lift_ret_val_i := 0; lift_ret_val_i < int(lift_ret_ptr.len); lift_ret_val_i++ {
				var empty_lift_ret_val C.uint8_t
				lift_ret_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.ptr)) +
					uintptr(lift_ret_val_i)*unsafe.Sizeof(empty_lift_ret_val)))
				var list_lift_ret_val uint8
				list_lift_ret_val = uint8(lift_ret_val_ptr)
				lift_ret_val[lift_ret_val_i] = list_lift_ret_val
			}
		}
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinKeyValueSet(store uint32, key string, value []uint8) Result[struct{}, FermyonSpinKeyValueError] {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var lower_value C.http_trigger_list_u8_t
	if len(value) == 0 {
		lower_value.ptr = nil
		lower_value.len = 0
	} else {
		var empty_lower_value C.uint8_t
		lower_value.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(value)) * C.size_t(unsafe.Sizeof(empty_lower_value))))
		lower_value.len = C.size_t(len(value))
		for lower_value_i := range value {
			lower_value_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_value.ptr)) +
				uintptr(lower_value_i)*unsafe.Sizeof(empty_lower_value)))
			lower_value_ptr_value := C.uint8_t(value[lower_value_i])
			*lower_value_ptr = lower_value_ptr_value
		}
	}
	defer C.http_trigger_list_u8_free(&lower_value)
	var ret C.http_trigger_result_void_error_t
	C.fermyon_spin_key_value_set(lower_store, &lower_key, &lower_value, &ret)
	var lift_ret Result[struct{}, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
	}
	return lift_ret
}

func FermyonSpinKeyValueDelete(store uint32, key string) Result[struct{}, FermyonSpinKeyValueError] {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_void_error_t
	C.fermyon_spin_key_value_delete(lower_store, &lower_key, &ret)
	var lift_ret Result[struct{}, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
	}
	return lift_ret
}

func FermyonSpinKeyValueExists(store uint32, key string) Result[bool, FermyonSpinKeyValueError] {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	var lower_key C.http_trigger_string_t

	lower_key.ptr = C.CString(key)
	lower_key.len = C.size_t(len(key))
	defer C.http_trigger_string_free(&lower_key)
	var ret C.http_trigger_result_bool_error_t
	C.fermyon_spin_key_value_exists(lower_store, &lower_key, &ret)
	var lift_ret Result[bool, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*bool)(unsafe.Pointer(&ret.val))
		lift_ret_val := lift_ret_ptr
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinKeyValueGetKeys(store uint32) Result[[]string, FermyonSpinKeyValueError] {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	var ret C.http_trigger_result_list_string_error_t
	C.fermyon_spin_key_value_get_keys(lower_store, &ret)
	var lift_ret Result[[]string, FermyonSpinKeyValueError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_key_value_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinKeyValueError
		if lift_ret_ptr.tag == 0 {
			lift_ret_val = FermyonSpinKeyValueErrorStoreTableFull()
		}
		if lift_ret_ptr.tag == 1 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchStore()
		}
		if lift_ret_ptr.tag == 2 {
			lift_ret_val = FermyonSpinKeyValueErrorAccessDenied()
		}
		if lift_ret_ptr.tag == 3 {
			lift_ret_val = FermyonSpinKeyValueErrorInvalidStore()
		}
		if lift_ret_ptr.tag == 4 {
			lift_ret_val = FermyonSpinKeyValueErrorNoSuchKey()
		}
		if lift_ret_ptr.tag == 5 {
			lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(&lift_ret_ptr.val))
			var lift_ret_val_val string
			lift_ret_val_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
			lift_ret_val = FermyonSpinKeyValueErrorIo(lift_ret_val_val)
		}
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.http_trigger_list_string_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val []string
		lift_ret_val = make([]string, lift_ret_ptr.len)
		if lift_ret_ptr.len > 0 {
			for lift_ret_val_i := 0; lift_ret_val_i < int(lift_ret_ptr.len); lift_ret_val_i++ {
				var empty_lift_ret_val C.http_trigger_string_t
				lift_ret_val_ptr := *(*C.http_trigger_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.ptr)) +
					uintptr(lift_ret_val_i)*unsafe.Sizeof(empty_lift_ret_val)))
				var list_lift_ret_val string
				list_lift_ret_val = C.GoStringN(lift_ret_val_ptr.ptr, C.int(lift_ret_val_ptr.len))
				lift_ret_val[lift_ret_val_i] = list_lift_ret_val
			}
		}
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

func FermyonSpinKeyValueClose(store uint32) {
	var lower_store C.uint32_t
	lower_store_val := C.uint32_t(store)
	lower_store = lower_store_val
	C.fermyon_spin_key_value_close(lower_store)
}

// Import functions from fermyon:spin/http-types
// Import functions from fermyon:spin/http
func FermyonSpinHttpSendRequest(req FermyonSpinHttpTypesRequest) Result[FermyonSpinHttpTypesResponse, FermyonSpinHttpTypesHttpError] {
	var lower_req C.fermyon_spin_http_types_request_t
	var lower_req_val C.fermyon_spin_http_types_request_t
	var lower_req_val_method C.fermyon_spin_http_types_method_t
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindGet {
		lower_req_val_method = 0
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindPost {
		lower_req_val_method = 1
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindPut {
		lower_req_val_method = 2
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindDelete {
		lower_req_val_method = 3
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindPatch {
		lower_req_val_method = 4
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindHead {
		lower_req_val_method = 5
	}
	if req.Method.Kind() == FermyonSpinHttpTypesMethodKindOptions {
		lower_req_val_method = 6
	}
	lower_req_val.method = lower_req_val_method
	var lower_req_val_uri C.http_trigger_string_t
	var lower_req_val_uri_val C.http_trigger_string_t

	lower_req_val_uri_val.ptr = C.CString(req.Uri)
	lower_req_val_uri_val.len = C.size_t(len(req.Uri))
	lower_req_val_uri = lower_req_val_uri_val
	lower_req_val.uri = lower_req_val_uri
	var lower_req_val_headers C.fermyon_spin_http_types_headers_t
	if len(req.Headers) == 0 {
		lower_req_val_headers.ptr = nil
		lower_req_val_headers.len = 0
	} else {
		var empty_lower_req_val_headers C.http_trigger_tuple2_string_string_t
		lower_req_val_headers.ptr = (*C.http_trigger_tuple2_string_string_t)(C.malloc(C.size_t(len(req.Headers)) * C.size_t(unsafe.Sizeof(empty_lower_req_val_headers))))
		lower_req_val_headers.len = C.size_t(len(req.Headers))
		for lower_req_val_headers_i := range req.Headers {
			lower_req_val_headers_ptr := (*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_req_val_headers.ptr)) +
				uintptr(lower_req_val_headers_i)*unsafe.Sizeof(empty_lower_req_val_headers)))
			var lower_req_val_headers_ptr_value C.http_trigger_tuple2_string_string_t
			var lower_req_val_headers_ptr_value_f0 C.http_trigger_string_t

			lower_req_val_headers_ptr_value_f0.ptr = C.CString(req.Headers[lower_req_val_headers_i].F0)
			lower_req_val_headers_ptr_value_f0.len = C.size_t(len(req.Headers[lower_req_val_headers_i].F0))
			lower_req_val_headers_ptr_value.f0 = lower_req_val_headers_ptr_value_f0
			var lower_req_val_headers_ptr_value_f1 C.http_trigger_string_t

			lower_req_val_headers_ptr_value_f1.ptr = C.CString(req.Headers[lower_req_val_headers_i].F1)
			lower_req_val_headers_ptr_value_f1.len = C.size_t(len(req.Headers[lower_req_val_headers_i].F1))
			lower_req_val_headers_ptr_value.f1 = lower_req_val_headers_ptr_value_f1
			*lower_req_val_headers_ptr = lower_req_val_headers_ptr_value
		}
	}
	lower_req_val.headers = lower_req_val_headers
	var lower_req_val_params C.fermyon_spin_http_types_params_t
	if len(req.Params) == 0 {
		lower_req_val_params.ptr = nil
		lower_req_val_params.len = 0
	} else {
		var empty_lower_req_val_params C.http_trigger_tuple2_string_string_t
		lower_req_val_params.ptr = (*C.http_trigger_tuple2_string_string_t)(C.malloc(C.size_t(len(req.Params)) * C.size_t(unsafe.Sizeof(empty_lower_req_val_params))))
		lower_req_val_params.len = C.size_t(len(req.Params))
		for lower_req_val_params_i := range req.Params {
			lower_req_val_params_ptr := (*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_req_val_params.ptr)) +
				uintptr(lower_req_val_params_i)*unsafe.Sizeof(empty_lower_req_val_params)))
			var lower_req_val_params_ptr_value C.http_trigger_tuple2_string_string_t
			var lower_req_val_params_ptr_value_f0 C.http_trigger_string_t

			lower_req_val_params_ptr_value_f0.ptr = C.CString(req.Params[lower_req_val_params_i].F0)
			lower_req_val_params_ptr_value_f0.len = C.size_t(len(req.Params[lower_req_val_params_i].F0))
			lower_req_val_params_ptr_value.f0 = lower_req_val_params_ptr_value_f0
			var lower_req_val_params_ptr_value_f1 C.http_trigger_string_t

			lower_req_val_params_ptr_value_f1.ptr = C.CString(req.Params[lower_req_val_params_i].F1)
			lower_req_val_params_ptr_value_f1.len = C.size_t(len(req.Params[lower_req_val_params_i].F1))
			lower_req_val_params_ptr_value.f1 = lower_req_val_params_ptr_value_f1
			*lower_req_val_params_ptr = lower_req_val_params_ptr_value
		}
	}
	lower_req_val.params = lower_req_val_params
	var lower_req_val_body C.http_trigger_option_body_t
	if req.Body.IsSome() {
		var lower_req_val_body_val C.fermyon_spin_http_types_body_t
		if len(req.Body.Unwrap()) == 0 {
			lower_req_val_body_val.ptr = nil
			lower_req_val_body_val.len = 0
		} else {
			var empty_lower_req_val_body_val C.uint8_t
			lower_req_val_body_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(req.Body.Unwrap())) * C.size_t(unsafe.Sizeof(empty_lower_req_val_body_val))))
			lower_req_val_body_val.len = C.size_t(len(req.Body.Unwrap()))
			for lower_req_val_body_val_i := range req.Body.Unwrap() {
				lower_req_val_body_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_req_val_body_val.ptr)) +
					uintptr(lower_req_val_body_val_i)*unsafe.Sizeof(empty_lower_req_val_body_val)))
				lower_req_val_body_val_ptr_value := C.uint8_t(req.Body.Unwrap()[lower_req_val_body_val_i])
				*lower_req_val_body_val_ptr = lower_req_val_body_val_ptr_value
			}
		}
		lower_req_val_body.val = lower_req_val_body_val
		lower_req_val_body.is_some = true
	}
	lower_req_val.body = lower_req_val_body
	lower_req = lower_req_val
	defer C.fermyon_spin_http_request_free(&lower_req)
	var ret C.http_trigger_result_response_http_error_t
	C.fermyon_spin_http_send_request(&lower_req, &ret)
	var lift_ret Result[FermyonSpinHttpTypesResponse, FermyonSpinHttpTypesHttpError]
	if ret.is_err {
		lift_ret_ptr := *(*C.fermyon_spin_http_http_error_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinHttpTypesHttpError
		var lift_ret_val_val FermyonSpinHttpTypesHttpError
		if lift_ret_ptr == 0 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorSuccess()
		}
		if lift_ret_ptr == 1 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorDestinationNotAllowed()
		}
		if lift_ret_ptr == 2 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorInvalidUrl()
		}
		if lift_ret_ptr == 3 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorRequestError()
		}
		if lift_ret_ptr == 4 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorRuntimeError()
		}
		if lift_ret_ptr == 5 {
			lift_ret_val_val = FermyonSpinHttpTypesHttpErrorTooManyRequests()
		}
		lift_ret_val = lift_ret_val_val
		lift_ret.SetErr(lift_ret_val)
	} else {
		lift_ret_ptr := *(*C.fermyon_spin_http_response_t)(unsafe.Pointer(&ret.val))
		var lift_ret_val FermyonSpinHttpTypesResponse
		var lift_ret_val_val FermyonSpinHttpTypesResponse
		var lift_ret_val_val_Status uint16
		var lift_ret_val_val_Status_val uint16
		lift_ret_val_val_Status_val = uint16(lift_ret_ptr.status)
		lift_ret_val_val_Status = lift_ret_val_val_Status_val
		lift_ret_val_val.Status = lift_ret_val_val_Status
		var lift_ret_val_val_Headers Option[[]FermyonSpinHttpTuple2StringStringT]
		if lift_ret_ptr.headers.is_some {
			var lift_ret_val_val_Headers_val []FermyonSpinHttpTuple2StringStringT
			lift_ret_val_val_Headers_val = make([]FermyonSpinHttpTuple2StringStringT, lift_ret_ptr.headers.val.len)
			if lift_ret_ptr.headers.val.len > 0 {
				for lift_ret_val_val_Headers_val_i := 0; lift_ret_val_val_Headers_val_i < int(lift_ret_ptr.headers.val.len); lift_ret_val_val_Headers_val_i++ {
					var empty_lift_ret_val_val_Headers_val C.http_trigger_tuple2_string_string_t
					lift_ret_val_val_Headers_val_ptr := *(*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.headers.val.ptr)) +
						uintptr(lift_ret_val_val_Headers_val_i)*unsafe.Sizeof(empty_lift_ret_val_val_Headers_val)))
					var list_lift_ret_val_val_Headers_val FermyonSpinHttpTuple2StringStringT
					var list_lift_ret_val_val_Headers_val_F0 string
					list_lift_ret_val_val_Headers_val_F0 = C.GoStringN(lift_ret_val_val_Headers_val_ptr.f0.ptr, C.int(lift_ret_val_val_Headers_val_ptr.f0.len))
					list_lift_ret_val_val_Headers_val.F0 = list_lift_ret_val_val_Headers_val_F0
					var list_lift_ret_val_val_Headers_val_F1 string
					list_lift_ret_val_val_Headers_val_F1 = C.GoStringN(lift_ret_val_val_Headers_val_ptr.f1.ptr, C.int(lift_ret_val_val_Headers_val_ptr.f1.len))
					list_lift_ret_val_val_Headers_val.F1 = list_lift_ret_val_val_Headers_val_F1
					lift_ret_val_val_Headers_val[lift_ret_val_val_Headers_val_i] = list_lift_ret_val_val_Headers_val
				}
			}
			lift_ret_val_val_Headers.Set(lift_ret_val_val_Headers_val)
		} else {
			lift_ret_val_val_Headers.Unset()
		}
		lift_ret_val_val.Headers = lift_ret_val_val_Headers
		var lift_ret_val_val_Body Option[[]uint8]
		if lift_ret_ptr.body.is_some {
			var lift_ret_val_val_Body_val []uint8
			lift_ret_val_val_Body_val = make([]uint8, lift_ret_ptr.body.val.len)
			if lift_ret_ptr.body.val.len > 0 {
				for lift_ret_val_val_Body_val_i := 0; lift_ret_val_val_Body_val_i < int(lift_ret_ptr.body.val.len); lift_ret_val_val_Body_val_i++ {
					var empty_lift_ret_val_val_Body_val C.uint8_t
					lift_ret_val_val_Body_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lift_ret_ptr.body.val.ptr)) +
						uintptr(lift_ret_val_val_Body_val_i)*unsafe.Sizeof(empty_lift_ret_val_val_Body_val)))
					var list_lift_ret_val_val_Body_val uint8
					list_lift_ret_val_val_Body_val = uint8(lift_ret_val_val_Body_val_ptr)
					lift_ret_val_val_Body_val[lift_ret_val_val_Body_val_i] = list_lift_ret_val_val_Body_val
				}
			}
			lift_ret_val_val_Body.Set(lift_ret_val_val_Body_val)
		} else {
			lift_ret_val_val_Body.Unset()
		}
		lift_ret_val_val.Body = lift_ret_val_val_Body
		lift_ret_val = lift_ret_val_val
		lift_ret.Set(lift_ret_val)
	}
	return lift_ret
}

// Export functions from fermyon:spin/inbound-http
var fermyon_spin_inbound_http ExportsFermyonSpinInboundHttp = nil

func SetExportsFermyonSpinInboundHttp(i ExportsFermyonSpinInboundHttp) {
	fermyon_spin_inbound_http = i
}

type ExportsFermyonSpinInboundHttp interface {
	HandleRequest(req FermyonSpinHttpTypesRequest) FermyonSpinHttpTypesResponse
}

//export exports_fermyon_spin_inbound_http_handle_request
func ExportsFermyonSpinInboundHttpHandleRequest(req *C.fermyon_spin_inbound_http_request_t, ret *C.fermyon_spin_inbound_http_response_t) {
	defer C.fermyon_spin_inbound_http_request_free(req)
	var lift_req FermyonSpinHttpTypesRequest
	var lift_req_val FermyonSpinHttpTypesRequest
	var lift_req_val_Method FermyonSpinHttpTypesMethod
	if req.method == 0 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodGet()
	}
	if req.method == 1 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodPost()
	}
	if req.method == 2 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodPut()
	}
	if req.method == 3 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodDelete()
	}
	if req.method == 4 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodPatch()
	}
	if req.method == 5 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodHead()
	}
	if req.method == 6 {
		lift_req_val_Method = FermyonSpinHttpTypesMethodOptions()
	}
	lift_req_val.Method = lift_req_val_Method
	var lift_req_val_Uri string
	var lift_req_val_Uri_val string
	lift_req_val_Uri_val = C.GoStringN(req.uri.ptr, C.int(req.uri.len))
	lift_req_val_Uri = lift_req_val_Uri_val
	lift_req_val.Uri = lift_req_val_Uri
	var lift_req_val_Headers []FermyonSpinInboundHttpTuple2StringStringT
	lift_req_val_Headers = make([]FermyonSpinInboundHttpTuple2StringStringT, req.headers.len)
	if req.headers.len > 0 {
		for lift_req_val_Headers_i := 0; lift_req_val_Headers_i < int(req.headers.len); lift_req_val_Headers_i++ {
			var empty_lift_req_val_Headers C.http_trigger_tuple2_string_string_t
			lift_req_val_Headers_ptr := *(*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(req.headers.ptr)) +
				uintptr(lift_req_val_Headers_i)*unsafe.Sizeof(empty_lift_req_val_Headers)))
			var list_lift_req_val_Headers FermyonSpinInboundHttpTuple2StringStringT
			var list_lift_req_val_Headers_F0 string
			list_lift_req_val_Headers_F0 = C.GoStringN(lift_req_val_Headers_ptr.f0.ptr, C.int(lift_req_val_Headers_ptr.f0.len))
			list_lift_req_val_Headers.F0 = list_lift_req_val_Headers_F0
			var list_lift_req_val_Headers_F1 string
			list_lift_req_val_Headers_F1 = C.GoStringN(lift_req_val_Headers_ptr.f1.ptr, C.int(lift_req_val_Headers_ptr.f1.len))
			list_lift_req_val_Headers.F1 = list_lift_req_val_Headers_F1
			lift_req_val_Headers[lift_req_val_Headers_i] = list_lift_req_val_Headers
		}
	}
	lift_req_val.Headers = lift_req_val_Headers
	var lift_req_val_Params []FermyonSpinInboundHttpTuple2StringStringT
	lift_req_val_Params = make([]FermyonSpinInboundHttpTuple2StringStringT, req.params.len)
	if req.params.len > 0 {
		for lift_req_val_Params_i := 0; lift_req_val_Params_i < int(req.params.len); lift_req_val_Params_i++ {
			var empty_lift_req_val_Params C.http_trigger_tuple2_string_string_t
			lift_req_val_Params_ptr := *(*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(req.params.ptr)) +
				uintptr(lift_req_val_Params_i)*unsafe.Sizeof(empty_lift_req_val_Params)))
			var list_lift_req_val_Params FermyonSpinInboundHttpTuple2StringStringT
			var list_lift_req_val_Params_F0 string
			list_lift_req_val_Params_F0 = C.GoStringN(lift_req_val_Params_ptr.f0.ptr, C.int(lift_req_val_Params_ptr.f0.len))
			list_lift_req_val_Params.F0 = list_lift_req_val_Params_F0
			var list_lift_req_val_Params_F1 string
			list_lift_req_val_Params_F1 = C.GoStringN(lift_req_val_Params_ptr.f1.ptr, C.int(lift_req_val_Params_ptr.f1.len))
			list_lift_req_val_Params.F1 = list_lift_req_val_Params_F1
			lift_req_val_Params[lift_req_val_Params_i] = list_lift_req_val_Params
		}
	}
	lift_req_val.Params = lift_req_val_Params
	var lift_req_val_Body Option[[]uint8]
	if req.body.is_some {
		var lift_req_val_Body_val []uint8
		lift_req_val_Body_val = make([]uint8, req.body.val.len)
		if req.body.val.len > 0 {
			for lift_req_val_Body_val_i := 0; lift_req_val_Body_val_i < int(req.body.val.len); lift_req_val_Body_val_i++ {
				var empty_lift_req_val_Body_val C.uint8_t
				lift_req_val_Body_val_ptr := *(*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(req.body.val.ptr)) +
					uintptr(lift_req_val_Body_val_i)*unsafe.Sizeof(empty_lift_req_val_Body_val)))
				var list_lift_req_val_Body_val uint8
				list_lift_req_val_Body_val = uint8(lift_req_val_Body_val_ptr)
				lift_req_val_Body_val[lift_req_val_Body_val_i] = list_lift_req_val_Body_val
			}
		}
		lift_req_val_Body.Set(lift_req_val_Body_val)
	} else {
		lift_req_val_Body.Unset()
	}
	lift_req_val.Body = lift_req_val_Body
	lift_req = lift_req_val
	result := fermyon_spin_inbound_http.HandleRequest(lift_req)
	var lower_result C.fermyon_spin_http_types_response_t
	var lower_result_val C.fermyon_spin_http_types_response_t
	var lower_result_val_status C.uint16_t
	lower_result_val_status_val := C.uint16_t(result.Status)
	lower_result_val_status = lower_result_val_status_val
	lower_result_val.status = lower_result_val_status
	var lower_result_val_headers C.http_trigger_option_headers_t
	if result.Headers.IsSome() {
		var lower_result_val_headers_val C.fermyon_spin_http_types_headers_t
		if len(result.Headers.Unwrap()) == 0 {
			lower_result_val_headers_val.ptr = nil
			lower_result_val_headers_val.len = 0
		} else {
			var empty_lower_result_val_headers_val C.http_trigger_tuple2_string_string_t
			lower_result_val_headers_val.ptr = (*C.http_trigger_tuple2_string_string_t)(C.malloc(C.size_t(len(result.Headers.Unwrap())) * C.size_t(unsafe.Sizeof(empty_lower_result_val_headers_val))))
			lower_result_val_headers_val.len = C.size_t(len(result.Headers.Unwrap()))
			for lower_result_val_headers_val_i := range result.Headers.Unwrap() {
				lower_result_val_headers_val_ptr := (*C.http_trigger_tuple2_string_string_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_result_val_headers_val.ptr)) +
					uintptr(lower_result_val_headers_val_i)*unsafe.Sizeof(empty_lower_result_val_headers_val)))
				var lower_result_val_headers_val_ptr_value C.http_trigger_tuple2_string_string_t
				var lower_result_val_headers_val_ptr_value_f0 C.http_trigger_string_t

				lower_result_val_headers_val_ptr_value_f0.ptr = C.CString(result.Headers.Unwrap()[lower_result_val_headers_val_i].F0)
				lower_result_val_headers_val_ptr_value_f0.len = C.size_t(len(result.Headers.Unwrap()[lower_result_val_headers_val_i].F0))
				lower_result_val_headers_val_ptr_value.f0 = lower_result_val_headers_val_ptr_value_f0
				var lower_result_val_headers_val_ptr_value_f1 C.http_trigger_string_t

				lower_result_val_headers_val_ptr_value_f1.ptr = C.CString(result.Headers.Unwrap()[lower_result_val_headers_val_i].F1)
				lower_result_val_headers_val_ptr_value_f1.len = C.size_t(len(result.Headers.Unwrap()[lower_result_val_headers_val_i].F1))
				lower_result_val_headers_val_ptr_value.f1 = lower_result_val_headers_val_ptr_value_f1
				*lower_result_val_headers_val_ptr = lower_result_val_headers_val_ptr_value
			}
		}
		lower_result_val_headers.val = lower_result_val_headers_val
		lower_result_val_headers.is_some = true
	}
	lower_result_val.headers = lower_result_val_headers
	var lower_result_val_body C.http_trigger_option_body_t
	if result.Body.IsSome() {
		var lower_result_val_body_val C.fermyon_spin_http_types_body_t
		if len(result.Body.Unwrap()) == 0 {
			lower_result_val_body_val.ptr = nil
			lower_result_val_body_val.len = 0
		} else {
			var empty_lower_result_val_body_val C.uint8_t
			lower_result_val_body_val.ptr = (*C.uint8_t)(C.malloc(C.size_t(len(result.Body.Unwrap())) * C.size_t(unsafe.Sizeof(empty_lower_result_val_body_val))))
			lower_result_val_body_val.len = C.size_t(len(result.Body.Unwrap()))
			for lower_result_val_body_val_i := range result.Body.Unwrap() {
				lower_result_val_body_val_ptr := (*C.uint8_t)(unsafe.Pointer(uintptr(unsafe.Pointer(lower_result_val_body_val.ptr)) +
					uintptr(lower_result_val_body_val_i)*unsafe.Sizeof(empty_lower_result_val_body_val)))
				lower_result_val_body_val_ptr_value := C.uint8_t(result.Body.Unwrap()[lower_result_val_body_val_i])
				*lower_result_val_body_val_ptr = lower_result_val_body_val_ptr_value
			}
		}
		lower_result_val_body.val = lower_result_val_body_val
		lower_result_val_body.is_some = true
	}
	lower_result_val.body = lower_result_val_body
	lower_result = lower_result_val
	*ret = lower_result

}
