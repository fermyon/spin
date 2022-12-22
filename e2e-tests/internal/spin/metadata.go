package spin

import (
	"fmt"
	"net/url"
	"regexp"
	"strings"
)

type Route struct {
	Name     string `json:"name"`
	RouteURL string `json:"routeURL"`
	Wildcard bool   `json:"wildcard"`
}

type Metadata struct {
	AppName   string  `json:"appName"`
	Base      string  `json:"base"`
	AppRoutes []Route `json:"appRoutes,omitempty"`
	Version   string  `json:"version"`
}

// fetches app url from deploy logs
func ExtractMetadataFromLogs(appname, logs string) (*Metadata, error) {
	metadata := &Metadata{
		AppName:   appname,
		AppRoutes: extractRoutes(appname, logs),
		Version:   extractVersion(appname, logs),
	}

	if len(metadata.AppRoutes) == 0 {
		return nil, fmt.Errorf("failed to fetch approutes %v from logs %s", metadata, logs)
	}

	u, err := url.Parse(metadata.AppRoutes[0].RouteURL)
	if err == nil {
		u.Path = ""
		metadata.Base = u.String()
	}

	return metadata, nil
}

func extractVersion(appname, logs string) string {
	re := regexp.MustCompile(fmt.Sprintf(`Uploading %s version (.*)\.\.\.`, appname))
	matches := re.FindStringSubmatch(logs)
	if len(matches) == 2 {
		return matches[1]
	}

	return ""
}

func extractRoutes(appname, logs string) []Route {
	re := regexp.MustCompile(`^\s+(.*): (https?://[^\s^\\(]+)(.*)`)
	routes := []Route{}
	routeStart := false

	lines := strings.Split(logs, "\n")
	for _, line := range lines {
		if !routeStart && strings.TrimSpace(line) != "Available Routes:" {
			continue
		}

		if !routeStart {
			routeStart = true
			continue
		}

		matches := re.FindStringSubmatch(line)
		if len(matches) >= 2 {
			route := Route{
				Name:     matches[1],
				RouteURL: matches[2],
				Wildcard: strings.TrimSpace(matches[3]) == "(wildcard)",
			}

			routes = append(routes, route)
		}
	}

	return routes
}

func (m *Metadata) GetRouteWithName(name string) Route {
	for _, r := range m.AppRoutes {
		if r.Name == name {
			return r
		}
	}

	return Route{}
}
