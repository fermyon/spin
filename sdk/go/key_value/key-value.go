// Package key_value provides access to key value stores within Spin
// components.
package key_value

import (
	"errors"
	"fmt"

	"github.com/fermyon/spin/sdk/go/generated"
)

type Store uint32

const (
	errorKindStoreTableFull = iota
	errorKindNoSuchStore
	errorKindAccessDenied
	errorKindInvalidStore
	errorKindNoSuchKey
	errorKindIo
)

func Open(name string) (Store, error) {
	res := reactor.FermyonSpinKeyValueOpen(name)
	if res.IsOk() {
		return Store(res.Unwrap()), nil
	}

	return Store(0), toErr(res.UnwrapErr())
}

func Get(store Store, key string) ([]byte, error) {
	res := reactor.FermyonSpinKeyValueGet(uint32(store), key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func Set(store Store, key string, value []byte) error {
	res := reactor.FermyonSpinKeyValueSet(uint32(store), key, value)
	if res.IsOk() {
		return nil
	}
	return toErr(res.UnwrapErr())
}

func Delete(store Store, key string) error {
	res := reactor.FermyonSpinKeyValueDelete(uint32(store), key)
	if res.IsOk() {
		return nil
	}
	return toErr(res.UnwrapErr())
}

func Exists(store Store, key string) (bool, error) {
	res := reactor.FermyonSpinKeyValueExists(uint32(store), key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return false, toErr(res.UnwrapErr())
}

func GetKeys(store Store) ([]string, error) {
	res := reactor.FermyonSpinKeyValueGetKeys(uint32(store))
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func Close(store Store) {
	reactor.FermyonSpinKeyValueClose(uint32(store))
}

func toErr(err reactor.FermyonSpinKeyValueError) error {
	switch err.Kind() {
	case reactor.FermyonSpinKeyValueErrorKindStoreTableFull:
		return errors.New("store table full")
	case reactor.FermyonSpinKeyValueErrorKindNoSuchStore:
		return errors.New("no such store")
	case reactor.FermyonSpinKeyValueErrorKindAccessDenied:
		return errors.New("access denied")
	case reactor.FermyonSpinKeyValueErrorKindInvalidStore:
		return errors.New("invalid store")
	case reactor.FermyonSpinKeyValueErrorKindNoSuchKey:
		return errors.New("no such key")
	case reactor.FermyonSpinKeyValueErrorKindIo:
		return errors.New(fmt.Sprintf("io error: %s", err.GetIo()))
	default:
		return errors.New(fmt.Sprintf("unrecognized error: %v", err.Kind()))
	}
}
