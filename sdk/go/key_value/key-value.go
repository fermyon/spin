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
	res := http_trigger.FermyonSpinKeyValueOpen(name)
	if res.IsOk() {
		return Store(res.Unwrap()), nil
	}

	return Store(0), toErr(res.UnwrapErr())
}

func Get(store Store, key string) ([]byte, error) {
	res := http_trigger.FermyonSpinKeyValueGet(uint32(store), key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func Set(store Store, key string, value []byte) error {
	res := http_trigger.FermyonSpinKeyValueSet(uint32(store), key, value)
	if res.IsOk() {
		return nil
	}
	return toErr(res.UnwrapErr())
}

func Delete(store Store, key string) error {
	res := http_trigger.FermyonSpinKeyValueDelete(uint32(store), key)
	if res.IsOk() {
		return nil
	}
	return toErr(res.UnwrapErr())
}

func Exists(store Store, key string) (bool, error) {
	res := http_trigger.FermyonSpinKeyValueExists(uint32(store), key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return false, toErr(res.UnwrapErr())
}

func GetKeys(store Store) ([]string, error) {
	res := http_trigger.FermyonSpinKeyValueGetKeys(uint32(store))
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func Close(store Store) {
	http_trigger.FermyonSpinKeyValueClose(uint32(store))
}

func toErr(err http_trigger.FermyonSpinKeyValueError) error {
	switch err.Kind() {
	case http_trigger.FermyonSpinKeyValueErrorKindStoreTableFull:
		return errors.New("store table full")
	case http_trigger.FermyonSpinKeyValueErrorKindNoSuchStore:
		return errors.New("no such store")
	case http_trigger.FermyonSpinKeyValueErrorKindAccessDenied:
		return errors.New("access denied")
	case http_trigger.FermyonSpinKeyValueErrorKindInvalidStore:
		return errors.New("invalid store")
	case http_trigger.FermyonSpinKeyValueErrorKindNoSuchKey:
		return errors.New("no such key")
	case http_trigger.FermyonSpinKeyValueErrorKindIo:
		return errors.New(fmt.Sprintf("io error: %s", err.GetIo()))
	default:
		return errors.New(fmt.Sprintf("unrecognized error: %v", err.Kind()))
	}
}
