#!/bin/bash

# Build the Kowalski (Rust) benchmark executable
echo "Building Kowalski benchmark for Scenario 1..."
cargo build --release --manifest-path ./Cargo.toml

if [ $? -eq 0 ]; then
    echo "Kowalski benchmark build successful."
else
    echo "Kowalski benchmark build failed." >&2
    exit 1
fi
