package framework

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func (testcase *Testcase) Run(t *testing.T, controller spin.Controller) {
	template := testcase.Template
	appName := testcase.AppName

	if reason, skip := testcase.ShouldSkip(controller); skip {
		t.Skip(reason)
	}

	//install required plugins if any
	if len(testcase.Plugins) > 0 {
		err := controller.InstallPlugins(testcase.Plugins)
		require.NoError(t, err)
	}

	//install templates again if template install args provided
	if len(testcase.TemplateInstallArgs) > 0 {
		err := controller.TemplatesInstall(testcase.TemplateInstallArgs...)
		require.NoError(t, err)
	}

	//create new app from template
	if template != "" {
		appName = testcase.AppName

		err := controller.New(template, appName)
		require.NoError(t, err)
	}

	if len(testcase.PreBuildHooks) > 0 {
		err := runCmds(appName, testcase.PreBuildHooks...)
		require.NoError(t, err)
	}

	//build the app
	err := controller.Build(appName)
	require.NoError(t, err)

	fetcher := spin.ExtractMetadataFromLogs
	if testcase.MetadataFetcher != nil {
		fetcher = testcase.MetadataFetcher
	}

	//deploy it
	defer func(appName string) {
		err := controller.StopApp(appName)
		if err != nil {
			fmt.Printf("failed to stop app %s. err: %v\n", appName, err)
		}
	}(appName)
	metadata, err := controller.Deploy(appName, testcase.DeployArgs, fetcher)
	require.NoError(t, err)
	require.NotNil(t, metadata)

	//wait for latest version
	ctx, cancelFunc := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancelFunc()

	err = controller.PollForLatestVersion(ctx, metadata)
	require.NoError(t, err)

	//run app specific tests
	testcase.SubTestsExecutor(t, metadata)
}
