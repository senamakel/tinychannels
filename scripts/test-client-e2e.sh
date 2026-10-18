#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"
if [[ "${RUNNER_OS:-}" == "Windows" ]]; then
  pwsh -NoProfile -File scripts/test-client-e2e.ps1
  exit
fi
TASK_TARGET_ROOT="$repo_root/target"
host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
  *-apple-darwin)
    library_name="libtinychannels_client_test_fixture.dylib"
    native_library_name="libtinychannels_module.dylib"
    ;;
  *-linux-gnu)
    library_name="libtinychannels_client_test_fixture.so"
    native_library_name="libtinychannels_module.so"
    ;;
  *-windows-*) echo "test-client-e2e.sh requires RUNNER_OS=Windows on a Windows runner" >&2; exit 1 ;;
  *) echo "unsupported test host: $host" >&2; exit 2 ;;
esac

fixture_manifest="$repo_root/crates/tinychannels-client/test-fixture/Cargo.toml"
fixture_library="$TASK_TARGET_ROOT/debug/$library_name"
native_module_library="$TASK_TARGET_ROOT/debug/$native_library_name"
stage="$TASK_TARGET_ROOT/tinychannels-client-e2e"
cache="$stage/cache"
archive_name="tinychannels-module-client-test.tar.gz"
archive="$stage/$archive_name"
native_stage="$TASK_TARGET_ROOT/tinychannels-module-client-native-e2e"
native_cache="$native_stage/cache"
native_archive_name="tinychannels-module-native-test.tar.gz"
native_archive="$native_stage/$native_archive_name"

cargo build --manifest-path "$fixture_manifest" --locked --target-dir "$TASK_TARGET_ROOT"
cargo build --manifest-path "$repo_root/Cargo.toml" --locked --target-dir "$TASK_TARGET_ROOT" -p tinychannels-module
rm -rf "$stage"
rm -rf "$native_stage"
mkdir -p "$cache"
mkdir -p "$native_cache"
chmod 700 "$stage" "$cache"
chmod 700 "$native_stage" "$native_cache"
install -m 755 "$fixture_library" "$cache/$library_name"
install -m 755 "$native_module_library" "$native_cache/$native_library_name"

if command -v sha256sum >/dev/null 2>&1; then
  library_sha256="$(sha256sum "$cache/$library_name" | awk '{print $1}')"
  archive_sha256() { sha256sum "$1" | awk '{print $1}'; }
else
  library_sha256="$(shasum -a 256 "$cache/$library_name" | awk '{print $1}')"
  archive_sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
fi
printf '"%s" = "%s"\n' "$library_name" "$library_sha256" > "$cache/modules.toml"
tar -C "$cache" -czf "$archive" "$library_name" modules.toml
install -m 644 "$archive" "$cache/$archive_name"
archive_hash="$(archive_sha256 "$archive")"

if command -v sha256sum >/dev/null 2>&1; then
  native_library_sha256="$(sha256sum "$native_cache/$native_library_name" | awk '{print $1}')"
else
  native_library_sha256="$(shasum -a 256 "$native_cache/$native_library_name" | awk '{print $1}')"
fi
printf '"%s" = "%s"\n' "$native_library_name" "$native_library_sha256" > "$native_cache/modules.toml"
tar -C "$native_cache" -czf "$native_archive" "$native_library_name" modules.toml
install -m 644 "$native_archive" "$native_cache/$native_archive_name"
native_archive_hash="$(archive_sha256 "$native_archive")"

export TINYCHANNELS_CLIENT_TEST_CACHE="$cache"
export TINYCHANNELS_CLIENT_TEST_ARCHIVE_NAME="$archive_name"
export TINYCHANNELS_CLIENT_TEST_ARCHIVE_SHA256="$archive_hash"
export TINYCHANNELS_CLIENT_TEST_LIBRARY_SHA256="$library_sha256"
export TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_CACHE="$native_cache"
export TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_ARCHIVE_NAME="$native_archive_name"
export TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_ARCHIVE_SHA256="$native_archive_hash"
export TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_LIBRARY_SHA256="$native_library_sha256"
export TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_RELEASE_URL="https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.37"

cargo test --manifest-path "$repo_root/Cargo.toml" --locked \
  -p tinychannels-client --features native-e2e --test native_e2e -- --test-threads=1
cargo test --manifest-path "$repo_root/Cargo.toml" --locked \
  -p tinychannels-client --features native-e2e --test runtime_native_e2e -- --test-threads=1
cargo test --manifest-path "$repo_root/Cargo.toml" --locked \
  -p tinychannels-client --features native-e2e --test native_module_runtime_e2e -- --test-threads=1
