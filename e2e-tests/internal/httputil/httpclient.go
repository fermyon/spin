package httputil

import (
	"encoding/json"
	"io"
	"net/http"
	"time"
)

func ParseInto(resp *http.Response, obj interface{}) error {
	body, err := BodyRaw(resp)
	if err != nil {
		return err
	}

	return json.Unmarshal(body, obj)
}

func BodyString(resp *http.Response) (string, error) {
	raw, err := BodyRaw(resp)
	if err != nil {
		return "", err
	}
	return string(raw), nil
}

func BodyRaw(resp *http.Response) ([]byte, error) {
	defer resp.Body.Close()
	raw, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	return raw, nil
}

func Get(url string) (*http.Response, error) {
	return client().Get(url)
}

func client() *http.Client {
	return &http.Client{
		Timeout: 2 * time.Second,
	}
}
