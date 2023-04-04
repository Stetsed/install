#!/bin/bash

RUSTFLAGS="-C target-cpu=native" rustc ./src/main.rs -o ./target/install
