"""Retention keeps the newest generations, pinned ones and every generation's record; it never touches champions."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import retention  # noqa: E402


class RetentionTest(unittest.TestCase):
    def test_keeps_the_newest_and_the_pinned(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for n in range(12):
                g = root / f"gen{n}"
                (g / "selfplay").mkdir(parents=True)
                (g / "selfplay" / "a.shard.zst").write_bytes(b"x")
                (g / "README.md").write_text("r")
                (g / "generation.json").write_text("{}")
                (root / "models" / f"gen{n}").mkdir(parents=True)
            (root / "gen2" / "PINNED").write_text("")
            (root / "champions" / "gen1").mkdir(parents=True)
            (root / "other").mkdir()

            self.assertTrue(retention.retire(root, keep=8, dry_run=True, log=lambda *_: None))
            self.assertTrue((root / "gen1" / "selfplay").exists(), "a dry run deletes nothing")

            retention.retire(root, keep=8, pins={"gen3"}, log=lambda *_: None)
            for n in range(12):
                g = root / f"gen{n}"
                kept = n >= 4 or n in (0, 2, 3)
                self.assertEqual((g / "selfplay").exists(), kept, g)
                self.assertEqual((root / "models" / g.name).exists(), kept, g)
                self.assertTrue((g / "README.md").exists() and (g / "generation.json").exists(), g)
                self.assertEqual((g / "RETIRED").exists(), not kept, g)
            self.assertTrue((root / "champions" / "gen1").exists())
            self.assertTrue((root / "other").exists())
            self.assertEqual(retention.retire(root, keep=8, pins={"gen3"}, log=lambda *_: None), [], "a second pass has nothing left")


if __name__ == "__main__":
    unittest.main()
