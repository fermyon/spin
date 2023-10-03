# Extending Spin with an orchestrator and application

To test:

* `cargo build --release`
* Copy: `cp ./target/release/trigger-orchestrator .`
* `tar czvf trigger-orchestrator.tar.gz trigger-orchestrator`
* Update the plugin manifest (`trigger-orchestrator.json`):
  * Get the SHA: `shasum -a 256 trigger-orchestrator.tar.gz` and copy it into the `sha256` field
  * Update the URL too, to reflect the directory where the tar file is
* `spin plugin install --file ./trigger-orchestrator.json --yes`
# (cargo build --release) && (cp ./target/release/trigger-orchestrator .) && (tar czvf trigger-orchestrator.tar.gz trigger-orchestrator) && (shasum -a 256 trigger-orchestrator.tar.gz)
Then you should be able to `spin build --up` the guest.
