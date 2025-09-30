#!/bin/sh
exec cargo run \
    --release \
    --target-dir=/tmp/rusbit-cli \
    --manifest-path $(dirname "$0")/Cargo.toml -- "$@"