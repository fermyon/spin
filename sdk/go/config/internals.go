package config

import (
	"errors"
	"github.com/fermyon/spin/sdk/go/generated"
)

func get(key string) (string, error) {
	res := http_trigger.FermyonSpinConfigGetConfig(key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return "", toError(res.UnwrapErr())
}

func toError(err http_trigger.FermyonSpinConfigError) error {
	// TODO: translate error
	return errors.New("")
}
