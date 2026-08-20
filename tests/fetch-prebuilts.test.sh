#!/bin/sh
# Checksum verification must match the published asset name, fail closed, and
# only then allow install as bin/chromad.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
VERIFY="$ROOT/scripts/fetch-prebuilts.sh --verify"
fail=0
ok() { printf 'ok  %s\n' "$1"; }
bad() { printf 'FAIL %s\n' "$1" >&2; fail=$((fail + 1)); }

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
printf 'hello-prebuilt\n' > "$dir/chromad-x86_64-linux"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$dir" && sha256sum chromad-x86_64-linux > SHA256SUMS)
else
  (cd "$dir" && shasum -a 256 chromad-x86_64-linux > SHA256SUMS)
fi

if sh $VERIFY "$dir" chromad-x86_64-linux >/dev/null; then
  ok "verify passes for published asset name"
else
  bad "verify should pass for chromad-x86_64-linux"
fi

cp "$dir/chromad-x86_64-linux" "$dir/chromad"
if sh $VERIFY "$dir" chromad >/dev/null 2>&1; then
  bad "verify must not pass for renamed bin/chromad against SHA256SUMS"
else
  ok "verify fails closed for renamed chromad"
fi

printf 'tampered\n' > "$dir/chromad-x86_64-linux"
if sh $VERIFY "$dir" chromad-x86_64-linux >/dev/null 2>&1; then
  bad "verify must fail on mismatch"
else
  ok "verify fails on checksum mismatch"
fi

rm -f "$dir/SHA256SUMS"
if sh $VERIFY "$dir" chromad-x86_64-linux >/dev/null 2>&1; then
  bad "verify must fail without SHA256SUMS"
else
  ok "verify fails without SHA256SUMS"
fi

printf '%s\n' "$fail failed"
[ "$fail" -eq 0 ]
