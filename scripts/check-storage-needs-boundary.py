"""Guard the storage-needs contract's standalone normal/build dependency graph."""

import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "proofs/storage-needs-consumer/Cargo.toml"
PIN = "c525a17d5153fc95e67e3cd44bcde1d90be60a71"

FORBIDDEN_PACKAGES = {
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
    "tinystoragedrivers-client",
    "tinystoragedrivers-core",
    "tinystoragedrivers-file",
    "tinystoragedrivers-mongodb",
    "tinystoragedrivers-s3",
    "tinystoragedrivers-secrets",
    "tinystoragedrivers-sqlite",
    "tinystoragedrivers-busproxy",
    "tokio-tungstenite",
    "tokio-rustls",
}
FORBIDDEN_TOKIO_FEATURES = {
    "fs",
    "io-util",
    "macros",
    "net",
    "process",
    "rt",
    "rt-multi-thread",
    "time",
}


def violations(metadata, tree_output):
    errors = []
    packages = metadata.get("packages", [])
    names = {package["name"] for package in packages}

    for name in sorted(names & FORBIDDEN_PACKAGES):
        errors.append(f"forbidden package in normal/build graph: {name}")

    storage_bus = [
        package for package in packages if package["name"] == "tinystoragedrivers-bus"
    ]
    if len(storage_bus) != 1:
        errors.append(
            "expected exactly one tinystoragedrivers-bus package, "
            f"found {len(storage_bus)}"
        )
    else:
        package = storage_bus[0]
        if package["version"] != "1.4.0":
            errors.append(f"unexpected storage bus version: {package['version']}")
        if PIN not in package.get("source", ""):
            errors.append("storage bus does not resolve to the expected canonical revision")
        node = next(
            (
                node
                for node in metadata.get("resolve", {}).get("nodes", [])
                if node["id"] == package["id"]
            ),
            None,
        )
        if node is None:
            errors.append("storage bus is missing from Cargo's resolved feature graph")
        elif "client" in node.get("features", []):
            errors.append("tinystoragedrivers-bus client feature is enabled")

    tokio = [package for package in packages if package["name"] == "tokio"]
    for package in tokio:
        node = next(
            (
                node
                for node in metadata.get("resolve", {}).get("nodes", [])
                if node["id"] == package["id"]
            ),
            None,
        )
        active = set(node.get("features", [])) if node else set()
        denied = sorted(active & FORBIDDEN_TOKIO_FEATURES)
        if denied:
            errors.append("forbidden Tokio features enabled: " + ", ".join(denied))

    tree_names = {line.strip().split(" ", 1)[0] for line in tree_output.splitlines() if line.strip()}
    for name in sorted(tree_names & FORBIDDEN_PACKAGES):
        message = f"forbidden package in normal/build cargo tree: {name}"
        if message not in errors:
            errors.append(message)

    return errors


def cargo_output(*args):
    result = subprocess.run(
        ["cargo", *args, "--locked", "--manifest-path", str(MANIFEST)],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def main():
    try:
        metadata = json.loads(
            cargo_output("metadata", "--format-version", "1")
        )
        tree = cargo_output("tree", "--edges", "normal,build", "--prefix", "none")
    except (subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1

    errors = violations(metadata, tree)
    if errors:
        print("storage-needs dependency boundary failed:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
        return 1

    print("storage-needs standalone normal/build/feature boundary is clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
