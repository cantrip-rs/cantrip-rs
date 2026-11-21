"""vast.ai, as the worker manager uses it (docs/workers.md, "Rented machines"): its REST API over urllib - offers
searched, an instance created, shown, given a key and destroyed - the choice among offers, the watchdog a rented box
runs, and what the cost ledger adds up to.

The API as `vast-ai/vast-cli` calls it: `POST /api/v0/bundles/` (the query object the body), `PUT /api/v0/asks/ID/`,
`GET /api/v0/instances/ID/`, `GET /api/v1/instances/` (paged), `POST /api/v0/instances/ID/ssh/`,
`DELETE /api/v0/instances/ID/`; the key a bearer token. Filters are the API's own fields in its own units (`cpu_ram`
in MB, `disk_space` in GB, `gpu_name` with spaces).
"""

from __future__ import annotations

import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request

from .store import Refused

OPS = {"eq": lambda a, b: a == b, "neq": lambda a, b: a != b, "gt": lambda a, b: a > b, "gte": lambda a, b: a >= b,
       "lt": lambda a, b: a < b, "lte": lambda a, b: a <= b, "in": lambda a, b: a in b, "notin": lambda a, b: a not in b}

# An instance in one of these never comes to run (vast's own note): it is destroyed and another tried.
DEAD = ("exited", "unknown", "offline")

STATE_DIR = "/var/lib/cantrip-workers"
HEARTBEAT = f"{STATE_DIR}/heartbeat"
STATUS = f"{STATE_DIR}/status"
WATCHDOG_LOG = f"{STATE_DIR}/watchdog.log"


class VastError(Refused):
    code = 74


def http(method: str, url: str, headers: dict, body: bytes | None, timeout: float = 60.0):
    """One request: (status, parsed JSON or text)."""
    req = urllib.request.Request(url, data=body, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:  # noqa: S310 - the configured API, https
            status, text = r.status, r.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        status, text = e.code, e.read().decode("utf-8", errors="replace")
    except (urllib.error.URLError, OSError) as e:
        raise VastError(f"vast.ai: {e}") from e
    try:
        return status, json.loads(text) if text.strip() else None
    except ValueError:
        return status, text


class Api:
    """The calls. `request` is `http` unless a test hands in its own; a 429 is tried again, a little later each time."""

    def __init__(self, key: str, url: str = "https://console.vast.ai", request=http, sleep=time.sleep):
        self.key, self.url, self.request, self.sleep = key, url.rstrip("/"), request, sleep

    def call(self, method: str, path: str, body=None, query: dict | None = None):
        url = self.url + path + ("?" + urllib.parse.urlencode(query) if query else "")
        headers = {"Authorization": f"Bearer {self.key}", "Accept": "application/json"}
        data = None
        if body is not None:
            data, headers["Content-Type"] = json.dumps(body).encode("utf-8"), "application/json"
        delay = 0.5
        for _ in range(6):
            status, answer = self.request(method, url, headers, data)
            if status != 429:
                return status, answer
            self.sleep(delay)
            delay *= 2
        return status, answer

    def search(self, query: dict) -> list[dict]:
        status, r = self.call("POST", "/api/v0/bundles/", query)
        if status != 200 or not isinstance(r, dict):
            raise VastError(f"vast.ai: searching offers: {status} {str(r)[:300]}")
        return [o for o in r.get("offers") or [] if not o.get("rented")]

    def create(self, offer: int, body: dict) -> int | None:
        """The new instance's id; None when the offer is gone (another is tried). Gone is vast.ai's `no_such_ask`,
        which its create-instance reference gives two ways: 404 with `"error": "invalid_args"` and
        `"msg": "error 404/3603: no_such_ask Instance type by id N is not available."`, and 410 with
        `"error": "no_such_ask"`. Nothing was made of such an offer. Any other answer is an error, and one after
        which an instance may exist: a bare 404 says nothing of the offer (a wrong key gets one too)."""
        status, r = self.call("PUT", f"/api/v0/asks/{int(offer)}/", body)
        if status == 200 and isinstance(r, dict) and r.get("success") and r.get("new_contract"):
            return int(r["new_contract"])
        if isinstance(r, dict) and r.get("success") is False and (
                r.get("error") == "no_such_ask" or "no_such_ask" in str(r.get("msg") or "")):
            return None
        raise VastError(f"vast.ai: creating an instance of offer {offer}: {status} {str(r)[:300]}")

    def show(self, instance: int) -> dict | None:
        """The instance, or None when vast.ai says there is none: `{"instances": null}`, or its own not-found. Any
        other answer is an error, never "gone": a 404 is also what a key that is wrong gets."""
        status, r = self.call("GET", f"/api/v0/instances/{int(instance)}/", query={"owner": "me"})
        if isinstance(r, dict) and r.get("error") == "not_found":
            return None
        if status != 200 or not isinstance(r, dict) or "instances" not in r:
            raise VastError(f"vast.ai: showing instance {instance}: {status} {str(r)[:300]}")
        return r["instances"] or None

    def instances(self) -> list[dict]:
        out, token = [], None
        for _ in range(100):
            query = {"select_filters": "{}", "order_by": json.dumps([{"col": "id", "dir": "asc"}]), "limit": "25"}
            if token:
                query["after_token"] = token
            status, r = self.call("GET", "/api/v1/instances/", query=query)
            if status != 200 or not isinstance(r, dict):
                raise VastError(f"vast.ai: listing instances: {status} {str(r)[:300]}")
            out += r.get("instances") or []
            token = r.get("next_token")
            if not token:
                break
        return out

    def attach_key(self, instance: int, public_key: str) -> bool:
        status, r = self.call("POST", f"/api/v0/instances/{int(instance)}/ssh/", {"ssh_key": public_key})
        return status == 200 and isinstance(r, dict) and bool(r.get("success"))

    def destroy(self, instance: int) -> bool:
        """True once the instance is gone: destroyed now, or vast.ai's own not-found for it
        (`{"success": false, "error": "not_found"}`). A bare 404 is not that: a key that is wrong gets one too."""
        status, r = self.call("DELETE", f"/api/v0/instances/{int(instance)}/", {})
        if status == 200 and isinstance(r, dict) and r.get("success"):
            return True
        return isinstance(r, dict) and r.get("error") == "not_found"


# --- Offers -------------------------------------------------------------------------------------------------------


def condition(value) -> dict:
    """A filter's value as the API takes it: `{"gte": 550}` as it is, a bare value as `{"eq": value}`."""
    if isinstance(value, dict):
        bad = set(value) - set(OPS)
        if bad:
            raise Refused(f"a vast filter's operators are {', '.join(OPS)}: not {', '.join(sorted(bad))}", 73)
        return value
    return {"eq": value}


def offer_query(cfg: dict, group=None) -> dict:
    """What is asked of vast.ai: verified, rentable, on-demand machines under the price cap that pass the pool's
    filters and, for a request, what it asks (its cores of the offer's effective ones, its RAM, its GPU)."""
    q = {"verified": {"eq": True}, "external": {"eq": False}, "rentable": {"eq": True}, "rented": {"eq": False}}
    for k, v in (cfg.get("filters") or {}).items():
        v = condition(v)
        q[k] = {op: x.replace("_", " ") if k == "gpu_name" and isinstance(x, str) else x for op, x in v.items()}
    q["dph_total"] = {"lte": float(cfg["max_dph"])}
    if group is not None:
        if group.cores:
            q["cpu_cores_effective"] = {"gte": max(group.cores, (q.get("cpu_cores_effective") or {}).get("gte", 0))}
        if group.ram:
            q["cpu_ram"] = {"gte": max(group.ram * 1000, (q.get("cpu_ram") or {}).get("gte", 0))}
        if group.gpu:
            q["num_gpus"] = q.get("num_gpus") or {"gte": 1}
        if group.gpu not in (False, True) and "gpu_name" not in q:
            # Asked of vast.ai itself, which matches a model's whole name ("RTX 5090"): checked only here, a model
            # that is not among the cheapest offers returned would be missed.
            q["gpu_name"] = {"eq": str(group.gpu).replace("_", " ")}
    # "on-demand" is what vast-cli's `search offers` sends for its default type (the API's documentation lists
    # "ondemand"; both answer with on-demand offers).
    q.update(order=[["dph_total", "asc"]], type="on-demand", limit=64, allocated_storage=float(cfg["disk_gb"]))
    return q


def passes(offer: dict, query: dict) -> bool:
    """The query, checked here too: an answer is not trusted to have applied it."""
    for k, cond in query.items():
        if not isinstance(cond, dict) or k not in offer or offer[k] is None:
            continue
        for op, want in cond.items():
            try:
                if not OPS[op](offer[k], want):
                    return False
            except TypeError:
                return False
    return True


def choose(offers: list[dict], cfg: dict, group=None) -> list[dict]:
    """The offers a box may be rented from, best first: those that pass the query, cost no more than the cap, whose
    physical CPU is one of `cpu_names` (when given) and whose GPU is the model a request names; by price, or by price
    per effective core."""
    query = offer_query(cfg, group)
    patterns = [re.compile(p, re.IGNORECASE) for p in cfg.get("cpu_names") or []]
    out = []
    for o in offers:
        if o.get("dph_total") is None or o["dph_total"] > float(cfg["max_dph"]) or not passes(o, query):
            continue
        if patterns and not any(p.search(str(o.get("cpu_name") or "")) for p in patterns):
            continue
        if group is not None and group.gpu not in (False, True):
            if str(group.gpu).lower().replace("_", " ") not in str(o.get("gpu_name") or "").lower():
                continue
        out.append(o)
    if cfg.get("order") == "price_per_core":
        out.sort(key=lambda o: (o["dph_total"] / max(1.0, float(o.get("cpu_cores_effective") or 1)), o["dph_total"]))
    else:
        out.sort(key=lambda o: o["dph_total"])
    return out


def describe(o: dict) -> str:
    return (f"${o['dph_total']:.3f}/h, {o.get('num_gpus') or 0} x {o.get('gpu_name')}, {o.get('cpu_name')}, "
            f"{float(o.get('cpu_cores_effective') or 0):.0f} cores, {float(o.get('cpu_ram') or 0) / 1000:.0f} GB, "
            f"{o.get('geolocation')}")


def address(instance: dict) -> tuple[str, int] | None:
    """Where an instance's SSH is: its direct port when it has one, else vast's proxy."""
    ports = instance.get("ports")
    direct = (ports.get("22/tcp") or [{}])[0].get("HostPort") if isinstance(ports, dict) else None
    if direct and instance.get("public_ipaddr"):
        return str(instance["public_ipaddr"]).strip(), int(direct)
    if instance.get("ssh_host") and instance.get("ssh_port"):
        return str(instance["ssh_host"]), int(instance["ssh_port"])
    return None


# --- The watchdog -------------------------------------------------------------------------------------------------

# A rented box's `onstart`: a loop that destroys the box's own instance once the heartbeat file has not been touched
# for LIMIT seconds - the tool that rented it is down, asleep or cut off, and nothing else would stop the bill. It
# uses the instance's own key and id, which vast.ai puts in the container's environment (`CONTAINER_API_KEY`,
# `CONTAINER_ID`: docs.vast.ai, "Docker environment"); `status` says whether that key answers for this instance, which
# the tool reads before it takes the box (`armed`), so a box that could not destroy itself is never used. Only a
# destroyed instance stops billing: a stopped one still pays for its storage.
WATCHDOG = r"""#!/bin/bash
D=/var/lib/cantrip-workers; mkdir -p $D; touch $D/heartbeat
[ -f $D/watchdog.pid ] && kill "$(cat $D/watchdog.pid)" 2>/dev/null
command -v curl >/dev/null || command -v python3 >/dev/null || { apt-get -qq update; apt-get -qq install -y curl; } >/dev/null 2>&1
cat > $D/watchdog.sh <<'EOF'
#!/bin/bash
D=/var/lib/cantrip-workers; LIMIT=@LIMIT@; URL="@URL@/api/v0/instances/$CONTAINER_ID/"
api() {
  if command -v curl >/dev/null; then
    curl -s --connect-timeout 20 --max-time 60 -o /dev/null -w '%{http_code}' -X "$1" -H "Authorization: Bearer $CONTAINER_API_KEY" -H 'Content-Type: application/json' ${2:+-d "$2"} "$URL"
  else
    M="$1" B="$2" U="$URL" python3 -c '
import os, urllib.request as u, urllib.error as e
r = u.Request(os.environ["U"], data=(os.environ["B"].encode() or None), method=os.environ["M"], headers={"Authorization": "Bearer " + os.environ["CONTAINER_API_KEY"], "Content-Type": "application/json"})
try: print(u.urlopen(r, timeout=60).status)
except e.HTTPError as x: print(x.code)
except Exception: print(0)'
  fi
}
if [ -z "$CONTAINER_API_KEY" ] || [ -z "$CONTAINER_ID" ]; then echo "unarmed: no instance key in the environment" > $D/status
else
  for i in 1 2 3 4 5 6 7 8 9 10 11 12; do code=$(api GET); [ "$code" = 200 ] && break; sleep 10; done
  if [ "$code" = 200 ]; then echo "armed $CONTAINER_ID" > $D/status; else echo "unarmed: the instance key answered $code" > $D/status; fi
fi
while sleep 30; do
  age=$(( $(date +%s) - $(stat -c %Y $D/heartbeat 2>/dev/null || echo 0) ))
  [ "$age" -le "$LIMIT" ] && continue
  echo "$(date -Is) no heartbeat for $age s: destroying instance $CONTAINER_ID: $(api DELETE '{}')" >> $D/watchdog.log
done
EOF
chmod +x $D/watchdog.sh
nohup setsid $D/watchdog.sh >/dev/null 2>&1 &
echo $! > $D/watchdog.pid
"""


def watchdog_script(cfg: dict) -> str:
    text = WATCHDOG.replace("@LIMIT@", str(int(float(cfg["watchdog_minutes"]) * 60))).replace("@URL@", cfg["api_url"].rstrip("/"))
    if len(text) > 4048:  # vast.ai's limit on `onstart`
        raise Refused("the watchdog script is longer than vast.ai's onstart takes", 73)
    return text


def create_body(cfg: dict, label: str) -> dict:
    """An instance as `vastai create instance --ssh --direct` asks for one, with the watchdog its `onstart` and the
    rental's label. The runtype is the string vast-cli sends for `--ssh --direct` ("ssh_direc ssh_proxy", spelled so
    in its `get_runtype`); the API's documentation lists "ssh_direct"."""
    return {"client_id": "me", "image": cfg["image"], "env": {}, "price": None, "disk": float(cfg["disk_gb"]),
            "label": label, "extra": None, "onstart": watchdog_script(cfg), "image_login": None,
            "python_utf8": False, "lang_utf8": False, "use_jupyter_lab": False, "jupyter_dir": None, "force": False,
            "cancel_unavail": False, "template_hash_id": None, "user": None, "runtype": "ssh_direc ssh_proxy"}


# --- The cost ledger ----------------------------------------------------------------------------------------------


def issued(ledger: list[dict]) -> set[str]:
    """Every label this pool ever gave a rental: an instance carrying one is this pool's."""
    return {e["label"] for e in ledger if e.get("event") == "intent" and e.get("label")}


def open_intents(ledger: list[dict]) -> list[dict]:
    """The intents that became neither a rental nor (`abandon`) nothing: an instance may yet exist under their label."""
    done = {e.get("label") for e in ledger if e.get("event") in ("rent", "abandon")}
    return [e for e in ledger if e.get("event") == "intent" and e.get("label") not in done]


def rentals(ledger: list[dict], now: float) -> list[dict]:
    """Every rental the ledger holds: `{instance, name, label, dph, start, end, open}`, an open one's end now."""
    out: dict[int, dict] = {}
    for e in ledger:
        if e.get("event") == "rent":
            out[e["instance"]] = dict(instance=e["instance"], name=e.get("name"), label=e.get("label"), dph=float(e["dph"]),
                                      start=float(e["at"]), end=now, open=True)
        elif e.get("event") == "destroy" and e.get("instance") in out:
            out[e["instance"]].update(end=float(e.get("ended", e["at"])), open=False, by=e.get("by"))
    return list(out.values())


def spend(ledger: list[dict], now: float) -> dict:
    """Dollars spent: in all, and since the UTC day began (a rental across midnight counts its hours of today)."""
    day = now - now % 86400
    total = today = 0.0
    for r in rentals(ledger, now):
        total += max(0.0, r["end"] - r["start"]) / 3600 * r["dph"]
        today += max(0.0, r["end"] - max(r["start"], day)) / 3600 * r["dph"]
    return dict(total=total, today=today)
