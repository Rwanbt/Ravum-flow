"""Harness self-tests: fast, no fixture build, no network.

Run: python -m unittest discover -s experiments/r0a/tests -v
"""
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from harness import common  # noqa: E402
from harness.redact import leaks, redact, write_redacted  # noqa: E402


class RedactionTests(unittest.TestCase):
    def test_canary_is_redacted(self) -> None:
        text, count = redact(f"value={common.CANARY} other")
        self.assertNotIn(common.CANARY, text)
        self.assertGreaterEqual(count, 1)
        self.assertEqual(leaks(text), [])

    def test_bearer_is_redacted(self) -> None:
        text, count = redact("Authorization: Bearer abcDEF1234567890")
        self.assertIn("[REDACTED]", text)
        self.assertNotIn("abcDEF1234567890", text)
        self.assertEqual(count, 1)

    def test_write_redacted_persists_only_clean_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "log.txt"
            write_redacted(target, f"canary {common.CANARY}\n")
            self.assertNotIn(common.CANARY, target.read_text())
            self.assertEqual(leaks(target.read_text()), [])

    def test_leaks_detects_a_token_shape(self) -> None:
        self.assertNotEqual(leaks("ghp_" + "a" * 24), [])


class TreeIdentityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name) / "repo"
        common.init_repo(self.repo)
        (self.repo / "a.txt").write_text("alpha\n", encoding="utf-8")
        common.commit_all(self.repo, "test: seed")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_tree_digest_is_stable_and_content_addressed(self) -> None:
        first = common.tree_digest(self.repo)
        second = common.tree_digest(self.repo)
        self.assertEqual(first["digest"], second["digest"])
        (self.repo / "a.txt").write_text("beta\n", encoding="utf-8")
        self.assertEqual(common.tree_digest(self.repo)["digest"], first["digest"])
        common.commit_all(self.repo, "test: change")
        self.assertNotEqual(common.tree_digest(self.repo)["digest"], first["digest"])

    def test_worktree_digest_tracks_dirty_files(self) -> None:
        before = common.worktree_digest(self.repo)["digest"]
        (self.repo / "a.txt").write_text("dirty\n", encoding="utf-8")
        self.assertNotEqual(common.worktree_digest(self.repo)["digest"], before)

    def test_status_reports_dirty_and_untracked(self) -> None:
        (self.repo / "a.txt").write_text("dirty\n", encoding="utf-8")
        (self.repo / "new.txt").write_text("new\n", encoding="utf-8")
        status = common.status_porcelain(self.repo)
        self.assertIn(" M a.txt", status)
        self.assertIn("?? new.txt", status)


class CleanupSafetyTests(unittest.TestCase):
    def test_cleanup_refuses_outside_path(self) -> None:
        outside = Path(tempfile.mkdtemp())
        try:
            (outside / "keep.txt").write_text("keep", encoding="utf-8")
            result = common.cleanup()
            self.assertTrue((outside / "keep.txt").exists())
            for skipped in result["skipped"]:
                self.assertEqual(skipped["reason"], "outside_work_root")
        finally:
            (outside / "keep.txt").unlink(missing_ok=True)
            outside.rmdir()


class ManifestShapeTests(unittest.TestCase):
    def test_environment_has_expected_keys(self) -> None:
        env = common.manifest_environment()
        for key in ("os", "python", "git", "cargo", "cpu_count"):
            self.assertIn(key, env)
        json.dumps(env)  # must be JSON-serialisable


if __name__ == "__main__":
    unittest.main()
