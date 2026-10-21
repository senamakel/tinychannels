#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --locked -p tinychannels-module --lib

target_directory="$(cargo metadata --no-deps --format-version=1 --manifest-path "$repo_root/Cargo.toml" \
  | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"

case "$(uname -s)" in
  Darwin*) artifact="$target_directory/debug/libtinychannels_module.dylib" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT) artifact="$target_directory/debug/tinychannels_module.dll" ;;
  *) artifact="$target_directory/debug/libtinychannels_module.so" ;;
esac

if [[ ! -f "$artifact" ]]; then
  echo "compiled TinyChannels module artifact not found: $artifact" >&2
  exit 1
fi

staging="$repo_root/target/tinychannels-module-artifact"
mkdir -p "$staging"
staged_artifact="$staging/$(basename "$artifact")"
cp "$artifact" "$staged_artifact"
if command -v sha256sum >/dev/null 2>&1; then
  digest="$(sha256sum "$staged_artifact" | awk '{print $1}')"
else
  digest="$(shasum -a 256 "$staged_artifact" | awk '{print $1}')"
fi
printf '"%s" = "%s"\n' "$(basename "$staged_artifact")" "$digest" > "$staging/modules.toml"

TINYCHANNELS_MODULE_ARTIFACT="$staged_artifact" \
  cargo test --locked -p tinychannels-module --test module_artifact --test runtime_artifact --test routing_artifact -- --ignored --nocapture
