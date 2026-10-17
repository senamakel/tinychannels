"""Negative controls for the standalone storage-needs dependency guard."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("check-storage-needs-boundary.py")
SPEC = importlib.util.spec_from_file_location("storage_needs_boundary", SCRIPT)
BOUNDARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOUNDARY)
EXPECTED_STORAGE_SOURCE = (
    "git+https://github.com/tinyhumansai/tinystoragedrivers"
    f"?rev={BOUNDARY.PIN}#{BOUNDARY.PIN}"
)


def fixture(extra_packages=(), tokio_features=("sync",), storage_features=()):
    packages = [
        {"id": "proof", "name": "tinychannels-storage-needs-consumer", "version": "0.1.0"},
        {"id": "channels", "name": "tinychannels-bus", "version": "0.1.34"},
        {
            "id": "storage",
            "name": "tinystoragedrivers-bus",
            "version": "1.5.0",
            "source": EXPECTED_STORAGE_SOURCE,
        },
        {"id": "tokio", "name": "tokio", "version": "1.0.0"},
        *extra_packages,
    ]
    nodes = [
        {
            "id": "proof",
            "features": [],
            "deps": [
                {"pkg": "channels", "dep_kinds": [{"kind": None}]},
                {"pkg": "storage", "dep_kinds": [{"kind": None}]},
            ],
        },
        {
            "id": "channels",
            "features": ["default"],
            "deps": [
                {"pkg": "storage", "dep_kinds": [{"kind": None}]},
                {"pkg": "tokio", "dep_kinds": [{"kind": None}]},
            ],
        },
        {"id": "storage", "features": list(storage_features), "deps": []},
        {"id": "tokio", "features": list(tokio_features), "deps": []},
    ]
    return {"packages": packages, "resolve": {"root": "proof", "nodes": nodes}}


class StorageNeedsBoundaryTests(unittest.TestCase):
    def test_existing_sync_only_tokio_baseline_is_allowed(self):
        self.assertEqual(BOUNDARY.violations(fixture(), ""), [])

    def test_tinybus_dependency_is_rejected(self):
        metadata = fixture(extra_packages=[{"id": "tinybus", "name": "tinybus", "version": "0.1.0"}])
        self.assertTrue(any("tinybus" in error for error in BOUNDARY.violations(metadata, "")))

    def test_storage_bus_client_feature_is_rejected(self):
        errors = BOUNDARY.violations(fixture(storage_features=("client",)), "")
        self.assertIn("tinystoragedrivers-bus client feature is enabled", errors)

    def test_non_sync_tokio_features_are_rejected(self):
        for extra in ("io-std", "signal", "rt", "macros"):
            with self.subTest(feature=extra):
                errors = BOUNDARY.violations(
                    fixture(tokio_features=("sync", extra)), ""
                )
                self.assertTrue(any(extra in error for error in errors))

    def test_missing_tokio_resolve_node_fails_closed(self):
        metadata = fixture()
        metadata["resolve"]["nodes"] = [
            node for node in metadata["resolve"]["nodes"] if node["id"] != "tokio"
        ]
        errors = BOUNDARY.violations(metadata, "")
        self.assertTrue(any("missing resolved normal/build node" in error for error in errors))

    def test_tag_source_with_matching_commit_suffix_is_rejected(self):
        metadata = fixture()
        metadata["packages"][2]["source"] = (
            "git+https://github.com/tinyhumansai/tinystoragedrivers"
            "?tag=v1.5.0#" + BOUNDARY.PIN
        )
        errors = BOUNDARY.violations(metadata, "")
        self.assertIn("storage bus source ID is not the exact canonical revision", errors)

    def test_truncated_revision_source_is_rejected(self):
        metadata = fixture()
        metadata["packages"][2]["source"] = (
            "git+https://github.com/tinyhumansai/tinystoragedrivers"
            f"?rev={BOUNDARY.PIN[:8]}#{BOUNDARY.PIN}"
        )
        errors = BOUNDARY.violations(metadata, "")
        self.assertIn("storage bus source ID is not the exact canonical revision", errors)

    def test_incomplete_normal_build_resolution_fails_closed(self):
        metadata = fixture()
        metadata["resolve"]["nodes"] = [
            node for node in metadata["resolve"]["nodes"] if node["id"] != "channels"
        ]
        errors = BOUNDARY.violations(metadata, "")
        self.assertTrue(any("missing resolved normal/build node" in error for error in errors))

    def test_normal_and_build_tree_is_checked_too(self):
        errors = BOUNDARY.violations(fixture(), "tinychannels-storage-needs-consumer v0.1.0\ntinychannels-runtime v0.1.0")
        self.assertTrue(any("normal/build cargo tree: tinychannels-runtime" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
