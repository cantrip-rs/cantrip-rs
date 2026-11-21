"""The pool's configuration (`pool.json`, docs/workers.md) and what a request for machines means.

A machine is `{"name", "os", "root", ...}`: a remote one has `host`, `user` and optionally `port` and its own `key`;
`"local": true` is this machine itself (no SSH). `cores`, `ram_gb` and `gpu` say what it has, for requests that ask;
`worker` holds whatever its users want handed back with a lease (the training driver's `lanes`, `server_args`, ...),
which the tool passes on and never reads. `enabled: false` leaves a machine out.

A request is groups joined by `+`, each a comma list (docs/workers.md, "Requests"):

    worker1,worker2             those machines, each whole
    all                         every enabled remote machine that is free, each whole
    os=linux,gpu,cores=32       any one machine that fits: Linux, a GPU, 32 of its cores (a share; without `cores`
                                the whole machine); `count=N` for N of them, `gpu=5090` for a model, `ram=64` GB,
                                `remote` or `local` for where, `rent` to rent one when none fits, `optional` to take
                                none without failing
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from .store import ConfigError, Store, read_json

NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")
FLAGS = ("all", "gpu", "rent", "remote", "local", "optional")
KEYS = ("os", "gpu", "cores", "ram", "count")

# The limits a rental is held to (`pool.json`'s `vast`); nothing is rented unless the file says so.
VAST_DEFAULTS = dict(
    auto_rent=False,          # whether a lease request with `rent` may rent; `vast rent` by hand does not need it
    max_dph=0.0,              # the most a box may cost an hour, storage included
    daily_budget=0.0,         # dollars a UTC day, over every box
    total_budget=0.0,         # dollars in all, over the whole ledger
    max_instances=0,          # boxes at once
    min_rental_hours=1.0,     # a box is rented only if the budgets cover this long at its price
    idle_minutes=15,          # a box with no live lease this long is destroyed
    watchdog_minutes=20,      # a box that has not heard from the tool this long destroys itself
    heartbeat_seconds=60,     # how often the daemon tells each box it is there
    start_minutes=30,         # a box not running and reachable this long after it was created is destroyed
    image="nvidia/cuda:12.8.1-cudnn-runtime-ubuntu24.04",
    disk_gb=100,
    root="/workspace/compute",
    filters={},               # the offers asked for: {"gpu_name": "RTX 5090", "gpu_max_power": {"gte": 550}, ...}
    cpu_names=[],             # regular expressions; an offer's physical CPU has to match one (none: any)
    order="price",            # or "price_per_core": dollars an hour per effective core
    worker={},                # handed back with a rented box's lease, as a machine's `worker`
    api_key_file="vast-api-key",
    api_url="https://console.vast.ai",
)


@dataclass
class Group:
    """One group of a request."""

    names: list[str] = field(default_factory=list)
    all: bool = False
    count: int = 1
    os: str | None = None
    gpu: bool | str = False
    cores: int | None = None
    ram: float | None = None
    where: str | None = None   # "local", "remote" or anywhere
    rent: bool = False
    optional: bool = False
    text: str = ""

    @property
    def exclusive(self) -> bool:
        return self.cores is None


def parse_request(text: str) -> list[Group]:
    groups = []
    for part in text.split("+"):
        part = part.strip()
        if not part:
            raise ConfigError(f"an empty group in the request {text!r}")
        g = Group(text=part)
        counted = False
        for token in (t.strip() for t in part.split(",")):
            if not token:
                continue
            if "=" in token:
                k, v = (x.strip() for x in token.split("=", 1))
                if k not in KEYS:
                    raise ConfigError(f"request {part!r}: {k}= is not one of {', '.join(KEYS)}")
                try:
                    if k == "os":
                        if v not in ("windows", "linux"):
                            raise ValueError
                        g.os = v
                    elif k == "gpu":
                        g.gpu = v
                    elif k == "cores":
                        g.cores = int(v)
                        if g.cores < 1:
                            raise ValueError
                    elif k == "ram":
                        g.ram = float(v)
                    else:
                        g.count, counted = int(v), True
                        if g.count < 1:
                            raise ValueError
                except ValueError:
                    raise ConfigError(f"request {part!r}: {token} is not a value {k} takes") from None
            elif token in FLAGS:
                if token == "all":
                    g.all = True
                elif token == "gpu":
                    g.gpu = g.gpu or True
                elif token in ("remote", "local"):
                    g.where = token
                else:
                    setattr(g, token, True)
            elif NAME.match(token):
                g.names.append(token)
            else:
                raise ConfigError(f"request {part!r}: {token!r} is neither a machine's name nor a term")
        if g.names and (g.all or counted or g.rent or g.where):
            raise ConfigError(f"request {part!r}: machines by name take only cores=, gpu, os=, ram= and optional")
        if g.all and (counted or g.rent or g.cores):
            raise ConfigError(f"request {part!r}: all takes whole machines, every one that fits; no count, cores or rent")
        if g.all and g.where == "local":
            raise ConfigError(f"request {part!r}: all is the remote machines")
        groups.append(g)
    if not groups:
        raise ConfigError("an empty request")
    return groups


class Pool:
    """`pool.json`, checked."""

    def __init__(self, store: Store):
        self.store = store
        try:
            d = read_json(store.pool)
        except ValueError as e:
            raise ConfigError(f"{store.pool} is not JSON ({e})") from None
        if d is None:
            raise ConfigError(f"no pool: {store.pool} is missing (`cantrip-workers init`, or `import` an old workers file)")
        if not isinstance(d, dict):
            raise ConfigError(f"{store.pool}: not a JSON object")
        self.raw = d
        self.lease_seconds = float(d.get("lease_seconds", 900))
        self.key = d.get("key")
        self.machines: dict[str, dict] = {}
        for m in d.get("machines", []):
            m = self.checked(m)
            if m["name"] in self.machines:
                raise ConfigError(f"{store.pool}: two machines are named {m['name']}")
            self.machines[m["name"]] = m
        unknown = set(d.get("vast", {})) - set(VAST_DEFAULTS)
        if unknown:
            raise ConfigError(f"{store.pool}: vast has no setting {', '.join(sorted(unknown))}")
        self.vast = {**VAST_DEFAULTS, **d.get("vast", {})}
        self.check_vast()

    def check_vast(self):
        """The rental settings, each of its kind and in its range: a typo here is money."""
        v, where = self.vast, f"{self.store.pool}: vast"

        def number(key: str, least: float, strict: bool = False):
            x = v[key]
            if isinstance(x, bool) or not isinstance(x, (int, float)) or x != x or x < least or (strict and x <= least):
                raise ConfigError(f"{where}.{key} is a number {'above' if strict else 'of at least'} {least:g}: not {x!r}")

        if not isinstance(v["auto_rent"], bool):
            raise ConfigError(f"{where}.auto_rent is true or false: not {v['auto_rent']!r}")
        for key in ("max_dph", "daily_budget", "total_budget"):
            number(key, 0)
        number("max_instances", 0)
        if not isinstance(v["max_instances"], int):
            raise ConfigError(f"{where}.max_instances is a whole number: not {v['max_instances']!r}")
        number("min_rental_hours", 0, strict=True)
        number("idle_minutes", 1)
        number("start_minutes", 1)
        number("heartbeat_seconds", 10)
        number("watchdog_minutes", 1)
        number("disk_gb", 0, strict=True)
        # The watchdog has to outlast a few heartbeats that did not get through, or it destroys boxes in use.
        if v["watchdog_minutes"] * 60 < 3 * v["heartbeat_seconds"] + 120:
            raise ConfigError(f"{where}.watchdog_minutes ({v['watchdog_minutes']:g}) is too short for a heartbeat every "
                              f"{v['heartbeat_seconds']:g} s: at least three heartbeats and two minutes")
        if v["order"] not in ("price", "price_per_core"):
            raise ConfigError(f"{where}.order is price or price_per_core: not {v['order']!r}")
        for key, kind in (("filters", dict), ("worker", dict), ("cpu_names", list), ("image", str), ("root", str),
                          ("api_key_file", str), ("api_url", str)):
            if not isinstance(v[key], kind) or (kind is str and not v[key]):
                raise ConfigError(f"{where}.{key} is {'an object' if kind is dict else 'a list' if kind is list else 'text'}: not {v[key]!r}")
        if not v["api_url"].startswith("https://"):
            raise ConfigError(f"{where}.api_url is an https address")
        for pattern in v["cpu_names"]:
            try:
                re.compile(str(pattern))
            except re.error as e:
                raise ConfigError(f"{where}.cpu_names: {pattern!r} is not a regular expression ({e})") from None

    def checked(self, m: dict) -> dict:
        where = str(self.store.pool)
        if not isinstance(m, dict) or not NAME.match(str(m.get("name", ""))):
            raise ConfigError(f"{where}: a machine needs a name of letters, digits, '.', '_' and '-': {m}")
        m = dict(m)
        if m.get("os") not in ("windows", "linux"):
            raise ConfigError(f"{where}: machine {m['name']}: os is windows or linux")
        needs = ("root",) if m.get("local") else ("host", "user", "root")
        for k in needs:
            if not m.get(k):
                raise ConfigError(f"{where}: machine {m['name']} has no {k}")
        if not isinstance(m.get("worker", {}), dict):
            raise ConfigError(f"{where}: machine {m['name']}: worker is an object")
        if not m.get("local") and not (m.get("key") or self.key):
            raise ConfigError(f"{where}: machine {m['name']} has no key, and the pool has none")
        m.setdefault("key", self.key)
        return m

    def key_path(self, m: dict) -> Path | None:
        if not m.get("key"):
            return None
        p = Path(m["key"]).expanduser()
        return p if p.is_absolute() else self.store.home / p


def fits(m: dict, g: Group) -> bool:
    """Whether a machine is one the group asks for, whoever holds it now."""
    if m.get("enabled", True) is False:
        return False
    if g.names and m["name"] not in g.names:
        return False
    if g.os and m["os"] != g.os:
        return False
    if g.where == "local" and not m.get("local"):
        return False
    if (g.where == "remote" or g.all) and m.get("local"):
        return False
    if g.gpu:
        if not m.get("gpu"):
            return False
        if g.gpu is not True and str(g.gpu).lower().replace("_", " ") not in str(m["gpu"]).lower():
            return False
    if g.cores is not None and (m.get("cores") or 0) < g.cores:
        return False
    if g.ram is not None and (m.get("ram_gb") or 0) < g.ram:
        return False
    return True


def free(m: dict, held: list[dict], g: Group) -> bool:
    """Whether the leases a machine holds (`held`) leave room for what the group asks of it: a whole machine needs it
    empty; a share needs no one to hold it whole, its cores, and its GPU if it asks for one."""
    if not held:
        return True
    if g.exclusive or any(l.get("cores") is None for l in held):
        return False
    if sum(l["cores"] for l in held) + g.cores > (m.get("cores") or 0):
        return False
    return not (g.gpu and any(l.get("gpu") for l in held))


def worker_record(m: dict, lease: str | None) -> dict:
    """What a lease's holder is told of its machine: its name, kind, root and size, and its `worker` settings - never
    its address, user or key (docs/workers.md, "Access")."""
    out = dict(m.get("worker") or {})
    out.update(name=m["name"], os=m["os"], root=m["root"])
    for k in ("cores", "ram_gb", "gpu"):
        if m.get(k) is not None:
            out[k] = m[k]
    if m.get("local"):
        out["local"] = True
    if m.get("rented"):
        out["rented"] = True
    if lease is not None:
        out["lease"] = lease
    return out
