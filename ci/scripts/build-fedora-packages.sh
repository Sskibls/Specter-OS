#!/usr/bin/env bash
set -euo pipefail

echo "[packaging] Fedora placeholder build"
test -f packaging/manifest/packages.toml
test -d editions/fedora/packaging/rpm

