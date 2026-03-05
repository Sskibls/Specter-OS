#!/usr/bin/env bash
set -euo pipefail

echo "[config-schema] validating placeholder files exist"
test -f interfaces/schema/phantomkernel-config-v1.json
test -f editions/shared/defaults/default.toml
test -f editions/debian/defaults/debian.toml
test -f editions/fedora/defaults/fedora.toml

echo "[config-schema] placeholder validation passed"

