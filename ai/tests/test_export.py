"""Export's graph buckets (`export.BUCKETS`): a model directory exported before the T96 bucket gets its 96-token graph
added from its `model.pt` (`--add-graphs`), checked against PyTorch, its manifest naming it, and nothing else touched;
and a row runs at the smallest bucket that holds it."""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import dataset, export, model, serve  # noqa: E402

TINY = Path(__file__).resolve().parents[2] / "crates/cantrip-ai/tests/data/tiny"


class Buckets(unittest.TestCase):
    def test_the_graph_buckets_hold_the_encodings(self):
        self.assertEqual(export.BUCKETS, serve.BUCKETS)
        self.assertTrue(set(export.ENCODED) <= set(export.BUCKETS))
        self.assertEqual(export.BUCKETS[-1], serve.MAX_TOKENS)
        self.assertEqual([export.bucket_of(n) for n in (0, 1, 32, 33, 64, 65, 96, 97, 128)],
                         [32, 32, 32, 64, 64, 96, 96, 128, 128])

    def test_a_batch_list_from_before_t96_still_reads(self):
        cuda = torch.device("cuda")  # names the defaults only; nothing runs
        self.assertEqual(serve.parse_batch("32:200,64:100,128:50", cuda), {32: 200, 64: 100, 96: 96, 128: 50})
        with self.assertRaises(SystemExit):
            serve.parse_batch("48:10", cuda)


class Pointers(unittest.TestCase):
    def test_a_row_runs_where_its_pointers_read_what_they_read(self):
        def need(length, p):
            ptrs = np.full((length, model.PTRS), model.NO_POINTER, np.uint8)
            ptrs[0, 0] = p
            return int(serve.tokens_needed(np.array([length]), np.zeros(length, np.int64), ptrs)[0])

        self.assertEqual(need(70, 3), 70)
        self.assertEqual(need(70, model.NO_POINTER), 70)
        # Encoded at 128: a pointer at 80 needs 81 tokens (T96 reads the same padding token); one at 100, 101 (T128).
        self.assertEqual(need(70, 80), 81)
        self.assertEqual(need(70, 100), 101)
        # Encoded at 64: a pointer at 70 reads zeros there as at any bucket, and moves nothing.
        self.assertEqual(need(40, 70), 40)


class Check(unittest.TestCase):
    def test_a_server_answering_nan_fails_its_check(self):
        """Every output NaN: each drift's max skips it and argmax picks the first option, so the drifts and the best
        options alone pass it; the non-finite and illegal counts fail it."""

        class Nan:
            def __init__(self, t):
                self.b, self.t = 64, t

            def run(self, parts):
                self.value = np.full(self.b, np.nan, np.float32)
                self.aux = np.full((self.b, 2), np.nan, np.float32)
                self.policy = np.full((self.b, self.t), np.nan, np.float32)

        served = type("Served", (), dict(dir=TINY, identity=1, name="tiny", buckets=export.ENCODED))()
        runners = {(1, t): Nan(t) for t in export.ENCODED}
        logged = []
        with self.assertRaises(SystemExit):
            serve.check(served, runners, log=logged.append, precision="fp16")
        self.assertIn("non_finite", logged[0])
        out = logged[0]
        self.assertNotIn("'non_finite': 0", out)
        self.assertNotIn("'illegal_argmax': 0", out)


class InputStage(unittest.TestCase):
    def test_the_server_s_network_keeps_its_input_stage_in_fp32_under_autocast(self):
        """Served's network (`inputs_in_fp32`): under autocast the card, numbers and pointers projections stay fp32 and
        the blocks' projections do not; a network as training builds it is as it was."""
        torch.manual_seed(0)
        net = model.build(model.Config(width=16, layers=1, heads=2, ffn=32), np.zeros((8, 8), np.float32)).eval()
        x = dict(kind=torch.ones(2, 32, dtype=torch.long), card=torch.zeros(2, 32, dtype=torch.long),
                 cats=torch.zeros(2, 32, model.CATS, dtype=torch.long),
                 ptrs=torch.full((2, 32, model.PTRS), model.NO_POINTER, dtype=torch.long),
                 nums=torch.randn(2, 32, model.NUMS), legal=torch.ones(2, 32, dtype=torch.bool))
        seen = {}
        for name in ("card", "nums", "pointers", "blocks.0.attention.qkv"):
            net.get_submodule(name).register_forward_hook(
                lambda _m, _i, out, name=name: seen.__setitem__(name, out.dtype))
        for fp32 in (False, True):
            net.inputs_in_fp32 = fp32
            with torch.no_grad(), torch.autocast("cpu", dtype=torch.bfloat16):
                net(**x)
            stage = torch.float32 if fp32 else torch.bfloat16
            self.assertEqual((seen["card"], seen["nums"], seen["pointers"]), (stage,) * 3, seen)
            self.assertEqual(seen["blocks.0.attention.qkv"], torch.bfloat16)

    def test_served_networks_keep_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            config = model.Config(width=16, layers=1, heads=2, ffn=32)
            net = model.build(config, np.zeros((8, 8), np.float32))
            export.write_weights(net, dict(config=model.config_dict(config), state=net.state_dict()), out / "model.pt")
            (out / "manifest.json").write_text(json.dumps(dict(schema=model.LAYOUT["schema"],
                                                               encoder_hash=dataset.encoder_hash())))
            self.assertTrue(serve.Served(out, torch.device("cpu")).net.inputs_in_fp32)
            self.assertFalse(model.build(config, np.zeros((8, 8), np.float32)).inputs_in_fp32)


class AddGraphs(unittest.TestCase):
    def test_a_directory_from_before_t96_gets_its_graph(self):
        try:
            import onnxruntime as ort  # noqa: PLC0415
        except ImportError:
            self.skipTest("onnxruntime is not installed")
        torch.manual_seed(0)
        rows = json.loads((TINY / "cards.json").read_text())["rows"]
        config = model.Config(width=16, layers=1, heads=2, ffn=32)
        net = model.build(config, np.random.default_rng(0).standard_normal((rows, 8)).astype(np.float32))
        net.eval()
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            export.write_weights(net, dict(config=model.config_dict(config), state=net.state_dict()), out / "model.pt")
            shutil.copytree(TINY / "reference", out / "reference")
            buckets = {}
            for t in export.ENCODED:
                export.export(net, t, out / f"model-t{t}.onnx")
                buckets[str(t)] = f"model-t{t}.onnx"
            manifest = dict(schema=model.LAYOUT["schema"], encoder_hash=dataset.encoder_hash(), buckets=buckets, ops=[])
            (out / "manifest.json").write_text(json.dumps(manifest))
            before = {p.name: p.read_bytes() for p in out.rglob("*") if p.is_file() and p.name != "manifest.json"}

            # Before: served at the encoding's buckets alone, a row of 65 to 96 tokens at 128, as always.
            fits = [p for p in export.read_positions(out / "reference" / "positions.bin") if 64 < p[2] <= 96]
            self.assertTrue(fits)
            frame = serve.decode(serve.encode(*(export.inputs_of(fits[:1], 128)[k] for k in model.INPUTS)))
            served = serve.Served(out, torch.device("cpu"))
            self.assertEqual(served.buckets, export.ENCODED)
            self.assertEqual([c.tokens for c in serve.request(None, frame, served.buckets).chunks], [128])

            self.assertEqual(export.add_graphs(out), [96])
            served = serve.Served(out, torch.device("cpu"))
            self.assertEqual(served.buckets, export.BUCKETS)
            self.assertEqual([c.tokens for c in serve.request(None, frame, served.buckets).chunks], [96])
            after = json.loads((out / "manifest.json").read_text())
            self.assertEqual(after["buckets"], dict(buckets, **{"96": "model-t96.onnx"}))
            for name, data in before.items():
                self.assertEqual(next(out.rglob(name)).read_bytes(), data, name)

            # The new graph is the network at 96 tokens, on references that fit.
            positions = [p for p in export.read_positions(out / "reference" / "positions.bin") if p[2] <= 96]
            x = export.inputs_of(positions, 96)
            session = ort.InferenceSession(str(out / "model-t96.onnx"), providers=["CPUExecutionProvider"])
            got = session.run(None, {k: x[k] for k in model.INPUTS})
            with torch.no_grad():
                want = net(*(torch.from_numpy(x[k]) for k in model.INPUTS))
            for a, b in zip(got[:3], want[:3]):
                np.testing.assert_allclose(a, b.numpy(), rtol=1e-4, atol=1e-4)
            # Again: nothing to add, nothing written.
            text = (out / "manifest.json").read_text()
            self.assertEqual(export.add_graphs(out), [])
            self.assertEqual((out / "manifest.json").read_text(), text)


if __name__ == "__main__":
    unittest.main()
