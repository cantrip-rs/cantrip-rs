"""Tests of what card-mutate.py builds when it builds few cards:

    python -m unittest tools/maintenance/test_card_mutate.py
"""

import importlib.util
import os
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
spec = importlib.util.spec_from_file_location("card_mutate", os.path.join(HERE, "card-mutate.py"))
card_mutate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(card_mutate)


class DeckCards(unittest.TestCase):
    def test_every_way_of_saying_a_card(self):
        text = ("# Mox Ruby in a comment\n[Bot1 Deck]\nMountain\n2 Lightning Bolt\nIsland x20\nIsland X3\n[Bot1 Sideboard]\nFury\n"
                "[Bot2 Companion]\nLurrus\n[Life]\nBot1 7\n[Actions]\nBot1.Hand.Fury.Cast\n")
        self.assertEqual(card_mutate.deck_cards(text), ["Mountain", "Lightning Bolt", "Island", "Island", "Fury", "Lurrus"])


class Played(unittest.TestCase):
    def setUp(self):
        self.tree = tempfile.TemporaryDirectory()
        self.addCleanup(self.tree.cleanup)
        root = self.tree.name
        cards = os.path.join(root, "cards")
        scenarios = os.path.join(root, "scenarios")
        os.mkdir(cards)
        os.mkdir(scenarios)
        for stem, text in (("alpha", "prepare: Some(&crate::beta::CARD)"), ("beta", "use super::*;"), ("gamma", ""), ("filler", ""),
                           ("zeta", "pub static EXTRA: &[&CardDef] = &[&EMBLEM];\n"),
                           ("kappa", "pub static EXTRA: &[&CardDef] = &[\n    &tokens::MAP,\n    &LOCAL,\n];\n"),
                           ("omega", "pub static EXTRA: &[&CardDef] = &[&tokens::MAP];\n")):
            with open(os.path.join(cards, stem + ".rs"), "w", encoding="utf-8") as f:
                f.write(text)
        for name, text in (("a", "[Bot1 Deck]\nAlpha\nFiller x3\n[Bot2 Deck]\n2 Island\n[Actions]\nGamma\n"), ("b", "[Bot1 Deck]\nGamma\n[Actions]\n")):
            with open(os.path.join(scenarios, name + ".cantrip"), "w", encoding="utf-8") as f:
                f.write(text)
        self.saved = card_mutate.new_card.SCRIPTS, card_mutate.new_card.CARDS
        self.addCleanup(self.restore)
        card_mutate.new_card.SCRIPTS, card_mutate.new_card.CARDS = scenarios, cards

    def restore(self):
        card_mutate.new_card.SCRIPTS, card_mutate.new_card.CARDS = self.saved

    def test_the_cards_played_and_the_files_they_name(self):
        self.assertEqual(card_mutate.played("Alpha"), {"alpha", "beta", "filler", "kappa"})

    def test_a_card_only_named_in_the_actions_is_not_played(self):
        self.assertNotIn("gamma", card_mutate.played("Alpha"))

    def test_a_card_in_no_deck_is_only_itself(self):
        self.assertEqual(card_mutate.played("Beta"), {"beta", "kappa"})

    def test_one_file_brings_each_shared_definition(self):
        self.assertEqual(card_mutate.shared_definitions(), {"kappa"})


if __name__ == "__main__":
    unittest.main()
