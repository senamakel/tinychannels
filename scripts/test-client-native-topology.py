"""Keep artifact-gated client proofs out of normal feature and coverage runs."""

from pathlib import Path
import re
import tomllib
import unittest


ROOT = Path(__file__).resolve().parent.parent
PROOF = Path("proof-workspaces/tinychannels-client-native-e2e")
PROOF_MANIFEST = ROOT / PROOF / "Cargo.toml"
TARGETS = {
    "native_e2e": (
        "all_public_wrappers_route_to_the_pinned_peer_and_decode_typed_results",
        "detailed_invalid_input_keeps_its_wire_classification_and_redacts_rendering",
        "malformed_public_responses_report_one_sanitized_module_fault",
        "wrong_release_pins_and_unattested_peers_are_refused_before_business_calls",
    ),
    "runtime_native_e2e": (
        "typed_driver_preserves_peer_through_dispatch_uncertainty_and_alias_handover",
    ),
    "native_module_runtime_e2e": (
        "actual_native_module_runs_provider_free_runtime_and_releases_callback_on_ack",
    ),
}
EXPECTED_COUNTS = {
    "native_e2e": 4,
    "runtime_native_e2e": 1,
    "native_module_runtime_e2e": 1,
}


class ClientNativeTopologyTests(unittest.TestCase):
    def test_artifact_gated_targets_are_excluded_from_workspace_all_features(self):
        root_manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn(str(PROOF), root_manifest["workspace"]["exclude"])
        self.assertIn("crates/tinychannels-client", root_manifest["workspace"]["members"])

        client_manifest = tomllib.loads(
            (ROOT / "crates/tinychannels-client/Cargo.toml").read_text(encoding="utf-8")
        )
        self.assertTrue(client_manifest["package"].get("autotests", False))
        self.assertNotIn("test", client_manifest)
        self.assertEqual(client_manifest["features"]["native-e2e"], [])

        proof_manifest = tomllib.loads(PROOF_MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(proof_manifest["package"]["name"], "tinychannels-client-native-e2e")
        self.assertEqual(proof_manifest["features"]["native-e2e"], [
            "dep:tinychannels-client",
            "tinychannels-client/native-e2e",
        ])
        targets = {target["name"]: target for target in proof_manifest["test"]}
        self.assertEqual(set(targets), set(TARGETS))
        for target in targets.values():
            self.assertEqual(target["required-features"], ["native-e2e"])

    def test_all_six_native_cases_remain_in_the_fixture_proof_workspace(self):
        self.assertEqual(sum(EXPECTED_COUNTS.values()), 6)
        for target, names in TARGETS.items():
            source = (ROOT / PROOF / "tests" / f"{target}.rs").read_text(encoding="utf-8")
            self.assertEqual(
                len(re.findall(r"(?m)^\s*#\[(?:tokio::)?test(?:\([^\n]*\))?\]", source)),
                EXPECTED_COUNTS[target],
            )
            for name in names:
                self.assertRegex(source, rf"(?m)^async fn {re.escape(name)}\(")
            self.assertNotRegex(source, r"(?m)^\s*#\[ignore(?:\(|\])")

    def test_native_ci_runners_call_every_target_and_enforce_test_counts(self):
        shell_runner = (ROOT / "scripts/test-client-e2e.sh").read_text(encoding="utf-8")
        powershell_runner = (ROOT / "scripts/test-client-e2e.ps1").read_text(encoding="utf-8")
        proof_path = str(PROOF / "Cargo.toml")
        self.assertIn(f"proof_manifest={proof_path}", shell_runner)
        self.assertIn("function run_counted_test", shell_runner)
        self.assertIn("function Invoke-CountedNativeTest", powershell_runner)
        for target, count in EXPECTED_COUNTS.items():
            self.assertEqual(shell_runner.count(f"run_counted_test {count} {target}"), 1)
            self.assertIn(f"'-Name', '{target}' -ExpectedCount {count}", powershell_runner)
            self.assertIn(f"'--test', '{target}'", powershell_runner)

        unix_e2e = (ROOT / "scripts/test-e2e.sh").read_text(encoding="utf-8")
        windows_e2e = (ROOT / "scripts/test-e2e.ps1").read_text(encoding="utf-8")
        self.assertIn("bash scripts/test-client-e2e.sh", unix_e2e)
        self.assertIn("test-client-e2e.ps1", windows_e2e)
        ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("module-e2e-script: scripts/test-e2e.sh", ci)

    def test_windows_runner_uses_native_tar_for_drive_letter_paths(self):
        powershell_runner = (ROOT / "scripts/test-client-e2e.ps1").read_text(encoding="utf-8")
        self.assertIn("System32/tar.exe", powershell_runner)
        self.assertNotRegex(powershell_runner, r"(?m)^&\s+tar\.exe\b")

    def test_feature_suite_and_coverage_keep_production_client_in_scope(self):
        feature_runner = (ROOT / "scripts/test-features.sh").read_text(encoding="utf-8")
        self.assertIn("python3 scripts/test-client-native-topology.py", feature_runner)
        self.assertIn("cargo test --locked --workspace --all-features", feature_runner)

        coverage_runner = (ROOT / "scripts/check-coverage.sh").read_text(encoding="utf-8")
        self.assertIn("cargo llvm-cov --locked --workspace --all-targets --all-features --no-report", coverage_runner)
        self.assertIn("TINYCHANNELS_CLIENT_COVERAGE=1 bash scripts/test-client-e2e.sh", coverage_runner)
        self.assertIn("cargo llvm-cov report", coverage_runner)
        self.assertIn("COVERAGE_REPORT_PREGENERATED=1", coverage_runner)
        coverage_check = (ROOT / ".github/scripts/check-file-coverage.sh").read_text(encoding="utf-8")
        self.assertIn("COVERAGE_REPORT_PREGENERATED", coverage_check)
        self.assertIn('cargo llvm-cov \\\n  --locked \\\n  --workspace \\\n  --all-targets \\\n  --all-features', coverage_check)
        self.assertIn("check-file-coverage.sh 90 coverage.json", coverage_runner)


if __name__ == "__main__":
    unittest.main()
