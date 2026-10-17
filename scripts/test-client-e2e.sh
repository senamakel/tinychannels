#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
TASK_TARGET_ROOT="$repo_root/target"
host="$(rustc -vV | sed -n 's/^host: //p')"
case "$host" in
  *-apple-darwin) library_name="libtinychannels_client_test_fixture.dylib" ;;
  *-linux-gnu) library_name="libtinychannels_client_test_fixture.so" ;;
  *-windows-*) echo "test-client-e2e.sh requires a Unix runner" >&2; exit 1 ;;
  *) echo "unsupported test host: $host" >&2; exit 2 ;;
esac

fixture_manifest="$repo_root/crates/tinychannels-client/test-fixture/Cargo.toml"
fixture_library="$TASK_TARGET_ROOT/debug/$library_name"
stage="$TASK_TARGET_ROOT/tinychannels-client-e2e"
cache="$stage/cache"
archive_name="tinychannels-module-client-test.tar.gz"
archive="$stage/$archive_name"

cargo build --manifest-path "$fixture_manifest" --locked --target-dir "$TASK_TARGET_ROOT"
rm -rf "$stage"
mkdir -p "$cache"
install -m 755 "$fixture_library" "$cache/$library_name"

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

TINYCHANNELS_CLIENT_TEST_CACHE="$cache" \
TINYCHANNELS_CLIENT_TEST_ARCHIVE_NAME="$archive_name" \
TINYCHANNELS_CLIENT_TEST_ARCHIVE_SHA256="$archive_hash" \
TINYCHANNELS_CLIENT_TEST_LIBRARY_SHA256="$library_sha256" \
cargo test --manifest-path "$repo_root/Cargo.toml" --locked \
  -p tinychannels-client --features native-e2e --test native_e2e -- --test-threads=1
