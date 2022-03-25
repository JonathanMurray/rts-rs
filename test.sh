#!/usr/bin/env bash

set -ex

cargo clippy --all-targets
cargo fmt
