# Testing Framework

The testing framework is a general framework for running tests against a Spin compliant runtime. The testing framework includes the ability to start up dependent services (e.g., Redis or MySQL).

## Services

Services allow for tests to be run against external sources. The service definitions can be found in the 'services' directory. Each test directory contains a 'services' file that configures the tests services. Each line of the services file should contain the name of a services file that needs to run. For example, the following 'services' file will run the `tcp-echo.py` service:

```txt
tcp-echo
```

Each service is run under a file lock meaning that all other tests that require that service must wait until the current test using that service has finished.

The following service types are supported:
* Python services (a python script ending in the .py file extension)
* Docker services (a docker file ending in the .Dockerfile extension)

When looking to add a new service, always prefer the Python based service as it's generally much quicker and lighter weight to run a Python script than a Docker container. Only use Docker when the service you require is not possible to achieve in cross platform way as a Python script.

### Signaling Service Readiness

Services can signal that they are ready so that tests aren't run against them until they are ready:

* Python: Python services signal they are ready by printing `READY` to stdout.
* Docker: Docker services signal readiness by exposing a Docker health check in the Dockerfile (e.g., `HEALTHCHECK --start-period=4s --interval=1s CMD  /usr/bin/mysqladmin ping --silent`)

### Exposing Ports

Both Docker and Python based services can expose some logical port number that will be mapped to a random free port number at runtime.

* Python: Python based services can do this by printing `PORT=($PORT1, $PORT2)` to stdout where the $PORT1 is the logical port the service exposes and $PORT2 is the random port actually being exposed (e.g., `PORT=(80, 59392)`)
* Docker: Docker services can do this by exposing the port in their Dockerfile (e.g., `EXPOSE 3306`)
