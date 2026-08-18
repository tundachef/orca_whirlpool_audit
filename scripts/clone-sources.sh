#!/usr/bin/env bash
# Re-clone Orca public sources into ./sources (full history).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/sources"
cd "$ROOT/sources"

clone() {
  local url="$1" dir="$2"
  if [[ -d "$dir/.git" ]]; then
    echo "updating $dir"
    git -C "$dir" fetch --tags --prune
    git -C "$dir" pull --ff-only || true
  else
    echo "cloning $dir"
    git clone "$url" "$dir"
  fi
}

clone https://github.com/orca-so/whirlpools.git whirlpools
clone https://github.com/orca-so/xorca.git xorca
clone https://github.com/orca-so/typescript-sdk.git typescript-sdk
clone https://github.com/orca-so/aquafarm-sdk.git aquafarm-sdk

echo "done. whirlpools commits: $(git -C whirlpools rev-list --count HEAD)"
echo "osec pin: e5f089bc5c49b01f5c8abb43c78457ab6c440568"
