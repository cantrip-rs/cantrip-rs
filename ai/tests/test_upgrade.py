"""`model.upgrade_schema1`: a schema-1 network upgraded to this encoding computes what it did."""

import unittest

import numpy as np
import torch

from cantrip_ai import model, upgrade


def tiny_schema1(seed: int = 3):
    """A small schema-1 network with random weights, and its config and state."""
    config = dict(width=32, layers=1, heads=2, ffn=64)
    vocabs = model.VOCABS
    model.VOCABS = model.SCHEMA1_VOCABS
    try:
        net = model.build(model.Config(**config), np.random.default_rng(seed).normal(size=(20, 8)).astype(np.float32), seed)
    finally:
        model.VOCABS = vocabs
    with torch.no_grad():
        for p in net.parameters():
            p.normal_(0, 0.2)
    return config, {k: v.clone() for k, v in net.state_dict().items()}


def positions(n: int, t: int, seed: int) -> list[dict]:
    rng = np.random.default_rng(seed)
    out = []
    for _ in range(n):
        kind = rng.integers(1, model.KINDS, t).astype(np.uint8)
        cats = np.stack([rng.integers(0, v, t) for v in model.SCHEMA1_VOCABS], axis=1).astype(np.uint8)
        ptrs = np.where(rng.random((t, model.PTRS)) < 0.5, rng.integers(0, t, (t, model.PTRS)), model.NO_POINTER).astype(np.uint8)
        nums = rng.normal(0, 1, (t, model.NUMS)).astype(np.float32)
        kinds = model.LAYOUT["kinds"]
        for name, cols in model.SCHEMA1_NEW_NUMS.items():  # schema 1 left these empty
            nums[np.ix_(kind == kinds.index(name), list(cols))] = 0.0
        for name, slots in model.SCHEMA1_NEW_PTRS.items():
            ptrs[np.ix_(kind == kinds.index(name), list(slots))] = model.NO_POINTER
        out.append(dict(bucket=t, kind=kind, card=rng.integers(0, 20, t).astype(np.uint32), cats=cats, ptrs=ptrs,
                        nums=nums, legal=rng.random(t) < 0.5))
    return out


class UpgradeSchema1(unittest.TestCase):
    def test_the_upgraded_network_computes_what_the_original_did_with_and_without_new_inputs(self):
        config, state = tiny_schema1()
        before = upgrade.original(config, state)
        new_config, new_state = model.upgrade_schema1(config, state)
        after = model.from_state(model.Config(**new_config), new_state)
        after.load_state_dict(new_state)
        after.eval()
        rng = np.random.default_rng(5)
        with torch.no_grad():
            for with_new in (False, True):
                raw, old = upgrade.batch(positions(8, 32, 7), 20, rng, with_new)
                for x, y in zip(after(**raw), before(**old)):
                    self.assertLess(float((x - y).abs().max()), 1e-5)
        # And the new inputs reach the new projections once they are trained away from zero.
        with torch.no_grad():
            after.nums_new.weight.normal_(0, 0.1)
            raw, _ = upgrade.batch(positions(8, 32, 9), 20, rng, True)
            self.assertGreater(float((after(**raw)[0] - before(**upgrade.batch(positions(8, 32, 9), 20, np.random.default_rng(5), True)[1])[0]).abs().max()), 0.0)

    def test_a_table_that_is_not_schema_1_is_refused(self):
        config, state = tiny_schema1()
        new_config, new_state = model.upgrade_schema1(config, state)
        with self.assertRaises(ValueError):
            model.upgrade_schema1(new_config, new_state)


if __name__ == "__main__":
    unittest.main()
