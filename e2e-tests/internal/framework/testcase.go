package framework

import (
	"fmt"
	"os/exec"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

type SkipCondition struct {
	Env    string
	Reason string
}

type Testcase struct {
	Name                string
	AppName             string
	Plugins             []string
	Template            string
	TemplateInstallArgs []string
	PreBuildHooks       []*exec.Cmd
	DeployArgs          []string
	SkipConditions      []SkipCondition
	MetadataFetcher     func(appname, logs string) (*spin.Metadata, error)
	SubTestsExecutor    func(t *testing.T, metadata *spin.Metadata)
}

func (tc *Testcase) ShouldSkip(cloud spin.Controller) (string, bool) {
	if len(tc.SkipConditions) == 0 {
		return "", false
	}

	for _, s := range tc.SkipConditions {
		if s.Env == cloud.Name() {
			return s.Reason, true
		}
	}

	return "", false
}

func runCmds(appname string, cmds ...*exec.Cmd) error {
	for _, cmd := range cmds {
		if cmd.Dir == "" {
			// run in context of app
			cmd.Dir = appname
		}

		err := cmd.Run()
		if err != nil {
			return fmt.Errorf("running %s in context of testing app %s: %w", cmd.Path, appname, err)
		}
	}

	return nil
}
