package spin

import (
	"bytes"
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"sync"
	"time"
)

const SpinUp = "using-spin-up"

type usespinup struct {
	cmds map[string]*exec.Cmd
	sync.Mutex
}

func WithSpinUp() Controller {
	return &usespinup{
		cmds: map[string]*exec.Cmd{},
	}
}

func (o *usespinup) Name() string {
	return SpinUp
}

func (o *usespinup) Login() error {
	//no op when running app using spin up
	return nil
}

func (o *usespinup) TemplatesInstall(args ...string) error {
	return templatesInstall(args...)
}

func (o *usespinup) New(template, appName string) error {
	return new(template, appName)
}

func (o *usespinup) Build(appName string) error {
	return build(appName)
}

func (o *usespinup) Deploy(name string, additionalArgs []string, metadataFetcher func(appname, logs string) (*Metadata, error)) (*Metadata, error) {
	port, err := getFreePort()
	if err != nil {
		return nil, err
	}

	args := []string{"up", "--listen", fmt.Sprintf("127.0.0.1:%d", port)}
	args = append(args, additionalArgs...)

	cmd := exec.Command("spin", args...)
	cmd.Dir = name
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err = cmd.Start()
	if err != nil {
		return nil, fmt.Errorf("running: %s\nstdout:%s\nstderr:%s\n: %w", cmd.String(), stdout.String(), stderr.String(), err)
	}

	o.Lock()
	o.cmds[name] = cmd
	o.Unlock()

	// TODO(rajat): make this dynamic instead of static sleep
	time.Sleep(10 * time.Second)
	return metadataFetcher(name, stdout.String())
}

func (o *usespinup) StopApp(appname string) error {
	o.Lock()
	cmd := o.cmds[appname]
	o.Unlock()

	defer func(o *usespinup, appname string) {
		o.Lock()
		delete(o.cmds, appname)
		o.Unlock()
	}(o, appname)

	if cmd.Process == nil {
		return nil
	}

	err := cmd.Process.Signal(os.Interrupt)
	if err != nil {
		return err
	}

	status, err := cmd.Process.Wait()
	if err != nil {
		return err
	}

	if status.Exited() {
		return nil
	}

	// last option to kill
	return cmd.Process.Kill()
}

// with spin up, we always get latest version
func (o *usespinup) PollForLatestVersion(ctx context.Context, metadata *Metadata) error {
	return nil
}

func getFreePort() (int, error) {
	addr, err := net.ResolveTCPAddr("tcp", "localhost:0")
	if err != nil {
		return 0, err
	}

	l, err := net.ListenTCP("tcp", addr)
	if err != nil {
		return 0, err
	}
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port, nil
}

func (o *usespinup) InstallPlugins(plugins []string) error {
	return installPlugins(plugins...)
}
