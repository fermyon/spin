package http

import "testing"

func TestString(t *testing.T) {
	tt := "hello"
	if tt != newSpinString(tt).String() {
		t.Fatal("strings do not match")
	}
}

func TestHeader(t *testing.T) {
	k, v := "hello", "world"
	gotK, gotV := newSpinHeader(k, v).Values()
	if k != gotK {
		t.Fatal("keys do not match")
	}
	if v != gotV {
		t.Fatal("values did not match")
	}
}
