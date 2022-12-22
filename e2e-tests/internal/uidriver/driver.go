package uidriver

import (
	"fmt"
	"time"

	"github.com/tebeka/selenium"
)

type Driver struct {
	selenium.WebDriver
}

func New() (*Driver, error) {
	caps := selenium.Capabilities{
		"browserName": "chrome",
	}

	wd, err := selenium.NewRemote(caps, fmt.Sprintf("http://localhost:%d/wd/hub", 4444))
	if err != nil {
		return nil, err
	}
	wd.SetImplicitWaitTimeout(30 * time.Second)

	return &Driver{
		WebDriver: wd,
	}, nil
}
