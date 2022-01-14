#!/bin/bash
cargo fmt -- --check --color always
cargo clippy --all-targets -- -D warnings
cargo test