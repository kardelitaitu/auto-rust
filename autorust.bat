@echo off
pushd "%~dp0"
cargo run --release --bin auto -- %*
popd
