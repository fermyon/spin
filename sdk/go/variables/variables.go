package variables

// Get an application variable value for the current component.
//
// The name must match one defined in in the component manifest.
func Get(key string) (string, error) {
	return get(key)
}
