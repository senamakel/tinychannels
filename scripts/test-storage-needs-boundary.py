"""Negative controls for the standalone TSD2 collection boundary guard."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("check-storage-needs-boundary.py")
SPEC = importlib.util.spec_from_file_location("storage_boundary", SCRIPT)
BOUNDARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOUNDARY)


def fixture(*, extra=(), source=None, types_version="2.1.2"):
    source = source or (
        "git+https://github.com/senamakel/tinystoragedrivers"
        "?tag=v2.1.2#400b71e5"
    )
    return {
        "packages": [
            {"id": "proof", "name": "tinychannels-storage-needs-consumer", "version": "0.1.0"},
            {"id": "bus", "name": "tinychannels-bus", "version": "0.1.39"},
            {
                "id": "types",
                "name": "tinystoragedrivers-types",
                "version": types_version,
                "source": source,
            },
            *extra,
        ]
    }


class StorageBoundaryTests(unittest.TestCase):
    def test_tsd2_types_declaration_contract_is_allowed(self):
        self.assertEqual(BOUNDARY.violations(fixture()), [])

    def test_storage_client_and_bus_packages_are_rejected(self):
        errors = BOUNDARY.violations(
            fixture(extra=[{"id": "proxy", "name": "tinystoragedrivers-busproxy", "version": "2.1.2"}])
        )
        self.assertTrue(any("tinystoragedrivers-busproxy" in error for error in errors))

    def test_wrong_tsd_version_is_rejected(self):
        errors = BOUNDARY.violations(fixture(types_version="1.9.0"))
        self.assertIn("collection declarations must use fork TSD types v2.1.2", errors)

    def test_wrong_source_is_rejected(self):
        errors = BOUNDARY.violations(
            fixture(source="git+https://github.com/tinyhumansai/tinystoragedrivers?tag=v2.1.2#400b71e5")
        )
        self.assertIn("collection declarations must use fork TSD types v2.1.2", errors)


if __name__ == "__main__":
    unittest.main()
