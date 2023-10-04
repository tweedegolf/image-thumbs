#!/bin/bash

set -e
set -o pipefail

cargo check
cargo fmt --check
cargo clippy -- -Dwarnings
cargo test