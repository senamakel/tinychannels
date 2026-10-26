"""Source and graph controls for the transport-free channel contract."""

import importlib.util
from pathlib import Path
import tomllib
import unittest


SCRIPT = Path(__file__).with_name("check-bus-purity.py")
SPEC = importlib.util.spec_from_file_location("bus_purity", SCRIPT)
BUS_PURITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUS_PURITY)


def metadata_fixture(*, extra=(), tokio_features=("sync",), types_version="2.1.2", types_source=None):
    types_source = types_source or BUS_PURITY.TSD_SOURCE_PREFIX + "400b71e5"
    packages = [
        {"id": "bus", "name": "tinychannels-bus", "version": "0.1.39"},
        {"id": "tokio", "name": "tokio", "version": "1.0.0"},
        {"id": "types", "name": "tinystoragedrivers-types", "version": types_version, "source": types_source},
        *extra,
    ]
    nodes = [
        {"id": "bus", "features": ["storage"], "deps": [
            {"pkg": "tokio", "name": "tokio", "dep_kinds": [{"kind": None}]},
            {"pkg": "types", "name": "tinystoragedrivers-types", "dep_kinds": [{"kind": None}]},
        ]},
        {"id": "tokio", "features": list(tokio_features), "deps": []},
        {"id": "types", "features": [], "deps": []},
    ]
    for package in extra:
        nodes[0]["deps"].append({"pkg": package["id"], "name": package["name"], "dep_kinds": [{"kind": None}]})
        nodes.append({"id": package["id"], "features": [], "deps": []})
    return {"packages": packages, "resolve": {"nodes": nodes}}


class BusPurityTests(unittest.TestCase):
    def test_bus_storage_schema_is_a_pure_tsd2_collection_declaration(self):
        self.assertEqual(BUS_PURITY.source_tree_violations(), [])
        schema = (BUS_PURITY.BUS_SOURCE / "storage/schema.rs").read_text()
        self.assertIn("CollectionSpec", schema)
        self.assertNotIn("async fn", schema)
        implementation = BUS_PURITY.ROOT / "src/storage"
        self.assertIn("pub async fn prepare", (implementation / "mod.rs").read_text())
        self.assertIn("pub fn stored_delivery_from_queued", (implementation / "records.rs").read_text())

    def test_bus_declares_only_optional_tsd2_types_for_storage_specs(self):
        manifest = tomllib.loads(BUS_PURITY.BUS_MANIFEST.read_text())
        dependency = manifest["dependencies"]["tinystoragedrivers-types"]
        self.assertTrue(dependency["optional"])
        self.assertEqual(manifest["features"]["storage"], ["dep:tinystoragedrivers-types"])
        self.assertEqual(BUS_PURITY.dependency_violations(metadata_fixture()), [])

    def test_storage_clients_are_rejected(self):
        metadata = metadata_fixture(extra=[{"id": "proxy", "name": "tinystoragedrivers-busproxy", "version": "2.1.2"}])
        self.assertTrue(any("unsafe contract dependencies" in error for error in BUS_PURITY.dependency_violations(metadata)))

    def test_wrong_types_version_is_rejected(self):
        errors = BUS_PURITY.dependency_violations(metadata_fixture(types_version="1.9.0"))
        self.assertIn("collection declarations must use fork TSD types v2.1.2", errors)

    def test_tokio_remains_sync_only(self):
        errors = BUS_PURITY.dependency_violations(metadata_fixture(tokio_features=("sync", "rt")))
        self.assertTrue(any("Tokio features must be exactly sync" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
