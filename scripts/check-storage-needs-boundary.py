#!/usr/bin/env python3
"""Guard the standalone TSD2 collection-declaration boundary."""

import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "proofs/storage-needs-consumer/Cargo.toml"
SOURCE_PREFIX = "git+https://github.com/senamakel/tinystoragedrivers?tag=v2.1.2#"
FORBIDDEN = {
    "axum",
    "hyper",
    "mongodb",
    "reqwest",
    "rusqlite",
    "sqlx",
    "tinybus",
    "tinybus-macros",
    "tinybus-module",
    "tinychannels",
    "tinychannels-module",
    "tinychannels-runtime",
    "tinystoragedrivers-busproxy",
    "tinystoragedrivers-core",
    "tinystoragedrivers-file",
    "tinystoragedrivers-mongodb",
    "tinystoragedrivers-s3",
    "tinystoragedrivers-sqlite",
    "tokio-tungstenite",
    "tokio-rustls",
}


def violations(metadata):
    """Check the resolved proof crate graph and exact declaration dependency."""
    packages = metadata.get("packages")
    if not isinstance(packages, list):
        return ["Cargo metadata has no package list"]
    errors = []
    by_name = {}
    for package in packages:
        by_name.setdefault(package.get("name"), []).append(package)
    required = {
        "tinychannels-storage-needs-consumer",
        "tinychannels-bus",
        "tinystoragedrivers-types",
    }
    for name in sorted(required):
        if len(by_name.get(name, [])) != 1:
            errors.append(f"expected one {name} package, found {len(by_name.get(name, []))}")
    for name in sorted(FORBIDDEN & by_name.keys()):
        errors.append(f"forbidden package in collection declaration graph: {name}")
    types = by_name.get("tinystoragedrivers-types", [])
    if len(types) == 1:
        package = types[0]
        if package.get("version") != "2.1.2" or not str(package.get("source", "")).startswith(SOURCE_PREFIX):
            errors.append("collection declarations must use fork TSD types v2.1.2")
    return errors


def main():
    try:
        output = subprocess.check_output(
            ["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(MANIFEST)],
            cwd=ROOT,
            text=True,
        )
        errors = violations(json.loads(output))
    except (subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    if errors:
        print("storage collection boundary failed:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
        return 1
    print("storage collection declarations use the standalone TSD2 types contract")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
