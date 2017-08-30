#! /usr/bin/bash

# sandybridge - cpu on test servers
# incremental turned off because it conflicts with LTO (see https://github.com/rust-lang/rust/issues/42439)
RUSTFLAGS='-C target-cpu=sandybridge' CARGO_INCREMENTAL=0 cargo build --release
