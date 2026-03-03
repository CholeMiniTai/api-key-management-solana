#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

README_FILE="README.md"
if [[ ! -f "$README_FILE" ]]; then
  echo "❌ README.md not found"
  exit 1
fi

echo "==> [1/3] Checking explorer links in README.md"
URLS="$(grep -oE 'https://explorer\.solana\.com/tx/[A-Za-z0-9]+' "$README_FILE" | awk '!seen[$0]++')"

if [[ -z "$URLS" ]]; then
  echo "❌ No explorer tx links found in README.md"
  exit 1
fi

FAIL=0
for base in $URLS; do
  url="${base}?cluster=devnet"
  code="$(curl -L -s -o /dev/null -w '%{http_code}' "$url")"
  if [[ "$code" == "200" ]]; then
    echo "✅ $code $url"
  else
    echo "❌ $code $url"
    FAIL=1
  fi
done

if [[ "$FAIL" -ne 0 ]]; then
  echo "❌ Explorer link check failed"
  exit 1
fi

echo "==> [2/3] Running anchor test"
anchor test >/tmp/anchor-test.log 2>&1 || {
  echo "❌ anchor test failed (last 80 lines):"
  tail -n 80 /tmp/anchor-test.log
  exit 1
}
echo "✅ anchor test passed"

echo "==> [3/3] Running cargo check for CLI"
cargo check -p api-key-cli >/tmp/api-key-cli-check.log 2>&1 || {
  echo "❌ cargo check -p api-key-cli failed (last 80 lines):"
  tail -n 80 /tmp/api-key-cli-check.log
  exit 1
}
echo "✅ cargo check -p api-key-cli passed"

echo "\n🎉 Submission readiness check: PASS"
