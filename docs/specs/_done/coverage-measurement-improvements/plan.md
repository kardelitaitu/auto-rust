# Plan

## Step 1

Document the current tarpaulin baseline and the desired coverage outputs.

## Step 2

Add a `cargo-llvm-cov` measurement path that can produce HTML and machine-readable output.

## Step 3

Wire the CI job to run the coverage path and fail when the threshold is below 40%.

## Step 4

Emit a stable summary artifact for trend tracking and keep the local command easy to run.

## Step 5

Validate the workflow with the repo checks and the coverage command itself.
