"""Negative controls for the standalone storage-needs dependency guard."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("check-storage-needs-boundary.py")
SPEC = importlib.util.spec_from_file_location("storage_needs_boundary", SCRIPT)
BOUNDARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOUNDARY)


def fixture(extra_packages=(), tokio_features=("sync",), storage_features=()):
    packages = [
        {"id": "proof", "name": "tinychannels-storage-needs-consumer", "version": "0.1.0"},
        {"id": "channels", "name": "tinychannels-bus", "version": "0.1.34"},
        {
            "id": "storage",
            "name": "tinystoragedrivers-bus",
            "version": "1.4.0",
            "source": "git+https://github.com/tinyhumansai/tinystoragedrivers?rev=" + BOUNDARY.PIN,
        },
        {"id": "tokio", "name": "tokio", "version": "1.0.0"},
        *extra_packages,
    ]
    nodes = [
        {"id": "storage", "features": list(storage_features)},
        {"id": "tokio", "features": list(tokio_features)},
    ]
    return {"packages": packages, "resolve": {"nodes": nodes}}


class StorageNeedsBoundaryTests(unittest.TestCase):
    def test_existing_sync_only_tokio_baseline_is_allowed(self):
        self.assertEqual(BOUNDARY.violations(fixture(), ""), [])

    def test_tinybus_dependency_is_rejected(self):
        metadata = fixture(extra_packages=[{"id": "tinybus", "name": "tinybus", "version": "0.1.0"}])
        self.assertTrue(any("tinybus" in error for error in BOUNDARY.violations(metadata, "")))

    def test_storage_bus_client_feature_is_rejected(self):
        errors = BOUNDARY.violations(fixture(storage_features=("client",)), "")
        self.assertIn("tinystoragedrivers-bus client feature is enabled", errors)

    def test_runtime_tokio_features_are_rejected(self):
        errors = BOUNDARY.violations(fixture(tokio_features=("sync", "rt", "macros")), "")
        self.assertTrue(any("Tokio features" in error for error in errors))

    def test_normal_and_build_tree_is_checked_too(self):
        errors = BOUNDARY.violations(fixture(), "tinychannels-storage-needs-consumer v0.1.0\ntinychannels-runtime v0.1.0")
        self.assertTrue(any("normal/build cargo tree: tinychannels-runtime" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
