#!/usr/bin/env bash
# TODO(factors): Remove after enabling CI for factors branch

cargo test -p '*factor*' -p spin-trigger -p spin-trigger-http
