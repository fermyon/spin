package sqlite

import (
	"reflect"
	"testing"
)

func TestValue(t *testing.T) {
	tests := []any{
		int64(1234),
		3.14,
		"foo",
		[]byte("bar"),
		nil,
	}

	for _, tc := range tests {
		got := fromSqliteValue(toSqliteValue(tc))
		if !reflect.DeepEqual(tc, got) {
			t.Errorf("want %T(%#v), got %T(%#v)", tc, tc, got, got)
		}
	}
}

func TestValueList(t *testing.T) {
	tc := []any{
		int64(1234),
		3.14,
		"foo",
		[]byte("bar"),
		nil,
	}

	got := fromSqliteListValue(toSqliteListValue(tc))
	if !reflect.DeepEqual(tc, got) {
		t.Errorf("want %v, got %v", tc, got)
	}
}
