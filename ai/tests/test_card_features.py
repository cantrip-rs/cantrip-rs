"""Card templates (`cantrip_ai.card_features`) and the templates network (`model.Config.card_features`): what a card's
items are, that the values line up with their templates, that a text network is as it was, and that a templates
network folds, grows and serves without its vectors moving."""

from __future__ import annotations

import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import card_features, dataset, export, model, serve  # noqa: E402

NAMES = ["Lightning Bolt", "Shock", "Hero of Bladehold", "Bedeck // Bedazzle", "Llanowar Elves"]
MORE = ["Counterspell", "Sheoldred, the Apocalypse"]


def batch(rows: int, tokens: int, cards: int, seed: int = 0) -> dict:
    """Random inputs of the encoder's shapes."""
    g = torch.Generator().manual_seed(seed)
    kind = torch.randint(1, model.KINDS, (rows, tokens), generator=g)
    kind[:, -2:] = 0
    cats = torch.stack([torch.randint(0, v, (rows, tokens), generator=g) for v in model.VOCABS], dim=-1)
    ptrs = torch.full((rows, tokens, model.PTRS), model.NO_POINTER, dtype=torch.long)
    ptrs[:, :, 0] = torch.randint(0, tokens, (rows, tokens), generator=g)
    return dict(kind=kind, card=torch.randint(0, cards + 1, (rows, tokens), generator=g), cats=cats, ptrs=ptrs,
                nums=torch.randn(rows, tokens, model.NUMS, generator=g), legal=torch.rand(rows, tokens, generator=g) > 0.5)


def run(net, x):
    with torch.no_grad():
        return net(x["kind"], x["card"], x["cats"], x["ptrs"], x["nums"], x["legal"])


class Items(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = card_features.card_items()
        cls.ai = card_features.ability_index()

    def keys(self, name):
        return [k for k, _ in self.items[name]]

    def test_bolt_and_shock_share_a_template_with_their_own_values(self):
        bolt = dict(self.items["Lightning Bolt"])
        shock = dict(self.items["Shock"])
        key = "part:effect|~ deal N damage to any target"
        self.assertEqual(bolt[key], ["3"])
        self.assertEqual(shock[key], ["2"])
        self.assertEqual(set(bolt), set(shock))

    def test_a_trigger_is_an_item_other_cards_share(self):
        hero = self.keys("Hero of Bladehold")
        self.assertIn("keyword|battle cry", hero)
        self.assertIn("part:trigger|whenever ~ attack", hero)
        others = [n for n, its in self.items.items() if n != "Hero of Bladehold"
                  and any(k == "part:trigger|whenever ~ attack" for k, _ in its)]
        self.assertTrue(others)

    def test_a_split_card_is_both_halves_and_each_half_its_own(self):
        card = json.loads(next(Path(card_features.REPO / "content").glob("*/cards/bedeck-bedazzle.json")).read_text(encoding="utf-8-sig"))
        halves = [h["name"] for h in card["halves"]]
        whole = self.items[card["name"]]
        self.assertEqual(whole, [i for h in halves for i in self.items[h]])
        self.assertNotEqual(self.items[halves[0]], self.items[halves[1]])

    def test_a_back_face_has_its_own_items(self):
        for path in sorted(Path(card_features.REPO / "content").glob("*/cards/*.json")):
            card = json.loads(path.read_text(encoding="utf-8-sig"))
            back = card.get("backFace")
            if isinstance(back, dict) and back.get("oracleText") and card.get("oracleText"):
                break
        else:
            self.skipTest("no double-faced card with text on both faces")
        self.assertIn(back["name"], self.items)
        self.assertTrue(self.items[back["name"]])
        self.assertNotEqual(self.items[card["name"]], self.items[back["name"]])

    def test_every_value_fills_an_N_or_an_M_of_its_template(self):
        for name, its in self.items.items():
            for key, values in its:
                if key.startswith(("ability|", "part:")):
                    slots = len(re.findall(r"\{M\}|N", key.split("|", 1)[1]))
                    self.assertEqual(slots, len(values), f"{name}: {key} {values}")

    def test_value_features(self):
        mana = card_features.value_features("{2}{r}")
        self.assertEqual((mana[0], mana[3], mana[4], mana[5 + card_features.COLOURS.index("r")]), (1, 1, 0.75, 1))
        self.assertAlmostEqual(card_features.value_features("+3")[1], np.log1p(3), places=6)
        self.assertAlmostEqual(card_features.value_features("-3")[1], -np.log1p(3), places=6)
        self.assertAlmostEqual(card_features.value_features("two")[1], np.log1p(2), places=6)
        self.assertEqual(card_features.value_features("x")[2], 1)
        self.assertEqual(card_features.value_features("{t}")[11], 1)
        hybrid = card_features.value_features("{w/u}")
        self.assertEqual((hybrid[5], hybrid[6], hybrid[4]), (0.5, 0.5, 0.25))

    def test_build_is_deterministic_and_extend_keeps_the_old_rows(self):
        a = card_features.build(NAMES, self.items)
        b = card_features.build(NAMES, self.items)
        self.assertEqual(a.vocab, b.vocab)
        np.testing.assert_array_equal(a.ids, b.ids)
        np.testing.assert_array_equal(a.values, b.values)
        grown = card_features.extend(a, NAMES + MORE, len(NAMES), self.items)
        self.assertEqual(grown.vocab[: len(a.vocab)], a.vocab)
        old = grown.rows <= len(NAMES)
        np.testing.assert_array_equal(grown.ids[old], a.ids)
        np.testing.assert_array_equal(grown.values[old][:, : a.slots], a.values)
        self.assertEqual(set(grown.rows[~old].tolist()), {len(NAMES) + 1, len(NAMES) + 2})


class Network(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = card_features.card_items()
        rng = np.random.default_rng(0)
        cls.table = rng.standard_normal((len(NAMES) + 1, 64)).astype(np.float32)
        cls.config = model.Config(width=32, layers=2, heads=2, ffn=64, card_features="templates")
        cls.features = card_features.build(NAMES, cls.items)
        cls.x = batch(4, 16, len(NAMES))

    def templates_net(self, seed=0):
        net = model.build(self.config, self.table, seed, self.features.buffers())
        # Trained-looking embeddings: zero-initialised ones would hide a lost or misplaced row.
        with torch.no_grad():
            net.templates.weight.normal_()
        return net.eval()

    def test_a_text_network_is_built_as_before(self):
        config = model.Config(width=32, layers=2, heads=2, ffn=64)
        net = model.build(config, self.table, 3).eval()
        self.assertFalse([k for k in net.state_dict() if k.startswith(("feat_", "templates", "template_item"))])
        again = model.from_state(config, net.state_dict())
        again.load_state_dict(net.state_dict())
        for a, b in zip(run(net, self.x), run(again.eval(), self.x)):
            torch.testing.assert_close(a, b, rtol=0, atol=0)
        self.assertEqual(model.must_fold(net), config.width < self.table.shape[1])  # the old rule, unchanged

    def test_templates_change_the_card_vectors(self):
        net = self.templates_net()
        with torch.no_grad():
            text = net.card(net.table)
            full = net.card_table()
        self.assertTrue(torch.equal(text[0], full[0]))  # the unknown card has no items
        self.assertFalse(torch.allclose(text[1:], full[1:]))
        # Bolt and Shock differ only in their values.
        self.assertFalse(torch.allclose(full[1] - text[1], full[2] - text[2]))

    def test_folded_is_the_same_function(self):
        net = self.templates_net()
        before = run(net, self.x)
        self.assertTrue(model.must_fold(net))
        model.fold(net)
        for a, b in zip(before, run(net, self.x)):
            torch.testing.assert_close(a, b, rtol=1e-5, atol=1e-5)

    def test_from_state_round_trip(self):
        net = self.templates_net()
        again = model.from_state(self.config, net.state_dict())
        again.load_state_dict(net.state_dict())
        for a, b in zip(run(net, self.x), run(again.eval(), self.x)):
            torch.testing.assert_close(a, b, rtol=0, atol=0)

    def test_growing_keeps_every_old_card_vector(self):
        net = self.templates_net()
        names = NAMES + MORE
        grown = card_features.extend(self.features, names, len(NAMES), self.items)
        table = np.concatenate([self.table, np.random.default_rng(1).standard_normal((len(MORE), 64)).astype(np.float32)])
        bigger = model.build(self.config, table, 5, grown.buffers())
        state = model.grow_state(bigger, dict(net.state_dict(), table=torch.from_numpy(table)))
        bigger.load_state_dict(state)
        with torch.no_grad():
            torch.testing.assert_close(bigger.card_table()[: len(NAMES) + 1], net.card_table(), rtol=1e-6, atol=1e-6)

    def test_served_and_exported(self):
        net = self.templates_net()
        model.fold(net)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            checkpoint = dict(config=model.config_dict(self.config), state=net.state_dict())
            export.write_weights(net, checkpoint, out / "model.pt")
            (out / "manifest.json").write_text(json.dumps(dict(schema=model.LAYOUT["schema"], encoder_hash=dataset.encoder_hash())))
            served = serve.Served(out, torch.device("cpu"))
            for a, b in zip(run(net, self.x), run(served.net, self.x)):
                torch.testing.assert_close(a, b, rtol=0, atol=0)
            try:
                import onnxruntime as ort  # noqa: PLC0415
            except ImportError:
                return
            path = out / "model-t16.onnx"
            ops = export.export(net, 16, path)
            self.assertNotIn("EmbeddingBag", ops)
            session = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
            got = session.run(None, {i.name: self.x[i.name].numpy() for i in session.get_inputs()})
            for a, b in zip(run(net, self.x), got):
                np.testing.assert_allclose(a.numpy(), b, rtol=1e-3, atol=1e-3)


if __name__ == "__main__":
    unittest.main()
