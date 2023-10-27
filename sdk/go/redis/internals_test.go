package redis

import (
	"reflect"
	"testing"
)

func TestCreateParameter(t *testing.T) {
	tests := []struct {
		in   any
		want argumentKind
	}{
		{in: "a", want: argumentKindBinary},
		{in: []byte("b"), want: argumentKindBinary},
		{in: 1, want: argumentKindInt},
		{in: int64(2), want: argumentKindInt},
		{in: int32(3), want: argumentKindInt},
	}

	for _, tc := range tests {
		p, err := createParameter(tc.in)
		if err != nil {
			t.Error(err)
		}
		if p.kind != tc.want {
			t.Errorf("want %s, got %s", tc.want, p.kind)
		}
	}
}

func TestRedisListString(t *testing.T) {
	list := []string{"a", "b", "c"}

	rlist := redisListStr(list)
	got := fromRedisListStr(&rlist)

	if !reflect.DeepEqual(list, got) {
		t.Errorf("want %s, got %s", list, got)
	}
}
