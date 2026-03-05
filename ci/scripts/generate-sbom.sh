#!/usr/bin/env bash
set -euo pipefail

echo "[sbom] generating placeholder SBOM artifact"
mkdir -p packaging/sbom/out
cat > packaging/sbom/out/phantomkernel-sbom-placeholder.json <<'EOF'
{
  "name": "phantomkernel",
  "version": "0.1.0",
  "note": "placeholder artifact"
}
EOF

