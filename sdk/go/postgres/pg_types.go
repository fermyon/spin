package postgres

import "fmt"

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
	val  interface{}
}

func (n PgError) Kind() PgErrorKind {
	return n.kind
}

func PgErrorSuccess() PgError {
	return PgError{kind: PgErrorKindSuccess}
}

func PgErrorConnectionFailed(v string) PgError {
	return PgError{kind: PgErrorKindConnectionFailed, val: v}
}

func (n PgError) GetConnectionFailed() string {
	if g, w := n.Kind(), PgErrorKindConnectionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *PgError) SetConnectionFailed(v string) {
	n.val = v
	n.kind = PgErrorKindConnectionFailed
}

func PgErrorBadParameter(v string) PgError {
	return PgError{kind: PgErrorKindBadParameter, val: v}
}

func (n PgError) GetBadParameter() string {
	if g, w := n.Kind(), PgErrorKindBadParameter; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *PgError) SetBadParameter(v string) {
	n.val = v
	n.kind = PgErrorKindBadParameter
}

func PgErrorQueryFailed(v string) PgError {
	return PgError{kind: PgErrorKindQueryFailed, val: v}
}

func (n PgError) GetQueryFailed() string {
	if g, w := n.Kind(), PgErrorKindQueryFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *PgError) SetQueryFailed(v string) {
	n.val = v
	n.kind = PgErrorKindQueryFailed
}

func PgErrorValueConversionFailed(v string) PgError {
	return PgError{kind: PgErrorKindValueConversionFailed, val: v}
}

func (n PgError) GetValueConversionFailed() string {
	if g, w := n.Kind(), PgErrorKindValueConversionFailed; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *PgError) SetValueConversionFailed(v string) {
	n.val = v
	n.kind = PgErrorKindValueConversionFailed
}

func PgErrorOtherError(v string) PgError {
	return PgError{kind: PgErrorKindOtherError, val: v}
}

func (n PgError) GetOtherError() string {
	if g, w := n.Kind(), PgErrorKindOtherError; g != w {
		panic(fmt.Sprintf("Attr kind is %v, not %v", g, w))
	}
	return n.val.(string)
}

func (n *PgError) SetOtherError(v string) {
	n.val = v
	n.kind = PgErrorKindOtherError
}
