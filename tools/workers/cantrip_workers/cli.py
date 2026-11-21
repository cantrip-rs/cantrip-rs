"""`cantrip-workers` (docs/workers.md).

    cantrip-workers lease REQUEST --holder WHO --purpose WHAT [--ttl S] [--no-sweep] [--json]
    cantrip-workers renew LEASE... [--ttl S]            cantrip-workers release LEASE...
    cantrip-workers exec LEASE [--timeout S] -- COMMAND...
    cantrip-workers push LEASE SRC DST [-r]             cantrip-workers pull LEASE SRC DST [-r]
    cantrip-workers forward LEASE LOCAL:REMOTE          cantrip-workers unforward LEASE LOCAL
    cantrip-workers status [--json]                     cantrip-workers machines REQUEST [--json]
    cantrip-workers vast offers|rent|destroy|show|ledger ...
    cantrip-workers reap | daemon | log | home | init | import FILE

Exit codes: a command's own for `exec`, `push` and `pull` (255: the machine could not be reached); 70 no live lease
(or the lease ended under the call); 71 no machine for the request; 72 a rental the limits refuse; 73 the pool's
configuration; 74 vast.ai; 124 a call's --timeout.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

from . import vast
from .access import Access, spawn_detached
from .manager import Manager
from .pool import parse_request
from .store import ConfigError, Refused, Store, read_json, read_lines, write_atomic

LAUNCHER = [sys.executable, str(Path(__file__).resolve().parents[1] / "cantrip-workers.py")]

EXAMPLE = {
    "key": "keys/workers_ed25519",
    "lease_seconds": 900,
    "machines": [
        {"name": "here", "local": True, "os": "windows", "root": "D:/compute", "cores": 32, "ram_gb": 64, "gpu": "RTX 4080",
         "enabled": False},
        {"name": "worker1", "host": "192.0.2.10", "user": "worker", "os": "windows", "root": "D:/compute", "cores": 64,
         "ram_gb": 128, "gpu": "RTX 4080", "worker": {}, "enabled": False},
    ],
    "vast": {"auto_rent": False, "max_dph": 0, "daily_budget": 0, "total_budget": 0, "max_instances": 0,
             "filters": {"gpu_name": "RTX 5090", "num_gpus": 1, "cpu_cores_effective": {"gte": 32}, "reliability": {"gt": 0.98}}},
}

# The fields of an old workers file (`generation.py --workers FILE`, before the pool) that say what a machine is; the
# rest were the driver's settings for it and go to its `worker`.
IMPORTED = ("name", "host", "port", "user", "os", "root", "key", "enabled", "gpu", "ram_gb", "notes")


def ensure_daemon(store: Store):
    """Starts the daemon unless one runs: whoever holds its lock is one. `CANTRIP_WORKERS_NO_DAEMON=1` starts none
    (tests, and a machine whose daemon is run some other way)."""
    if os.environ.get("CANTRIP_WORKERS_NO_DAEMON") == "1":
        return
    try:
        store.daemon_lock().acquire().release()
    except BlockingIOError:
        return
    # Of this directory, said outright: the command that needs it may have been given its --home.
    spawn_detached([*LAUNCHER, "--home", str(store.home), "daemon"])


def run_daemon(m: Manager, once: bool) -> int:
    out = m.store.home / "daemon.log"

    def log(message: str):
        with open(out, "a", encoding="utf-8") as f:
            f.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {message}\n")
        if once:
            print(message, file=sys.stderr)

    rounds = 0
    interval = 60.0
    first = True
    while True:
        try:
            lock = m.store.daemon_lock().acquire()
        except BlockingIOError:
            if first:
                print("cantrip-workers: the daemon is already running", file=sys.stderr)
            return 0
        if first:
            m.log = log
            first = False
        try:
            # Nothing but having nothing left to watch ends it: a round that fails, for whatever reason (the pool's
            # file half written, vast.ai down, a bug), is logged and the next one runs. The daemon is what stops
            # the bill.
            while True:
                try:
                    more = m.tick(reconcile=rounds % 10 == 0)
                except Refused as e:
                    log(f"a round failed: {e}")
                    more = True
                except Exception as e:  # noqa: BLE001
                    log(f"a round failed: {type(e).__name__}: {e}")
                    more = True
                rounds += 1
                if once or not more:
                    break
                try:
                    interval = float(m.pool.vast["heartbeat_seconds"])
                except Exception:  # noqa: BLE001 - no usable pool this moment: the last interval stands
                    pass
                m.sleep(interval)
        finally:
            lock.release()
        if once:
            return 0
        # Work that came between the last round and the lock's release found the lock held and started no daemon:
        # looked for once more now that the lock is free. Whatever comes after this look finds it free.
        try:
            if not m.watching():
                return 0
        except Exception as e:  # noqa: BLE001
            log(f"a round failed: {type(e).__name__}: {e}")


def import_workers(store: Store, path: Path, skip_disabled: bool, replace: bool, dry: bool) -> dict:
    """An old workers file's machines, added to the pool."""
    old = read_json(path)
    if not isinstance(old, dict) or not isinstance(old.get("workers"), list):
        raise ConfigError(f"{path} is not a workers file: {{\"key\": ..., \"workers\": [...]}}")
    pool = read_json(store.pool) or {"machines": []}
    pool.setdefault("machines", [])
    if old.get("key") and not pool.get("key"):
        pool["key"] = old["key"]
    names = {m.get("name"): k for k, m in enumerate(pool["machines"])}
    added, skipped = [], []
    for w in old["workers"]:
        if skip_disabled and w.get("enabled", True) is False:
            skipped.append(w.get("name"))
            continue
        m = {k: w[k] for k in IMPORTED if k in w}
        m.setdefault("os", "windows")
        if old.get("key") and pool.get("key") != old["key"]:
            m.setdefault("key", old["key"])
        if w.get("cpu_threads"):
            m["cores"] = w["cpu_threads"]
        rest = {k: v for k, v in w.items() if k not in IMPORTED and k != "cpu_threads"}
        if rest:
            m["worker"] = rest
        if m.get("name") in names:
            if not replace:
                raise ConfigError(f"the pool already has a machine named {m['name']}: --replace to overwrite it")
            pool["machines"][names[m["name"]]] = m
        else:
            pool["machines"].append(m)
        added.append(m.get("name"))
    if not dry:
        store.home.mkdir(parents=True, exist_ok=True)
        write_atomic(store.pool, json.dumps(pool, indent=2) + "\n")
        from .pool import Pool  # noqa: PLC0415

        Pool(store)  # what was written is a pool
    return dict(pool=str(store.pool), added=added, skipped=skipped, written=not dry)


def parser() -> argparse.ArgumentParser:
    a = argparse.ArgumentParser(prog="cantrip-workers", description="Leases on compute machines, and every call made on one (docs/workers.md).")
    a.add_argument("--home", help="the local directory (default: CANTRIP_WORKERS_HOME, else the per-user one)")
    sub = a.add_subparsers(dest="command", required=True)

    def add(name, help, **kw):  # noqa: A002
        return sub.add_parser(name, help=help, description=help, **kw)

    p = add("lease", "take leases: the request's machines, all of them or none")
    p.add_argument("request")
    p.add_argument("--holder", required=True, help="who holds it: the same holder asking again gets its leases back")
    p.add_argument("--purpose", required=True)
    p.add_argument("--ttl", type=float, help="seconds until it expires unless renewed (default: the pool's lease_seconds)")
    p.add_argument("--no-sweep", action="store_true", help="leave its processes on the machine when it ends")
    p.add_argument("--json", action="store_true")
    p = add("renew", "put off the end of leases")
    p.add_argument("leases", nargs="+")
    p.add_argument("--ttl", type=float)
    p.add_argument("--json", action="store_true")
    p = add("release", "end leases now")
    p.add_argument("leases", nargs="+")
    p = add("exec", "run a command on a lease's machine: exec LEASE [--timeout S] -- COMMAND...")
    p.add_argument("lease")
    p.add_argument("--timeout", type=float)
    for name, help in (("push", "copy a file of this machine's to a lease's machine"), ("pull", "copy a file from a lease's machine here")):  # noqa: A001
        p = add(name, help)
        p.add_argument("lease")
        p.add_argument("source")
        p.add_argument("destination")
        p.add_argument("-r", "--recursive", action="store_true")
        p.add_argument("--timeout", type=float)
    p = add("forward", "forward a local port to a port of a lease's machine, until the lease ends")
    p.add_argument("lease")
    p.add_argument("ports", metavar="LOCAL:REMOTE", help="LOCAL 0: a free port, which is printed")
    p = add("unforward", "end a forward")
    p.add_argument("lease")
    p.add_argument("local", type=int)
    p = add("forward-run", argparse.SUPPRESS)
    p.add_argument("lease")
    p.add_argument("local", type=int)
    p.add_argument("remote", type=int)
    p = add("status", "the machines, who holds them, the rented boxes and what was spent")
    p.add_argument("--json", action="store_true")
    p = add("machines", "what a request could be served by, nothing leased")
    p.add_argument("request")
    p.add_argument("--json", action="store_true")
    v = add("vast", "rented machines (vast.ai)").add_subparsers(dest="vast", required=True)
    p = v.add_parser("offers", help="the offers a box would be rented from, best first")
    p.add_argument("request", nargs="?")
    p.add_argument("--json", action="store_true")
    p = v.add_parser("rent", help="rent a box now, within the limits")
    p.add_argument("request", nargs="?")
    p.add_argument("--name")
    p = v.add_parser("destroy", help="destroy a rented box now")
    p.add_argument("name")
    p = v.add_parser("show", help="the rented boxes against the account's instances")
    p.add_argument("--json", action="store_true")
    p = v.add_parser("ledger", help="every rental and what it cost")
    p.add_argument("--json", action="store_true")
    add("reap", "destroy rented boxes that are idle or over budget, now")
    p = add("daemon", "what reaps rented boxes and tells them the tool is there (started by a rental)")
    p.add_argument("--once", action="store_true")
    p = add("log", "the last calls")
    p.add_argument("-n", type=int, default=30)
    add("home", "the local directory")
    add("init", "write an example pool.json to fill in")
    p = add("import", "add an old workers file's machines to the pool")
    p.add_argument("file")
    p.add_argument("--skip-disabled", action="store_true")
    p.add_argument("--replace", action="store_true")
    p.add_argument("--dry-run", action="store_true")
    return a


def one_group(text: str | None):
    if not text:
        return None
    groups = parse_request(text)
    if len(groups) != 1 or groups[0].names or groups[0].all:
        raise Refused("a rental's request is one group of what the box has to have: cores=, ram=, gpu", 2)
    return groups[0]


def show(value, as_json: bool, lines):
    if as_json:
        print(json.dumps(value, indent=1))
    else:
        for line in lines(value):
            print(line)


def status_lines(s: dict):
    for m in s["machines"]:
        what = ", ".join(str(x) for x in (m["os"], f"{m['cores']} cores" if m.get("cores") else None, m.get("gpu"),
                                          "this machine" if m.get("local") else None, "rented" if m.get("rented") else None,
                                          None if m["enabled"] else "disabled") if x)
        yield f"{m['name']}: {what}"
        for l in m["leases"]:
            share = f"{l['cores']} cores" if l["cores"] else "whole"
            yield f"  {l['lease']}  {l['holder']}: {l['purpose']} ({share}; expires in {l['expires_in']} s)"
        if not m["leases"] and m["enabled"]:
            yield "  free"
    for b in s["rented"]:
        if b["status"] != "ready":
            yield f"{b['name']}: {b['status']}{' (' + b['why'] + ')' if b.get('why') else ''} (instance {b['instance']})"
        else:
            yield (f"  {b['name']} is instance {b['instance']}, ${b['dph']:.3f}/h for {b['hours']} h"
                   + (f", idle {b['idle_minutes']} min" if b["idle_minutes"] is not None else ""))
        if b.get("watchdog"):
            yield f"  {b['name']}'s watchdog: {b['watchdog']}"
    if "unsettled" in s:
        yield ("the ledger holds a rental the state does not account for: the daemon is checking the account"
               + (f", and cannot: {s['unsettled']['error']}" if s["unsettled"].get("error") else ""))
    for x in s.get("strays") or []:
        yield f"on the account, not this pool's: instance {x['instance']} ({x['label']}, {x['status']}, ${x['dph'] or 0:.3f}/h)"
    if "spend" in s:
        yield f"spent on rentals: ${s['spend']['today']:.2f} today (UTC), ${s['spend']['total']:.2f} in all"


def run(args, m: Manager, command: list[str]) -> int:
    store, acc = m.store, Access(m)
    c = args.command
    if c == "home":
        print(store.home)
    elif c == "init":
        if store.pool.exists():
            raise Refused(f"{store.pool} is already there", 2)
        store.home.mkdir(parents=True, exist_ok=True)
        write_atomic(store.pool, json.dumps(EXAMPLE, indent=2) + "\n")
        print(f"wrote {store.pool}: its machines are examples, disabled; nothing is rented while vast's limits are 0")
    elif c == "import":
        r = import_workers(store, Path(args.file), args.skip_disabled, args.replace, args.dry_run)
        print(f"{'added' if r['written'] else 'would add'} to {r['pool']}: {', '.join(r['added']) or 'nothing'}"
              + (f"; left out (disabled): {', '.join(r['skipped'])}" if r["skipped"] else ""))
    elif c == "lease":
        workers = m.lease(args.request, args.holder, args.purpose, args.ttl, sweep=not args.no_sweep)
        show(dict(workers=workers), args.json, lambda v: (f"{w['lease']}  {w['name']}" for w in v["workers"]))
    elif c == "renew":
        show(dict(leases=m.renew(args.leases, args.ttl)), args.json, lambda v: (f"{l['lease']}  {l['name']}" for l in v["leases"]))
    elif c == "release":
        for i in m.release(args.leases):
            print(f"released {i}")
    elif c == "exec":
        return acc.exec(args.lease, command, args.timeout)
    elif c in ("push", "pull"):
        return getattr(acc, c)(args.lease, args.source, args.destination, args.recursive, args.timeout)
    elif c == "forward":
        try:
            local, remote = (int(x) for x in args.ports.split(":"))
        except ValueError:
            raise Refused("forward takes LOCAL:REMOTE, two port numbers", 2) from None
        print(acc.forward(args.lease, local, remote, [*LAUNCHER, "--home", str(store.home)]))
    elif c == "unforward":
        if not acc.unforward(args.lease, args.local):
            raise Refused(f"no forward of port {args.local} under that lease", 2)
    elif c == "forward-run":
        return acc.forward_run(args.lease, args.local, args.remote)
    elif c == "status":
        s = m.status()
        s["forwards"] = acc.forwards()
        show(s, args.json, status_lines)
    elif c == "machines":
        show(m.candidates(args.request), args.json,
             lambda v: [f"{w['name']}: {w['os']}{', busy' if w['busy'] else ''}" for w in v["workers"]]
             + (["a box may be rented for it"] if v["may_rent"] else []))
    elif c == "reap":
        for name in m.reap():
            print(f"destroyed {name}")
    elif c == "daemon":
        return run_daemon(m, args.once)
    elif c == "log":
        for e in read_lines(store.log)[-args.n:]:
            at = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(e.pop("at", 0)))
            print(at, e.pop("event", "?"), " ".join(f"{k}={v}" for k, v in e.items()))
    elif c == "vast":
        return run_vast(args, m)
    return 0


def run_vast(args, m: Manager) -> int:
    c = args.vast
    if c == "offers":
        offers = m.offers(one_group(args.request))
        show(offers, args.json, lambda v: [f"{o['id']}: {vast.describe(o)}" for o in v] or ["no offer passes the limits and filters"])
    elif c == "rent":
        r = m.rent(one_group(args.request), name=args.name)
        print(f"{r['name']}: instance {r['instance']}, ${r['dph']:.3f}/h; destroyed after "
              f"{m.pool.vast['idle_minutes']:g} minutes with no lease")
    elif c == "destroy":
        m.destroy(args.name, by="manual")
    elif c == "show":
        r = m.reconcile()
        s = m.status()
        show(dict(rented=s["rented"], **r), args.json, lambda v: (
            [f"{b['name']}: instance {b['instance']}, {b['status']}" for b in v["rented"]]
            + [f"{n}: gone from the account; written off" for n in v["gone"]]
            + [f"instance {i}: this pool's and no machine of it; destroyed" for i in v["orphans"]]
            + [f"not this pool's: instance {x['instance']} ({x['label']}, {x['status']}, ${x['dph'] or 0:.3f}/h)" for x in v["strays"]]
            or ["nothing rented"]))
    elif c == "ledger":
        now = m.clock()
        rows = vast.rentals(read_lines(m.store.ledger), now)
        show(dict(rentals=rows, spend=vast.spend(read_lines(m.store.ledger), now)), args.json, lambda v: (
            [f"{time.strftime('%Y-%m-%d %H:%M', time.localtime(r['start']))}  {r['name']} (instance {r['instance']}): "
             f"{(r['end'] - r['start']) / 3600:.2f} h at ${r['dph']:.3f}/h = ${(r['end'] - r['start']) / 3600 * r['dph']:.2f}"
             + (" (still rented)" if r["open"] else f" ({r.get('by')})") for r in v["rentals"]]
            + [f"${v['spend']['today']:.2f} today (UTC), ${v['spend']['total']:.2f} in all"]))
    return 0


def main(argv: list[str] | None = None, manager=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    command: list[str] = []
    if "--" in argv:
        k = argv.index("--")
        argv, command = argv[:k], argv[k + 1:]
    args = parser().parse_args(argv)
    if command and args.command != "exec":
        print("cantrip-workers: only exec takes a command after --", file=sys.stderr)
        return 2
    try:
        store = Store(Path(args.home) if args.home else None)
        m = manager or Manager(store, ensure_daemon=lambda: ensure_daemon(store))
        code = run(args, m, command)
        if args.command not in ("daemon", "forward-run", "exec", "push", "pull", "home", "init"):
            state = m.store.read_state()
            if state["rented"] or state["leases"] or (m.store.ledger.exists() and m.unsettled()):
                # Something has to be watching: a rented box, a lease that will expire, a rental not accounted for.
                m.ensure_daemon()
        return code
    except Refused as e:
        print(f"cantrip-workers: {e}", file=sys.stderr)
        return e.code
    except KeyboardInterrupt:
        return 130
    except Exception as e:  # noqa: BLE001 - said in a line, with a code of its own, never a traceback
        print(f"cantrip-workers: {type(e).__name__}: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
