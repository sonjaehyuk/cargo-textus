"""Test publishing decisions with a fake Cargo; never contact a registry."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "publish-crates.sh"


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="textus-publish-test-")
        self.addCleanup(self.directory.cleanup)
        self.directory_path = Path(self.directory.name)
        self.log = self.directory_path / "calls.json"
        fake_cargo = self.directory_path / "cargo"
        fake_cargo.write_text(
            """#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
if sys.argv[1:] == ["--version"]:
    print(os.environ.get("TEST_CARGO_VERSION", "cargo 1.95.0 (test)"))
else:
    Path(os.environ["TEST_CARGO_LOG"]).write_text(json.dumps({
        "args": sys.argv[1:],
        "cwd": os.getcwd(),
        "token_present": bool(os.environ.get("CARGO_REGISTRY_TOKEN")),
        "language_present": "TEXTUS_LANG" in os.environ,
    }))
    sys.exit(int(os.environ.get("TEST_CARGO_EXIT", "0")))
""",
            encoding="utf-8",
        )
        fake_cargo.chmod(0o700)
        self.env = os.environ.copy()
        self.env.pop("CARGO_REGISTRY_TOKEN", None)
        self.env.update(
            PATH=str(self.directory_path) + os.pathsep + self.env["PATH"],
            TEST_CARGO_LOG=str(self.log),
            TEXTUS_LANG="ko",
        )

    def run_script(self, *args, trace=False):
        return subprocess.run(
            ["bash", *(["-x"] if trace else []), str(SCRIPT), *args],
            cwd=self.directory_path,
            env=self.env,
            text=True,
            capture_output=True,
            check=False,
        )

    def recorded(self):
        return json.loads(self.log.read_text(encoding="utf-8"))

    def test_default_never_uploads_and_selects_only_release_crates(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        call = self.recorded()
        self.assertEqual(call["args"], [
            "publish", "--registry", "crates-io", "--locked", "--dry-run",
            "--package", "textus-core", "--package", "cargo-textus",
        ])
        self.assertFalse(call["token_present"])
        self.assertFalse(call["language_present"])
        self.assertEqual(Path(call["cwd"]), ROOT)

    def test_publication_requires_token_before_calling_cargo(self):
        result = self.run_script("--publish")
        self.assertEqual(result.returncode, 2)
        self.assertIn("CARGO_REGISTRY_TOKEN is required", result.stderr)
        self.assertFalse(self.log.exists())

    def test_explicit_publication_keeps_token_out_of_arguments_and_logs(self):
        token = "synthetic-test-token-not-a-credential"
        self.env["CARGO_REGISTRY_TOKEN"] = token
        result = self.run_script("--publish", "--package", "cargo-textus", trace=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(token, result.stdout + result.stderr + self.log.read_text())
        call = self.recorded()
        self.assertTrue(call["token_present"])
        self.assertEqual(call["args"], [
            "publish", "--registry", "crates-io", "--locked", "--package", "cargo-textus",
        ])

    def test_rejects_unknown_packages_and_ambiguous_modes(self):
        for args in [
            ("--package", "textus-demo"),
            ("--package", "textus"),
            ("--package", "textus; echo injected"),
            ("--package",),
            ("--package", "cargo-textus", "--package", "textus-core"),
            ("--dry-run", "--publish"),
            ("--allow-dirty",),
        ]:
            with self.subTest(args=args):
                self.assertEqual(self.run_script(*args).returncode, 2)
                self.assertFalse(self.log.exists())

    def test_rejects_cargo_before_multi_package_support(self):
        self.env["TEST_CARGO_VERSION"] = "cargo 1.89.0 (test)"
        result = self.run_script()
        self.assertEqual(result.returncode, 2)
        self.assertIn("1.90 or newer", result.stderr)
        self.assertFalse(self.log.exists())

    def test_preserves_cargo_failure(self):
        self.env["TEST_CARGO_EXIT"] = "101"
        self.assertEqual(self.run_script("--dry-run").returncode, 101)


if __name__ == "__main__":
    unittest.main()
