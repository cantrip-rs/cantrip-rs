"""The worker manager itself (docs/workers.md): leases on the pool's machines, and the rented ones' whole life.

A lease is `{id, machine, holder, purpose, created, expires, ttl, cores, gpu, sweep}` in `state.json`: a machine held
whole (`cores` null) or a share of its cores, by a holder, until it expires - which a renewal puts off - or is
released. A machine is busy exactly when a live lease holds it: nothing is guessed from what runs on it. A lease that
ends has its processes on a Linux machine killed (`access.sweep_line`), unless it was taken with `sweep` off; the
calls made under it end themselves (`Access.watch`).

A rented machine is a vast.ai instance in `state.json`'s `rented`, a machine like the others while it is `ready`.
Renting is held to the pool's limits (`check_limits`); every rental and destruction is a line of `ledger.jsonl`; a
box with no live lease for `idle_minutes` is destroyed (`reap`), and so is every box once a budget is spent. The
daemon (`tick`) does that every minute and tells each box it is there (`heartbeat`); a box that stops hearing it
destroys itself (`vast.WATCHDOG`).
"""

from __future__ import annotations

import contextlib
import copy
import secrets
import sys
import time

from . import access, vast
from .pool import Group, Pool, fits, free, parse_request, worker_record
from .store import ConfigError, LeaseError, LimitError, Refused, Store, Unmet, append_line, read_lines


# How long a rental may sit between taking its place and having an instance (two calls of vast.ai's) before the
# process doing it is taken for dead, and how long after an intent an instance could still appear from it.
CREATING_GRACE = 600.0


def say(message: str):
    print(f"cantrip-workers: {message}", file=sys.stderr, flush=True)


class Manager:
    def __init__(self, store: Store | None = None, clock=time.time, sleep=time.sleep, remote=None, api=None, log=say,
                 ensure_daemon=None):
        self.store = store or Store()
        self.clock, self.sleep, self.log = clock, sleep, log
        self._pool: Pool | None = None
        # `remote(machine, line, timeout) -> (code, output)`: the tool's own errands on a machine.
        self.remote = remote or (lambda m, line, timeout=60.0: access.remote(self.pool, m, line, timeout))
        self._api = api
        # Called once a box is rented: something has to reap it and tell it the tool is there.
        self.ensure_daemon = ensure_daemon or (lambda: None)

    @property
    def pool(self) -> Pool:
        if self._pool is None:
            self._pool = Pool(self.store)
        return self._pool

    @property
    def api(self):
        if self._api is None:
            import os  # noqa: PLC0415

            key = os.environ.get("VAST_API_KEY")
            if not key:
                p = self.store.home / self.pool.vast["api_key_file"]
                key = p.read_text(encoding="utf-8").strip() if p.exists() else None
            if not key:
                raise ConfigError(f"no vast.ai API key: VAST_API_KEY, or the file {self.store.home / self.pool.vast['api_key_file']}")
            self._api = vast.Api(key, self.pool.vast["api_url"], sleep=self.sleep)
        return self._api

    def event(self, kind: str, **fields):
        append_line(self.store.log, dict(event=kind, at=round(self.clock(), 3), **fields))

    # --- Machines and leases --------------------------------------------------------------------------------------

    def machines(self, state: dict) -> dict[str, dict]:
        """Every machine there is now: the pool's, and each rented box that is ready."""
        out = dict(self.pool.machines)
        for name, box in state["rented"].items():
            if box.get("status") == "ready":
                out[name] = dict(name=name, os="linux", host=box["host"], port=box["port"], user="root",
                                 root=self.pool.vast["root"], key=self.pool.key, cores=box.get("cores"),
                                 ram_gb=box.get("ram_gb"), gpu=box.get("gpu"), rented=True,
                                 alias=f"cantrip-vast-{box['instance']}", worker=dict(self.pool.vast.get("worker") or {}))
        return out

    def live_lease(self, lease: dict) -> bool:
        return lease["expires"] > self.clock()

    def end(self, state: dict, lease: dict, why: str) -> dict:
        """A lease taken out of the state: logged, and its box idle from now if nothing else holds it."""
        state["leases"].pop(lease["id"], None)
        self.event("end", lease=lease["id"], machine=lease["machine"], holder=lease["holder"], why=why,
                   seconds=round(self.clock() - lease["created"], 3))
        box = state["rented"].get(lease["machine"])
        if box is not None and not any(l["machine"] == lease["machine"] for l in state["leases"].values()):
            # Idle from when the lease was over, whenever that is noticed.
            box["idle_since"] = min(self.clock(), lease["expires"])
        return lease

    def prune(self, state: dict) -> list[dict]:
        return [self.end(state, l, "expired") for l in list(state["leases"].values()) if not self.live_lease(l)]

    def sweep(self, ended: list[dict], machines: dict[str, dict]):
        """The processes of leases that ended, killed where they can be found: on Linux machines still there."""
        for lease in ended:
            m = machines.get(lease["machine"])
            if lease.get("sweep", True) and m is not None and m["os"] == "linux" and not (m.get("local") and sys.platform == "win32"):
                code, _ = self.remote(m, access.sweep_line(lease["id"]), 60.0)
                self.event("sweep", lease=lease["id"], machine=lease["machine"], code=code)

    def new_lease(self, state: dict, m: dict, g: Group, holder: str, purpose: str, ttl: float, sweep: bool) -> dict:
        now = self.clock()
        lease = dict(id="L" + secrets.token_hex(8), machine=m["name"], holder=holder, purpose=purpose, created=now,
                     expires=now + ttl, ttl=ttl, cores=g.cores, gpu=bool(g.gpu) or g.exclusive and bool(m.get("gpu")),
                     sweep=sweep)
        state["leases"][lease["id"]] = lease
        if m["name"] in state["rented"]:
            state["rented"][m["name"]]["idle_since"] = None
        return lease

    def lease_event(self, lease: dict):
        self.event("lease", lease=lease["id"], machine=lease["machine"], holder=lease["holder"], purpose=lease["purpose"],
                   cores=lease["cores"], ttl=lease["ttl"])

    def grant(self, state: dict, groups: list[Group], holder: str, purpose: str, ttl: float, sweep: bool):
        """Every lease the request can have now, written into `state`: (leases, the groups still short and by how
        many). A machine its holder already holds the same way is the same lease again, renewed - so a holder that
        was restarted finds its machines its own."""
        machines = self.machines(state)
        got: list[dict] = []
        short: list[tuple[Group, int]] = []

        def take(m: dict, g: Group) -> bool:
            held = [l for l in state["leases"].values() if l["machine"] == m["name"]]
            mine = next((l for l in held if l["holder"] == holder and l.get("cores") == g.cores), None)
            if mine is not None:
                if mine not in got:
                    mine.update(expires=self.clock() + ttl, ttl=ttl, purpose=purpose)
                    got.append(mine)
                    return True
                return False
            if not free(m, held, g):
                return False
            got.append(self.new_lease(state, m, g, holder, purpose, ttl, sweep))
            return True

        for g in groups:
            if g.names:
                for name in g.names:
                    m = machines.get(name)
                    if m is None:
                        raise ConfigError(f"no machine is named {name}")
                    if not fits(m, g):
                        if g.optional:
                            continue
                        raise Unmet(f"{name} is not what {g.text!r} asks for" + ("" if m.get("enabled", True) else " (it is disabled)"))
                    if not take(m, g) and not g.optional:
                        raise Unmet(f"{name} is busy: {self.holders(state, name)}")
            elif g.all:
                for m in machines.values():
                    if fits(m, g):
                        take(m, g)
            else:
                wanted = g.count
                def mine(m: dict) -> bool:
                    return any(l["machine"] == m["name"] and l["holder"] == holder and l.get("cores") == g.cores
                               for l in state["leases"].values())

                # What this holder already holds first (a holder that was restarted has its machines back, rented
                # ones too, before any other is taken), then this machine, the pool's own, and boxes already rented.
                for m in sorted((m for m in machines.values() if fits(m, g)),
                                key=lambda m: (not mine(m), not m.get("local"), bool(m.get("rented")), m["name"])):
                    if wanted and take(m, g):
                        wanted -= 1
                if wanted:
                    short.append((g, wanted))
        return got, short

    def holders(self, state: dict, name: str) -> str:
        return "; ".join(f"{l['holder']} ({l['purpose']})" for l in state["leases"].values() if l["machine"] == name) or "nobody"

    def lease(self, request: str, holder: str, purpose: str, ttl: float | None = None, sweep: bool = True) -> list[dict]:
        """The request's leases, all of them or none: `[{"lease": id, "name", "os", "root", ...}]`
        (`pool.worker_record`). A group no machine can serve now fails the request (`Unmet`), unless it is `optional`
        or says `rent` and the pool allows a rental, in which case a box is rented for it within the limits (a rental
        that fails fails the request, unless the group is `optional` too: then it goes without). The leases already
        granted are kept alive while a box is rented, which can take many minutes."""
        if not holder or not purpose:
            raise Refused("a lease names its holder and its purpose", 2)
        groups = parse_request(request)
        ttl = float(ttl or self.pool.lease_seconds)
        with self.store.lock():
            state = self.store.read_state()
            before = set(state["leases"])
            ended = self.prune(state)
            machines = self.machines(state)
            pruned = copy.deepcopy(state)
            refused = None
            try:
                got, short = self.grant(state, groups, holder, purpose, ttl, sweep)
                need = []
                for g, n in short:
                    if g.rent and self.pool.vast["auto_rent"]:
                        need.append((g, n))
                    elif g.optional:
                        continue
                    elif not g.rent:
                        raise Unmet(f"no free machine for {g.text!r}" + self.busy_note(pruned, g))
                    else:
                        raise Unmet(f"no free machine for {g.text!r}, and the pool does not rent on request (vast.auto_rent)")
            except Refused as e:
                refused = e
            # None of a request that is refused: what was pruned still is.
            self.store.write_state(pruned if refused else state)
            if not refused:
                for l in got:
                    if l["id"] not in before:
                        self.lease_event(l)
        self.sweep(ended, machines)
        if refused:
            raise refused
        try:
            for g, n in need:
                for _ in range(n):
                    try:
                        got.append(self.rent(g, lease=dict(holder=holder, purpose=purpose, ttl=ttl, sweep=sweep),
                                             waiting=lambda: self.touch([l["id"] for l in got]))["lease"])
                    except Refused as e:
                        if not g.optional:
                            raise
                        self.log(f"no box was rented for {g.text!r} ({e}); the request goes without")
                        break
        except BaseException:
            self.release([l["id"] for l in got if l["id"] not in before], why="its request could not be met")
            raise
        state = self.touch([l["id"] for l in got])
        machines = self.machines(state)
        return [worker_record(machines[l["machine"]], i) | dict(expires=l["expires"])
                for i, l in ((l["id"], state["leases"].get(l["id"])) for l in got) if l is not None and l["machine"] in machines]

    def touch(self, ids: list[str]) -> dict:
        """Each of these leases that is still live, its end put off from now (one that expired is not brought back,
        whether or not anything has pruned it yet). The state."""
        with self.store.lock():
            state = self.store.read_state()
            for i in ids:
                if i in state["leases"] and self.live_lease(state["leases"][i]):
                    state["leases"][i]["expires"] = self.clock() + state["leases"][i]["ttl"]
            self.store.write_state(state)
        return state

    def busy_note(self, state: dict, g: Group) -> str:
        busy = [m["name"] for m in self.machines(state).values() if fits(m, g)]
        return f" ({', '.join(f'{n}: {self.holders(state, n)}' for n in busy)})" if busy else " (none in the pool fits)"

    def candidates(self, request: str) -> dict:
        """What a request could be served by, nothing leased: the machines that fit, each with whether it is busy
        now, and whether a box might be rented for it."""
        groups = parse_request(request)
        state = self.store.read_state()
        machines = self.machines(state)
        out: dict[str, dict] = {}
        for g in groups:
            for name in g.names:
                if name not in machines:
                    raise ConfigError(f"no machine is named {name}")
            for m in machines.values():
                if fits(m, g) and m["name"] not in out:
                    held = [l for l in state["leases"].values() if l["machine"] == m["name"] and self.live_lease(l)]
                    out[m["name"]] = worker_record(m, None) | dict(busy=not free(m, held, g))
        return dict(workers=list(out.values()), may_rent=any(g.rent for g in groups) and bool(self.pool.vast["auto_rent"]))

    def renew(self, ids: list[str], ttl: float | None = None) -> list[dict]:
        """Each lease's end put off by its own length (or `ttl`). One that has ended is not brought back."""
        with self.store.lock():
            state = self.store.read_state()
            ended = self.prune(state)
            machines = self.machines(state)
            gone = [i for i in ids if i not in state["leases"]]
            out = []
            for i in ids:
                if i in state["leases"]:
                    l = state["leases"][i]
                    l["ttl"] = float(ttl or l["ttl"])
                    l["expires"] = self.clock() + l["ttl"]
                    out.append(dict(lease=i, name=l["machine"], expires=l["expires"]))
            self.store.write_state(state)
        self.sweep(ended, machines)
        if gone:
            raise LeaseError(f"no live lease {', '.join(gone)}" + (f" (renewed: {', '.join(o['lease'] for o in out)})" if out else ""))
        return out

    def release(self, ids: list[str], why: str = "released") -> list[str]:
        """Each lease ended now; one that is not there is passed over. The ids ended."""
        with self.store.lock():
            state = self.store.read_state()
            ended = self.prune(state)
            machines = self.machines(state)
            done = [self.end(state, state["leases"][i], why) for i in ids if i in state["leases"]]
            self.store.write_state(state)
        self.sweep(ended + done, machines)
        return [l["id"] for l in done]

    def resolve(self, lease_id: str) -> tuple[dict, dict]:
        """A live lease and its machine, or `LeaseError`."""
        state = self.store.read_state()
        lease = state["leases"].get(lease_id)
        if lease is None or not self.live_lease(lease):
            raise LeaseError(f"no live lease {lease_id}")
        m = self.machines(state).get(lease["machine"])
        if m is None:
            raise LeaseError(f"lease {lease_id}: its machine {lease['machine']} is gone")
        return lease, m

    def live(self, lease_id: str) -> bool:
        try:
            self.resolve(lease_id)
            return True
        except LeaseError:
            return False
        except (OSError, ValueError):
            return True  # a state file caught mid-replace says nothing either way; the next look decides

    def status(self) -> dict:
        state = self.store.read_state()
        now = self.clock()
        machines = self.machines(state)
        rows = []
        for m in machines.values():
            held = [l for l in state["leases"].values() if l["machine"] == m["name"] and self.live_lease(l)]
            rows.append(worker_record(m, None) | dict(
                enabled=m.get("enabled", True) is not False,
                leases=[dict(lease=l["id"], holder=l["holder"], purpose=l["purpose"], cores=l.get("cores"),
                             expires_in=round(l["expires"] - now)) for l in held]))
        boxes = [dict(name=n, instance=b.get("instance"), status=b.get("status"), dph=b.get("dph"),
                      hours=round((now - b["rented_at"]) / 3600, 2) if b.get("rented_at") else None,
                      idle_minutes=round((now - b["idle_since"]) / 60, 1) if b.get("idle_since") else None,
                      why=b.get("why"), watchdog=b.get("watchdog"))
                 for n, b in state["rented"].items()]
        out = dict(machines=rows, rented=boxes, strays=state.get("strays") or [])
        if self.store.ledger.exists() and self.unsettled():
            # A rental the ledger has open with no box, or an intent nothing settled: the daemon checks the account
            # for it at every round, and says here when it cannot.
            out["unsettled"] = dict(error=(state.get("account_error") or {}).get("error"))
        if boxes or self.store.ledger.exists():
            out["spend"] = {k: round(v, 2) for k, v in vast.spend(read_lines(self.store.ledger), now).items()}
        return out

    # --- Rented machines ------------------------------------------------------------------------------------------
    #
    # Every rental is known three ways, so that none is ever forgotten while it bills: an `intent` line in the ledger
    # with the label the instance will carry (written before vast.ai is asked for anything), the `rent` line once the
    # instance has an id, and the box in the state. A label is one rental's alone (`cantrip-<name>-<random>`), so an
    # instance on the account carrying a label this ledger issued is this pool's, whatever else was lost.

    def check_limits(self, state: dict, name: str | None = None, dph: float | None = None):
        """Whether a box may be rented, or `LimitError`: `name` is its own place in the state if it has one already,
        `dph` its price once an offer is chosen (until then the cap, which is what its place reserves). Every other
        box counts against the budgets at the price it reserves or pays."""
        cfg = self.pool.vast
        if not (cfg["max_dph"] > 0 and cfg["max_instances"] > 0 and cfg["daily_budget"] > 0 and cfg["total_budget"] > 0):
            raise LimitError("renting is off: the pool's vast section sets max_dph, max_instances, daily_budget and total_budget")
        others = [b for n, b in state["rented"].items() if n != name]
        if len(others) >= cfg["max_instances"]:
            raise LimitError(f"{len(others)} box(es) rented already: the limit is {cfg['max_instances']}")
        mine = float(cfg["max_dph"]) if dph is None else dph
        if mine > cfg["max_dph"]:
            raise LimitError(f"${mine:.3f}/h is over the cap of ${cfg['max_dph']:.2f}/h")
        spent = vast.spend(read_lines(self.store.ledger), self.clock())
        ahead = (mine + sum(float(b.get("dph") or cfg["max_dph"]) for b in others)) * float(cfg["min_rental_hours"])
        if spent["today"] + ahead > cfg["daily_budget"]:
            raise LimitError(f"the daily budget of ${cfg['daily_budget']:.2f} does not cover it: ${spent['today']:.2f} spent today")
        if spent["total"] + ahead > cfg["total_budget"]:
            raise LimitError(f"the total budget of ${cfg['total_budget']:.2f} does not cover it: ${spent['total']:.2f} spent")

    def public_key(self) -> str:
        key = self.pool.key_path(dict(key=self.pool.key))
        if key is None:
            raise ConfigError("renting needs the pool's key: a rented box is reached with it")
        pub = key.with_name(key.name + ".pub")
        if not pub.exists():
            raise ConfigError(f"renting needs the public key {pub}: it is put on the box")
        return pub.read_text(encoding="utf-8").strip()

    def offers(self, group: Group | None = None) -> list[dict]:
        cfg = self.pool.vast
        return vast.choose(self.api.search(vast.offer_query(cfg, group)), cfg, group)

    def rent(self, group: Group | None = None, name: str | None = None, lease: dict | None = None, waiting=None) -> dict:
        """A box rented within the limits, started, checked and ready: `{name, instance, dph, lease}` (the lease when
        `lease` gives its holder, purpose and length: the box is the holder's from the moment it is ready). A box
        that does not come up, cannot be reached or cannot destroy itself is destroyed, and that is an error.
        `waiting` is called while the box starts (a request's other leases are kept alive by it)."""
        cfg = self.pool.vast
        self.check_limits(self.store.read_state())
        public_key = self.public_key()
        api = self.api
        with self.store.lock():
            state = self.store.read_state()
            self.check_limits(state)
            if name is None:
                n = 1
                while f"vast{n}" in state["rented"] or f"vast{n}" in self.pool.machines:
                    n += 1
                name = f"vast{n}"
            elif name in state["rented"] or name in self.pool.machines:
                raise Refused(f"a machine is already named {name}", 2)
            label = f"cantrip-{name}-{secrets.token_hex(4)}"
            # Its place taken, at the most it may cost, before anything is asked of vast.ai: two rentals at once can
            # neither both pass the count nor both fit the same budget.
            state["rented"][name] = dict(status="creating", at=self.clock(), label=label, dph=float(cfg["max_dph"]))
            self.store.write_state(state)
        instance, box, asked = None, None, False
        try:
            # Something watches from the moment there is a place: should this process die anywhere past here, the
            # daemon finds what it left (the place, and any instance under its label) and destroys it.
            self.ensure_daemon()
            append_line(self.store.ledger, dict(event="intent", at=self.clock(), name=name, label=label))
            chosen = self.offers(group)
            if not chosen:
                raise Unmet(f"no vast.ai offer under ${cfg['max_dph']:.2f}/h passes the pool's filters"
                            + (f" and {group.text!r}" if group is not None and group.text else ""))
            offer = None
            for o in chosen[:5]:
                with self.store.lock():
                    state = self.store.read_state()
                    place = state["rented"].get(name, {})
                    if place.get("label") != label or place.get("status") != "creating":
                        raise Refused(f"{name} was taken down while it was being rented")
                    self.check_limits(state, name, float(o["dph_total"]))
                    state["rented"][name]["dph"] = float(o["dph_total"])
                    self.store.write_state(state)
                self.log(f"renting {name}: offer {o['id']}: {vast.describe(o)}")
                asked = True
                instance = api.create(o["id"], vast.create_body(cfg, label))
                if instance is not None:
                    offer = o
                    break
                asked = False  # the offer was gone: nothing was made of it
                self.log(f"offer {o['id']} is gone; the next")
            if offer is None:
                raise Unmet("every vast.ai offer tried was gone")
            now = self.clock()
            box = dict(status="starting", instance=instance, label=label, offer=offer["id"], dph=float(offer["dph_total"]),
                       rented_at=now, cores=int(float(offer.get("cpu_cores_effective") or 0)) or None,
                       ram_gb=round(float(offer.get("cpu_ram") or 0) / 1000) or None,
                       gpu=offer.get("gpu_name") if offer.get("num_gpus") else None, cpu=offer.get("cpu_name"))
            # The ledger first: from here the box costs money, whatever happens to this process.
            append_line(self.store.ledger, dict(event="rent", at=now, instance=instance, name=name, label=label,
                                                offer=offer["id"], dph=box["dph"], machine=vast.describe(offer)))
            self.place_box(name, label, box)
            self.start(name, box, public_key, waiting)
            with self.store.lock():
                state = self.store.read_state()
                box = state["rented"].get(name)
                if box is None or box.get("instance") != instance or box.get("status") != "starting":
                    raise Refused(f"{name} was taken down while it started")
                box.update(status="ready", idle_since=self.clock(), last_heartbeat=self.clock())
                made = None
                if lease is not None:
                    made = self.new_lease(state, self.machines(state)[name], group or Group(), lease["holder"],
                                          lease["purpose"], lease["ttl"], lease.get("sweep", True))
                self.store.write_state(state)
        except BaseException as e:
            self.abandon(name, label, instance, asked, e)
            raise
        if made is not None:
            self.lease_event(made)
        self.log(f"{name} is ready: instance {instance}, ${box['dph']:.3f}/h")
        return dict(name=name, instance=instance, dph=box["dph"], lease=made)

    def abandon(self, name: str, label: str, instance: int | None, asked: bool, why: BaseException):
        """A rental that failed, undone without trusting anything but what is known for certain. With an instance:
        it is destroyed by its id (the state may not hold it), or left marked for the reaper. Without one: if a
        create was asked and its answer never came, an instance may exist all the same - whatever on the account
        carries this rental's label is destroyed, and the intent stays open for `reconcile` to look again."""
        if instance is not None:
            self.log(f"{name} did not come up ({why}): destroying it")
            try:
                if not self.api.destroy(instance):
                    raise vast.VastError(f"vast.ai did not destroy instance {instance}")
                self.closed(name, instance, "failed-start")
                self.drop_place(name, label)  # should the state never have taken the instance
            except Refused as again:
                # Still rented: the reaper destroys it at its next look, and every look until it is gone (and should
                # the state not take even that, `reconcile` finds the ledger's open rental).
                self.log(f"{name} was not destroyed ({again})")
                with contextlib.suppress(OSError, BlockingIOError):
                    with self.store.lock():
                        state = self.store.read_state()
                        if state["rented"].get(name, {}).get("label") == label:
                            self.condemn(state, name, "failed-start")
                            self.store.write_state(state)
            return
        if asked:
            self.ensure_daemon()  # which looks again, should the instance show up later
            self.log(f"{name}: the create's answer was lost ({why}): looking for its instance")
            try:
                self.settle(label)
            except Refused as again:
                self.log(f"{name}: the account was not checked ({again}); checked again by the daemon")
        else:
            # Nothing was asked of vast.ai that could have made an instance: the intent is closed (and if even that
            # cannot be written, `reconcile` closes it once it is old).
            with contextlib.suppress(OSError):
                append_line(self.store.ledger, dict(event="abandon", at=self.clock(), name=name, label=label))
        self.drop_place(name, label)

    def drop_place(self, name: str, label: str):
        """A rental's place in the state given up, if it still holds no instance."""
        with contextlib.suppress(OSError, BlockingIOError):
            with self.store.lock():
                state = self.store.read_state()
                if state["rented"].get(name, {}).get("label") == label and state["rented"][name].get("instance") is None:
                    state["rented"].pop(name)
                    self.store.write_state(state)

    def settle(self, label: str, there: list[dict] | None = None) -> list[int]:
        """Every instance on the account that carries a label this pool issued, taken into the ledger if it is not
        there and destroyed. The instances destroyed."""
        out = []
        for x in (self.api.instances() if there is None else there):
            if x.get("label") != label:
                continue
            instance = int(x["id"])
            ledger = read_lines(self.store.ledger)
            if not any(e.get("event") == "rent" and e.get("instance") == instance for e in ledger):
                intent = next((e for e in ledger if e.get("event") == "intent" and e.get("label") == label), {})
                append_line(self.store.ledger, dict(
                    event="rent", at=float(x.get("start_date") or intent.get("at") or self.clock()), instance=instance,
                    name=intent.get("name"), label=label, dph=float(x.get("dph_total") or self.pool.vast["max_dph"]),
                    machine="found on the account under a label this pool issued"))
            self.log(f"instance {instance} ({label}) is this pool's and no machine of it: destroying it")
            if not self.api.destroy(instance):
                raise vast.VastError(f"vast.ai did not destroy instance {instance} ({label}): it is still rented")
            self.closed(None, instance, "orphan")
            out.append(instance)
        return out

    def place_box(self, name: str, label: str, box: dict):
        """A new instance put in the place its rental took - only if the place is still that rental's and still
        waiting for it: one taken down meanwhile (condemned, or given up and its name taken by another rental) is not
        brought back, and the instance is the caller's to destroy."""
        with self.store.lock():
            state = self.store.read_state()
            place = state["rented"].get(name, {})
            if place.get("label") != label or place.get("status") != "creating":
                raise Refused(f"{name} was taken down while it was being rented")
            state["rented"][name] = dict(box)
            self.store.write_state(state)

    def update_box(self, name: str, fields: dict):
        with self.store.lock():
            state = self.store.read_state()
            if name in state["rented"]:
                state["rented"][name].update(fields)
            self.store.write_state(state)

    def start(self, name: str, box: dict, public_key: str, waiting=None):
        """Waits for a new instance to run, puts the key on it, and takes it only once it answers over SSH and its
        watchdog is armed - within `start_minutes`, or it is an error (and the caller destroys it)."""
        cfg = self.pool.vast
        deadline = box["rented_at"] + float(cfg["start_minutes"]) * 60
        keyed = self.api.attach_key(box["instance"], public_key)
        where = None
        while True:
            if waiting is not None:
                waiting()
            shown = self.api.show(box["instance"])
            if shown is None:
                raise Refused(f"instance {box['instance']} is gone")
            status = shown.get("actual_status")
            if status in vast.DEAD:
                raise Refused(f"instance {box['instance']} is {status}: {shown.get('status_msg') or ''}".strip())
            if status == "running":
                if not keyed:
                    keyed = self.api.attach_key(box["instance"], public_key)
                where = vast.address(shown)
                if where is not None:
                    host, port = where
                    m = dict(name=name, os="linux", host=host, port=port, user="root", key=self.pool.key,
                             alias=f"cantrip-vast-{box['instance']}")
                    code, out = self.remote(m, f"cat {vast.STATUS} 2>/dev/null; touch {vast.HEARTBEAT}", 60.0)
                    word = out.strip().splitlines()[-1] if out.strip() else ""
                    if code == 0 and word.startswith("armed"):
                        self.update_box(name, dict(host=host, port=port))
                        return
                    if code == 0 and word.startswith("unarmed"):
                        raise Refused(f"its watchdog cannot destroy it ({word}): a box that cannot stop its own bill is not used")
            if self.clock() >= deadline:
                raise Refused(f"not running and reachable {cfg['start_minutes']:g} minutes after it was created "
                              f"(last: {status or 'no status'}{'' if where else ', no address'})")
            self.sleep(20.0)

    def condemn(self, state: dict, name: str, why: str):
        """A box marked for destruction in the state: no machine any more (nothing can lease it, its leases end
        now), told nothing further (its watchdog takes over if the destruction keeps failing), and destroyed at every
        look until vast.ai says it is gone."""
        box = state["rented"][name]
        if box.get("status") != "destroying":
            box.update(status="destroying", why=why)
        for l in [l for l in state["leases"].values() if l["machine"] == name]:
            self.end(state, l, f"its machine is being destroyed ({why})")

    def destroy(self, name: str, by: str = "manual") -> dict:
        """A rented box destroyed, its leases ended and its cost written to the ledger. It is condemned first, and
        stays in the state until vast.ai says it is gone, so a destruction that failed is tried again by the reaper."""
        with self.store.lock():
            state = self.store.read_state()
            box = state["rented"].get(name)
            if box is None:
                raise Refused(f"no rented machine is named {name}", 2)
            self.condemn(state, name, by)
            by = box["why"]
            self.store.write_state(state)
        if box.get("instance") is None:
            # A rental that never got as far as an instance it knew of: one may exist under its label all the same.
            if box.get("label"):
                self.settle(box["label"])
            with self.store.lock():
                state = self.store.read_state()
                if state["rented"].get(name, {}).get("label") == box.get("label"):
                    state["rented"].pop(name)
                    self.store.write_state(state)
            return {}
        if not self.api.destroy(box["instance"]):
            raise vast.VastError(f"vast.ai did not destroy instance {box['instance']} ({name}): it is still rented")
        return self.closed(name, box["instance"], by)

    def closed(self, name: str | None, instance: int, by: str, ended: float | None = None) -> dict:
        """An instance that is gone, taken out of the books: its box out of the state and its leases ended, and its
        rental closed in the ledger - from the ledger's own `rent` line, so it does not depend on the state having
        held the box."""
        now = self.clock()
        with self.store.lock():
            state = self.store.read_state()
            for n, box in list(state["rented"].items()):
                if box.get("instance") == instance:
                    name = name or n
                    state["rented"].pop(n)
                    for l in [l for l in state["leases"].values() if l["machine"] == n]:
                        self.end(state, l, f"its machine was destroyed ({by})")
            self.store.write_state(state)
        ledger = read_lines(self.store.ledger)
        rent = next((e for e in ledger if e.get("event") == "rent" and e.get("instance") == instance), None)
        if rent is None or any(e.get("event") == "destroy" and e.get("instance") == instance for e in ledger):
            return {}
        end = min(now, ended) if ended is not None else now
        hours = max(0.0, end - float(rent["at"])) / 3600
        entry = dict(event="destroy", at=now, ended=end, instance=instance, name=name or rent.get("name"), by=by,
                     hours=round(hours, 4), dph=rent["dph"], cost=round(hours * float(rent["dph"]), 4))
        append_line(self.store.ledger, entry)
        self.log(f"{entry['name']}: instance {instance} destroyed ({by}) after {hours:.2f} h, about ${hours * float(rent['dph']):.2f}")
        return entry

    def heartbeat(self):
        """Each ready box told the tool is there (its watchdog's file touched), and asked what its watchdog last
        wrote: a self-destruction vast.ai refused is shown by `status`. A box being destroyed is told nothing, so if
        its destruction keeps failing here its own watchdog ends it."""
        state = self.store.read_state()
        machines = self.machines(state)
        for name, box in state["rented"].items():
            if box.get("status") == "ready":
                code, out = self.remote(machines[name], f"touch {vast.HEARTBEAT}; tail -n 1 {vast.WATCHDOG_LOG} 2>/dev/null", 30.0)
                if code == 0:
                    self.update_box(name, dict(last_heartbeat=self.clock(), watchdog=out.strip()[-300:] or None))

    def reap(self) -> list[str]:
        """Destroys what should not be rented any longer: every box once a budget is spent; a box no live lease has
        held for `idle_minutes`; one whose start was abandoned; one whose destruction failed before. What is decided
        is decided and marked under the state's lock (`condemn`), so a lease cannot be granted on a box between the
        decision and the destruction. The names destroyed."""
        cfg = self.pool.vast
        now = self.clock()
        spent = vast.spend(read_lines(self.store.ledger), now)
        over = ("the daily budget is spent" if spent["today"] >= cfg["daily_budget"] else
                "the total budget is spent" if spent["total"] >= cfg["total_budget"] else None)
        condemned = []
        with self.store.lock():
            state = self.store.read_state()
            ended = self.prune(state)
            machines = self.machines(state)
            for name, box in state["rented"].items():
                status, why = box.get("status"), None
                leased = any(l["machine"] == name for l in state["leases"].values())
                if status == "destroying":
                    why = box.get("why") or "manual"
                elif status == "ready":
                    if not leased and box.get("idle_since") is None:
                        box["idle_since"] = now
                    if over:
                        why = "budget"
                        self.log(f"{name}: {over}")
                    elif not leased and now - box["idle_since"] >= float(cfg["idle_minutes"]) * 60:
                        why = "idle"
                elif status == "creating":
                    if now - box.get("at", now) > CREATING_GRACE:
                        why = "failed-start"  # the process that was renting it is gone
                elif now - box.get("rented_at", now) > (float(cfg["start_minutes"]) + 5) * 60:
                    why = "failed-start"  # the process that was starting it is gone
                if why:
                    self.condemn(state, name, why)
                    condemned.append(name)
            self.store.write_state(state)
        self.sweep(ended, machines)
        out = []
        for name in condemned:
            try:
                self.destroy(name)
                out.append(name)
            except Refused as e:
                self.log(f"{name} not destroyed ({e}); tried again at the next look")
        return out

    def unsettled(self) -> bool:
        """Whether the ledger holds anything the state does not account for: a rental still open with no box, or an
        intent that became neither a rental nor nothing. `reconcile` settles them."""
        ledger = read_lines(self.store.ledger)
        state = self.store.read_state()
        held = {b.get("instance") for b in state["rented"].values()}
        labels = {b.get("label") for b in state["rented"].values()}
        return (any(r["open"] and r["instance"] not in held and r["label"] not in labels for r in vast.rentals(ledger, self.clock()))
                or any(i["label"] not in labels for i in vast.open_intents(ledger)))

    def reconcile(self) -> dict:
        """The books against the account, so that nothing this pool rented bills unseen and nothing gone is counted:

        - a box the account's list lacks, and that vast.ai says is not there when asked for it by id, is destroyed
          once more to be sure and written off (its watchdog destroyed it, or vast.ai did; its cost counted to when
          it was last heard from plus the watchdog's wait, at most to now);
        - an instance carrying a label this pool issued that no box holds is this pool's, forgotten (a create whose
          answer was lost, a process killed between the create and the books): it is destroyed;
        - a rental the ledger has open with no box is closed: destroyed if it is there, written off if it is not;
        - an intent that made nothing, once it is old enough that nothing can still come of it, is closed;
        - the account's other `cantrip-` instances are named (`strays`), logged, and never destroyed: another
          machine's pool may share the account."""
        cfg = self.pool.vast
        now = self.clock()
        listed = self.api.instances()
        there = {int(i["id"]): i for i in listed}
        gone = []
        for name, box in list(self.store.read_state()["rented"].items()):
            instance = box.get("instance")
            if instance is None or instance in there or self.api.show(instance) is not None:
                continue
            with contextlib.suppress(Refused):
                self.api.destroy(instance)
            last = box.get("last_heartbeat") or box.get("rented_at") or now
            self.closed(name, instance, "gone", ended=last + float(cfg["watchdog_minutes"]) * 60 + 60)
            gone.append(name)
        ledger = read_lines(self.store.ledger)
        issued = vast.issued(ledger)

        def accounted(instance: int, label: str | None) -> bool:
            """Whether the state, read now, has a box for the instance, or the place of a rental under way that
            carries its label (between its create and the books it is no orphan). Asked just before each decision
            to destroy, and after the ledger was read: a rental takes its place before it writes a line, so whatever
            the ledger showed, the state read after it shows the place or the box - unless there is none."""
            with self.store.lock():
                boxes = self.store.read_state()["rented"].values()
            return any(b.get("instance") == instance or (label is not None and b.get("label") == label and b.get("status") == "creating")
                       for b in boxes)

        orphans = []
        for instance, x in there.items():
            if x.get("label") in issued and not accounted(instance, x["label"]):
                orphans += self.settle(x["label"], [x])
        for r in vast.rentals(ledger, now):
            if not r["open"] or r["instance"] in orphans or accounted(r["instance"], r["label"]):
                continue
            if r["instance"] in there or self.api.show(r["instance"]) is not None:
                self.log(f"instance {r['instance']} is rented in the ledger and no machine of this pool: destroying it")
                if not self.api.destroy(r["instance"]):
                    raise vast.VastError(f"vast.ai did not destroy instance {r['instance']}: it is still rented")
                self.closed(None, r["instance"], "orphan")
                orphans.append(r["instance"])
            else:
                self.closed(None, r["instance"], "gone")
        carried = {x.get("label") for x in there.values()}
        renting = {b.get("label") for b in self.store.read_state()["rented"].values() if b.get("status") == "creating"}
        for i in vast.open_intents(read_lines(self.store.ledger)):
            if i["label"] not in renting and i["label"] not in carried and now - float(i["at"]) > CREATING_GRACE:
                append_line(self.store.ledger, dict(event="abandon", at=now, name=i.get("name"), label=i["label"]))
        held = {b.get("instance") for b in self.store.read_state()["rented"].values()}
        strays = [dict(instance=i, label=x.get("label"), status=x.get("actual_status"), dph=x.get("dph_total"))
                  for i, x in there.items() if i not in held and i not in orphans and x.get("label") not in issued
                  and str(x.get("label") or "").startswith("cantrip-")]
        with self.store.lock():
            state = self.store.read_state()
            state["strays"] = strays
            state.pop("account_error", None)
            self.store.write_state(state)
        for s in strays:
            self.log(f"not this pool's, left alone: instance {s['instance']} ({s['label']}, {s['status']}, ${s['dph'] or 0:.3f}/h)")
        return dict(gone=gone, orphans=orphans, strays=strays)

    def reload_pool(self):
        """The pool as its file says it now; a file that is wrong this moment (half written, a typo) changes nothing:
        the last good one stands."""
        try:
            self._pool = Pool(self.store)
        except (Refused, OSError, ValueError, TypeError, AttributeError) as e:
            self.log(f"the pool's file is not usable ({e})" + ("; the last good one stands" if self._pool is not None else ""))

    def tick(self, reconcile: bool = False) -> bool:
        """One round of the daemon: leases that expired ended and swept, boxes told the tool is there, the idle and
        the over-budget ones destroyed, and (when asked, and at once while the ledger holds something unaccounted
        for) the books checked against the account. Whether anything is left to watch: a rented box, a live lease,
        or something unsettled."""
        self.reload_pool()
        if self._pool is None:
            # No pool was ever usable here: nothing can be done, and there is something to watch only if the state
            # says so (a daemon started where there is no pool at all ends at once).
            state = self.store.read_state()
            return bool(state["rented"] or state["leases"])
        self.heartbeat()
        self.reap()
        unsettled = self.unsettled()
        if unsettled or (reconcile and self.store.read_state()["rented"]):
            try:
                self.reconcile()
            except Refused as e:
                self.log(f"the account was not checked ({e})")
                # Said where it is seen (`status`), not only in the daemon's log: while this lasts, what the ledger
                # holds unaccounted for stays so.
                with self.store.lock():
                    state = self.store.read_state()
                    state["account_error"] = dict(at=self.clock(), error=str(e))
                    self.store.write_state(state)
            unsettled = self.unsettled()
        state = self.store.read_state()
        return bool(state["rented"] or state["leases"] or unsettled)

    def watching(self) -> bool:
        """Whether there is anything for a daemon to watch, as the files say now."""
        state = self.store.read_state()
        return bool(state["rented"] or state["leases"] or (self.store.ledger.exists() and self.unsettled()))
