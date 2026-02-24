#!/bin/sh
cargo build --profile=release
cp target/release/openaiclient ~/.local/bin

