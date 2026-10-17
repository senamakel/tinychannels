"""Guard the storage-needs contract's standalone normal/build dependency graph."""

import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "proofs/storage-needs-consumer/Cargo.toml"
PIN = "c9ea6ff1615b77b55d7269403af5fd11bedc9ce8"
EXPECTED_STORAGE_SOURCE = (
    "git+https://github.com/tinyhumansai/tinystoragedrivers"
    f"?rev={PIN}#{PIN}"
)

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
ALLOWED_TOKIO_FEATURES = {"sync"}
NORMAL_BUILD_KINDS = {None, "normal", "build"}


def reachable_normal_build(metadata):
    errors = []
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        return {}, {}, set(), ["Cargo metadata is missing packages or resolve data"]

    packages_by_id = {}
    for package in packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            errors.append("Cargo metadata contains a malformed package record")
            continue
        if package["id"] in packages_by_id:
            errors.append(f"Cargo metadata repeats package ID {package['id']}")
        packages_by_id[package["id"]] = package

    nodes = resolve.get("nodes")
    root = resolve.get("root")
    if not isinstance(nodes, list) or not isinstance(root, str):
        return packages_by_id, {}, set(), errors + [
            "Cargo metadata is missing resolved nodes or workspace root"
        ]

    nodes_by_id = {}
    for node in nodes:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            errors.append("Cargo resolve graph contains a malformed node")
            continue
        if node["id"] in nodes_by_id:
            errors.append(f"Cargo resolve graph repeats node {node['id']}")
        nodes_by_id[node["id"]] = node

    reachable = set()
    pending = [root]
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        package = packages_by_id.get(package_id)
        node = nodes_by_id.get(package_id)
        if package is None or node is None:
            errors.append(
                f"missing resolved normal/build node or package record for {package_id}"
            )
            continue
        reachable.add(package_id)
        dependencies = node.get("deps")
        if not isinstance(dependencies, list):
            errors.append(f"resolved node {package_id} has no dependency evidence")
            continue
        for dependency in dependencies:
            if not isinstance(dependency, dict):
                errors.append(f"resolved node {package_id} has a malformed dependency")
                continue
            kinds = dependency.get("dep_kinds")
            if not isinstance(kinds, list) or not kinds:
                errors.append(f"resolved dependency from {package_id} has no kind evidence")
                continue
            if not any(
                isinstance(kind, dict) and kind.get("kind") in NORMAL_BUILD_KINDS
                for kind in kinds
            ):
                continue
            child = dependency.get("pkg")
            if not isinstance(child, str):
                errors.append(f"resolved dependency from {package_id} has no package ID")
                continue
            if child not in nodes_by_id or child not in packages_by_id:
                child_name = dependency.get("name", child)
                errors.append(
                    f"missing resolved normal/build node for {child_name} ({child})"
                )
                continue
            pending.append(child)

    return packages_by_id, nodes_by_id, reachable, errors


def violations(metadata, tree_output):
    packages_by_id, nodes_by_id, reachable, errors = reachable_normal_build(metadata)
    packages = [packages_by_id[package_id] for package_id in reachable]
    names = {package["name"] for package in packages}

    for name in sorted(names & FORBIDDEN_PACKAGES):
        errors.append(f"forbidden package in reachable normal/build graph: {name}")

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
        if package["version"] != "1.5.0":
            errors.append(f"unexpected storage bus version: {package['version']}")
        if package.get("source") != EXPECTED_STORAGE_SOURCE:
            errors.append("storage bus source ID is not the exact canonical revision")
        node = nodes_by_id[package["id"]]
        features = node.get("features")
        if not isinstance(features, list):
            errors.append("storage bus resolve node is missing feature evidence")
        elif "client" in features:
            errors.append("tinystoragedrivers-bus client feature is enabled")

    tokio = [package for package in packages if package["name"] == "tokio"]
    if not tokio:
        errors.append("Tokio is missing from the reachable normal/build graph")
    for package in tokio:
        node = nodes_by_id[package["id"]]
        features = node.get("features")
        if not isinstance(features, list):
            errors.append(f"Tokio resolve node {package['id']} is missing feature evidence")
            continue
        active = set(features)
        if active != ALLOWED_TOKIO_FEATURES:
            errors.append(
                "Tokio features must be exactly sync; found "
                + ", ".join(sorted(active))
            )

    tree_names = {line.strip().split(" ", 1)[0] for line in tree_output.splitlines() if line.strip()}
    reachable_names = {package["name"] for package in packages}
    if not tree_names:
        errors.append("normal/build cargo tree output is empty")
    for required in {
        "tinychannels-storage-needs-consumer",
        "tinychannels-bus",
        "tinystoragedrivers-bus",
    } - tree_names:
        errors.append(f"normal/build cargo tree omits required package: {required}")
    for name in sorted(tree_names - reachable_names):
        errors.append(f"normal/build cargo tree package is absent from resolve graph: {name}")
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
