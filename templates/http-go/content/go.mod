module github.com/{{project-name | snake_case}}

go 1.17

require github.com/fermyon/spin/sdk/go v0.0.0

replace github.com/fermyon/spin/sdk/go v0.0.0 => ../../sdk/go/
