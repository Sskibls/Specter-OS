#!/usr/bin/env bash
set -euo pipefail

echo "[vm-smoke] validating service unit placeholders"
test -f editions/shared/systemd/phantomkernel.target
test -f editions/shared/systemd/phantomkernel-policyd.service

echo "[vm-smoke] running cross-edition startup/core workflow smoke tests"
cargo test -p phantomkernel-test-harness --test milestone4_runtime_smoke

echo "[vm-smoke] smoke checks passed"
