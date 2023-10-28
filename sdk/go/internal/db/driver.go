package db

import (
	"database/sql/driver"
)

// GlobalParameterConverter is a global valueConverter instance to convert parameters.
var GlobalParameterConverter = &valueConverter{}

var _ driver.ValueConverter = (*valueConverter)(nil)

// valueConverter is a no-op value converter.
type valueConverter struct{}

func (c *valueConverter) ConvertValue(v any) (driver.Value, error) {
	return driver.Value(v), nil
}
