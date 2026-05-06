# Plan

## Step 1

Record the current TwitterActivity contract drift and decide the source of truth for duration and scroll count.

## Step 2

Tighten payload parsing so malformed numeric values fail validation instead of falling back silently.

## Step 3

Wire the scroll budget into the TwitterActivity runtime so the task actually honors `scroll_count`.

## Step 4

Add deterministic tests for default duration resolution, explicit overrides, malformed payloads, and scroll budget behavior.

## Step 5

Update the task docs and examples so the public contract matches the runtime behavior.

## Step 6

Run the focused checks first, then the full gate.
