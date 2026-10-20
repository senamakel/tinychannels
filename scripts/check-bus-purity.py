#!/usr/bin/env python3
"""Audit the transport-free channel contract source and normal/build closure."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
BUS_SOURCE = ROOT / "crates/tinychannels-bus/src"
BUS_MANIFEST = ROOT / "crates/tinychannels-bus/Cargo.toml"
TSD_REV = "c9ea6ff1615b77b55d7269403af5fd11bedc9ce8"
TSD_SOURCE = (
    "git+https://github.com/tinyhumansai/tinystoragedrivers"
    f"?rev={TSD_REV}#{TSD_REV}"
)

# This is the observed default/all-feature normal + build closure. New packages
# require review because this crate is consumed directly by host libraries.
ALLOWED_PACKAGES = frozenset(
    {
        "anyhow",
        "async-trait",
        "base64",
        "dyn-clone",
        "itoa",
        "memchr",
        "pin-project-lite",
        "proc-macro2",
        "quote",
        "ref-cast",
        "ref-cast-impl",
        "schemars",
        "schemars_derive",
        "serde",
        "serde_core",
        "serde_derive",
        "serde_derive_internals",
        "serde_json",
        "syn",
        "thiserror",
        "thiserror-impl",
        "tinychannels-bus",
        "tinystoragedrivers-bus",
        "tokio",
        "unicode-ident",
        "zmij",
    }
)
NORMAL_BUILD_KINDS = {None, "normal", "build"}
TOKIO_FEATURES = frozenset({"sync"})

# Async trait signatures and Tokio's sync-only sender type are contract
# carriers. The source scan therefore targets concrete execution and I/O seams.
FORBIDDEN_SOURCE = (
    (re.compile(r"\bstd::net\b"), "socket/network implementation"),
    (re.compile(r"\btokio::net\b"), "Tokio network implementation"),
    (re.compile(r"\bstd::fs\b|\btokio::fs\b"), "filesystem implementation"),
    (re.compile(r"\bstd::process\b|\btokio::process\b"), "process execution"),
    (re.compile(r"\bstd::env\b"), "process environment access"),
    (re.compile(r"\bstd::time\b"), "clock access"),
    (re.compile(r"\bstd::thread\b"), "thread execution"),
    (re.compile(r"\bstd::sync::(?:mpsc|atomic)::"), "thread/IPC coordination"),
    (re.compile(r"\bparking_lot::"), "runtime synchronization"),
    (re.compile(r"\btokio::(?:spawn|task::|time::)"), "Tokio task/time execution"),
    (
        re.compile(
            r"\btokio::sync::(?:Mutex|RwLock|OnceCell|Notify|oneshot::|watch::|broadcast::)"
        ),
        "Tokio runtime synchronization",
    ),
    (
        re.compile(r"\b(?:TcpListener|TcpStream|UdpSocket|UnixListener|UnixStream)\b"),
        "socket type",
    ),
    (
        re.compile(r"\b(?:rusqlite|mongodb|sqlx|reqwest|hyper|axum)::"),
        "database or HTTP implementation",
    ),
    (re.compile(r"\bimpl\s+Channel\s+for\b"), "concrete provider executor"),
    (
        re.compile(r"\b(?:sha2|hmac|rand|uuid)::"),
        "cryptographic, random, or identity implementation",
    ),
    (
        re.compile(
            r"\bfn\s+(?:build_session_key|classify_send_error|"
            r"authenticated_inbound_event|segment_for_delivery)\b"
        ),
        "implementation algorithm",
    ),
)


def source_violations(source_text, source_path=None):
    """Return runtime/implementation constructs found in one Rust source."""
    errors = [
        label
        for pattern, label in FORBIDDEN_SOURCE
        if pattern.search(source_text)
    ]
    if source_path is not None and Path(source_path).name != "traits.rs":
        if re.search(r"\basync\s+fn\b", source_text):
            errors.append("async implementation outside the channel interface")
        if re.search(r"\basync_trait\b", source_text):
            errors.append("async trait macro outside the channel interface")
    return errors


def source_tree_violations(source_root=BUS_SOURCE):
    """Scan production Rust files while leaving colocated behavior tests alone."""
    errors = []
    storage = Path(source_root) / "storage"
    for name in ("schema.rs", "migrate.rs", "schema_tests.rs", "migrate_tests.rs"):
        if (storage / name).exists():
            errors.append(f"{(storage / name).relative_to(ROOT)}: executable storage schema belongs to tinychannels")
    records = storage / "records.rs"
    if records.exists():
        text = records.read_text(encoding="utf-8")
        if "impl From<" in text or "to_stored_ms" in text or "from_stored_ms" in text:
            errors.append(f"{records.relative_to(ROOT)}: storage projections belong to tinychannels")
    for source in sorted(Path(source_root).rglob("*.rs")):
        if source.name.endswith("_tests.rs"):
            continue
        for violation in source_violations(
            source.read_text(encoding="utf-8"), source_path=source
        ):
            errors.append(f"{source.relative_to(ROOT)}: {violation}")
    return errors


def reachable_normal_build(metadata, root_package="tinychannels-bus"):
    """Resolve the bus package's complete normal/build closure, fail closed."""
    errors = []
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        return {}, {}, set(), ["Cargo metadata is missing packages or resolve data"]

    packages_by_id = {}
    for package in packages:
        if (
            not isinstance(package, dict)
            or not isinstance(package.get("id"), str)
            or not isinstance(package.get("name"), str)
        ):
            errors.append("Cargo metadata contains a malformed package record")
            continue
        if package["id"] in packages_by_id:
            errors.append(f"Cargo metadata repeats package ID {package['id']}")
        packages_by_id[package["id"]] = package

    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        return packages_by_id, {}, set(), errors + [
            "Cargo metadata is missing resolved nodes"
        ]
    nodes_by_id = {}
    for node in nodes:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            errors.append("Cargo resolve graph contains a malformed node")
            continue
        if node["id"] in nodes_by_id:
            errors.append(f"Cargo resolve graph repeats node {node['id']}")
        nodes_by_id[node["id"]] = node

    bus_ids = [
        package_id
        for package_id, package in packages_by_id.items()
        if package["name"] == root_package
    ]
    if len(bus_ids) != 1:
        return packages_by_id, nodes_by_id, set(), errors + [
            f"expected one {root_package} package, found {len(bus_ids)}"
        ]

    reachable = set()
    pending = list(bus_ids)
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        package = packages_by_id.get(package_id)
        node = nodes_by_id.get(package_id)
        if package is None or node is None:
            errors.append(f"missing resolved normal/build node or package record for {package_id}")
            continue
        reachable.add(package_id)
        if not isinstance(node.get("features"), list):
            errors.append(f"resolved node {package_id} has no feature evidence")
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
            if any(
                not isinstance(kind, dict)
                or kind.get("kind") not in {None, "normal", "dev", "build"}
                for kind in kinds
            ):
                errors.append(f"resolved dependency from {package_id} has malformed kind evidence")
                continue
            if not any(kind.get("kind") in NORMAL_BUILD_KINDS for kind in kinds):
                continue
            child = dependency.get("pkg")
            if not isinstance(child, str):
                errors.append(f"resolved dependency from {package_id} has no package ID")
            elif child not in nodes_by_id or child not in packages_by_id:
                errors.append(
                    f"missing resolved normal/build node for "
                    f"{dependency.get('name', child)} ({child})"
                )
            else:
                pending.append(child)

    return packages_by_id, nodes_by_id, reachable, errors


def dependency_violations(metadata):
    packages_by_id, nodes_by_id, reachable, errors = reachable_normal_build(metadata)
    packages = [packages_by_id[package_id] for package_id in reachable]
    names = {package["name"] for package in packages}
    missing = sorted(ALLOWED_PACKAGES - names)
    core_packages = [
        package for package in packages if package["name"] == "tinystoragedrivers-core"
    ]
    if core_packages:
        errors.append("TSD core must remain outside every bus feature closure")

    unsafe = sorted(
        packages_by_id[package_id]["name"]
        for package_id in reachable
        if packages_by_id[package_id]["name"] not in ALLOWED_PACKAGES
    )
    if missing:
        errors.append(f"normal/build graph is incomplete; missing packages: {missing}")
    if unsafe:
        errors.append(f"unsafe contract dependencies: {unsafe}")
    repeated_names = sorted(
        name for name in names if sum(package["name"] == name for package in packages) > 1
    )
    if repeated_names:
        errors.append(f"normal/build graph has duplicate package names: {repeated_names}")

    tsd_packages = [
        package for package in packages if package["name"] == "tinystoragedrivers-bus"
    ]
    if len(tsd_packages) != 1:
        errors.append(
            f"expected one tinystoragedrivers-bus package, found {len(tsd_packages)}"
        )
    else:
        package = tsd_packages[0]
        if package.get("version") != "1.5.0" or package.get("source") != TSD_SOURCE:
            errors.append("storage bus must be canonical TSD 1.5.0 at the pinned revision")
        node = nodes_by_id.get(package["id"])
        features = node.get("features") if node else None
        if not isinstance(features, list):
            errors.append("storage bus resolve node is missing feature evidence")
        elif features:
            errors.append(f"storage bus features must be empty; found {features}")

    tokio_packages = [package for package in packages if package["name"] == "tokio"]
    if len(tokio_packages) != 1:
        errors.append(f"expected one Tokio package, found {len(tokio_packages)}")
    for package in tokio_packages:
        node = nodes_by_id.get(package["id"])
        features = node.get("features") if node else None
        if not isinstance(features, list):
            errors.append(f"Tokio resolve node {package['id']} is missing feature evidence")
        elif set(features) != TOKIO_FEATURES:
            errors.append(
                "Tokio features must be exactly sync; found "
                + ", ".join(sorted(features))
            )

    return errors


def run_metadata(manifest, offline=False):
    command = ["cargo", "metadata"]
    if offline:
        command.append("--offline")
    command.extend(["--format-version=1", "--manifest-path", str(manifest)])
    output = subprocess.check_output(command, cwd=ROOT, text=True)
    return json.loads(output)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true", help="Use only cached Cargo registry data")
    args = parser.parse_args(argv)

    source_errors = source_tree_violations()
    if source_errors:
        print("unsafe contract source:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in source_errors), file=sys.stderr)
        return 1

    features = [
        feature
        for feature in tomllib.loads(BUS_MANIFEST.read_text(encoding="utf-8")).get("features", {})
        if feature != "storage"
    ]
    for mode in ("default", "all", "storage"):
        probe = ROOT / "target/bus-purity" / mode
        (probe / "src").mkdir(parents=True, exist_ok=True)
        (probe / "src/lib.rs").write_text("pub use tinychannels_bus::*;\n", encoding="utf-8")
        dependency = (
            "tinychannels-bus = { path = "
            + json.dumps(str(ROOT / "crates/tinychannels-bus"))
            + (
                ", features = " + json.dumps(features)
                if mode == "all"
                else ', features = ["storage"]'
                if mode == "storage"
                else ""
            )
            + " }\n"
        )
        (probe / "Cargo.toml").write_text(
            '[package]\nname="contract-purity-probe"\nversion="0.0.0"\nedition="2024"\n'
            "[workspace]\n[dependencies]\n"
            + dependency,
            encoding="utf-8",
        )
        metadata = run_metadata(probe / "Cargo.toml", offline=args.offline)
        errors = dependency_violations(metadata)
        if errors:
            print(f"{mode}: contract dependency boundary failed:", file=sys.stderr)
            print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
            return 1
        names = {
            package["name"]
            for package in metadata["packages"]
            if package["name"] in ALLOWED_PACKAGES
        }
        print(f"{mode}: pure contract closure ({len(names)} packages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
