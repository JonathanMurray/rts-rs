#!/usr/bin/env bash

set -ex

cargo fmt
cargo clippy --all-targets
cargo test