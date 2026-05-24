#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "repository hygiene failed: $*" >&2
  exit 1
}

[ -f Cargo.toml ] || fail "missing Cargo.toml"
[ -f README.md ] || fail "missing README.md"
[ -f ARCHITECTURE.md ] || fail "missing ARCHITECTURE.md"
[ -f USERGUIDE.md ] || fail "missing USERGUIDE.md"
[ -f SCOPE.md ] || fail "missing SCOPE.md"
[ -f ROADMAP.md ] || fail "missing ROADMAP.md"
[ -f target-xteink-x4/Cargo.toml ] || fail "missing target-xteink-x4/Cargo.toml"
[ -f target-xteink-x4/src/rustmix_x4/apps/home.rs ] || fail "missing Rustmix X4 Home app"
[ -f target-xteink-x4/src/rustmix_x4/x4_apps/apps/reader/mod.rs ] || fail "missing Rustmix X4 Reader app"

if [ -d docs ]; then
  fail "docs/ should not exist; documentation is consolidated into README.md, ARCHITECTURE.md, USERGUIDE.md, SCOPE.md, and ROADMAP.md"
fi

if find . -maxdepth 1 \
  \( -name '*.zip' \
     -o -name 'README-APPLY.md' \
     -o -name 'MANIFEST.txt' \
     -o -name '*_repair' \
     -o -name '*_restore' \
     -o -name '*_cleanup' \
     -o -name '*_contract' \
     -o -name '*_reset' \
     -o -name '*_overlay' \
     -o -name '*_fix' \
     -o -name '*_polish' \
     -o -name '*_rollout' \
     -o -name '*_parity' \
  \) -print | grep -q .; then
  find . -maxdepth 1 \
    \( -name '*.zip' \
       -o -name 'README-APPLY.md' \
       -o -name 'MANIFEST.txt' \
       -o -name '*_repair' \
       -o -name '*_restore' \
       -o -name '*_cleanup' \
       -o -name '*_contract' \
       -o -name '*_reset' \
       -o -name '*_overlay' \
       -o -name '*_fix' \
       -o -name '*_polish' \
       -o -name '*_rollout' \
       -o -name '*_parity' \
    \) -print >&2
  fail "generated root delivery artifacts remain"
fi

if find scripts -maxdepth 1 -type f \
  \( -name 'patch_*' -o -name 'apply_*' -o -name 'cleanup_*' \) -print | grep -q .; then
  find scripts -maxdepth 1 -type f \
    \( -name 'patch_*' -o -name 'apply_*' -o -name 'cleanup_*' \) -print >&2
  fail "generated patch/apply/cleanup scripts remain"
fi

if find . \
  \( -path './.git' -o -path './target' -o -path './vendor' -o -path './dist' \) -prune -o \
  \( -name '__pycache__' -o -name '*.pyc' -o -name '__MACOSX' -o -name '.DS_Store' \) -print | grep -q .; then
  find . \
    \( -path './.git' -o -path './target' -o -path './vendor' -o -path './dist' \) -prune -o \
    \( -name '__pycache__' -o -name '*.pyc' -o -name '__MACOSX' -o -name '.DS_Store' \) -print >&2
  fail "generated cache or OS metadata remains"
fi

if rg -n 'pulp_os::|package = "x4-os"|x4-kernel =' Cargo.toml target-xteink-x4/Cargo.toml target-xteink-x4/src >/tmp/rustmix_active_pulp_refs.txt; then
  cat /tmp/rustmix_active_pulp_refs.txt >&2
  fail "active old Pulp package references remain"
fi

if rg -n -i 'phase[[:space:]_-]*[0-9]|README-APPLY|overlay|patch deliverable|deliverable zip|repair pack' \
  README.md ARCHITECTURE.md USERGUIDE.md SCOPE.md ROADMAP.md AGENTS.md .github --glob '*.md' --glob '*.yml' --glob '*.yaml' >/tmp/rustmix_doc_delivery_refs.txt; then
  cat /tmp/rustmix_doc_delivery_refs.txt >&2
  fail "delivery-history references remain in current docs or workflows"
fi

echo "repository_hygiene=ok"
