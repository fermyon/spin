package spin

import "context"

type Controller interface {
	Name() string
	Login() error
	TemplatesInstall(args ...string) error
	New(template, appName string) error
	Build(name string) error
	InstallPlugins(plugins []string) error
	Deploy(name string, args []string, metadataFetcher func(appname, logs string) (*Metadata, error)) (*Metadata, error)
	PollForLatestVersion(ctx context.Context, metadata *Metadata) error
	StopApp(appname string) error
}
