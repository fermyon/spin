package http

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"os/exec"
	"testing"
	"time"
)

const spin_binary = "../../target/debug/spin"

func retryGet(t *testing.T, url string) *http.Response {
	t.Helper()

	const tries = 10
	for i := 1; i < tries; i++ {
		if res, err := http.Get(url); err != nil {
			t.Log(err)
		} else {
			return res
		}
		time.Sleep(3 * time.Second)
	}
	t.Fatal("Get request timeout: ", url)
	return nil
}

type testSpin struct {
	cancel func()
	url    string
	cmd    *exec.Cmd
}

func startSpin(t *testing.T, spinfile string) *testSpin {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)

	url := getFreePort(t)

	cmd := exec.CommandContext(ctx, spin_binary, "up", "--file", spinfile, "--listen", url)
	stderr := new(bytes.Buffer)
	cmd.Stderr = stderr
	if err := cmd.Start(); err != nil {
		t.Log(stderr.String())
		t.Fatal(err)
	}

	return &testSpin{
		cancel: cancel,
		url:    fmt.Sprintf("http://%s", url),
		cmd:    cmd,
	}
}

func buildTinyGo(t *testing.T, dir string) {
	t.Helper()

	t.Log("building example: ", dir)

	cmd := exec.Command("tinygo", "build", "-wasm-abi=generic", "-target=wasi", "-gc=leaking", "-o", "main.wasm", "main.go")
	cmd.Dir = dir

	stderr := new(bytes.Buffer)
	cmd.Stderr = stderr
	if err := cmd.Run(); err != nil {
		t.Log(stderr.String())
		t.Errorf("Failed to build %q, %v", dir, err)
	}
}

func TestHTTPTriger(t *testing.T) {
	buildTinyGo(t, "http/testdata/http-tinygo")
	spin := startSpin(t, "http/testdata/http-tinygo/spin.toml")
	defer spin.cancel()

	resp := retryGet(t, spin.url+"/hello")
	spin.cancel()
	if resp.Body == nil {
		t.Fatal("body is nil")
	}
	t.Log(resp.Status)
	b, err := io.ReadAll(resp.Body)
	resp.Body.Close()
	if err != nil {
		t.Fatal(err)
	}

	// assert response body
	want := "Hello world!\n"
	got := string(b)
	if want != got {
		t.Fatalf("body is not equal: want = %q got = %q", want, got)
	}

	// assert response header
	if resp.Header.Get("foo") != "bar" {
		t.Fatal("header 'foo' was not set")
	}
}

// TestBuildExamples ensures that the tinygo examples will build successfully.
func TestBuildExamples(t *testing.T) {
	for _, example := range []string{
		"../../examples/config-tinygo",
		"../../examples/http-tinygo",
		"../../examples/http-tinygo-outbound-http",
		"../../examples/tinygo-outbound-redis",
		"../../examples/tinygo-redis",
	} {
		buildTinyGo(t, example)
	}
}

func getFreePort(t *testing.T) string {
	t.Helper()

	a, err := net.ResolveTCPAddr("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal("failed to get free port: ", err)
	}

	l, err := net.ListenTCP("tcp", a)
	if err != nil {
		t.Fatal("failed to get free port: ", err)
	}
	l.Close()
	return l.Addr().String()
}
