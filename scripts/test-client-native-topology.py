"""Keep artifact-gated client proofs staged and counted in every CI lane."""

from pathlib import Path
import re
import tomllib
import unittest


ROOT = Path(__file__).resolve().parent.parent
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
    "runtime_readiness_e2e": (
        "cold_bind_waits_for_ready_and_fails_closed_when_ready_is_withheld",
    ),
    "native_module_runtime_e2e": (
        "actual_native_module_runs_provider_free_runtime_and_releases_callback_on_ack",
    ),
}
EXPECTED_COUNTS = {
    "native_e2e": 4,
    "runtime_native_e2e": 1,
    "runtime_readiness_e2e": 1,
    "native_module_runtime_e2e": 1,
}


class ClientNativeTopologyTests(unittest.TestCase):
    def test_workspace_keeps_all_three_native_targets_in_the_client_package(self):
        client_manifest = tomllib.loads(
            (ROOT / "crates/tinychannels-client/Cargo.toml").read_text(encoding="utf-8")
        )
        self.assertIn("crates/tinychannels-client", tomllib.loads(
            (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        )["workspace"]["members"])
        self.assertEqual(client_manifest["features"]["native-e2e"], [])
        targets = {target["name"]: target for target in client_manifest["test"]}
        self.assertEqual(set(targets), set(TARGETS))
        for target in targets.values():
            self.assertEqual(target["required-features"], ["native-e2e"])

    def test_all_native_cases_remain_unchanged_and_not_ignored(self):
        self.assertEqual(sum(EXPECTED_COUNTS.values()), 7)
        for target, names in TARGETS.items():
            source = (ROOT / "crates/tinychannels-client/tests" / f"{target}.rs").read_text(
                encoding="utf-8"
            )
            self.assertEqual(
                len(re.findall(r"(?m)^\s*#\[(?:tokio::)?test(?:\([^\n]*\))?\]", source)),
                EXPECTED_COUNTS[target],
            )
            for name in names:
                self.assertRegex(source, rf"(?m)^async fn {re.escape(name)}\(")
            self.assertNotRegex(source, r"(?m)^\s*#\[ignore(?:\(|\])")

    def test_runtime_bind_diagnostic_is_finite_and_content_free(self):
        support = (ROOT / "crates/tinychannels-client/tests/support/mod.rs").read_text(
            encoding="utf-8"
        )
        runtime_test = (ROOT / "crates/tinychannels-client/tests/runtime_native_e2e.rs").read_text(
            encoding="utf-8"
        )
        for stage in (
            "preparation_not_started",
            "cached_load_in_progress",
            "cached_load_failed",
            "cached_load_complete",
            "readiness_in_progress",
            "readiness_failed",
            "ready_before_peer_resolution",
        ):
            self.assertIn(f'"{stage}"', support)
        self.assertIn("native fixture stage: {}", runtime_test)
        self.assertIn("fixture.bind_stage()", runtime_test)

    def test_native_ci_runners_call_every_target_and_enforce_test_counts(self):
        shell_runner = (ROOT / "scripts/test-client-e2e.sh").read_text(encoding="utf-8")
        powershell_runner = (ROOT / "scripts/test-client-e2e.ps1").read_text(encoding="utf-8")
        self.assertIn("--prepare-only", shell_runner)
        self.assertIn("run_counted_test() {", shell_runner)
        self.assertIn("function Invoke-CountedNativeTest", powershell_runner)
        self.assertIn("& cargo @Arguments", powershell_runner)
        self.assertEqual(powershell_runner.count("-Arguments (@('test') + $common + @("), 4)
        for target, count in EXPECTED_COUNTS.items():
            self.assertEqual(shell_runner.count(f"run_counted_test {count} {target}"), 1)
            self.assertIn(f"--test {target}", shell_runner)
            self.assertIn(f"-Name '{target}' -ExpectedCount {count}", powershell_runner)
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
        self.assertIn('($environment -join "`n") + "`n"', powershell_runner)
        self.assertIn("[System.IO.File]::WriteAllText", powershell_runner)
        self.assertNotIn("[System.IO.File]::WriteAllLines", powershell_runner)

    def test_feature_and_coverage_runs_prepare_fixture_before_all_feature_tests(self):
        feature_runner = (ROOT / "scripts/test-features.sh").read_text(encoding="utf-8")
        self.assertIn("python3 scripts/test-client-native-topology.py", feature_runner)
        self.assertIn("cargo test --locked --workspace --all-features", feature_runner)
        prepare = feature_runner.index("scripts/test-client-e2e.sh --prepare-only")
        source = feature_runner.index("source target/tinychannels-client-e2e.env")
        test = feature_runner.index("cargo test --locked --workspace --all-features")
        self.assertLess(prepare, source)
        self.assertLess(source, test)

        coverage_runner = (ROOT / "scripts/check-coverage.sh").read_text(encoding="utf-8")
        prepare = coverage_runner.index("scripts/test-client-e2e.sh --prepare-only")
        source = coverage_runner.index("source target/tinychannels-client-e2e.env")
        coverage = coverage_runner.index("check-file-coverage.sh 90 coverage.json")
        self.assertLess(prepare, source)
        self.assertLess(source, coverage)
        coverage_check = (ROOT / ".github/scripts/check-file-coverage.sh").read_text(
            encoding="utf-8"
        )
        self.assertIn("cargo llvm-cov", coverage_check)
        self.assertIn("--workspace", coverage_check)
        self.assertIn("--all-targets", coverage_check)
        self.assertIn("--all-features", coverage_check)
        self.assertIn("check-file-coverage.sh 90 coverage.json", coverage_runner)


if __name__ == "__main__":
    unittest.main()
