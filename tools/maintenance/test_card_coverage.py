"""Checks for explicit integration-test coverage selection."""
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import unittest

spec = importlib.util.spec_from_file_location("card_coverage", Path(__file__).with_name("card-coverage.py"))
coverage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(coverage)


def result(passed=1, code=0, artifacts=None):
    if artifacts is None:
        artifacts = [{"reason": "compiler-artifact", "package_id": "pkg#1", "target": {"name": "typed", "kind": ["test"]}, "profile": {"test": True}, "executable": "/tmp/typed"}]
    return SimpleNamespace(returncode=code, stdout="\n".join(json.dumps(x) for x in artifacts) + f"\ntest result: ok. {passed} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")


class TestIntegrationSelection(unittest.TestCase):
    def test_accepts_real_nonempty_test_binary(self):
        self.assertEqual(coverage.test_binary(result(), "pkg#1", "typed"), "/tmp/typed")

    def test_rejects_empty_failed_or_wrong_artifact(self):
        self.assertIsNone(coverage.test_binary(result(passed=0), "pkg#1", "typed"))
        self.assertIsNone(coverage.test_binary(result(code=1), "pkg#1", "typed"))
        self.assertIsNone(coverage.test_binary(result(), "other", "typed"))
        self.assertIsNone(coverage.test_binary(result(), "pkg#1", "other"))
        self.assertIsNone(coverage.test_binary(result(artifacts=[]), "pkg#1", "typed"))

    def test_rejects_ambiguous_binaries(self):
        art = {"reason": "compiler-artifact", "package_id": "pkg#1", "target": {"name": "typed", "kind": ["test"]}, "profile": {"test": True}}
        self.assertIsNone(coverage.test_binary(result(artifacts=[dict(art, executable="a"), dict(art, executable="b")]), "pkg#1", "typed"))


if __name__ == "__main__":
    unittest.main()
