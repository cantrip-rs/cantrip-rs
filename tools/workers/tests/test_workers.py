"""The worker manager (docs/workers.md), with nothing real under it: a clock the test moves, an ssh that records what
it was asked, a vast.ai that keeps its instances in a dict. No network, no waiting, no rentals.

    python -m unittest discover -s tools/workers/tests
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from cantrip_workers import access, cli, vast  # noqa: E402
from cantrip_workers.manager import Manager  # noqa: E402
from cantrip_workers.pool import parse_request  # noqa: E402
from cantrip_workers.store import ConfigError, LeaseError, LimitError, Refused, Store, Unmet, read_lines  # noqa: E402

# No test starts the daemon: the real command would, in a directory the test is about to remove.
os.environ["CANTRIP_WORKERS_NO_DAEMON"] = "1"

DAY = 86400.0
T0 = 20000 * DAY + 3600  # an hour into a UTC day


class Clock:
    def __init__(self, now: float = T0):
        self.now = now

    def __call__(self) -> float:
        return self.now

    def sleep(self, seconds: float):
        self.now += seconds


class Remote:
    """The tool's own errands on a machine: each recorded, answered by `answer(machine, line)`."""

    def __init__(self):
        self.calls: list[tuple[str, str]] = []
        self.watchdog_log = ""
        self.answer = lambda m, line: (0, "armed 1\n" if vast.STATUS in line else self.watchdog_log)

    def __call__(self, m, line, timeout=60.0):
        self.calls.append((m["name"], line))
        return self.answer(m, line)


class FakeVast:
    """vast.ai's account: offers, and the instances created from them."""

    def __init__(self, clock: Clock):
        self.clock = clock
        self.offers: list[dict] = []
        self.instances_: dict[int, dict] = {}
        self.next = 7000
        self.gone_offers: set[int] = set()
        self.runs_after = 40.0        # seconds from creation until an instance is running
        self.status = "running"       # what it comes to
        self.refuse_destroy = False
        self.lose_create = False      # the create is applied and its answer never comes
        self.hidden: set[int] = set() # instances the account's list leaves out
        self.on_destroy = None
        self.destroyed: list[int] = []
        self.shown: list[int] = []
        self.keys: dict[int, str] = {}
        self.created: list[dict] = []
        self.queries: list[dict] = []

    def search(self, query):
        self.queries.append(query)
        return [dict(o) for o in self.offers]

    def create(self, offer, body):
        if offer in self.gone_offers:
            return None
        self.next += 1
        o = next(o for o in self.offers if o["id"] == offer)
        self.instances_[self.next] = dict(id=self.next, label=body["label"], dph_total=o["dph_total"], created=self.clock(),
                                          start_date=self.clock())
        self.created.append(body)
        if self.lose_create:
            raise vast.VastError("vast.ai: timed out")
        return self.next

    def show(self, instance):
        self.shown.append(instance)
        i = self.instances_.get(instance)
        if i is None:
            return None
        running = self.clock() - i["created"] >= self.runs_after
        return dict(i, actual_status=self.status if running else "loading", public_ipaddr="192.0.2.77",
                    ports={"22/tcp": [{"HostPort": "40022"}]} if running else None)

    def instances(self):
        return [dict(i, actual_status="running") for i in self.instances_.values() if i["id"] not in self.hidden]

    def attach_key(self, instance, public_key):
        self.keys[instance] = public_key
        return True

    def destroy(self, instance):
        if self.on_destroy:
            self.on_destroy(instance)
        if self.refuse_destroy:
            return False
        self.destroyed.append(instance)
        self.instances_.pop(instance, None)
        return True


def offer(id, dph, cores=64, gpu="RTX 5090", cpu="AMD EPYC 7763 64-Core Processor", ram=128000, power=575):  # noqa: A002
    return dict(id=id, dph_total=dph, cpu_cores_effective=float(cores), cpu_name=cpu, cpu_ram=ram, gpu_name=gpu, num_gpus=1,
                gpu_max_power=power, geolocation="Nowhere, ZZ", rentable=True, verified=True)


POOL = {
    "key": "keys/id",
    "lease_seconds": 900,
    "machines": [
        {"name": "here", "local": True, "os": "windows", "root": "D:/compute", "cores": 32, "gpu": "RTX 4080"},
        {"name": "win1", "host": "192.0.2.10", "user": "svc-account", "os": "windows", "root": "D:/compute", "cores": 128,
         "ram_gb": 256, "gpu": "RTX 4080", "worker": {"lanes": 200, "server_args": "--precision fp16"}},
        {"name": "lin1", "host": "192.0.2.11", "port": 2200, "user": "root", "os": "linux", "root": "/work", "cores": 64},
        {"name": "off", "host": "192.0.2.12", "user": "u", "os": "linux", "root": "/work", "enabled": False},
    ],
    "vast": {"max_dph": 1.10, "daily_budget": 10.0, "total_budget": 40.0, "max_instances": 1, "idle_minutes": 15,
             "watchdog_minutes": 20, "start_minutes": 30, "filters": {"gpu_name": "RTX_5090", "gpu_max_power": {"gte": 550}},
             "worker": {"lanes": 76}},
}


class Case(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="workers-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.store = Store(self.tmp / "home")
        self.clock = Clock()
        self.remote = Remote()
        self.vast = FakeVast(self.clock)
        self.daemons = []
        self.write_pool(POOL)
        (self.store.home / "keys").mkdir()
        (self.store.home / "keys" / "id").write_text("PRIVATE")
        (self.store.home / "keys" / "id.pub").write_text("ssh-ed25519 AAAA test\n")
        self.m = self.manager()

    def manager(self) -> Manager:
        return Manager(self.store, clock=self.clock, sleep=self.clock.sleep, remote=self.remote, api=self.vast,
                       log=lambda *_: None, ensure_daemon=lambda: self.daemons.append(1))

    def write_pool(self, pool: dict, **vast_settings):
        pool = json.loads(json.dumps(pool))
        pool.setdefault("vast", {}).update(vast_settings)
        self.store.home.mkdir(parents=True, exist_ok=True)
        self.store.pool.write_text(json.dumps(pool))
        if hasattr(self, "m"):
            self.m._pool = None

    def lease(self, request, holder="me", purpose="a test", **kw):
        return self.m.lease(request, holder, purpose, **kw)

    def ledger(self, *kinds):
        """The ledger's rentals and destructions (or the lines of `kinds`)."""
        return [e for e in read_lines(self.store.ledger) if e["event"] in (kinds or ("rent", "destroy"))]

    def events(self, kind):
        return [e for e in read_lines(self.store.log) if e["event"] == kind]


class Requests(unittest.TestCase):
    def test_a_request_is_names_all_or_what_a_machine_has_to_have(self):
        (g,) = parse_request("a,b")
        self.assertEqual((g.names, g.exclusive), (["a", "b"], True))
        (g,) = parse_request("all,os=linux")
        self.assertEqual((g.all, g.os), (True, "linux"))
        g, h = parse_request("worker1 + count=2,os=linux,gpu=5090,cores=32,ram=64,rent,remote,optional")
        self.assertEqual(g.names, ["worker1"])
        self.assertEqual((h.count, h.os, h.gpu, h.cores, h.ram, h.rent, h.where, h.optional, h.exclusive),
                         (2, "linux", "5090", 32, 64.0, True, "remote", True, False))

    def test_a_request_that_means_nothing_is_refused(self):
        for bad in ("", "a++b", "lanes=4", "os=mac", "cores=0", "count=x", "all,count=2", "all,rent", "a,rent", "a b", "all,local"):
            with self.assertRaises(ConfigError, msg=bad):
                parse_request(bad)


class Leases(Case):
    def test_a_lease_tells_its_holder_the_machine_but_never_how_to_reach_it(self):
        (w,) = self.lease("win1")
        self.assertEqual({k: w[k] for k in ("name", "os", "root", "cores", "gpu", "lanes", "server_args")},
                         dict(name="win1", os="windows", root="D:/compute", cores=128, gpu="RTX 4080", lanes=200,
                              server_args="--precision fp16"))
        self.assertTrue(w["lease"].startswith("L"))
        told = json.dumps([w, self.m.status(), self.m.candidates("all")])
        for secret in ("192.0.2", "svc-account", "keys/id", "2200"):
            self.assertNotIn(secret, told)

    def test_a_machine_is_busy_exactly_while_a_live_lease_holds_it(self):
        (w,) = self.lease("win1", holder="a")
        with self.assertRaisesRegex(Unmet, "win1 is busy: a \\(a test\\)"):
            self.lease("win1", holder="b")
        self.clock.now += 899
        with self.assertRaises(Unmet):
            self.lease("win1", holder="b")
        self.clock.now += 2  # its 900 seconds are over: nobody renewed it
        (again,) = self.lease("win1", holder="b")
        self.assertNotEqual(again["lease"], w["lease"])
        with self.assertRaises(LeaseError):
            self.m.resolve(w["lease"])
        self.assertEqual([e["why"] for e in self.events("end")], ["expired"])

    def test_a_renewal_puts_the_end_off_and_never_brings_a_lease_back(self):
        (w,) = self.lease("win1")
        for _ in range(5):
            self.clock.now += 600
            self.m.renew([w["lease"]])
        self.assertTrue(self.m.live(w["lease"]))
        self.clock.now += 901
        with self.assertRaises(LeaseError):
            self.m.renew([w["lease"]])
        self.assertFalse(self.m.live(w["lease"]))

    def test_a_release_ends_it_now_and_twice_is_once(self):
        (w,) = self.lease("win1")
        self.assertEqual(self.m.release([w["lease"]]), [w["lease"]])
        self.assertEqual(self.m.release([w["lease"]]), [])
        self.lease("win1", holder="someone else")

    def test_the_same_holder_asking_again_has_its_lease_back(self):
        (w,) = self.lease("win1", holder="driver", purpose="gen 3")
        self.clock.now += 500
        (again,) = self.lease("win1", holder="driver", purpose="gen 3 again")
        self.assertEqual(again["lease"], w["lease"])
        self.assertEqual(again["expires"], self.clock.now + 900)
        self.assertEqual(len(self.events("lease")), 1)

    def test_all_is_every_enabled_remote_machine_that_is_free(self):
        self.lease("lin1", holder="other")
        self.assertEqual([w["name"] for w in self.lease("all")], ["win1"])
        self.assertEqual(self.lease("all", holder="third"), [])

    def test_a_request_is_all_of_it_or_none(self):
        self.lease("lin1", holder="other")
        with self.assertRaises(Unmet):
            self.lease("win1,lin1")
        self.assertEqual(self.m.status()["machines"][1]["leases"], [])  # win1 was not kept
        self.assertEqual(len(self.events("lease")), 1)
        with self.assertRaisesRegex(ConfigError, "no machine is named nope"):
            self.lease("nope")
        with self.assertRaisesRegex(Unmet, "disabled"):
            self.lease("off")
        self.assertEqual([w["name"] for w in self.lease("win1,lin1,optional")], ["win1"])

    def test_cores_are_a_share_and_a_whole_machine_needs_it_empty(self):
        a = self.lease("win1,cores=64", holder="a")
        self.lease("win1,cores=64", holder="b")
        with self.assertRaises(Unmet):
            self.lease("win1,cores=1", holder="c")
        with self.assertRaises(Unmet):
            self.lease("win1", holder="c")
        self.m.release([a[0]["lease"]])
        self.lease("win1,cores=60", holder="c")
        with self.assertRaises(Unmet):
            self.lease("cores=200", holder="d")

    def test_a_gpu_is_one_holders_at_a_time(self):
        self.lease("win1,cores=8,gpu", holder="a")
        with self.assertRaises(Unmet):
            self.lease("win1,cores=8,gpu", holder="b")
        self.lease("win1,cores=8", holder="b")

    def test_what_a_machine_has_to_have_picks_this_machine_first_then_the_pools(self):
        self.assertEqual([w["name"] for w in self.lease("cores=16", holder="a")], ["here"])
        self.assertEqual([w["name"] for w in self.lease("cores=16,remote,os=linux", holder="b")], ["lin1"])
        self.assertEqual([w["name"] for w in self.lease("gpu=4080,count=2,cores=8", holder="c")], ["here", "win1"])
        with self.assertRaisesRegex(Unmet, "none in the pool fits"):
            self.lease("gpu=5090", holder="d")
        self.assertEqual(self.lease("gpu=5090,optional", holder="d"), [])
        with self.assertRaisesRegex(Unmet, "does not rent on request"):
            self.lease("gpu=5090,rent", holder="d")

    def test_candidates_says_what_could_serve_a_request_and_leases_nothing(self):
        self.lease("lin1", holder="other")
        c = self.m.candidates("all")
        self.assertEqual([(w["name"], w["os"], w["busy"]) for w in c["workers"]], [("win1", "windows", False), ("lin1", "linux", True)])
        self.assertFalse(c["may_rent"])
        self.assertEqual(len(self.store.read_state()["leases"]), 1)

    def test_a_lease_that_ends_has_its_processes_on_a_linux_machine_killed(self):
        (lin,) = self.lease("lin1")
        (win,) = self.lease("win1")
        (kept,) = self.lease("lin1,cores=1,optional", holder="x") or [None]
        self.m.release([lin["lease"], win["lease"]])
        self.assertEqual(self.remote.calls, [("lin1", access.sweep_line(lin["lease"]))])
        self.assertIn(f"CANTRIP_LEASE={lin['lease']}$", self.remote.calls[0][1])
        # Not one taken with its sweep off (the training driver's: its runs end by their own lease).
        self.remote.calls.clear()
        (own,) = self.lease("lin1", holder="driver", sweep=False)
        self.clock.now += 1000
        self.lease("lin1", holder="next")
        self.assertEqual(self.remote.calls, [])

    def test_a_pool_that_is_wrong_says_so(self):
        for change, message in ((dict(machines=[{"name": "a b", "os": "linux", "root": "/"}]), "needs a name"),
                                (dict(machines=[{"name": "a", "os": "linux", "root": "/w"}]), "has no host"),
                                (dict(machines=[{"name": "a", "os": "bsd", "root": "/w"}]), "windows or linux"),
                                (dict(vast={"budget": 1}), "no setting budget"),
                                (dict(machines=POOL["machines"][:2] + POOL["machines"][:2]), "two machines")):
            self.write_pool({**POOL, **change})
            with self.assertRaisesRegex(ConfigError, message):
                self.manager().lease("all", "me", "x")
        self.store.pool.unlink()
        with self.assertRaisesRegex(ConfigError, "no pool"):
            self.manager().lease("all", "me", "x")


class Proc:
    """A process `Access` started: it ends with `code` after `after` waits, or never."""

    def __init__(self, cmd, kw, code=0, after=0):
        self.cmd, self.kw, self.code, self.after, self.killed = cmd, kw, code, after, False

    def wait(self, timeout=None):
        if self.killed:
            return -9
        if self.after is None or self.after > 0:
            if self.after:
                self.after -= 1
            self.clock.now += timeout or 0
            raise subprocess.TimeoutExpired(self.cmd, timeout)
        return self.code

    def kill(self):
        self.killed = True


class Calls(Case):
    def setUp(self):
        super().setUp()
        self.procs: list[Proc] = []
        self.next = dict(code=0, after=0)
        self.acc = access.Access(self.m, popen=self.popen)

    def popen(self, cmd, **kw):
        p = Proc(cmd, kw, **self.next)
        p.clock = self.clock
        self.procs.append(p)
        return p

    def test_exec_is_the_systems_ssh_with_the_pools_address_and_key(self):
        (w,) = self.lease("lin1")
        self.assertEqual(self.acc.exec(w["lease"], ["ls", "-la", "/work"]), 0)
        cmd = self.procs[-1].cmd
        self.assertEqual(cmd[0], "ssh")
        # The whole command, its shell included, carries the lease in its environment (the sweep finds it by that).
        self.assertEqual(cmd[-2:], ["root@192.0.2.11", f"exec env CANTRIP_LEASE={w['lease']} \"${{SHELL:-sh}}\" -c 'ls -la /work'"])
        self.assertEqual(access.marked("L1", "echo 'a b'; sleep 1"), """exec env CANTRIP_LEASE=L1 "${SHELL:-sh}" -c 'echo '\\''a b'\\''; sleep 1'""")
        self.assertEqual(cmd[cmd.index("-p") + 1], "2200")
        self.assertEqual(Path(cmd[cmd.index("-i") + 1]), self.store.home / "keys" / "id")
        self.assertTrue(any(x.startswith("UserKnownHostsFile=") and "home/known_hosts" in x for x in cmd))
        self.assertIn("BatchMode=yes", cmd)
        # Its input and output are the caller's own handles: nothing is piped through this process.
        self.assertFalse({"stdin", "stdout", "stderr"} & set(self.procs[-1].kw))
        (e,) = self.events("exec")
        self.assertEqual((e["lease"], e["machine"], e["holder"], e["command"], e["code"]), (w["lease"], "lin1", "me", "ls -la /work", 0))
        # A Windows machine's line is as it was given.
        (win,) = self.lease("win1")
        self.next = dict(code=3, after=2)
        self.assertEqual(self.acc.exec(win["lease"], ["powershell", "-EncodedCommand", "AAA"]), 3)
        self.assertEqual(self.procs[-1].cmd[-2:], ["svc-account@192.0.2.10", "powershell -EncodedCommand AAA"])

    def test_no_call_without_a_live_lease(self):
        (w,) = self.lease("lin1")
        self.m.release([w["lease"]])
        for call in (lambda: self.acc.exec(w["lease"], ["true"]), lambda: self.acc.push(w["lease"], "a", "/b"),
                     lambda: self.acc.pull(w["lease"], "/a", "b"), lambda: self.acc.exec("Lnever", ["true"])):
            with self.assertRaises(LeaseError):
                call()
        self.assertEqual(self.procs, [])

    def test_a_call_is_ended_when_its_lease_ends(self):
        (w,) = self.lease("lin1", ttl=60)
        self.next = dict(code=0, after=None)  # it would never end
        self.assertEqual(self.acc.exec(w["lease"], ["sleep", "infinity"]), access.ENDED)
        self.assertTrue(self.procs[-1].killed)
        self.assertGreaterEqual(self.clock.now, T0 + 60)
        self.assertLess(self.clock.now, T0 + 66)

    def test_a_call_past_its_timeout_is_ended(self):
        (w,) = self.lease("lin1")
        self.next = dict(code=0, after=None)
        self.assertEqual(self.acc.exec(w["lease"], ["sleep", "infinity"], timeout=30), access.TIMED_OUT)
        self.assertTrue(self.procs[-1].killed)

    def test_push_and_pull_are_the_systems_scp(self):
        (w,) = self.lease("lin1")
        self.assertEqual(self.acc.push(w["lease"], "C:/data/blob", "/work/blobs/x.tmp"), 0)
        cmd = self.procs[-1].cmd
        self.assertEqual((cmd[0], cmd[-2:]), ("scp", ["C:/data/blob", "root@192.0.2.11:/work/blobs/x.tmp"]))
        self.assertEqual(cmd[cmd.index("-P") + 1], "2200")
        self.assertNotIn("-r", cmd)
        self.assertEqual(self.acc.pull(w["lease"], "/work/out", "C:/data/out", recursive=True), 0)
        cmd = self.procs[-1].cmd
        self.assertEqual(cmd[-3:], ["-r", "root@192.0.2.11:/work/out", "C:/data/out"])
        self.assertEqual([e["event"] for e in read_lines(self.store.log)][-2:], ["push", "pull"])

    def test_this_machine_is_reached_without_ssh(self):
        (w,) = self.lease("here")
        self.acc.exec(w["lease"], ["echo", "hi"])
        p = self.procs[-1]
        self.assertEqual((p.cmd, p.kw["shell"], p.kw["env"]["CANTRIP_LEASE"]), ("echo hi", True, w["lease"]))
        (self.tmp / "a.txt").write_text("x")
        self.assertEqual(self.acc.push(w["lease"], str(self.tmp / "a.txt"), str(self.tmp / "b.txt")), 0)
        self.assertEqual((self.tmp / "b.txt").read_text(), "x")
        self.assertEqual(len(self.procs), 1)
        with self.assertRaises(Refused):
            self.acc.forward(w["lease"], 0, 80, ["x"])

    def test_a_forward_runs_until_its_lease_ends_or_it_is_taken_down(self):
        (w,) = self.lease("lin1", ttl=60)
        mark = self.acc.forward_file(w["lease"], 5000)
        mark.parent.mkdir(parents=True)
        mark.write_text("{}")
        self.assertEqual(self.acc.forwards(), [dict(lease=w["lease"], local=5000)])
        self.next = dict(code=0, after=None)
        self.assertEqual(self.acc.forward_run(w["lease"], 5000, 8080), access.ENDED)
        cmd = self.procs[-1].cmd
        self.assertEqual(cmd[-4:], ["ExitOnForwardFailure=yes", "-L", "127.0.0.1:5000:127.0.0.1:8080", "root@192.0.2.11"])
        self.assertIn("-N", cmd)
        self.assertFalse(mark.exists())
        # Taken down by hand: its mark removed, its process sees that.
        (w,) = self.lease("lin1", holder="b")
        mark = self.acc.forward_file(w["lease"], 5001)
        mark.write_text("{}")
        self.assertTrue(self.acc.unforward(w["lease"], 5001))
        self.assertEqual(self.acc.forward_run(w["lease"], 5001, 8080), access.ENDED)

    def test_a_rented_box_is_known_by_a_name_of_its_own(self):
        m = dict(name="vast1", os="linux", host="192.0.2.77", port=40022, user="root", key="keys/id", alias="cantrip-vast-7001")
        cmd = access.ssh_command(self.m.pool, m)
        self.assertIn("HostKeyAlias=cantrip-vast-7001", cmd)


class Renting(Case):
    def setUp(self):
        super().setUp()
        self.vast.offers = [offer(1, 0.95, cores=128), offer(2, 0.80), offer(3, 1.30), offer(4, 0.50, gpu="RTX 4090"),
                            offer(5, 0.70, power=450)]

    def rented(self):
        return self.store.read_state()["rented"]

    def test_nothing_is_rented_until_the_pool_sets_every_limit(self):
        for off in ("max_dph", "daily_budget", "total_budget", "max_instances"):
            self.write_pool(POOL, **{off: 0})
            with self.assertRaisesRegex(LimitError, "renting is off"):
                self.m.rent()
        self.write_pool({k: v for k, v in POOL.items() if k != "vast"})
        with self.assertRaisesRegex(LimitError, "renting is off"):
            self.m.rent()
        self.assertEqual((self.vast.created, self.ledger(), self.rented()), ([], [], {}))

    def test_the_offer_taken_is_the_cheapest_that_passes_the_cap_and_the_filters(self):
        self.assertEqual([o["id"] for o in self.m.offers()], [2, 1])
        q = self.vast.queries[-1]
        self.assertEqual((q["gpu_name"], q["gpu_max_power"], q["dph_total"], q["type"], q["verified"], q["rentable"]),
                         ({"eq": "RTX 5090"}, {"gte": 550}, {"lte": 1.1}, "on-demand", {"eq": True}, {"eq": True}))
        self.write_pool(POOL, order="price_per_core")
        self.assertEqual([o["id"] for o in self.m.offers()], [1, 2])
        self.write_pool(POOL, cpu_names=["Threadripper", "EPYC 9"])
        self.assertEqual(self.m.offers(), [])
        (g,) = parse_request("cores=100,ram=64")
        self.write_pool(POOL)
        self.assertEqual([o["id"] for o in self.m.offers(g)], [1])

    def test_a_box_is_rented_started_checked_and_ready(self):
        r = self.m.rent()
        self.assertEqual((r["name"], r["instance"], r["dph"]), ("vast1", 7001, 0.80))
        box = self.rented()["vast1"]
        self.assertEqual((box["status"], box["host"], box["port"], box["idle_since"]), ("ready", "192.0.2.77", 40022, self.clock.now))
        self.assertEqual(self.vast.keys, {7001: "ssh-ed25519 AAAA test"})
        body = self.vast.created[0]
        self.assertEqual((body["runtype"], body["image"]), ("ssh_direc ssh_proxy", POOL_IMAGE))
        # Its label is this rental's alone, and in the ledger before vast.ai was asked for anything.
        self.assertRegex(body["label"], r"^cantrip-vast1-[0-9a-f]{8}$")
        self.assertEqual([(e["event"], e["label"]) for e in self.ledger("intent", "rent")],
                         [("intent", body["label"]), ("rent", body["label"])])
        self.assertIn("LIMIT=1200", body["onstart"])
        (line,) = self.ledger()
        self.assertEqual((line["event"], line["instance"], line["dph"], line["name"]), ("rent", 7001, 0.80, "vast1"))
        self.assertEqual(self.daemons, [1])
        # It was taken only once its watchdog said it can destroy the box.
        self.assertIn(vast.STATUS, self.remote.calls[-1][1])
        # A machine like the others now, its lease's record without an address.
        (w,) = self.lease("vast1")
        self.assertEqual({k: w[k] for k in ("name", "os", "root", "gpu", "cores", "lanes", "rented")},
                         dict(name="vast1", os="linux", root="/workspace/compute", gpu="RTX 5090", cores=64, lanes=76, rented=True))
        self.assertNotIn("192.0.2.77", json.dumps(w))
        self.assertEqual([w["name"] for w in self.lease("all", holder="b")], ["win1", "lin1"])

    def test_a_box_that_cannot_destroy_itself_is_not_used(self):
        self.remote.answer = lambda m, line: (0, "unarmed: no instance key in the environment\n")
        with self.assertRaisesRegex(Refused, "cannot stop its own bill"):
            self.m.rent()
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))
        self.assertEqual([(e["event"], e.get("by")) for e in self.ledger()], [("rent", None), ("destroy", "failed-start")])

    def test_a_box_that_never_comes_up_is_destroyed(self):
        self.vast.runs_after = 10 * 3600
        with self.assertRaisesRegex(Refused, "not running and reachable 30 minutes"):
            self.m.rent()
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))
        (_, d) = self.ledger()
        self.assertAlmostEqual(d["hours"], 0.5, places=1)
        self.assertAlmostEqual(d["cost"], 0.4, places=1)
        # One vast.ai gave up on is not waited for.
        self.vast.runs_after, self.vast.status = 40, "exited"
        start = self.clock.now
        with self.assertRaisesRegex(Refused, "is exited"):
            self.m.rent()
        self.assertLess(self.clock.now - start, 120)
        self.assertEqual(self.vast.instances_, {})

    def test_a_box_whose_destruction_fails_stays_in_the_books_until_it_is_gone(self):
        self.remote.answer = lambda m, line: (255, "")
        self.vast.refuse_destroy = True
        with self.assertRaises(Refused):
            self.m.rent()
        self.assertEqual((self.rented()["vast1"]["status"], self.rented()["vast1"]["why"]), ("destroying", "failed-start"))
        self.assertEqual(self.m.reap(), [])
        self.assertIn("vast1", self.rented())
        self.vast.refuse_destroy = False
        self.assertEqual(self.m.reap(), ["vast1"])
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))

    def test_an_offer_that_is_gone_is_passed_over(self):
        self.vast.gone_offers = {2}
        self.assertEqual(self.m.rent()["dph"], 0.95)

    def test_the_limits_hold(self):
        self.m.rent()
        with self.assertRaisesRegex(LimitError, "the limit is 1"):
            self.m.rent()
        self.m.destroy("vast1")
        self.vast.offers = [offer(3, 1.30)]
        with self.assertRaisesRegex(Unmet, "no vast.ai offer under"):
            self.m.rent()
        self.assertEqual(self.rented(), {})
        # The budgets: an hour of the box has to fit in what is left of each.
        self.vast.offers = [offer(2, 0.80)]
        self.write_pool(POOL, daily_budget=0.5)
        with self.assertRaisesRegex(LimitError, "daily budget"):
            self.m.rent()
        self.write_pool(POOL, total_budget=0.5)
        with self.assertRaisesRegex(LimitError, "total budget"):
            self.m.rent()
        self.assertEqual((self.rented(), len(self.vast.created)), ({}, 1))

    def test_a_request_rents_only_when_the_pool_allows_it(self):
        with self.assertRaisesRegex(Unmet, "does not rent on request"):
            self.lease("gpu=5090,os=linux,rent")
        self.assertEqual(self.vast.created, [])
        self.write_pool(POOL, auto_rent=True)
        self.assertTrue(self.m.candidates("gpu=5090,rent")["may_rent"])
        (w,) = self.lease("gpu=5090,os=linux,rent", holder="driver")
        self.assertEqual((w["name"], w["rented"]), ("vast1", True))
        self.assertIsNone(self.rented()["vast1"]["idle_since"])
        # The box is there now: the next request for one takes it when it is free, and rents no second.
        with self.assertRaisesRegex(LimitError, "the limit is 1"):
            self.lease("gpu=5090,rent", holder="other")
        self.m.release([w["lease"]])
        (again,) = self.lease("gpu=5090,rent", holder="other")
        self.assertEqual((again["name"], len(self.vast.created)), ("vast1", 1))

    def test_a_request_whose_rental_fails_keeps_none_of_its_leases(self):
        self.write_pool(POOL, auto_rent=True)
        self.vast.offers = []
        with self.assertRaises(Unmet):
            self.lease("win1+gpu=5090,rent")
        self.assertEqual(self.store.read_state()["leases"], {})

    def test_an_idle_box_is_destroyed_after_its_minutes_and_a_leased_one_never(self):
        self.m.rent()
        self.clock.now += 14 * 60
        self.assertEqual(self.m.reap(), [])
        (w,) = self.lease("vast1")
        for _ in range(20):  # held for five hours, renewed
            self.clock.now += 15 * 60 - 1
            self.m.renew([w["lease"]])
            self.assertEqual(self.m.reap(), [])
        self.m.release([w["lease"]])
        self.clock.now += 14 * 60
        self.assertEqual(self.m.reap(), [])
        self.clock.now += 61
        self.assertEqual(self.m.reap(), ["vast1"])
        self.assertEqual((self.rented(), self.vast.instances_), ({}, {}))
        d = self.ledger()[-1]
        self.assertEqual((d["event"], d["by"], d["instance"]), ("destroy", "idle", 7001))
        self.assertAlmostEqual(d["cost"], d["hours"] * 0.80, places=3)

    def test_a_lease_nobody_renews_stops_holding_its_box(self):
        self.m.rent()
        self.lease("vast1")
        self.clock.now += 900 + 14 * 60  # expired a quarter of an hour in; idle since then
        self.assertEqual(self.m.reap(), [])
        self.clock.now += 61
        self.assertEqual(self.m.reap(), ["vast1"])

    def test_a_spent_budget_destroys_every_box(self):
        self.m.rent()
        (w,) = self.lease("vast1", ttl=8 * 3600)
        self.write_pool(POOL, daily_budget=2.0)
        self.clock.now += 2 * 3600
        self.assertEqual(self.m.reap(), [])
        self.clock.now += 3600
        self.assertEqual(self.m.reap(), ["vast1"])
        self.assertEqual(self.ledger()[-1]["by"], "budget")
        with self.assertRaises(LeaseError):
            self.m.resolve(w["lease"])

    def test_what_was_spent_is_counted_by_the_utc_day(self):
        led = [dict(event="rent", at=T0 - 7200, instance=1, name="a", dph=1.0),
               dict(event="destroy", at=T0 + 1800, instance=1),
               dict(event="rent", at=T0 + 3600, instance=2, name="b", dph=2.0)]
        s = vast.spend(led, T0 + 7200)
        self.assertAlmostEqual(s["total"], 2.5 + 2.0)
        self.assertAlmostEqual(s["today"], 1.5 + 2.0)  # the first began an hour before midnight

    def test_the_tool_tells_each_box_it_is_there(self):
        self.m.rent()
        self.remote.calls.clear()
        self.clock.now += 60
        self.assertTrue(self.m.tick())
        self.assertEqual(self.remote.calls, [("vast1", f"touch {vast.HEARTBEAT}; tail -n 1 {vast.WATCHDOG_LOG} 2>/dev/null")])
        self.assertEqual(self.rented()["vast1"]["last_heartbeat"], self.clock.now)
        self.clock.now += 16 * 60
        self.assertFalse(self.m.tick())  # idle: destroyed, and nothing left to watch

    def test_a_box_that_destroyed_itself_is_written_off_and_strangers_are_only_named(self):
        self.m.rent()
        self.m.heartbeat()
        heard = self.clock.now
        self.vast.instances_.clear()
        self.vast.instances_[9] = dict(id=9, label="cantrip-elsewhere", dph_total=0.4, created=0)
        self.vast.instances_[10] = dict(id=10, label="not ours", dph_total=0.4, created=0)
        self.clock.now += 5 * 3600  # this machine slept
        logged = []
        self.m.log = logged.append
        r = self.m.reconcile()
        self.assertEqual(r["gone"], ["vast1"])
        self.assertEqual([s["instance"] for s in r["strays"]], [9])
        self.assertEqual(set(self.vast.instances_), {9, 10})
        # Asked for by id before it was written off, and destroyed once more to be sure.
        self.assertEqual((self.vast.shown[-1], self.vast.destroyed[-1]), (7001, 7001))
        # The stranger is said every time, and `status` shows it.
        self.assertTrue(any("not this pool's" in line and "instance 9" in line for line in logged))
        self.assertEqual([x["instance"] for x in self.m.status()["strays"]], [9])
        d = self.ledger()[-1]
        self.assertEqual(d["by"], "gone")
        self.assertAlmostEqual(d["ended"], heard + 21 * 60)
        self.assertLess(d["cost"], 0.5)
        self.assertEqual(self.rented(), {})

    def test_the_watchdog_is_a_script_bash_takes_within_vasts_limit(self):
        text = vast.watchdog_script(self.m.pool.vast)
        self.assertLess(len(text), 4048)
        for needed in ("CONTAINER_API_KEY", "CONTAINER_ID", "https://console.vast.ai/api/v0/instances/$CONTAINER_ID/",
                       "DELETE", "LIMIT=1200", vast.HEARTBEAT.rsplit("/", 1)[1], "armed"):
            self.assertIn(needed, text)
        bash = shutil.which("bash")
        if bash:
            p = subprocess.run([bash, "-n"], input=text.encode(), capture_output=True)
            self.assertEqual(p.returncode, 0, p.stderr.decode())


POOL_IMAGE = "nvidia/cuda:12.8.1-cudnn-runtime-ubuntu24.04"


class Wire(unittest.TestCase):
    """vast.ai's API as the client calls it."""

    def setUp(self):
        self.sent = []
        self.answers = []

        def request(method, url, headers, body, timeout=60.0):
            self.sent.append((method, url, headers, json.loads(body) if body else None))
            return self.answers.pop(0)

        self.api = vast.Api("KEY", request=request, sleep=lambda s: None)

    def test_each_call_is_the_documented_one(self):
        self.answers += [(200, {"offers": [{"id": 1}, {"id": 2, "rented": True}]}), (200, {"success": True, "new_contract": 55}),
                         (200, {"instances": {"id": 55, "actual_status": "running"}}), (200, {"success": True}),
                         (200, {"success": True, "msg": "gone"}), (404, {"success": False, "error": "not_found"})]
        self.assertEqual(self.api.search({"q": 1}), [{"id": 1}])
        self.assertEqual(self.api.create(1, {"image": "x"}), 55)
        self.assertEqual(self.api.show(55)["actual_status"], "running")
        self.assertTrue(self.api.attach_key(55, "ssh-ed25519 A"))
        self.assertTrue(self.api.destroy(55))
        self.assertTrue(self.api.destroy(55))  # already gone is gone
        base = "https://console.vast.ai/api/v0"
        self.assertEqual([(m, u.split("?")[0], b) for m, u, _, b in self.sent],
                         [("POST", f"{base}/bundles/", {"q": 1}), ("PUT", f"{base}/asks/1/", {"image": "x"}),
                          ("GET", f"{base}/instances/55/", None), ("POST", f"{base}/instances/55/ssh/", {"ssh_key": "ssh-ed25519 A"}),
                          ("DELETE", f"{base}/instances/55/", {}), ("DELETE", f"{base}/instances/55/", {})])
        self.assertTrue(all(h["Authorization"] == "Bearer KEY" for _, _, h, _ in self.sent))

    def test_the_list_is_paged_and_a_busy_api_is_asked_again(self):
        self.answers += [(429, "API requests too frequent"), (200, {"instances": [{"id": 1}], "next_token": "t"}),
                         (200, {"instances": [{"id": 2}], "next_token": None})]
        self.assertEqual([i["id"] for i in self.api.instances()], [1, 2])
        self.assertIn("after_token=t", self.sent[-1][1])
        self.assertIn("/api/v1/instances/", self.sent[-1][1])

    def test_an_offer_gone_is_none_and_any_other_refusal_an_error(self):
        self.answers += [(410, {"success": False, "error": "no_such_ask"}), (400, {"success": False, "error": "invalid_args"}),
                         (500, "oops"), (200, {"success": False})]
        self.assertIsNone(self.api.create(1, {}))
        with self.assertRaises(vast.VastError):
            self.api.create(1, {})
        with self.assertRaises(vast.VastError):
            self.api.search({})
        self.assertFalse(self.api.destroy(5))

    def test_an_instances_address_is_its_direct_port_else_the_proxy(self):
        self.assertEqual(vast.address(dict(public_ipaddr=" 192.0.2.1 ", ports={"22/tcp": [{"HostPort": "4022"}]}, ssh_host="p", ssh_port=1)),
                         ("192.0.2.1", 4022))
        self.assertEqual(vast.address(dict(public_ipaddr="192.0.2.1", ports={}, ssh_host="proxy.example", ssh_port=7)), ("proxy.example", 7))
        self.assertIsNone(vast.address(dict(ports=None)))


class Command(Case):
    def run_cli(self, *argv, manager=True):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["--home", str(self.store.home), *argv], manager=self.m if manager else None)
        return code, out.getvalue(), err.getvalue()

    def test_a_lease_comes_back_as_json_and_goes_through_its_life(self):
        code, out, _ = self.run_cli("lease", "win1+lin1", "--holder", "driver", "--purpose", "gen 1", "--json", "--no-sweep")
        self.assertEqual(code, 0)
        workers = json.loads(out)["workers"]
        self.assertEqual([(w["name"], w["os"], w["root"]) for w in workers], [("win1", "windows", "D:/compute"), ("lin1", "linux", "/work")])
        self.assertFalse(self.store.read_state()["leases"][workers[0]["lease"]]["sweep"])
        ids = [w["lease"] for w in workers]
        self.assertEqual(self.run_cli("renew", *ids)[0], 0)
        code, out, _ = self.run_cli("status", "--json")
        self.assertEqual([len(m["leases"]) for m in json.loads(out)["machines"]], [0, 1, 1, 0])
        self.assertEqual(self.run_cli("release", *ids)[0], 0)
        code, _, err = self.run_cli("renew", ids[0])
        self.assertEqual((code, "no live lease" in err), (70, True))

    def test_what_is_refused_has_its_exit_code(self):
        self.lease("win1", holder="other")
        self.assertEqual(self.run_cli("lease", "win1", "--holder", "a", "--purpose", "b")[0], 71)
        self.assertEqual(self.run_cli("exec", "Lnone", "--", "true")[0], 70)
        self.assertEqual(self.run_cli("vast", "rent", "worker1")[0], 2)
        self.write_pool(POOL, max_dph=0)
        self.assertEqual(self.run_cli("vast", "rent")[0], 72)
        self.assertEqual(self.run_cli("lease", "bad=1", "--holder", "a", "--purpose", "b")[0], 73)
        self.assertEqual(self.run_cli("status", "--", "x")[0], 2)

    def test_an_old_workers_file_is_imported_its_settings_kept_for_the_driver(self):
        old = self.tmp / "workers.json"
        old.write_text(json.dumps(dict(key="C:/keys/old", workers=[
            dict(name="lan", host="192.0.2.20", user="worker", root="D:/c", cpu_threads=128, ram_gb=255, gpu="RTX 4080",
                 notes="a note", enabled=True, lanes=120, selfplay_args=["--no-autoscale"]),
            dict(name="old-box", os="linux", host="192.0.2.21", port=1, user="root", root="/w", enabled=False)])))
        code, out, _ = self.run_cli("import", str(old), "--skip-disabled", "--dry-run")
        self.assertEqual((code, "would add" in out, "old-box" in out), (0, True, True))
        self.assertEqual(self.run_cli("import", str(old), "--skip-disabled")[0], 0)
        self.m._pool = None
        m = self.m.pool.machines["lan"]
        self.assertEqual({k: m[k] for k in ("host", "user", "os", "root", "cores", "ram_gb", "gpu", "key", "worker")},
                         dict(host="192.0.2.20", user="worker", os="windows", root="D:/c", cores=128, ram_gb=255, gpu="RTX 4080",
                              key="C:/keys/old", worker=dict(lanes=120, selfplay_args=["--no-autoscale"])))
        self.assertNotIn("old-box", self.m.pool.machines)
        self.assertEqual(self.run_cli("import", str(old))[0], 73)  # lan is there already
        self.assertEqual(self.run_cli("import", str(old), "--replace")[0], 0)
        (w,) = self.lease("lan")
        self.assertEqual((w["lanes"], w["selfplay_args"], w["cores"]), (120, ["--no-autoscale"], 128))

    def test_init_writes_a_pool_that_reaches_and_rents_nothing(self):
        self.store.pool.unlink()
        self.assertEqual(self.run_cli("init")[0], 0)
        self.m._pool = None
        self.assertEqual(self.lease("all"), [])
        with self.assertRaises(LimitError):
            self.m.rent()
        self.assertEqual(self.run_cli("init")[0], 2)

    def test_the_real_command_passes_a_calls_input_output_and_exit_code_straight_through(self):
        script = self.tmp / "upper.py"
        script.write_text("import sys\nsys.stdout.write(sys.stdin.read().upper())\nsys.exit(7)\n")
        launcher = [sys.executable, str(Path(cli.__file__).resolve().parents[1] / "cantrip-workers.py"), "--home", str(self.store.home)]
        p = subprocess.run([*launcher, "lease", "here", "--holder", "t", "--purpose", "passthrough", "--json"], capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        lease = json.loads(p.stdout)["workers"][0]["lease"]
        p = subprocess.run([*launcher, "exec", lease, "--timeout", "60", "--", f'"{sys.executable}"', f'"{script}"'],
                           input=b"a blob of bytes\n", capture_output=True)
        self.assertEqual((p.returncode, p.stdout.strip()), (7, b"A BLOB OF BYTES"), p.stderr)
        self.assertEqual(subprocess.run([*launcher, "release", lease], capture_output=True).returncode, 0)
        p = subprocess.run([*launcher, "exec", lease, "--", "echo", "x"], capture_output=True, text=True)
        self.assertEqual((p.returncode, p.stdout), (70, ""))

    def test_the_daemon_is_started_in_the_directory_of_the_command_that_needs_it(self):
        spawned = []
        with unittest.mock.patch.object(cli, "spawn_detached", spawned.append), \
                unittest.mock.patch.dict(os.environ, {"CANTRIP_WORKERS_NO_DAEMON": "0"}):
            cli.ensure_daemon(self.store)
            self.assertEqual(spawned[0][-3:], ["--home", str(self.store.home), "daemon"])
            # Not a second one while one runs.
            held = self.store.daemon_lock().acquire()
            try:
                cli.ensure_daemon(self.store)
            finally:
                held.release()
            self.assertEqual(len(spawned), 1)
            # A command given its --home hands the same to the daemon it starts (never the per-user directory).
            out = io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(cli.main(["--home", str(self.store.home), "lease", "lin1", "--holder", "a", "--purpose", "b"]), 0)
            self.assertEqual(spawned[-1][-3:], ["--home", str(self.store.home), "daemon"])
        # And a daemon where there is no pool and nothing to watch ends at once.
        empty = Manager(Store(self.tmp / "nowhere"), clock=self.clock, sleep=self.clock.sleep, log=lambda *_: None)
        self.assertEqual(cli.run_daemon(empty, once=False), 0)

    def test_the_daemon_is_one_at_a_time_and_ends_when_nothing_is_rented(self):
        self.vast.offers = [offer(2, 0.80)]
        self.m.rent()
        self.clock.now += 14 * 60
        self.assertEqual(cli.run_daemon(self.m, once=False), 0)  # heartbeats, then reaps it at the quarter hour, then ends
        self.assertEqual(self.store.read_state()["rented"], {})
        self.assertIn("destroyed (idle)", (self.store.home / "daemon.log").read_text())
        held = self.store.daemon_lock().acquire()
        try:
            self.assertEqual(self.run_cli("daemon", "--once")[0], 0)
        finally:
            held.release()


if __name__ == "__main__":
    unittest.main()
