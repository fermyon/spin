package main

import (
	"net/http"
	"reflect"
	"fmt"

	spin_http "github.com/fermyon/spin/sdk/go/http"
	"github.com/fermyon/spin/sdk/go/key-value"
)

func init() {

	// handler for the http trigger
	spin_http.Handle(func(w http.ResponseWriter, r *http.Request) {
		store, err := key_value.Open("default");
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		defer key_value.Close(store)

		if err := key_value.Set(store, "foo", []byte("bar")); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		{
			expected := []byte("bar")
			if value, err := key_value.Get(store, "foo"); err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			} else if !reflect.DeepEqual(value, expected) {
				http.Error(
					w,
					fmt.Sprintf("expected %v, got %v", expected, value),
					http.StatusInternalServerError,
				)
				return
			}
		}

		{
			expected := []string{"foo"}
			if value, err := key_value.GetKeys(store); err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			} else if !reflect.DeepEqual(value, expected) {
				http.Error(
					w,
					fmt.Sprintf("expected %v, got %v", expected, value),
					http.StatusInternalServerError,
				)
				return
			}
		}

		if err := key_value.Delete(store, "foo"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		if exists, err := key_value.Exists(store, "foo"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else if exists {
			http.Error(w, "key was not deleted as expected", http.StatusInternalServerError)
			return
		}
	})
}

func main() {}
