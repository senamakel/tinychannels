"""Test the legacy bus-purity entrypoint, source guard and graph controls."""

import contextlib
import importlib.util
import io
from pathlib import Path
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
    remove_nodes=(),
    storage_source=TSD_SOURCE,
):
    packages = []
    for name in sorted(BUS_PURITY.ALLOWED_PACKAGES):
        version = {
            "tinychannels-bus": "0.1.35",
            "tinystoragedrivers-bus": "1.5.0",
            "tokio": "1.0.0",
        }.get(name, "1.0.0")
        package = {"id": f"pkg:{name}", "name": name, "version": version}
        if name == "tinystoragedrivers-bus":
            package["source"] = storage_source
        packages.append(package)

    package_names = {package["name"] for package in packages}
    package_names.update(package["name"] for package in extra_packages)
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
                for dependency in sorted(package_names - {"tinychannels-bus"})
            ]
        nodes.append(
            {"id": f"pkg:{name}", "features": features, "deps": dependencies}
        )
    nodes = [node for node in nodes if node["id"] not in remove_nodes]
    return {"packages": packages, "resolve": {"nodes": nodes}}


class BusPurityTests(unittest.TestCase):
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
        self.assertEqual(BUS_PURITY.source_violations(source), [])

    def test_runtime_tasks_sockets_databases_and_provider_executors_are_rejected(self):
        snippets = (
            ("tokio::spawn(async move { work().await });", "Tokio task/time execution"),
            ("std::net::TcpListener::bind(address)", "socket/network implementation"),
            ("rusqlite::Connection::open(path)", "database or HTTP implementation"),
            ("impl Channel for DiscordProvider {}", "concrete provider executor"),
            ("std::fs::read(path)", "filesystem implementation"),
        )
        for source, expected in snippets:
            with self.subTest(source=source):
                self.assertIn(expected, BUS_PURITY.source_violations(source))

    def test_default_contract_dependency_closure_is_exact(self):
        self.assertEqual(BUS_PURITY.dependency_violations(metadata_fixture()), [])

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
                    f"?tag=v1.5.0#{BUS_PURITY.TSD_REV}"
                )
            )
        )
        self.assertTrue(any("canonical TSD 1.5.0" in error for error in errors))

    def test_incomplete_metadata_fails_closed(self):
        errors = BUS_PURITY.dependency_violations(
            metadata_fixture(remove_nodes=("pkg:tokio",))
        )
        self.assertTrue(any("missing resolved normal/build node" in error for error in errors))

    def test_actual_entrypoint_runs_source_and_graph_guards(self):
        metadata = metadata_fixture()
        stdout = io.StringIO()
        with mock.patch.object(BUS_PURITY, "run_metadata", return_value=metadata):
            with contextlib.redirect_stdout(stdout):
                result = BUS_PURITY.main(["--offline"])
        self.assertEqual(result, 0)
        self.assertEqual(stdout.getvalue().count("pure contract closure (26 packages)"), 2)

    def test_actual_entrypoint_rejects_an_injected_executor_dependency(self):
        metadata = metadata_fixture(
            extra_packages=(
                {"id": "pkg:tinybus", "name": "tinybus", "version": "0.1.7"},
            )
        )
        stderr = io.StringIO()
        with mock.patch.object(BUS_PURITY, "run_metadata", return_value=metadata):
            with contextlib.redirect_stderr(stderr):
                result = BUS_PURITY.main(["--offline"])
        self.assertEqual(result, 1)
        self.assertIn("unsafe contract dependencies", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
