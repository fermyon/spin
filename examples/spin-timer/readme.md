# Extending Spin with a trigger and application

To test:

* `cargo build --release`
* Copy: `cp ./target/release/trigger-timer .`
* `tar czvf trigger-timer.tar.gz trigger-timer`
* Update the plugin manifest (`trigger-timer.json`):
  * Get the SHA: `shasum -a 256 trigger-timer.tar.gz` and copy it into the `sha256` field
  * Update the URL too, to reflect the directory where the tar file is
* `spin plugin install --file ./trigger-timer.json --yes`

Then you should be able to `spin build --up` the guest.
