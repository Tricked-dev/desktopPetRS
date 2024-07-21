#!/bin/bash

# Array of build targets
targets=(
    "x86_64-pc-windows-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-unknown-linux-gnu"
)
bin="desktopPetRS"

pids=()

# Run each build command in the background and store its PID
for target in "${targets[@]}"; do
    cross +nightly build --target $target --release &
    pids+=($!)
done

# Wait for all background jobs to complete
for pid in "${pids[@]}"; do
    wait $pid
done

rm -rf build
mkdir -p build

for target in "${targets[@]}"; do
    if [ -f "target/$target/release/$bin" ]; then
        7z a build/$target.zip ./target/$target/release/$bin
    fi
    if [ -f "target/$target/release/$bin.exe" ]; then
        7z a build/$target.zip ./target/$target/release/$bin.exe
    fi
done

echo Done