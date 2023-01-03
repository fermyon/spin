package fermyon

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

type App struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	StorageID   string    `json:"storageId"`
	Description string    `json:"description"`
	Channels    []Channel `json:"channels"`
}

type Channel struct {
	ID                   string    `json:"id"`
	Name                 string    `json:"name"`
	ActiveRevisionNumber string    `json:"activeRevisionNumber"`
	Domain               string    `json:"domain"`
	Created              time.Time `json:"created"`
}
type GetAppsResponse struct {
	Apps       []App `json:"items"`
	TotalItems int   `json:"totalItems"`
	PageIndex  int   `json:"pageIndex"`
	PageSize   int   `json:"pageSize"`
	IsLastPage bool  `json:"isLastPage"`
}

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

func getAllApps(cloudLink, apiToken string) ([]App, error) {
	req, err := http.NewRequest(http.MethodGet, fmt.Sprintf("%s/api/apps", cloudLink), nil)
	if err != nil {
		return nil, err
	}

	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", apiToken))

	client := http.Client{
		Timeout: 10 * time.Second,
	}

	rawresp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer rawresp.Body.Close()

	rawbody, err := io.ReadAll(rawresp.Body)
	if err != nil {
		return nil, err
	}

	if rawresp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("error getting apps for user. Expected status code: %d, got: %d. Body: %s", http.StatusOK, rawresp.StatusCode, string(rawbody))
	}

	var resp GetAppsResponse
	err = json.Unmarshal(rawbody, &resp)
	if err != nil {
		return nil, err
	}

	return resp.Apps, nil
}

func getAppIdWithName(cloudLink, apiToken, name string) (string, error) {
	apps, err := getAllApps(cloudLink, apiToken)
	if err != nil {
		return "", err
	}

	for _, app := range apps {
		if app.Name == name {
			return app.ID, nil
		}
	}

	return "", fmt.Errorf("no app found with name %s", name)
}

func deleteAppById(cloudLink, apiToken, appId string) error {
	req, err := http.NewRequest(http.MethodDelete, fmt.Sprintf("%s/api/apps/%s", cloudLink, appId), nil)
	if err != nil {
		return err
	}

	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", apiToken))

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

func DeleteAppByName(cloudLink, apiToken, appName string) error {
	appId, err := getAppIdWithName(cloudLink, apiToken, appName)
	if err != nil {
		return err
	}

	return deleteAppById(cloudLink, apiToken, appId)
}

func DeleteAllApps(cloudLink, apiToken string) error {
	apps, err := getAllApps(cloudLink, apiToken)
	if err != nil {
		return err
	}

	var wg sync.WaitGroup
	for _, app := range apps {
		wg.Add(1)

		go func(appId string) {
			defer wg.Done()
			err := deleteAppById(cloudLink, apiToken, appId)
			if err != nil {
				fmt.Println(err)
			}
		}(app.ID)
	}

	wg.Wait()
	return nil
}
