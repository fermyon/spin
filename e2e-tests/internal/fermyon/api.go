package fermyon

import (
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

func ActivateDeviceCode(cloudLink, apiToken, userCode string) error {
	body := strings.NewReader(fmt.Sprintf(`{"userCode": "%s"}`, userCode))
	req, err := http.NewRequest(http.MethodPost, fmt.Sprintf("%s/api/device-codes/activate", cloudLink), body)
	if err != nil {
		return err
	}

	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", apiToken))
	req.Header.Set("Content-Type", "application/json")

	client := http.Client{
		Timeout: 10 * time.Second,
	}

	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	rawbody, err := io.ReadAll(resp.Body)
	if err != nil {
		return err
	}

	if resp.StatusCode != http.StatusNoContent {
		return fmt.Errorf("error activating user code. Expected status code: %d, got: %d. Body: %s", http.StatusNoContent, resp.StatusCode, string(rawbody))
	}

	return nil
}
