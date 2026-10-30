"""The inference server (`cantrip_ai.serve`): its wire form, a row's outputs whatever it is batched with, and the
refusals. On the GPU when there is one (CUDA or ROCm), else the CPU; a small random network, so no model is needed."""

from __future__ import annotations

import json
import socket
import struct
import sys
import tempfile
import threading
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import dataset, export, model, serve  # noqa: E402

DEVICE = serve.choose_device("auto")
CARDS = 40


def make_model(directory: Path, seed: int = 0) -> serve.Served:
    """A random network written as export writes a model directory's `model.pt` and manifest."""
    rng = np.random.default_rng(seed)
    table = rng.standard_normal((CARDS + 1, 64)).astype(np.float32)
    config = model.Config(width=32, layers=2, heads=2, ffn=64)
    net = model.build(config, table, seed)
    net.eval()
    model.fold(net)
    directory.mkdir(parents=True, exist_ok=True)
    checkpoint = dict(config=model.config_dict(config), state=net.state_dict())
    export.write_weights(net, checkpoint, directory / "model.pt")
    manifest = dict(schema=model.LAYOUT["schema"], encoder_hash=dataset.encoder_hash(), seed=seed)
    (directory / "manifest.json").write_text(json.dumps(manifest))
    return serve.Served(directory, DEVICE)


def positions(n: int, seed: int = 1, lengths=None):
    """`n` random positions as padded [n, 128] arrays: valid kinds, cards, categoricals, pointers inside the position,
    sparse f16 numbers, legal options."""
    rng = np.random.default_rng(seed)
    t = serve.MAX_TOKENS
    kind = np.zeros((n, t), np.int64)
    card = np.zeros((n, t), np.int64)
    cats = np.zeros((n, t, serve.CATS), np.int64)
    ptrs = np.full((n, t, serve.PTRS), serve.NO_POINTER, np.int64)
    nums = np.zeros((n, t, serve.NUMS), np.float32)
    legal = np.zeros((n, t), bool)
    for i in range(n):
        length = int(lengths[i]) if lengths is not None else int(rng.integers(1, t + 1))
        kind[i, :length] = rng.integers(1, model.KINDS, length)
        card[i, :length] = rng.integers(0, CARDS + 1, length)
        for c, vocab in enumerate(model.VOCABS):
            cats[i, :length, c] = rng.integers(0, vocab, length)
        point = rng.random((length, serve.PTRS)) < 0.3
        ptrs[i, :length][point] = rng.integers(0, length, int(point.sum()))
        values = rng.standard_normal((length, serve.NUMS)).astype(np.float16).astype(np.float32)
        nums[i, :length] = np.where(rng.random((length, serve.NUMS)) < 0.08, values, 0.0)
        first = int(rng.integers(0, length))
        legal[i, first : first + int(rng.integers(1, 6))] = True
        legal[i, length:] = False
    return kind, card, cats, ptrs, nums, legal


def chunk(arrays, i: int) -> serve.Chunk:
    """Row `i` alone, as the server holds it once decoded."""
    return serve.request(None, serve.decode(serve.encode(*(a[i : i + 1] for a in arrays)))).chunks[0]


def alone(runner: serve.Runner, arrays, i: int, slot: int = 0, companions=None):
    """Row `i` at `slot` of a batch, the other slots padding or `companions`' rows."""
    others = [chunk(companions, j) for j in range(runner.b - 1)] if companions is not None else []
    parts = [(c, 0, 1) for c in others[:slot]] + [(chunk(arrays, i), 0, 1)] + [(c, 0, 1) for c in others[slot:]]
    runner.run(parts)
    return runner.value[slot].copy(), runner.aux[slot].copy(), runner.policy[slot].copy()


class Wire(unittest.TestCase):
    def test_a_request_decodes_to_its_real_tokens(self):
        arrays = positions(9)
        d = serve.decode(serve.encode(*arrays))
        kind, card, cats, ptrs, nums, legal = arrays
        lengths = (kind != 0).sum(1)
        self.assertEqual(d["lengths"].tolist(), lengths.tolist())
        real = np.arange(128)[None, :] < lengths[:, None]
        for name, x in zip(("kind", "card", "cats", "ptrs", "legal"), (kind, card, cats, ptrs, legal)):
            np.testing.assert_array_equal(d[name], x[real], err_msg=name)
        dense = np.zeros((int(lengths.sum()), serve.NUMS), np.float32)
        dense[d["ntok"], d["nfeat"]] = d["nval"]
        self.assertTrue((dense.view(np.uint32) == nums[real].view(np.uint32)).all())

    def test_rows_split_by_bucket_in_any_order(self):
        arrays = positions(12, seed=4)
        lengths = (arrays[0] != 0).sum(1)
        for order in (np.arange(12), np.argsort([serve.bucket_of(int(n)) for n in lengths], kind="stable")):
            r = serve.request(None, serve.decode(serve.encode(*(a[order] for a in arrays))))
            self.assertEqual(sum(len(c.rows) for c in r.chunks), 12)
            for c in r.chunks:
                for j, row in enumerate(c.rows):
                    i = order[row]
                    t0, t1 = c.tok_cum[j], c.tok_cum[j + 1]
                    self.assertEqual(serve.bucket_of(max(int(lengths[i]), 1)), c.tokens)
                    np.testing.assert_array_equal(c.kind[t0:t1], arrays[0][i, : lengths[i]])
                    np.testing.assert_array_equal(c.pos[t0:t1], np.arange(lengths[i]))
                    np.testing.assert_array_equal(c.row[t0:t1], j)
                    k0, k1 = c.num_cum[j], c.num_cum[j + 1]
                    got = np.zeros((lengths[i], serve.NUMS), np.float32)
                    got[c.ntok[k0:k1] - t0, c.nfeat[k0:k1]] = c.nval[k0:k1]
                    np.testing.assert_array_equal(got, arrays[4][i, : lengths[i]])

    def test_numbers_f16_cannot_hold_go_as_f32(self):
        arrays = list(positions(3))
        arrays[4] = arrays[4].copy()
        arrays[4][0, 0, 0] = 0.1
        payload = serve.encode(*arrays)
        self.assertEqual(payload[6], serve.NUMS_DENSE)
        d = serve.decode(payload)
        self.assertEqual(d["nval"][(d["ntok"] == 0) & (d["nfeat"] == 0)].tolist(), [np.float32(0.1)])

    def test_a_frame_cut_short_is_refused(self):
        payload = serve.encode(*positions(4))
        for cut in (1, 9, len(payload) // 2, len(payload) - 1):
            with self.assertRaises(serve.ProtocolError):
                serve.decode(payload[:cut])
        with self.assertRaises(serve.ProtocolError):
            serve.decode(payload + b"\0")


class Reproducible(unittest.TestCase):
    """A row's outputs do not depend on what it is batched with, nor on its slot."""

    @classmethod
    def setUpClass(cls):
        serve.deterministic("fp32")
        cls.tmp = tempfile.TemporaryDirectory()
        cls.served = make_model(Path(cls.tmp.name) / "m")

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_alone_and_among_companions_are_bitwise_equal(self):
        # fp16 too, where the GPU has it: autocast keeps one shape a bucket, so its kernels are as fixed as fp32's.
        for half in (None, torch.float16) if DEVICE.type == "cuda" else (None,):
            for t, b in ((32, 16), (64, 8), (128, 8)):
                runner = serve.Runner(self.served.net, DEVICE, t, b, DEVICE.type == "cuda", half)
                arrays = positions(4, seed=t, lengths=[t, t // 2 + 1, 1, t - 3])
                for i in range(4):
                    v, a, p = alone(runner, arrays, i)
                    for seed, slot in ((5, 1), (6, b - 1), (7, b // 2)):
                        companions = positions(b, seed=seed, lengths=np.minimum(np.arange(1, b + 1) * 7, t))
                        v2, a2, p2 = alone(runner, arrays, i, slot, companions)
                        self.assertEqual(v.view(np.uint32), v2.view(np.uint32), (half, t, i, slot))
                        np.testing.assert_array_equal(a.view(np.uint32), a2.view(np.uint32))
                        np.testing.assert_array_equal(p.view(np.uint32), p2.view(np.uint32))


    def test_every_pipeline_makes_the_same_bits(self):
        # The server runs a bucket on any of its pipelines, each with its own buffers, stream and captured graph.
        graphs = DEVICE.type == "cuda"
        one, two = (serve.Runner(self.served.net, DEVICE, 64, 8, graphs) for _ in range(2))
        arrays = positions(3, seed=11, lengths=[60, 33, 64])
        for i in range(3):
            for x, y in zip(alone(one, arrays, i), alone(two, arrays, i, slot=5, companions=positions(8, seed=12, lengths=[64, 1, 30, 50, 63, 2, 64, 17]))):
                np.testing.assert_array_equal(np.asarray(x).view(np.uint32), np.asarray(y).view(np.uint32))


class Running(unittest.TestCase):
    """A server on a port, its clients on threads."""

    @classmethod
    def setUpClass(cls):
        serve.deterministic("fp32")
        cls.tmp = tempfile.TemporaryDirectory()
        cls.served = make_model(Path(cls.tmp.name) / "m")
        cls.other = make_model(Path(cls.tmp.name) / "other", seed=3)
        cls.batch = {32: 16, 64: 8, 128: 4}
        graphs = DEVICE.type == "cuda"
        cls.server = serve.Server([cls.served, cls.other], DEVICE, cls.batch, 0.002, 2, graphs, log=lambda *_: None)
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen(64)
        cls.address = listener.getsockname()
        ready = threading.Event()
        cls.thread = threading.Thread(target=cls.server.serve, args=(listener, ready.set), daemon=True)
        cls.thread.start()
        ready.wait(10)
        # Each row alone, at its own bucket and the server's batch sizes: what every answer must equal bit for bit.
        cls.reference_lock = threading.Lock()
        cls.reference = {t: serve.Runner(cls.served.net, DEVICE, t, cls.batch[t], graphs) for t in serve.BUCKETS}

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()
        cls.thread.join(5)
        cls.tmp.cleanup()

    def connect(self, identity=None, **hello) -> socket.socket:
        s = socket.create_connection(self.address)
        s.sendall(serve.frame(serve.hello(self.served.identity if identity is None else identity, "m", **hello)))
        return s

    @staticmethod
    def receive(s: socket.socket) -> bytes:
        def exactly(n):
            out = b""
            while len(out) < n:
                chunk = s.recv(n - len(out))
                if not chunk:
                    raise ConnectionError("closed")
                out += chunk
            return out

        (n,) = struct.unpack("<I", exactly(4))
        return exactly(n)

    def expected(self, arrays, i):
        length = int((arrays[0][i] != 0).sum())
        with self.reference_lock:  # the clients' threads share the reference runners
            return alone(self.reference[serve.bucket_of(max(length, 1))], arrays, i)

    def test_answers_are_each_row_alone_whatever_the_traffic(self):
        clients, rounds = 6, 5
        failures = []

        def client(k):
            try:
                s = self.connect()
                self.assertEqual(self.receive(s)[0], 0)
                for r in range(rounds):
                    arrays = positions(1 + (k + r) % 7, seed=100 * k + r)
                    if r % 2:  # by bucket, as the Rust client sends them
                        lengths = (arrays[0] != 0).sum(1)
                        order = np.argsort([serve.bucket_of(max(int(n), 1)) for n in lengths], kind="stable")
                        arrays = tuple(a[order] for a in arrays)
                    s.sendall(serve.frame(serve.encode(*arrays)))
                    value, aux, policy = serve.read_reply(self.receive(s), arrays[5])
                    for i in range(len(value)):
                        v, a, p = self.expected(arrays, i)
                        if v.view(np.uint32) != value[i].view(np.uint32) or not (a.view(np.uint32) == aux[i].view(np.uint32)).all():
                            failures.append((k, r, i, "value"))
                        legal = arrays[5][i][: len(p)]
                        if not (p[legal].view(np.uint32) == policy[i][: len(p)][legal].view(np.uint32)).all():
                            failures.append((k, r, i, "policy"))
                s.close()
            except Exception as e:  # noqa: BLE001
                failures.append((k, repr(e)))

        threads = [threading.Thread(target=client, args=(k,)) for k in range(clients)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(120)
        self.assertEqual(failures, [])

    def test_an_empty_request_is_answered(self):
        s = self.connect()
        self.receive(s)
        s.sendall(serve.frame(struct.pack("<BIBB", serve.EVALUATE, 0, 2, serve.NUMS_SPARSE)))
        self.assertEqual(self.receive(s), b"\0")
        s.close()

    def test_two_models_are_served_apart(self):
        s = self.connect(self.other.identity)
        self.assertEqual(self.receive(s)[0], 0)
        arrays = positions(2, seed=9, lengths=[20, 50])
        s.sendall(serve.frame(serve.encode(*arrays)))
        value, _, _ = serve.read_reply(self.receive(s), arrays[5])
        s.close()
        mine = self.expected(arrays, 0)[0]
        self.assertNotEqual(value[0], mine)

    def refused(self, s: socket.socket) -> str:
        reply = self.receive(s)
        self.assertEqual(reply[0], 1)
        (n,) = struct.unpack_from("<H", reply, 1)
        self.assertEqual(s.recv(1), b"", "the server closes a refused connection")
        s.close()
        return reply[3 : 3 + n].decode()

    def test_the_hello_is_checked(self):
        self.assertIn("no model", self.refused(self.connect(identity=12345)))
        self.assertIn("encoding", self.refused(self.connect(encoder=1)))
        self.assertIn("encoding", self.refused(self.connect(schema=99)))
        s = socket.create_connection(self.address)
        s.sendall(serve.frame(b"HELLO" * 10))
        self.assertIn("not a Cantrip", self.refused(s))
        s = socket.create_connection(self.address)
        s.sendall(serve.frame(serve.hello(self.served.identity, "m")[:8] + struct.pack("<I", 99) + b"\0" * 22))
        self.assertIn("protocol", self.refused(s))

    def test_a_bad_frame_ends_the_connection(self):
        s = self.connect()
        self.receive(s)
        payload = serve.encode(*positions(2))
        s.sendall(serve.frame(payload[: len(payload) // 2]))
        self.assertIn("cut short", self.refused(s))
        s = self.connect()
        self.receive(s)
        s.sendall(struct.pack("<I", serve.MAX_FRAME + 1))
        self.assertIn("bytes", self.refused(s))
        # The server goes on serving.
        s = self.connect()
        self.assertEqual(self.receive(s)[0], 0)
        s.close()


if __name__ == "__main__":
    unittest.main()
