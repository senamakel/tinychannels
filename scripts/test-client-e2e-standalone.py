#!/usr/bin/env python3
"""Exercise the client E2E script from a standalone checkout with fake Cargo."""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import shutil
import subprocess


ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target" / "evidence" / "client-e2e-standalone-smoke"


def fail(message: str) -> None:
    raise SystemExit(message)


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    path.chmod(0o755)


def main() -> None:
    if WORK.exists():
        shutil.rmtree(WORK)
    repo = WORK / "tinychannels"
    (repo / "scripts").mkdir(parents=True)
    shutil.copy2(ROOT / "Cargo.toml", repo / "Cargo.toml")
    shutil.copy2(ROOT / "scripts" / "test-client-e2e.sh", repo / "scripts")
    shutil.copy2(ROOT / "scripts" / "test-client-e2e.ps1", repo / "scripts")
    shutil.copytree(
        ROOT / "crates" / "tinychannels-client" / "test-fixture",
        repo / "crates" / "tinychannels-client" / "test-fixture",
        ignore=shutil.ignore_patterns("src", "target"),
    )

    fake_bin = WORK / "bin"
    write(
        fake_bin / "rustc",
        "#!/bin/sh\nprintf '%s\\n' 'host: aarch64-apple-darwin'\n",
    )
    cargo_stub = fake_bin / "cargo_stub.py"
    write(
        cargo_stub,
        """#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

args = sys.argv[1:]
root = Path(os.environ['TINYCHANNELS_FAKE_REPO'])
target = Path(os.environ['TINYCHANNELS_FAKE_TARGET'])
record = {
    'args': args,
    'fixture_cache': os.environ.get('TINYCHANNELS_CLIENT_TEST_CACHE'),
    'native_cache': os.environ.get('TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_CACHE'),
}
with Path(os.environ['TINYCHANNELS_FAKE_CARGO_LOG']).open('a', encoding='utf-8') as stream:
    stream.write(json.dumps(record) + '\\n')
if not args:
    raise SystemExit('fake cargo received no command')
if args[0] == 'build':
    output = target / 'debug'
    output.mkdir(parents=True, exist_ok=True)
    library = 'libtinychannels_module.dylib' if '-p' in args else 'libtinychannels_client_test_fixture.dylib'
    (output / library).write_bytes(b'fake native library: ' + library.encode())
elif args[0] != 'test':
    raise SystemExit(f'unexpected fake cargo command: {args[0]}')
""",
    )
    write(fake_bin / "cargo", f"#!/bin/sh\nexec python3 '{cargo_stub}' \"$@\"\n")

    cargo_log = WORK / "cargo.jsonl"
    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{fake_bin}{os.pathsep}{env['PATH']}",
            "TINYCHANNELS_FAKE_REPO": str(repo),
            "TINYCHANNELS_FAKE_TARGET": str(repo / "target"),
            "TINYCHANNELS_FAKE_CARGO_LOG": str(cargo_log),
        }
    )
    result = subprocess.run(
        ["bash", str(repo / "scripts" / "test-client-e2e.sh")],
        cwd=repo,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        fail(
            "standalone Bash E2E script failed with fake Cargo "
            f"(exit {result.returncode}):\n{result.stdout}{result.stderr}"
        )

    bash_script = (ROOT / "scripts" / "test-client-e2e.sh").read_text(encoding="utf-8")
    powershell_script = (ROOT / "scripts" / "test-client-e2e.ps1").read_text(
        encoding="utf-8"
    )
    for label, script in (("Bash", bash_script), ("PowerShell", powershell_script)):
        if "ci-cancel-aware.sh" in script or "../../scripts" in script:
            fail(f"{label} client E2E script depends on files outside its standalone repository")
    if re.search(r"\$cancelAware|&\s*bash\s+\$cancelAware", powershell_script):
        fail("PowerShell client E2E script still wraps Cargo through an external helper")

    calls = [json.loads(line) for line in cargo_log.read_text(encoding="utf-8").splitlines()]
    if len(calls) != 5 or [call["args"][0] for call in calls] != [
        "build",
        "build",
        "test",
        "test",
        "test",
    ]:
        fail(f"expected two builds and three test commands, captured: {calls!r}")

    expected_tests = ["native_e2e", "runtime_native_e2e", "native_module_runtime_e2e"]
    for call, expected in zip(calls[2:], expected_tests, strict=True):
        args = call["args"]
        if "--test" not in args or args[args.index("--test") + 1] != expected:
            fail(f"expected Cargo test target {expected}, captured: {args!r}")
        manifest = Path(args[args.index("--manifest-path") + 1])
        if manifest != repo / "Cargo.toml":
            fail(f"Cargo test escaped the standalone repository: {manifest}")
        for key in ("fixture_cache", "native_cache"):
            if not call[key] or not Path(call[key]).is_relative_to(repo / "target"):
                fail(f"Cargo test {key} is missing or outside the standalone target")

    expected_outputs = [
        repo / "target" / "tinychannels-client-e2e" / "cache" / "tinychannels-module-client-test.tar.gz",
        repo / "target" / "tinychannels-module-client-native-e2e" / "cache" / "tinychannels-module-native-test.tar.gz",
    ]
    for output in expected_outputs:
        if not output.is_file():
            fail(f"expected local staged module archive: {output}")

    print("standalone client E2E scripts use only checkout-local files; fake Cargo captured 2 builds and 3 tests")


if __name__ == "__main__":
    main()
