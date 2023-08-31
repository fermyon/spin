# Example Plugin

This `example.sh` script acts as an example Spin plugin for testing Spin plugin functionality.
It is referenced in the `spin plugins` [integration tests](../integration.rs)

To recreate:

1. Package and zip it by running `tar czvf example.tar.gz example`.
2. Get checksum: `shasum -a 256 example.tar.gz`.
3. Modify plugin manifest in the tests to use the correct checksum.
