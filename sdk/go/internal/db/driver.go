package db

import (
	"database/sql/driver"
)

// globalValueConverter is a valueConverter instance.
var GlobalValueConverter = &valueConverter{}

// valueConverter is a no-op value converter.
type valueConverter struct{}

func (c *valueConverter) ConvertValue(v any) (driver.Value, error) {
	return driver.Value(v), nil
}
