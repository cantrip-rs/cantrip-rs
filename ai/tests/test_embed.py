"""The card-text vectors are made, never checked in: fitted once, kept by a checkpoint, and a new card projected into
the same basis without moving the rows already made.

    python -m unittest discover -s ai/tests
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from cantrip_ai import embed  # noqa: E402

KNOWN = {
    "Shock": "Instant\n{R}\ncardname deals 2 damage to any target.",
    "Lightning Bolt": "Instant\n{R}\ncardname deals 3 damage to any target.",
    "Opt": "Instant\n{U}\nScry 1.\nDraw a card.",
    "Divination": "Sorcery\n{2}{U}\nDraw two cards.",
    "Grizzly Bears": "Creature - Bear\n{1}{G}\n",
    "Llanowar Elves": "Creature - Elf Druid\n{G}\n{T}: Add {G}.",
    "Spark Jolt": "Instant\n{R}\ncardname deals 1 damage to any target.\nScry 1.",
}


class EmbedTest(unittest.TestCase):
    def test_projecting_the_corpus_remakes_its_rows(self):
        names = ["Divination", "Grizzly Bears", "Lightning Bolt", "Llanowar Elves", "Opt", "Shock"]
        table, basis = embed.rows(names, None, KNOWN)
        self.assertEqual(table.shape, (len(names) + 1, embed.DIM))
        self.assertTrue((table[0] == 0).all())
        norms = np.linalg.norm(table[1:], axis=1)
        np.testing.assert_allclose(norms[norms > 0], 1.0, atol=1e-5)
        again = embed.project(embed.Basis.from_state(basis.state()), [KNOWN[n] for n in names])
        np.testing.assert_allclose(again, table[1:], atol=1e-5)

    def test_a_name_without_text_is_zeros(self):
        table, _ = embed.rows(["Shock", "Soldier token", "Opt"], None, KNOWN)
        self.assertTrue((table[2] == 0).all())

    def test_extending_keeps_the_rows_and_places_the_new_card(self):
        names = ["Divination", "Grizzly Bears", "Lightning Bolt", "Llanowar Elves", "Opt", "Shock"]
        table, basis = embed.rows(names, None, KNOWN)
        grown = embed.extend(table, names + ["Spark Jolt"], basis, KNOWN)
        np.testing.assert_array_equal(grown[: len(table)], table)
        jolt = grown[-1]
        # Nearer the burn spells than the creatures.
        self.assertGreater(float(jolt @ table[names.index("Shock") + 1]), float(jolt @ table[names.index("Grizzly Bears") + 1]))
        # No basis: the new card is the unknown card's zeros.
        self.assertTrue((embed.extend(table, names + ["Spark Jolt"], None, KNOWN)[-1] == 0).all())

    def test_an_older_checkpoints_basis_is_fitted_again(self):
        names = ["Divination", "Grizzly Bears", "Lightning Bolt", "Llanowar Elves", "Opt", "Shock"]
        table, _ = embed.rows(names, None, KNOWN)
        self.assertIsNotNone(embed.legacy_basis(names, table, log=lambda *_: None, known=KNOWN))
        other = table.copy()
        other[1:] = other[1:][::-1]
        self.assertIsNone(embed.legacy_basis(names, other, log=lambda *_: None, known=KNOWN))


if __name__ == "__main__":
    unittest.main()
