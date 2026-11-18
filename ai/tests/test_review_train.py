"""The training options of the review of 2026-11-13 (docs/ai-training.md, "Training options"): the per-sample columns
the dataset keeps (where a policy target came from, the fast searches, the per-option root values), the search targets'
policy-only copies, each loss option against its formula, the frozen networks, the weight average, the optimizer
carried into a run, the numbers read through a logarithm, and whole runs with the options on, on the CPU.

    nix-shell ai/shell.nix --run "python -m unittest discover -s ai/tests"
"""

from __future__ import annotations

import contextlib
import gc
import io
import json
import struct
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS.parent))
sys.path.insert(0, str(TESTS))
from cantrip_ai import dataset, embed, export, generation, model, targets, train  # noqa: E402
from test_dataset import write_shard  # noqa: E402

LAYOUT = dataset.LAYOUT
KIND = {k: i for i, k in enumerate(LAYOUT["kinds"])}
CAT = {c["name"]: c["slot"] for c in LAYOUT["categorical"]}
ZONE = {z: i + 1 for i, z in enumerate(LAYOUT["zones"])}
SELF, OPPONENT = 1, 2


def token(kind: str, card: int = 0, zone: str | None = None, controller: int = 0, owner: int = 0,
          numbers: dict | None = None) -> bytes:
    """A token's compact bytes: its kind, card row, zone, controller and owner, and `numbers` {slot: value}."""
    cats = bytearray(dataset.CATS)
    if zone is not None:
        cats[CAT["zone"]] = ZONE[zone]
    cats[CAT["controller"]] = controller
    cats[CAT["owner"]] = owner
    mask = bytearray(dataset.NUMS // 8)
    values = b""
    for slot in sorted(numbers or {}):
        mask[slot // 8] |= 1 << (slot % 8)
        values += np.float16(numbers[slot]).tobytes()
    return (bytes([KIND[kind], card & 0xFF, card >> 8]) + bytes(cats) + bytes([dataset.NO_POINTER] * dataset.PTRS)
            + bytes(mask) + values)


def position_of(tokens: list[bytes], options: int = 2) -> bytes:
    """The game token, `tokens`, then `options` legal options, each its own index (so a network tells them apart)."""
    index = next(f["offset"] for f in LAYOUT["features"]["option"] if f["name"] == "index")
    toks = [token("game")] + tokens + [token("option", numbers={index: i / 8}) for i in range(options)]
    first = len(toks) - options
    bits = ((1 << options) - 1).to_bytes((options + 7) // 8, "little")
    return bytes([LAYOUT["schema"], 32, len(toks), first, options, 0]) + bits + b"".join(toks)


def raw_sample(position: bytes, decision: int = 0, step: int = 0, turn: int = 1, seat: int = 0, outcome: int = 1,
               chosen: int = 0, visits=((0, 1.0),)) -> bytes:
    out = struct.pack("<BIHHbfBHH", seat, decision, step, turn, outcome, 0.0, 0, chosen, len(visits))
    for o, v in visits:
        out += struct.pack("<Hf", o, v)
    out += struct.pack("<H", 0)
    return out + struct.pack("<I", len(position)) + position


def selfplay_game(seed: int) -> tuple[dict, list[bytes]]:
    """A self-played game: decisions 0 and 4 full searches (0's second sub-step the network's own answer, which only a
    per-sample flag can tell), 1 unsearched, 2 a fast search, 3 a single option."""
    p = position_of([token("card", 1, "battlefield", SELF, SELF)])
    meta = dict(seed=seed, root_values=[[0, 0.5], [2, -0.25], [4, 0.1]], fast_decisions=[2])
    return meta, [
        raw_sample(p, 0, 0, visits=((0, 0.75), (1, 0.25))),
        raw_sample(p, 0, 1, chosen=1, visits=((1, 1.0),)),
        raw_sample(p, 1, visits=((0, 1.0),)),
        raw_sample(p, 2, visits=((0, 1.0),)),
        raw_sample(position_of([], 1), 3, visits=((0, 1.0),)),
        raw_sample(p, 4, chosen=1, visits=((0, 0.1), (1, 0.9))),
    ]


def imitation_game(seed: int) -> tuple[dict, list[bytes]]:
    p = position_of([token("card", 2, "battlefield", OPPONENT, OPPONENT)])
    return dict(seed=seed), [raw_sample(p, 0, outcome=-1, chosen=1, visits=((1, 1.0),)),
                             raw_sample(p, 1, outcome=-1, visits=((0, 1.0),))]


@contextlib.contextmanager
def per_sample_flags(flags: dict):
    """`shards.scan` as a reader that records them would make it: `flags[(seed, index)]` is the sample's
    (`searched`, `option_q`)."""
    original = dataset.shards.scan

    class Games:
        def __init__(self, gs):
            self.gs = gs

        def __iter__(self):
            for g in self.gs:
                samples = g.samples
                for i, s in enumerate(samples):
                    if (g.meta.get("seed"), i) in flags:
                        s.searched, s.option_q = flags[(g.meta.get("seed"), i)]
                yield type("Game", (), dict(meta=g.meta, samples=samples))()

        def close(self):
            self.gs.close()

    def scan(path):
        h, gs = original(path)
        return h, Games(gs)

    with unittest.mock.patch.object(dataset.shards, "scan", scan):
        yield


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
        root = Path(self.tmp.name)
        self.root = root
        self.selfplay, self.imitation = root / "selfplay", root / "imitation"
        self.selfplay.mkdir()
        self.imitation.mkdir()
        write_shard(self.selfplay / "a.shard.zst", "selfplay:m@64", [selfplay_game(1)])
        write_shard(self.imitation / "a.shard.zst", "tactical:x", [imitation_game(2)])

    def tearDown(self):
        gc.collect()
        self.tmp.cleanup()

    def load(self, **kw) -> dataset.Data:
        return dataset.load([self.selfplay, self.imitation], log=lambda *_: None, workers=0, **kw)


class Columns(Fixture):
    def test_where_each_policy_target_came_from(self):
        data = self.load()
        S, N, R = dataset.SEARCHED, dataset.NOT_SEARCHED, dataset.RECORDED
        np.testing.assert_array_equal(data.searched, [S, S, N, S, N, S, R, R])
        np.testing.assert_array_equal(data.fast, [0, 0, 0, 1, 0, 0, 0, 0])
        np.testing.assert_array_equal(data.policy, [1, 1, 1, 0, 1, 1, 1, 1])
        # By decision, the second sub-step of decision 0 counts as searched: the fallback's leak.
        np.testing.assert_array_equal(dataset.search_targets(data), [1, 1, 0, 0, 0, 1, 0, 0])
        self.assertEqual(len(data.q_value), 0)
        self.assertEqual([d["per_sample_searched"] for d in data.directories], [0, 0])

    def test_a_per_sample_flag_and_the_option_values(self):
        flags = {(1, i): (True, []) for i in range(6)}
        flags[(1, 1)] = (False, [])
        flags[(1, 2)] = (False, [])
        flags[(1, 0)] = (True, [(0, 0.3), (1, -0.1), (2, float("nan"))])
        with per_sample_flags(flags):
            data = self.load()
            pruned = self.load(searched_policy=True)
        np.testing.assert_array_equal(dataset.search_targets(data), [1, 0, 0, 0, 0, 1, 0, 0])
        self.assertEqual(data.directories[0]["per_sample_searched"], 6)
        # Under searched_policy the sub-step the search never reached is no policy target; imitation keeps its own.
        np.testing.assert_array_equal(pruned.policy, [1, 0, 0, 0, 1, 1, 1, 1])
        np.testing.assert_array_equal(np.diff(data.q_offset), [2, 0, 0, 0, 0, 0, 0, 0])
        rows = np.arange(len(data))
        labels = dataset.labels(data, rows, dataset.decode(data.blob, data.offset, rows))
        first = data.blob[data.offset[0] + 3]
        np.testing.assert_allclose(labels["q"][0, first : first + 2], [0.3, -0.1])
        self.assertTrue(np.isnan(labels["q"][0, first + 2 :]).all())
        self.assertTrue(np.isnan(labels["q"][1]).all())

    def test_search_targets_dealt_again_policy_only(self):
        data = self.load()
        rows = np.arange(len(data))
        plain = dataset.batches(data, rows, 64, np.random.default_rng(3))
        same = dataset.batches(data, rows, 64, np.random.default_rng(3), search_repeat=1.0)
        self.assertEqual([b.tolist() for b in plain], [b.tolist() for b in same], "a repeat of 1 deals as before")
        dealt = np.concatenate(dataset.batches(data, rows, 64, np.random.default_rng(3), search_repeat=4.0))
        real, fresh = dataset.unmark(data, dealt)
        self.assertEqual(int(fresh.sum()), len(data))
        np.testing.assert_array_equal(np.sort(real[~fresh]), [0, 0, 0, 1, 1, 1, 5, 5, 5])
        # Held out (no rng), nothing is dealt again.
        held = np.concatenate(dataset.batches(data, rows, 64, None, search_repeat=4.0))
        self.assertEqual(len(held), len(data))


class Losses(Fixture):
    def setUp(self):
        super().setUp()
        self.data = self.load()
        self.known = embed.texts()
        table, self.basis = embed.rows(self.data.names, None, self.known)
        self.table = table
        self.config = model.Config(width=16, layers=1, heads=2, ffn=32)
        self.net = model.build(self.config, table, 0)
        self.b = self.batch(self.data)

    def batch(self, data, rows=None):
        rows = np.arange(len(data)) if rows is None else rows
        return {k: v for k, v in train.collate(data)(rows).items()}

    def manual(self, b, net=None, value_target=None, policy_rows=None, target=None):
        """The loss as train.py wrote it before the options, from its own formula."""
        out = (net or self.net).train_heads(b["kind"], b["card"], b["cats"], b["ptrs"], b["nums"], b["legal"])
        logp = F.log_softmax(out["policy"], -1)
        visits = b["visits"] if target is None else target
        ce = -torch.where(visits > 0, visits * logp, torch.zeros_like(logp)).sum(-1)
        choice = b["choice"]
        rows = choice if policy_rows is None else choice & policy_rows
        policy = (ce * rows).sum() / choice.sum().clamp(min=1)
        want = b["outcome"] if value_target is None else value_target
        value = F.mse_loss(out["value"], want)
        aux = F.smooth_l1_loss(out["swing"], b["swing"] / model.SWING_SCALE) + F.binary_cross_entropy_with_logits(out["dead"], b["dead"])
        return policy, value, policy + value + train.AUX_WEIGHT * aux, out

    def test_the_defaults_are_the_old_loss(self):
        total, m = train.losses(self.net, self.b, False)
        again, _ = train.losses(self.net, self.b, False, opts=train.LossOptions())
        policy, value, expected, _ = self.manual(self.b)
        self.assertTrue(torch.equal(total, expected))
        self.assertTrue(torch.equal(again, expected))
        self.assertEqual(m["policy"], policy.item())
        self.assertEqual(m["search_choices"], 3)

    def test_value_mix_by_budget(self):
        _, m = train.losses(self.net, self.b, False, opts=train.LossOptions(value_mix_full=0.75, value_mix_fast=0.5))
        want = torch.tensor([0.25 + 0.75 * 0.5, 0.25 + 0.75 * 0.5, 1.0, 0.5 - 0.5 * 0.25, 1.0, 0.25 + 0.75 * 0.1, -1.0, -1.0])
        self.assertAlmostEqual(m["value"], self.manual(self.b, value_target=want)[1].item(), places=6)
        # Both at --value-mix's share: --value-mix's own target.
        _, a = train.losses(self.net, self.b, False, value_mix=0.5, opts=train.LossOptions(value_mix_full=0.5, value_mix_fast=0.5))
        _, b = train.losses(self.net, self.b, False, value_mix=0.5)
        self.assertAlmostEqual(a["value"], b["value"], places=7)
        # The lambda-return still wins over any mix.
        td = dict(self.b, td=torch.zeros(8))
        _, c = train.losses(self.net, td, False, opts=train.LossOptions(value_mix_full=0.75))
        self.assertAlmostEqual(c["value"], self.manual(self.b, value_target=torch.zeros(8))[1].item(), places=6)

    def test_imitation_policy_weight(self):
        _, m = train.losses(self.net, self.b, False, opts=train.LossOptions(imitation_policy_weight=0.0))
        # At 0 the imitation corpus is out of the policy: its loss is the loss of a batch of the self-played rows alone.
        selfplay = np.flatnonzero(self.data.searched != dataset.RECORDED)
        _, alone = train.losses(self.net, self.batch(self.data, selfplay), False)
        self.assertAlmostEqual(m["policy"], alone["policy"], places=6)
        # Between, a weighted mean: each row's cross-entropy at its weight, over the weights' sum.
        _, half = train.losses(self.net, self.b, False, opts=train.LossOptions(imitation_policy_weight=0.5))
        out = self.net.train_heads(self.b["kind"], self.b["card"], self.b["cats"], self.b["ptrs"], self.b["nums"], self.b["legal"])
        logp = F.log_softmax(out["policy"], -1)
        ce = -torch.where(self.b["visits"] > 0, self.b["visits"] * logp, torch.zeros_like(logp)).sum(-1)
        w = self.b["choice"] * torch.where(self.b["searched"] == dataset.RECORDED, 0.5, 1.0)
        self.assertAlmostEqual(half["policy"], ((ce * w).sum() / w.sum()).item(), places=6)

    def test_policy_only_copies_skip_the_other_losses(self):
        fresh = torch.tensor([1, 0, 1, 1, 1, 0, 1, 1], dtype=torch.float32)
        _, m = train.losses(self.net, dict(self.b, fresh=fresh), False)
        _, _, _, out = self.manual(self.b)
        expected = ((out["value"] - self.b["outcome"]) ** 2 * fresh).sum() / 8
        self.assertAlmostEqual(m["value"], expected.item(), places=6)
        _, plain = train.losses(self.net, self.b, False)
        self.assertEqual(m["policy"], plain["policy"], "the policy reads every row")

    def test_value_games_cap(self):
        weight = targets.game_cap(self.data, 3.0)
        np.testing.assert_allclose(weight, [0.5] * 6 + [1.0] * 2)
        thinned = targets.game_cap(self.data, 3.0, trivial=0.0)
        np.testing.assert_allclose(thinned, [0.6] * 6 + [1.0] * 2)
        self.data.value_weight = weight
        b = self.batch(self.data)
        _, m = train.losses(self.net, b, False)
        _, _, _, out = self.manual(b)
        expected = ((out["value"] - b["outcome"]) ** 2 * torch.from_numpy(weight)).sum() / 8
        self.assertAlmostEqual(m["value"], expected.item(), places=6)
        # The held-out value is against every position alike.
        held = train.evaluate(self.net, self.data, dataset.batches(self.data, np.arange(8), 64, None), 0, False, torch.device("cpu"))
        del self.data.value_weight
        plain = train.evaluate(self.net, self.data, dataset.batches(self.data, np.arange(8), 64, None), 0, False, torch.device("cpu"))
        self.assertEqual(held["value_mse"], plain["value_mse"])
        self.assertEqual(held["search_choices"], 3)

    def test_surprise_weighting(self):
        opts = train.LossOptions(surprise_weight=1.0, surprise=train.Surprise())
        opts.surprise.SEED = 1  # this batch's three targets set the mean
        _, m = train.losses(self.net, self.b, False, opts=opts)
        out = self.manual(self.b)[3]
        logp = F.log_softmax(out["policy"], -1).detach()
        v = self.b["visits"]
        kl = torch.where(v > 0, v * (torch.log(v.clamp(min=1e-12)) - logp), torch.zeros_like(v)).sum(-1)
        rows = torch.tensor([1, 1, 0, 0, 0, 1, 0, 0], dtype=torch.bool)
        self.assertAlmostEqual(opts.surprise.mean, kl[rows].mean().item(), places=5)
        scale = torch.where(rows, kl / kl[rows].mean(), torch.ones(8))
        ce = -torch.where(v > 0, v * F.log_softmax(out["policy"], -1), torch.zeros_like(v)).sum(-1)
        expected = (ce * self.b["choice"] * scale).sum() / self.b["choice"].sum()
        self.assertAlmostEqual(m["policy"], expected.item(), places=5)
        _, none = train.losses(self.net, self.b, False, opts=train.LossOptions(surprise_weight=0.0))
        self.assertEqual(none["policy"], self.manual(self.b)[0].item())
        # Until the mean is over `SEED` targets, they weigh 1: a first batch's few cannot set it.
        seeding = train.LossOptions(surprise_weight=1.0)
        _, first = train.losses(self.net, self.b, False, opts=seeding)
        self.assertIsNone(seeding.surprise.mean)
        self.assertEqual(first["policy"], self.manual(self.b)[0].item())

    def test_surprise_weights_are_capped_and_the_mean_seeded(self):
        s = train.Surprise(decay=0.5)
        self.assertEqual(s.update(torch.tensor([1.0] * 40)), 0.0, "fewer than SEED targets: no mean yet")
        self.assertAlmostEqual(s.update(torch.tensor([4.0] * 40)), (0.5 * 40 + 160) / (0.5 * 40 + 40))
        kl = torch.tensor([0.0, 1.0, 1000.0])
        np.testing.assert_allclose(s.weight(kl, 1.0, 2.0).numpy(), [0.0, 0.5, train.Surprise.CAP])
        np.testing.assert_allclose(s.weight(kl, 0.5, 2.0).numpy(), [0.5, 0.75, 0.5 + 0.5 * train.Surprise.CAP])

    def test_smoothing_and_entropy(self):
        _, m = train.losses(self.net, self.b, False, opts=train.LossOptions(policy_smoothing=0.1))
        legal = self.b["legal"].float()
        smoothed = 0.9 * self.b["visits"] + 0.1 * legal / legal.sum(-1, keepdim=True)
        self.assertAlmostEqual(m["policy"], self.manual(self.b, target=smoothed)[0].item(), places=6)
        total, e = train.losses(self.net, self.b, False, opts=train.LossOptions(entropy_weight=0.5))
        plain, _ = train.losses(self.net, self.b, False)
        self.assertGreater(e["entropy"], 0.0)
        self.assertAlmostEqual(total.item(), plain.item() - 0.5 * e["entropy"], places=5)

    def test_binary_cross_entropy_value(self):
        _, m = train.losses(self.net, self.b, False, opts=train.LossOptions(value_loss="bce"))
        out = self.manual(self.b)[3]
        expected = F.binary_cross_entropy_with_logits(out["logit"], (self.b["outcome"] + 1) / 2)
        self.assertAlmostEqual(m["value"], expected.item(), places=6)
        # The logit is the value's: tanh(logit / 2).
        torch.testing.assert_close(torch.tanh(0.5 * out["logit"]), out["value"])

    def checkpoint(self, net, path: Path) -> Path:
        torch.save(dict(config=model.config_dict(net.config), state=net.state_dict(), header=self.data.header,
                        cards=self.data.names, embed=self.basis.state(), step=7), path)
        return path

    def test_anchor_kl_and_a_frozen_network(self):
        path = self.checkpoint(self.net, self.root / "anchor.pt")
        anchor = train.Frozen(str(path), self.data.names, self.known, self.data.header, torch.device("cpu"), log=lambda *_: None)
        value, policy = anchor(self.b)
        out = self.manual(self.b)[3]
        torch.testing.assert_close(value, out["value"])
        torch.testing.assert_close(policy, out["policy"])
        opts = train.LossOptions(anchor_kl=1.0, anchor=anchor)
        _, m = train.losses(self.net, self.b, False, opts=opts)
        self.assertAlmostEqual(m["anchor"], 0.0, places=6)
        moved = model.build(self.config, self.table, 1)
        total, m = train.losses(moved, self.b, False, opts=opts)
        self.assertGreater(m["anchor"], 0.0)
        # On the self-played positions with a choice that are no search target: rows 2 (unsearched) and 3 (fast).
        out = moved.train_heads(*(self.b[k] for k in model.INPUTS))
        la, lp = F.log_softmax(policy, -1), F.log_softmax(out["policy"], -1)
        kl = torch.where(self.b["legal"], la.exp() * (la - lp), torch.zeros_like(la)).sum(-1)
        self.assertAlmostEqual(m["anchor"], kl[[2, 3]].mean().item(), places=5)
        total.backward()

    def test_a_frozen_network_reads_new_card_names(self):
        """A frozen network made for fewer names: the training index's rows map onto its own."""
        path = self.root / "old.pt"
        torch.save(dict(config=model.config_dict(self.config), state=self.net.state_dict(), header=self.data.header,
                        cards=self.data.names[:1], embed=self.basis.state(), step=1), path)
        state = dict(self.net.state_dict(), table=torch.from_numpy(self.table[:2].copy()))
        torch.save(dict(config=model.config_dict(self.config), state=state, header=self.data.header,
                        cards=self.data.names[:1], embed=self.basis.state(), step=1), path)
        frozen = train.Frozen(str(path), list(reversed(self.data.names)), self.known, self.data.header,
                              torch.device("cpu"), log=lambda *_: None)
        self.assertEqual(frozen.lut.tolist(), [0, 2, 1])

    def test_distillation_targets(self):
        teacher = train.Frozen(str(self.checkpoint(model.build(self.config, self.table, 2), self.root / "t.pt")),
                               self.data.names, self.known, self.data.header, torch.device("cpu"), log=lambda *_: None)
        _, m = train.losses(self.net, self.b, False, opts=train.LossOptions(teacher=teacher))
        t_value, t_policy = teacher(self.b)
        out = self.manual(self.b)[3]
        self.assertAlmostEqual(m["value"], F.mse_loss(out["value"], t_value).item(), places=6)
        taught = torch.softmax(t_policy, -1)
        multi = self.b["legal"].sum(-1) > 1
        logp = F.log_softmax(out["policy"], -1)
        ce = -torch.where(taught > 0, taught * logp, torch.zeros_like(logp)).sum(-1)
        self.assertAlmostEqual(m["policy"], ((ce * multi).sum() / multi.sum()).item(), places=5)
        self.assertEqual(m["choices"], 7, "every position with a choice is a target, the fast search's too")

    def test_q_head(self):
        flags = {(1, 0): (True, [(0, 0.3), (1, -0.1)])}
        with per_sample_flags(flags):
            data = self.load()
        net = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, q_head=True), self.table, 0)
        b = self.batch(data)
        _, m = train.losses(net, b, False, opts=train.LossOptions(q_weight=1.0))
        q = net.train_heads(*(b[k] for k in model.INPUTS))["q"]
        first = int(data.blob[data.offset[0] + 3])
        expected = ((q[0, first : first + 2] - torch.tensor([0.3, -0.1])) ** 2).mean()
        self.assertAlmostEqual(m["q"], expected.item(), places=6)
        # A network without the head, or the weight at 0, trains no Q.
        self.assertEqual(train.losses(self.net, b, False, opts=train.LossOptions(q_weight=1.0))[1]["q"], 0.0)

    def test_ema_warms_up(self):
        net = model.build(self.config, self.table, 0)
        ema = train.EMA(net, 0.999)
        self.assertEqual([ema.decay_at(t) for t in (0, 1, 90)], [0.1, 2 / 11, 0.91])
        self.assertEqual(ema.decay_at(10**6), 0.999)
        before = {k: p.detach().clone() for k, p in net.named_parameters()}
        with torch.no_grad():
            for p in net.parameters():
                p.add_(1.0)
        ema.update(net)
        self.assertEqual(ema.updates, 1)
        for k, p in net.named_parameters():
            torch.testing.assert_close(ema.shadow[k], before[k] + 0.9)
        # An average carried from a checkpoint continues its count: no warm-up again.
        self.assertEqual(train.EMA(net, 0.999, ema.state_dict(net), updates=10**6).decay_at(10**6), 0.999)

    def test_value_mix_and_distillation_combine(self):
        distill = torch.tensor([0.2, float("nan"), 0.4, float("nan"), 0.0, -0.6, float("nan"), 1.0])
        b = dict(self.b, distill=distill)
        _, m = train.losses(self.net, b, False, distill_mix=0.5, opts=train.LossOptions(value_mix_full=0.75, value_mix_fast=0.5))
        mixed = torch.tensor([0.25 + 0.75 * 0.5, 0.25 + 0.75 * 0.5, 1.0, 0.5 - 0.5 * 0.25, 1.0, 0.25 + 0.75 * 0.1, -1.0, -1.0])
        want = torch.where(torch.isfinite(distill), 0.5 * mixed + 0.5 * torch.nan_to_num(distill), mixed)
        self.assertAlmostEqual(m["value"], self.manual(self.b, value_target=want)[1].item(), places=6)
        # --value-mix alike; and with no mix, the outcome's as before.
        _, a = train.losses(self.net, b, False, value_mix=0.5, distill_mix=0.5)
        rooted = torch.isfinite(self.b["root"])
        plain = torch.where(rooted, 0.5 * self.b["outcome"] + 0.5 * torch.nan_to_num(self.b["root"]), self.b["outcome"])
        want = torch.where(torch.isfinite(distill), 0.5 * plain + 0.5 * torch.nan_to_num(distill), plain)
        self.assertAlmostEqual(a["value"], self.manual(self.b, value_target=want)[1].item(), places=6)
        _, c = train.losses(self.net, b, False, distill_mix=0.5)
        want = torch.where(torch.isfinite(distill), 0.5 * self.b["outcome"] + 0.5 * torch.nan_to_num(distill), self.b["outcome"])
        self.assertAlmostEqual(c["value"], self.manual(self.b, value_target=want)[1].item(), places=6)

    def test_ema(self):
        net = model.build(self.config, self.table, 0)
        ema = train.EMA(net, 0.5, updates=1000)  # past its warm-up
        before = {k: p.detach().clone() for k, p in net.named_parameters()}
        with torch.no_grad():
            for p in net.parameters():
                p.add_(1.0)
        ema.update(net)
        for k, p in net.named_parameters():
            torch.testing.assert_close(ema.shadow[k], before[k] + 0.5)
        with ema.applied(net):
            torch.testing.assert_close(dict(net.named_parameters())["nums.weight"], before["nums.weight"] + 0.5)
        torch.testing.assert_close(dict(net.named_parameters())["nums.weight"], before["nums.weight"] + 1.0)
        state = ema.state_dict(net)
        torch.testing.assert_close(state["nums.weight"], before["nums.weight"] + 0.5)
        self.assertIn("table", state)

    def test_the_optimizer_carried_by_name(self):
        net = model.build(self.config, self.table, 0)
        groups = train.param_groups(net)
        named = dict(net.named_parameters())
        optimizer = torch.optim.AdamW([dict(params=[named[k] for k in groups[0]]), dict(params=[named[k] for k in groups[1]])])
        loss, _ = train.losses(net, self.b, False)
        loss.backward()
        optimizer.step()
        source = dict(config=model.config_dict(self.config), state=net.state_dict(), optimizer=optimizer.state_dict())
        # A network that grew a head and reads the numbers through the logarithm: the rest carried, the numbers'
        # projection's moments in its first columns.
        grown = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, q_head=True, nums_log=True), self.table, 0)
        gnamed = dict(grown.named_parameters())
        ggroups = train.param_groups(grown)
        new = torch.optim.AdamW([dict(params=[gnamed[k] for k in ggroups[0]]), dict(params=[gnamed[k] for k in ggroups[1]])])
        for with_names in (False, True):
            carried = train.carry_optimizer(new, grown, dict(source, param_names=groups) if with_names else source,
                                            log=lambda *_: None)
            self.assertEqual(carried, len(optimizer.state), "every parameter with a state (a head no loss reads has none)")
        old = optimizer.state[named["nums.weight"]]["exp_avg"]
        now = new.state[gnamed["nums.weight"]]["exp_avg"]
        torch.testing.assert_close(now[:, : old.shape[1]], old)
        self.assertEqual(float(now[:, old.shape[1] :].abs().sum()), 0.0)
        self.assertNotIn(gnamed["q.0.weight"], new.state)
        new.step()  # the carried state is one AdamW takes


class Numbers(unittest.TestCase):
    def setUp(self):
        self.table = np.random.default_rng(0).normal(size=(3, 8)).astype(np.float32)

    def inputs(self, scale: float, n: int = 3, tokens: int = 32):
        g = torch.Generator().manual_seed(0)
        kind = torch.ones(n, tokens, dtype=torch.long)
        card = torch.randint(0, 3, (n, tokens), generator=g)
        cats = torch.zeros(n, tokens, model.CATS, dtype=torch.long)
        ptrs = torch.full((n, tokens, model.PTRS), model.NO_POINTER)
        nums = torch.randn(n, tokens, model.NUMS, generator=g) * scale
        legal = torch.ones(n, tokens, dtype=torch.bool)
        return kind, card, cats, ptrs, nums, legal

    def test_an_old_configuration_reads_the_numbers_as_before(self):
        old = dict(width=16, layers=1, heads=2, ffn=32, future=0, residual=False, card_features="text")
        config = model.Config(**old)
        self.assertFalse(config.nums_log)
        net = model.build(config, self.table, 0)
        self.assertEqual(net.nums.weight.shape[1], model.NUMS)
        x = self.inputs(1000.0)
        self.assertIs(net.read_nums(x[4]), x[4])
        tiny = Path(__file__).resolve().parents[2] / "crates/cantrip-ai/tests/data/tiny"
        manifest = json.loads((tiny / "manifest.json").read_text())
        self.assertFalse(model.Config(**manifest["config"]).nums_log)

    def test_a_widened_network_computes_the_same_within_the_clip(self):
        plain = model.build(model.Config(width=16, layers=1, heads=2, ffn=32), self.table, 0)
        wide = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, nums_log=True, nums_clip=8.0), self.table, 0)
        wide.load_state_dict(model.widen_nums(wide, plain.state_dict()))
        x = self.inputs(2.0)
        x[4].clamp_(-8.0, 8.0)
        with torch.no_grad():
            for a, b in zip(plain(*x), wide(*x)):
                torch.testing.assert_close(a, b)
        # Past the clip the linear copy holds and only the logarithm moves.
        big = self.inputs(1e5)
        read = wide.read_nums(big[4])
        self.assertLessEqual(float(read[..., : model.NUMS].abs().max()), 8.0)
        torch.testing.assert_close(read[..., model.NUMS :], torch.sign(big[4]) * torch.log1p(big[4].abs()))

    def test_a_network_widened_unclipped_computes_the_same_for_every_input(self):
        plain = model.build(model.Config(width=16, layers=1, heads=2, ffn=32), self.table, 0)
        wide = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, nums_log=True, nums_clip=0.0), self.table, 0)
        wide.load_state_dict(model.widen_nums(wide, plain.state_dict()))
        for scale in (2.0, 50.0, 3000.0):
            x = self.inputs(scale)
            self.assertGreater(float(x[4].abs().max()), 8.0 if scale > 2 else 0.0)
            with torch.no_grad():
                for a, b in zip(plain(*x), wide(*x)):
                    torch.testing.assert_close(a, b)
        read = wide.read_nums(self.inputs(1e5)[4])
        self.assertGreater(float(read[..., : model.NUMS].abs().max()), 1e4, "the linear copy is unclipped")

    def test_the_graph_exports_on_the_allow_list(self):
        net = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, nums_log=True), self.table, 0).eval()
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "m.onnx"
            ops = export.export(net, 32, path)
            self.assertIn("Log", ops)
            import onnxruntime as ort

            x = self.inputs(300.0, n=2)
            with torch.no_grad():
                want = net(*x)
            got = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"]).run(
                None, {k: v.numpy() for k, v in zip(model.INPUTS, x)})
            for a, b in zip(want, got):
                np.testing.assert_allclose(a.numpy(), b, rtol=1e-4, atol=1e-4)


class Fates(unittest.TestCase):
    def test_the_battlefield_and_the_hand_two_turns_on(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            bear, bolt = 1, 2
            now = position_of([token("card", bear, "battlefield", SELF, SELF), token("card", bolt, "battlefield", OPPONENT, OPPONENT),
                               token("card", bolt, "hand", SELF, SELF), token("card", bolt, "hand", SELF, SELF)])
            later = position_of([token("card", bear, "battlefield", SELF, SELF), token("card", bolt, "hand", SELF, SELF)])
            other = position_of([token("card", bolt, "battlefield", OPPONENT, OPPONENT)])
            game = (dict(seed=1), [raw_sample(now, 0, turn=1), raw_sample(other, 1, turn=2, seat=1),
                                   raw_sample(later, 2, turn=3), raw_sample(now, 3, turn=9)])
            write_shard(d / "a.shard.zst", "p", [game])
            data = dataset.load([d], log=lambda *_: None, workers=0)
            targets.fates(data, log=lambda *_: None)
            # Seat 0's sample on turn 1 reads its own turn-3 position; the last one reads itself.
            np.testing.assert_array_equal(data.fate_snap[[0, 2]], [data.fate_snap[2]] * 2)
            rows = np.arange(len(data))
            x = dataset.decode(data.blob, data.offset, rows)
            x["card"] = dataset.cards(data, rows, x["card"])
            labels = targets.fate_labels(data, rows, x)
            fate, mask = labels["fates"], labels["fate_mask"]
            # Turn 1: the bear stays (1), the opponent's bolt is gone (0); one of the two bolts in hand has left (0.5).
            np.testing.assert_array_equal(mask[0, 1:5], [[1, 0], [1, 0], [0, 1], [0, 1]])
            np.testing.assert_allclose(fate[0, 1:5], [[1, 0], [0, 0], [0, 0.5], [0, 0.5]])
            self.assertEqual(float(mask[0, 0].sum() + mask[0, 5:].sum()), 0.0, "the game and the options have none")
            # The last sample reads itself: everything stays.
            np.testing.assert_allclose(fate[3, 1:5], [[1, 0], [1, 0], [0, 0], [0, 0]])
            # Through a loss: a network with the head trains on them.
            net = model.build(model.Config(width=16, layers=1, heads=2, ffn=32, fates=targets.FATES),
                              np.zeros((len(data.names) + 1, 8), np.float32), 0)
            b = train.collate(data)(rows)
            self.assertIn("fates", b)
            _, m = train.losses(net, b, False, opts=train.LossOptions(fate_weight=1.0))
            self.assertGreater(m["fates"], 0.0)


class Runs(unittest.TestCase):
    """Whole runs of train.py on the CPU with the options on: they train, write what export reads, and export."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
        self.root = Path(self.tmp.name)
        self.data = self.root / "data"
        self.data.mkdir()
        write_shard(self.data / "a.shard.zst", "selfplay:m@64", [selfplay_game(g) for g in range(10)])
        write_shard(self.data / "b.shard.zst", "tactical:x", [imitation_game(100 + g) for g in range(10)])

    def tearDown(self):
        gc.collect()
        self.tmp.cleanup()

    def run_train(self, out: Path, *extra: str) -> str:
        argv = ["--data", str(self.data), "--out", str(out), "--batch", "4", "--width", "16", "--layers", "1",
                "--heads", "1", "--ffn", "32", "--threads", "1", "--workers", "0", "--load-workers", "0", "--device", "cpu",
                "--progress", "off", "--warmup", "2", "--held-out", "0.3", "--checkpoint-every", "0", "--eval-every", "4",
                *extra]
        with contextlib.redirect_stdout(io.StringIO()):
            train.main(argv)
        gc.collect()
        return (out / "train.log").read_text()

    def test_the_options_together_then_continued_and_distilled(self):
        first = self.root / "first"
        log = self.run_train(first, "--search-target-repeat", "4", "--target-uses", "2", "--imitation-policy-weight", "0.05",
                             "--surprise-weight", "0.5", "--value-mix-full", "0.75", "--value-mix-fast", "0.5",
                             "--value-games-cap", "2", "--value-loss", "bce", "--policy-smoothing", "0.05",
                             "--entropy-weight", "0.01", "--q-weight", "0.1", "--fate-weight", "0.1", "--future-weight", "0.1",
                             "--log-nums", "--ema", "0.9", "--schedule", "constant", "--trivial", "1")
        self.assertIn("each search target used about 2 times", log)
        self.assertIn("search targets:", log)
        self.assertIn("fate targets:", log)
        ck = torch.load(first / "checkpoint.pt", map_location="cpu", weights_only=False)
        c = ck["config"]
        self.assertTrue(c["nums_log"] and c["q_head"])
        self.assertEqual((c["fates"], c["future"]), (targets.FATES, targets.FUTURE))
        self.assertIn("train_state", ck)
        self.assertFalse(torch.equal(ck["state"]["nums.weight"], ck["train_state"]["nums.weight"]), "the average is written")
        # Two uses at four deals an epoch: half an epoch.
        batches = int(re.search(r"(\d+) batches an epoch", log).group(1))
        self.assertEqual(ck["total"], max(1, int(batches * 0.5)))
        self.assertIn("search_policy_ce", ck["metrics"]["held_out"])

        # Exported: the graphs (with the logarithm) and the server's weights.
        model_dir = self.root / "model"
        with contextlib.redirect_stdout(io.StringIO()):
            export.main(["--checkpoint", str(first / "checkpoint.pt"), "--out", str(model_dir)])
        manifest = json.loads((model_dir / "manifest.json").read_text())
        self.assertIn("Log", manifest["ops"])
        self.assertTrue((model_dir / "model.pt").exists())

        # Continued: the optimizer, the unaveraged weights and the average carried; the champion as anchor.
        second = self.root / "second"
        log = self.run_train(second, "--init", str(first / "checkpoint.pt"), "--init-optimizer", "--ema", "0.9",
                             "--schedule", "constant", "--anchor", str(first / "checkpoint.pt"), "--anchor-kl", "0.1",
                             "--steps", "3")
        self.assertRegex(log, r"carried the optimizer state of \d+ of \d+ parameters")
        self.assertIn("the average carried from the checkpoint", log)
        self.assertIn("frozen ", log)

        # Grown by distillation: a wider network from scratch on the champion's outputs.
        third = self.root / "third"
        log = self.run_train(third, "--distill-from", str(first / "checkpoint.pt"), "--width", "24", "--steps", "3")
        self.assertIn("frozen ", log)
        self.assertEqual(torch.load(third / "checkpoint.pt", map_location="cpu", weights_only=False)["config"]["width"], 24)

    def test_continued_without_an_average_starts_from_the_gated_model_and_does_not_warm_up(self):
        first = self.root / "first"
        self.run_train(first, "--ema", "0.9", "--steps", "6")
        ck = torch.load(first / "checkpoint.pt", map_location="cpu", weights_only=False)
        self.assertEqual(ck["ema_updates"], 6)
        second = self.root / "second"
        log = self.run_train(second, "--init", str(first / "checkpoint.pt"), "--init-optimizer", "--steps", "0")
        self.assertIn("no warm-up: the optimizer state is carried", log)
        got = torch.load(second / "checkpoint.pt", map_location="cpu", weights_only=False)
        self.assertEqual(got["warmup"], 0)
        for k, v in ck["state"].items():
            if k != "table" and v.is_floating_point():
                torch.testing.assert_close(got["state"][k], v, msg=f"{k}: the gated (averaged) weights")
        # With --ema it continues the unaveraged weights, its average the gated one, its count carried.
        third = self.root / "third"
        self.run_train(third, "--init", str(first / "checkpoint.pt"), "--init-optimizer", "--ema", "0.9", "--steps", "0")
        got = torch.load(third / "checkpoint.pt", map_location="cpu", weights_only=False)
        self.assertEqual(got["ema_updates"], 6)
        torch.testing.assert_close(got["train_state"]["nums.weight"], ck["train_state"]["nums.weight"])
        torch.testing.assert_close(got["state"]["nums.weight"], ck["state"]["nums.weight"])

    def test_resumed_with_a_head_added(self):
        out = self.root / "run"
        self.run_train(out, "--steps", "2", "--eval-every", "100", "--checkpoint-every", "1")
        log = self.run_train(out, "--resume", "--steps", "4", "--q-weight", "0.1", "--eval-every", "100")
        self.assertRegex(log, r"--resume: carried the optimizer state of \d+ of \d+ parameters")
        ck = torch.load(out / "checkpoint.pt", map_location="cpu", weights_only=False)
        self.assertEqual(ck["step"], 4)
        self.assertTrue(ck["config"]["q_head"])


class Driver(unittest.TestCase):
    def test_the_training_stage_takes_the_options(self):
        sys.path.insert(0, str(TESTS))
        from test_generation import root_with  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--decks-dir", "", "--target-uses", "4",
                                 "--window", "8", "--older-weight", "0.5", "--search-target-repeat", "8",
                                 "--imitation-policy-weight", "0.05", "--value-mix-full", "0.75", "--value-mix-fast", "0.5",
                                 "--schedule", "constant", "--carry-optimizer",
                                 "--train-args", "--anchor {champion} --anchor-kl 0.05"])
            line = next(x for x in out.getvalue().splitlines() if "cantrip_ai.train" in x)
            champion = str(root / "champions" / "gen0" / "train" / "checkpoint.pt")
            self.assertIn("--target-uses 0.8889 ", line)
            self.assertNotIn("--steps", line)
            self.assertNotIn("--hours", line)
            for part in ("--search-target-repeat 8", "--imitation-policy-weight 0.05", "--value-mix-full 0.75",
                         "--value-mix-fast 0.5", "--schedule constant", "--init-optimizer", f"--anchor {champion} --anchor-kl 0.05"):
                self.assertIn(part, line)
            # --train-steps still wins, and without the options the command is as it was.
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--decks-dir", "", "--target-uses", "4",
                                 "--train-steps", "100"])
            line = next(x for x in out.getvalue().splitlines() if "cantrip_ai.train" in x)
            self.assertIn("--steps 100 ", line)
            self.assertNotIn("--target-uses", line)
            for flag in ("--search-target-repeat", "--schedule", "--init-optimizer", "--value-mix-f"):
                self.assertNotIn(flag, line)


import re  # noqa: E402

if __name__ == "__main__":
    unittest.main()
