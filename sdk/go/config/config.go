package config

// Get a configuration value for the current component.
// The config key must match one defined in in the component manifest.
func Get(key string) (string, error) {
	return get(key)
}
