package spin

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"time"

	"github.com/fermyon/spin/e2e-tests/internal/fermyon"
	"github.com/fermyon/spin/e2e-tests/internal/httputil"
	"github.com/sirupsen/logrus"
)

const FermyonCloud = "fermyon-cloud"

// Run on Fermyon cloud
type onFermyonCloud struct{}

func WithFermyonCloud() Controller {
	return &onFermyonCloud{}
}

func (o *onFermyonCloud) Name() string {
	return FermyonCloud
}

func (o *onFermyonCloud) TemplatesInstall(args ...string) error {
	return templatesInstall(args...)
}

func (o *onFermyonCloud) New(template, appName string) error {
	return new(template, appName)
}

func (o *onFermyonCloud) Build(appName string) error {
	return build(appName)
}

func (o *onFermyonCloud) Deploy(name string, additionalArgs []string, metadataFetcher func(appname, logs string) (*Metadata, error)) (*Metadata, error) {
	args := []string{"deploy"}
	args = append(args, additionalArgs...)

	var stdout, stderr bytes.Buffer
	cmd := exec.Command("spin", args...)
	cmd.Dir = name
	cmd.Env = os.Environ()
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := runCmd(cmd)
	if err != nil {
		return nil, err
	}

	return metadataFetcher(name, stdout.String())
}

func (o *onFermyonCloud) Login() error {
	cloudLink := fermyon.GetCloudLink(os.Getenv("environment"))

	code, err := generateDeviceCode(cloudLink)
	if err != nil {
		return fmt.Errorf("generating device code %w", err)
	}

	apiToken, err := fermyon.LoginWithGithub(cloudLink, os.Getenv("GH_USERNAME"), os.Getenv("GH_PASSWORD"))
	if err != nil {
		return fmt.Errorf("login with Github to Fermyon cloud: %w", err)
	}

	err = fermyon.ActivateDeviceCode(cloudLink, apiToken, code.UserCode)
	if err != nil {
		return fmt.Errorf("activating device code: %w", err)
	}

	err = checkDeviceCode(cloudLink, code.DeviceCode)
	if err != nil {
		return fmt.Errorf("checking device code: %w", err)
	}

	logrus.Info("device authorized successfully with Fermyon Cloud")
	return nil
}

func (o *onFermyonCloud) StopApp(appname string) error {
	return runCmd(exec.Command("fermyon", "apps", "delete", appname))
}

// TODO(rjindal): verify with https://github.com/fermyon/spin/pull/870
func (o *onFermyonCloud) PollForLatestVersion(ctx context.Context, metadata *Metadata) error {
	pollTicker := time.NewTicker(2 * time.Second)
	defer pollTicker.Stop()

	var lastError error
	for {
		select {
		case <-ctx.Done():
			return fmt.Errorf("timedout waiting for latest version %w", lastError)
		case <-pollTicker.C:
			currentMeta, err := GetMetadata(metadata.Base)
			if err != nil {
				lastError = fmt.Errorf("fetching metadata %w", err)
				continue
			}

			if metadata.Version != currentMeta.Version {
				lastError = fmt.Errorf("expected version: %s, got version: %s from %s", metadata.Version, currentMeta.Version, metadata.Base)
				continue
			}

			return nil
		}
	}
}

func GetMetadata(approute string) (*Metadata, error) {
	resp, err := httputil.Get(fmt.Sprintf("%s/.well-known/spin/info", approute))
	if err != nil {
		return nil, err
	}

	actualMeta := Metadata{}
	err = httputil.ParseInto(resp, &actualMeta)
	return &actualMeta, err
}

func (o *onFermyonCloud) InstallPlugins(plugins []string) error {
	return installPlugins(plugins...)
}
