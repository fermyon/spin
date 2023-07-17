package config

import (
	"errors"
	reactor "github.com/fermyon/spin/sdk/go/generated"
)

func get(key string) (string, error) {
	res := reactor.FermyonSpinConfigGetConfig(key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return "", toError(res.UnwrapErr())
}

func toError(err reactor.FermyonSpinConfigError) error {
	switch err.Kind() {
	case reactor.FermyonSpinConfigErrorKindProvider:
		return errors.New(err.GetProvider())
	case reactor.FermyonSpinConfigErrorKindInvalidKey:
		return errors.New(err.GetInvalidKey())
	case reactor.FermyonSpinConfigErrorKindInvalidSchema:
		return errors.New(err.GetInvalidSchema())
	default:
		return errors.New(err.GetOther())
	}
}
