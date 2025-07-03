#!/bin/bash

# Ensure the build script has been run
if [ ! -f "./target/release/benchmark_s4_kowalski" ]; then
    echo "Kowalski executable not found. Please run ./build.sh first." >&2
    exit 1
fi

# Run Kowalski benchmark
echo "Running Kowalski benchmark for Scenario 4..."
./target/release/benchmark_s4_kowalski > kowalski_s4_output.log 2>&1

if [ $? -eq 0 ]; then
    echo "Kowalski benchmark finished. Output in kowalski_s4_output.log"
else
    echo "Kowalski benchmark failed. Check kowalski_s4_output.log for details." >&2
fi

# Run LangChain benchmark
echo "Running LangChain benchmark for Scenario 4..."
python3 run_langchain.py > langchain_s4_output.log 2>&1

if [ $? -eq 0 ]; then
    echo "LangChain benchmark finished. Output in langchain_s4_output.log"
else
    echo "LangChain benchmark failed. Check langchain_s4_output.log for details." >&2
fi
