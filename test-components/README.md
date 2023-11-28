# Test Components

Test components for use in runtime testing. Each test component has a README on what it tests.

## Contract

Test components have the following contract with the outside world:

* They do not look at the incoming request.
* If nothing errors a 200 with no body will be returned.
* If an error occurs a 500 with a body describing the error will be returned.
