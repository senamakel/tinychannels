"""Test the legacy bus-purity entrypoint, source guard and graph controls."""

import contextlib
import importlib.util
import io
from pathlib import Path
import tomllib
import unittest
from unittest import mock


SCRIPT = Path(__file__).with_name("check-bus-purity.py")
SPEC = importlib.util.spec_from_file_location("bus_purity", SCRIPT)
BUS_PURITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUS_PURITY)
TSD_SOURCE = BUS_PURITY.TSD_SOURCE


def metadata_fixture(
    *,
    extra_packages=(),
    tokio_features=("sync",),
    storage_features=(),
    storage_core=False,
    storage_core_source="git+https://example.invalid/tinystoragedrivers#injected",
    remove_nodes=(),
    storage_source=TSD_SOURCE,
):
    packages = []
    for name in sorted(BUS_PURITY.ALLOWED_PACKAGES):
        version = {
            "tinychannels-bus": "0.1.35",
            "tinystoragedrivers-bus": "1.10.0",
            "tokio": "1.0.0",
        }.get(name, "1.0.0")
        package = {"id": f"pkg:{name}", "name": name, "version": version}
        if name == "tinystoragedrivers-bus":
            package["source"] = storage_source
        packages.append(package)

    package_names = {package["name"] for package in packages}
    package_names.update(package["name"] for package in extra_packages)
    root_dependency_names = package_names - {"tinychannels-bus"}
    if storage_core:
        core = {
            "id": "pkg:tinystoragedrivers-core",
            "name": "tinystoragedrivers-core",
            "version": "1.0.0",
            "source": storage_core_source,
        }
        packages.append(core)
        packages.append(
            {
                "id": "pkg:storage-private-dependency",
                "name": "storage-private-dependency",
                "version": "1.0.0",
            }
        )
        package_names.update(
            {"tinystoragedrivers-core", "storage-private-dependency"}
        )
        root_dependency_names.add("tinystoragedrivers-core")
    packages.extend(extra_packages)
    nodes = []
    for name in sorted(package_names):
        features = []
        if name == "tinychannels-bus":
            features = ["default"]
        elif name == "tokio":
            features = list(tokio_features)
        elif name == "tinystoragedrivers-bus":
            features = list(storage_features)
        dependencies = []
        if name == "tinychannels-bus":
            dependencies = [
                {
                    "pkg": f"pkg:{dependency}",
                    "name": dependency,
                    "dep_kinds": [{"kind": None}],
                }
                for dependency in sorted(root_dependency_names)
            ]
        elif name == "tinystoragedrivers-core":
            dependencies = [
                {
                    "pkg": "pkg:storage-private-dependency",
                    "name": "storage-private-dependency",
                    "dep_kinds": [{"kind": None}],
                },
                {
                    "pkg": "pkg:tokio",
                    "name": "tokio",
                    "dep_kinds": [{"kind": None}],
                },
            ]
        nodes.append(
            {"id": f"pkg:{name}", "features": features, "deps": dependencies}
        )
    if storage_core:
        next(
            node for node in nodes if node["id"] == "pkg:tinystoragedrivers-core"
        )["features"] = []
        next(node for node in nodes if node["id"] == "pkg:tokio")["features"] = [
            "io-util",
            "rt",
            "sync",
            "time",
        ]
    nodes = [node for node in nodes if node["id"] not in remove_nodes]
    return {"packages": packages, "resolve": {"nodes": nodes}}


class BusPurityTests(unittest.TestCase):
    def test_storage_schema_migrations_and_projections_are_owned_by_tinychannels(self):
        bus_storage = BUS_PURITY.BUS_SOURCE / "storage"
        self.assertFalse((bus_storage / "schema.rs").exists())
        self.assertFalse((bus_storage / "migrate.rs").exists())
        self.assertFalse((bus_storage / "schema_tests.rs").exists())
        self.assertFalse((bus_storage / "migrate_tests.rs").exists())
        bus_records = (bus_storage / "records.rs").read_text()
        self.assertNotIn("impl From<", bus_records)
        self.assertNotIn("to_stored_ms", bus_records)
        self.assertNotIn("from_stored_ms", bus_records)

        implementation = BUS_PURITY.ROOT / "src/storage"
        self.assertIn("Schema::new", (implementation / "schema.rs").read_text())
        self.assertTrue((implementation / "migrate.rs").exists())
        self.assertTrue((implementation / "schema_tests.rs").exists())
        self.assertTrue((implementation / "migrate_tests.rs").exists())
        record_projection = (implementation / "records.rs").read_text()
        self.assertIn("pub fn stored_delivery_from_queued", record_projection)
        self.assertIn("pub fn queued_from_stored_delivery", record_projection)

    def test_bus_has_no_storage_implementation_dependency_under_any_feature(self):
        manifest = tomllib.loads(BUS_PURITY.BUS_MANIFEST.read_text())
        self.assertNotIn("tinystoragedrivers-core", manifest.get("dependencies", {}))
        self.assertNotIn("tinystoragedrivers-core", manifest.get("dev-dependencies", {}))
        self.assertEqual(manifest.get("features", {}).get("storage"), [])
        bus_lib = (BUS_PURITY.BUS_SOURCE / "lib.rs").read_text()
        self.assertIn("pub mod storage;", bus_lib)
        self.assertNotIn('#[cfg(feature = "storage")]\npub mod storage;', bus_lib)
        root_manifest = tomllib.loads((BUS_PURITY.ROOT / "Cargo.toml").read_text())
        bus_dependency = root_manifest["dependencies"]["tinychannels-bus"]
        self.assertNotIn("storage", bus_dependency.get("features", []))

    def test_current_contract_source_allows_async_traits_sync_types_and_lazy_metadata(self):
        self.assertEqual(BUS_PURITY.source_tree_violations(), [])
        source = """
use std::sync::{Arc, LazyLock, Mutex};
use async_trait::async_trait;
#[async_trait]
trait Channel {
    async fn listen(&self, messages: tokio::sync::mpsc::Sender<Message>);
}
static NEEDS: LazyLock<StorageNeeds<'static>> = LazyLock::new(build_metadata);
"""
        self.assertEqual(
            BUS_PURITY.source_violations(source, source_path="traits.rs"), []
        )

    def test_runtime_tasks_sockets_databases_and_provider_executors_are_rejected(self):
        snippets = (
            ("tokio::spawn(async move { work().await });", "Tokio task/time execution"),
            ("std::net::TcpListener::bind(address)", "socket/network implementation"),
            ("rusqlite::Connection::open(path)", "database or HTTP implementation"),
            ("impl Channel for DiscordProvider {}", "concrete provider executor"),
            ("std::fs::read(path)", "filesystem implementation"),
            ("tokio::sync::Mutex::new(())", "Tokio runtime synchronization"),
            ("std::time::Instant::now()", "clock access"),
            ("std::sync::mpsc::channel()", "thread/IPC coordination"),
            ("parking_lot::Mutex::new(())", "runtime synchronization"),
            ("sha2::Sha256::digest(value)", "cryptographic, random, or identity implementation"),
        )
        for source, expected in snippets:
            with self.subTest(source=source):
                self.assertIn(expected, BUS_PURITY.source_violations(source))

    def test_async_implementation_outside_the_provider_interface_is_rejected(self):
        errors = BUS_PURITY.source_violations(
            "async fn open_socket() {}", source_path="transport.rs"
        )
        self.assertIn("async implementation outside the channel interface", errors)

    def test_async_trait_macro_outside_the_provider_interface_is_rejected(self):
        errors = BUS_PURITY.source_violations(
            "use async_trait::async_trait;", source_path="provider.rs"
        )
        self.assertIn("async trait macro outside the channel interface", errors)

    def test_default_contract_dependency_closure_is_exact(self):
        self.assertEqual(BUS_PURITY.dependency_violations(metadata_fixture()), [])

    def test_empty_storage_compatibility_feature_keeps_the_pure_closure(self):
        self.assertEqual(BUS_PURITY.dependency_violations(metadata_fixture()), [])

    def test_storage_core_is_rejected_under_any_feature(self):
        metadata = metadata_fixture(
            storage_core=True,
        )
        errors = BUS_PURITY.dependency_violations(metadata)
        self.assertTrue(any("TSD core must remain outside every bus feature closure" in error for error in errors))

    def test_storage_core_does_not_exempt_unrelated_executor_dependencies(self):
        extra = {"id": "pkg:tinybus", "name": "tinybus", "version": "0.1.7"}
        metadata = metadata_fixture(
            extra_packages=(extra,),
            storage_core=True,
        )
        errors = BUS_PURITY.dependency_violations(metadata)
        self.assertTrue(any("unsafe contract dependencies" in error for error in errors))

    def test_tinybus_runtime_dependency_is_rejected(self):
        extra = {"id": "pkg:tinybus", "name": "tinybus", "version": "0.1.7"}
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(extra_packages=(extra,))
        )
        self.assertTrue(any("unsafe contract dependencies" in error for error in errors))

    def test_dev_only_edges_are_excluded_from_the_host_dependency_closure(self):
        metadata = metadata_fixture()
        metadata["packages"].append(
            {"id": "pkg:wiremock", "name": "wiremock", "version": "0.6.0"}
        )
        root = next(
            node
            for node in metadata["resolve"]["nodes"]
            if node["id"] == "pkg:tinychannels-bus"
        )
        root["deps"].append(
            {
                "pkg": "pkg:wiremock",
                "name": "wiremock",
                "dep_kinds": [{"kind": "dev"}],
            }
        )
        metadata["resolve"]["nodes"].append(
            {"id": "pkg:wiremock", "features": [], "deps": []}
        )
        self.assertEqual(BUS_PURITY.dependency_violations(metadata), [])

    def test_non_sync_tokio_feature_is_rejected(self):
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(tokio_features=("sync", "time"))
        )
        self.assertTrue(any("Tokio features must be exactly sync" in error for error in errors))

    def test_tinybus_storage_client_feature_is_rejected(self):
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(storage_features=("client",))
        )
        self.assertTrue(any("storage bus features must be empty" in error for error in errors))

    def test_tsd_source_id_must_match_the_full_canonical_pin(self):
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(
                storage_source=(
                    "git+https://github.com/tinyhumansai/tinystoragedrivers"
                    f"?tag=v1.10.0#{BUS_PURITY.TSD_REV}"
                )
            )
        )
        self.assertTrue(any("canonical TSD 1.10.0" in error for error in errors))

    def test_incomplete_metadata_fails_closed(self):
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(remove_nodes=("pkg:tokio",))
        )
        self.assertTrue(any("missing resolved normal/build node" in error for error in errors))

    def test_actual_entrypoint_runs_source_and_graph_guards(self):
        def fixture_for_mode(manifest, **_kwargs):
            return metadata_fixture()

        stdout = io.StringIO()
        with mock.patch.object(BUS_PURITY, "run_metadata", side_effect=fixture_for_mode):
            with contextlib.redirect_stdout(stdout):
                result = BUS_PURITY.main(["--offline"])
        self.assertEqual(result, 0)
        self.assertEqual(stdout.getvalue().count("pure contract closure (26 packages)"), 3)

    def test_actual_entrypoint_rejects_an_injected_executor_dependency(self):
        extra = {"id": "pkg:tinybus", "name": "tinybus", "version": "0.1.7"}
        stderr = io.StringIO()
        def fixture_for_mode(manifest, **_kwargs):
            return metadata_fixture(extra_packages=(extra,))

        with mock.patch.object(BUS_PURITY, "run_metadata", side_effect=fixture_for_mode):
            with contextlib.redirect_stderr(stderr):
                result = BUS_PURITY.main(["--offline"])
        self.assertEqual(result, 1)
        self.assertIn("unsafe contract dependencies", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
