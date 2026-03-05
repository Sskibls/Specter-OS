#!/usr/bin/env bash
set -euo pipefail

echo "[packaging] Debian placeholder build"
test -f packaging/manifest/packages.toml
test -d editions/debian/packaging/debian

