#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo +stable fmt --all -- --check
./scripts/validate_source_contract.sh
./scripts/test-host.sh
