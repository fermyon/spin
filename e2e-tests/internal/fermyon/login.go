package fermyon

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/fermyon/spin/e2e-tests/internal/uidriver"
	"github.com/sirupsen/logrus"
	"github.com/tebeka/selenium"
	"github.com/xlzd/gotp"
)

type Token struct {
	Token      string    `json:"token"`
	Expiration time.Time `json:"expiration"`
}

func LoginWithGithub(cloudLink string, username, password string) (string, error) {
	ui, err := uidriver.New()
	if err != nil {
		return "", fmt.Errorf("connecting to selenium: %w", err)
	}

	defer func(ui *uidriver.Driver) {
		screenshot, err := ui.WebDriver.Screenshot()
		if err != nil {
			logrus.Warnf("capturing screenshot: %v", err)
		}

		err = os.WriteFile("screenshot.png", screenshot, 0644)
		if err != nil {
			logrus.Warnf("saving screenshot: %v", err)
		}

		ui.WebDriver.Close()
		ui.WebDriver.Quit()
	}(ui)

	logrus.Infof("opening Fermyon cloud at %s", cloudLink)
	err = ui.WebDriver.Get(cloudLink)
	if err != nil {
		return "", err
	}

	logrus.Infof("clicking on login with github")
	el, err := ui.WebDriver.FindElement(selenium.ByXPATH, "//button/span[text()='Login with GitHub']")
	if err != nil {
		return "", err
	}

	err = el.Click()
	if err != nil {
		return "", err
	}

	logrus.Infof("Entering creds on github login page")
	el, err = ui.WebDriver.FindElement(selenium.ByID, "login_field")
	if err != nil {
		return "", err
	}

	err = el.SendKeys(username)
	if err != nil {
		return "", err
	}

	el, err = ui.WebDriver.FindElement(selenium.ByID, "password")
	if err != nil {
		return "", err
	}

	err = el.SendKeys(password)
	if err != nil {
		return "", err
	}

	el, err = ui.WebDriver.FindElement(selenium.ByName, "commit")
	if err != nil {
		return "", err
	}

	err = el.Click()
	if err != nil {
		return "", err
	}

	logrus.Infof("handling diff auth challenges offered by Github")
	err = handle2FA(ui)
	if err != nil {
		return "", err
	}

	logrus.Infof("login with github completed successfully !")
	//wait for signout button on Fermyon cloud
	_, err = ui.WebDriver.FindElement(selenium.ByXPATH, "//app-user-menu")
	if err != nil {
		return "", err
	}

	logrus.Infof("Getting cloud api token")
	raw, err := ui.WebDriver.ExecuteScript("return localStorage.getItem('token');", nil)
	if err != nil {
		return "", err
	}

	token := &Token{}
	err = json.Unmarshal([]byte(raw.(string)), token)
	if err != nil {
		return "", err
	}

	return token.Token, nil
}

func handle2FA(ui *uidriver.Driver) error {
	el, err := ui.WebDriver.FindElement(selenium.ByID, "totp")
	if err != nil {
		return err
	}

	otp := gotp.NewDefaultTOTP(os.Getenv("GH_TOTP_SECRET")).Now()
	err = el.SendKeys(otp)
	if err != nil {
		return err
	}
	return nil
}
