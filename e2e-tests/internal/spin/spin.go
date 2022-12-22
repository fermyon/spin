package spin

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"time"

	"github.com/fluxcd/pkg/lockedfile"
)

type GetDeviceCodeOutput struct {
	DeviceCode      string `json:"deviceCode"`
	UserCode        string `json:"userCode"`
	VerificationURL string `json:"verificationUrl"`
	ExpiredIn       int    `json:"expiresIn"`
	Interval        int    `json:"interval"`
}

func generateDeviceCode(cloudLink string) (*GetDeviceCodeOutput, error) {
	cmd := exec.Command("spin", "login", "--url", cloudLink, "--get-device-code")
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := runCmd(cmd)
	if err != nil {
		return nil, err
	}

	dc := &GetDeviceCodeOutput{}
	err = json.Unmarshal(stdout.Bytes(), dc)
	if err != nil {
		return nil, err
	}

	return dc, nil
}

func checkDeviceCode(cloudLink, deviceCode string) error {
	return runCmd(exec.Command("spin", "login", "--url", cloudLink, "--check-device-code", deviceCode))
}

func templatesInstall(additionalArgs ...string) error {
	args := []string{"templates", "install"}
	args = append(args, additionalArgs...)

	return runCmd(exec.Command("spin", args...))
}

func new(template, appName string) error {
	err := os.RemoveAll(appName)
	if err != nil {
		return err
	}

	return runCmd(exec.Command("spin", "new", template, appName, "--accept-defaults"))
}

func build(appName string) error {
	cmd := exec.Command("spin", "build")
	cmd.Dir = appName
	return runCmd(cmd)
}

func installPlugins(plugins ...string) error {
	err := pullPluginsMeta()
	if err != nil {
		return err
	}

	for _, pluginName := range plugins {
		err := installPlugin(pluginName)
		if err != nil {
			return err
		}
	}

	return nil
}

func installPlugin(name string) error {
	return runCmd(exec.Command("spin", "plugin", "install", name, "--yes"))
}

func pullPluginsMeta() error {
	// when running  concurrently getting error
	// fatal: destination path '/home/runner/.local/share/spin/plugins/.spin-plugins' already exists
	// and is not an empty directory.
	ctx, cancelFunc := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancelFunc()

	unlockFunc, err := waitForLock(ctx, "spin-plugin-update.lock")
	if err != nil {
		return err
	}
	defer unlockFunc()

	return runCmd(exec.Command("spin", "plugin", "update"))
}

func waitForLock(ctx context.Context, lockfile string) (func(), error) {
	pollTicker := time.NewTicker(2 * time.Second)
	defer pollTicker.Stop()

	for {
		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("timedout waiting for lock")
		case <-pollTicker.C:
			lock := lockedfile.MutexAt(lockfile)
			unlockFunc, err := lock.Lock()
			if err != nil {
				continue
			}

			return unlockFunc, nil
		}
	}
}

func runCmd(cmd *exec.Cmd) error {
	var stdout, stderr bytes.Buffer
	stdoutWriters := []io.Writer{&stdout}
	stderrWriters := []io.Writer{&stderr}

	if cmd.Stdout != nil {
		stdoutWriters = append(stdoutWriters, cmd.Stdout)
	}

	if cmd.Stderr != nil {
		stderrWriters = append(stderrWriters, cmd.Stderr)
	}

	cmd.Stderr = io.MultiWriter(stderrWriters...)
	cmd.Stdout = io.MultiWriter(stdoutWriters...)

	err := cmd.Run()
	if err != nil {
		return fmt.Errorf("running: %s\nstdout:%s\nstderr:%s\n: %w", cmd.String(), stdout.String(), stderr.String(), err)
	}

	return nil
}
