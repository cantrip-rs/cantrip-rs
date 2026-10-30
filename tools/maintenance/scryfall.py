"""Scryfall's bulk data, cached once a day for every worktree of the repository: each card's current Oracle text and
its official rulings, by name.

    python tools/maintenance/scryfall.py "Card Name"     # each face's cost, type line, stats and Oracle text; rulings
    python tools/maintenance/scryfall.py --text "regex"  # every printed card whose Oracle text matches (any face,
                                                         # case-insensitive), with the matching line: whether a
                                                         # wording is printed on a second card, implemented or not
    python tools/maintenance/scryfall.py --refresh       # the coordinator only, from the main checkout: download
                                                         # today's bulk files and rebuild the index

    import scryfall
    scryfall.card("Card Name")   # bulk data, then Scryfall's exact-name API if absent; normalized card or None
                                 # {"name", "layout", "faces": [{"name", "mana_cost", "type_line", "power", "toughness",
                                 #  "loyalty", "defense", "color_indicator", "oracle_text"}], "rulings": [...]} or None
    scryfall.oracle_cards()      # every Oracle Cards object, for whole-pool reports (ability-index.py rank)

The bulk files go to target/scryfall in the main checkout (a worktree's git common directory), shared by the pool's
slots. Only `--refresh` writes there: it downloads today's files (keeping the last ones if Scryfall can't be reached)
and rebuilds by-name.json. It is the coordinator's, run from the main checkout before agents start, and refused inside a
pool slot, so agents started together never race to write the shared files. Every other use only reads the newest files
there; an index older than them is rebuilt in memory, and with none at all `card` returns None and says why on stderr.
"""

import datetime
import gzip
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HEADERS = {"User-Agent": "cantrip-maintenance/1.0", "Accept": "application/json"}
SKIPPED_LAYOUTS = {"token", "double_faced_token", "emblem", "art_series", "vanguard", "scheme", "planar"}
# What by-name.json keeps of each face; FORMAT changes whenever it does, so an older cache is rebuilt.
FACE_KEYS = ("mana_cost", "type_line", "power", "toughness", "loyalty", "defense", "color_indicator")
FORMAT = 2


def _main_checkout():
    """The repository's main working tree, which a worktree shares its .git directory with."""
    try:
        common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
                                capture_output=True, text=True, check=True, cwd=os.path.dirname(os.path.abspath(__file__))).stdout.strip()
        if common:
            return os.path.dirname(os.path.normpath(common))
    except (OSError, subprocess.CalledProcessError):
        pass
    return os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))


CACHE = os.path.join(_main_checkout(), "target", "scryfall")


def _newest(kind):
    """The newest bulk file of a kind there is, or None."""
    if not os.path.isdir(CACHE):
        return None
    files = sorted(f for f in os.listdir(CACHE) if f.startswith(kind + "-") and f.endswith(".jsonl.gz"))
    return os.path.join(CACHE, files[-1]) if files else None


def _download(kind):
    """Today's bulk file of a kind, downloaded if it isn't there yet; the files it replaces are removed. If Scryfall
    can't be reached, the newest one there is serves. For `--refresh` alone."""
    os.makedirs(CACHE, exist_ok=True)
    today = os.path.join(CACHE, f"{kind}-{datetime.date.today():%Y%m%d}.jsonl.gz")
    if not os.path.exists(today):
        try:
            meta = json.load(urllib.request.urlopen(urllib.request.Request(
                f"https://api.scryfall.com/bulk-data/{kind}", headers=HEADERS), timeout=30))
            print(f"downloading {meta['jsonl_download_uri']}", file=sys.stderr)
            with urllib.request.urlopen(urllib.request.Request(meta["jsonl_download_uri"], headers=HEADERS), timeout=300) as r:
                data = r.read()
            partial = f"{today}.{os.getpid()}.part"
            with open(partial, "wb") as f:
                f.write(data)
            os.replace(partial, today)
        except (OSError, ValueError, KeyError) as e:
            print(f"scryfall: could not download {kind} ({e}); keeping the files there are", file=sys.stderr)
            return _newest(kind)
    for old in os.listdir(CACHE):
        stale = old.startswith(kind + "-") and os.path.join(CACHE, old) != today
        if stale or (old.startswith(kind + "-") and old.endswith(".part")):
            os.remove(os.path.join(CACHE, old))
    return today


def bulk(kind):
    """The path of the newest bulk file of a kind ("oracle-cards", "rulings") there is, or None. Never downloads:
    `--refresh` does, for the coordinator."""
    found = _newest(kind)
    if found is None:
        print(f"scryfall: no {kind} file in {CACHE}; the coordinator runs scryfall.py --refresh from the main checkout",
              file=sys.stderr)
    return found


def _lines(path):
    with gzip.open(path) as f:
        for line in f:
            if line.strip():
                yield json.loads(line)


def oracle_cards():
    """Every card object of the Oracle Cards file but tokens, emblems and the like."""
    path = bulk("oracle-cards")
    if path is None:
        return
    for d in _lines(path):
        if d.get("layout") not in SKIPPED_LAYOUTS:
            yield d


_index = None


def _load(write=False):
    """{lowercase name: card} for every card and face name, rulings attached, cached beside the bulk files. An index
    older than the bulk files is rebuilt; it is written back only by `--refresh` (`write`)."""
    global _index
    if _index is not None:
        return _index
    oracle, rulings = bulk("oracle-cards"), bulk("rulings")
    if oracle is None:
        _index = {}
        return _index
    stamp = f"v{FORMAT}+" + os.path.basename(oracle) + "+" + (os.path.basename(rulings) if rulings else "")
    built = os.path.join(CACHE, "by-name.json")
    try:
        with open(built, encoding="utf-8") as f:
            data = json.load(f)
        if data.get("stamp") == stamp:
            _index = data["cards"]
            return _index
    except (OSError, ValueError):
        pass
    by_oracle = {}
    if rulings:
        for r in _lines(rulings):
            by_oracle.setdefault(r["oracle_id"], []).append({"date": r["published_at"], "text": r["comment"]})
    # A paper card wins a name over a playtest, Alchemy or un-set card of the same name.
    lesser = {"funny": 1, "playtest": 2, "memorabilia": 3, "alchemy": 4}
    cards, rank = {}, {}
    for d in _lines(oracle):
        if d.get("layout") in SKIPPED_LAYOUTS:
            continue
        faces = d.get("card_faces") or [d]
        card = _normalize(d, by_oracle.get(d.get("oracle_id") or faces[0].get("oracle_id"), []))
        order = lesser.get(d.get("set_type"), 0)
        for name in {d["name"]} | {f["name"] for f in faces}:
            key = name.lower()
            if key not in cards or order < rank[key]:
                cards[key], rank[key] = card, order
    if write:
        partial = f"{built}.{os.getpid()}.part"
        with open(partial, "w", encoding="utf-8") as f:
            json.dump({"stamp": stamp, "cards": cards}, f)
        os.replace(partial, built)
    else:
        print("scryfall: by-name.json is older than the bulk files; rebuilt in memory (scryfall.py --refresh writes it)",
              file=sys.stderr)
    _index = cards
    return _index


def refresh():
    """Downloads today's bulk files and rebuilds by-name.json. The coordinator's, from the main checkout: refused inside a
    pool slot (.claude/worktrees), so agents started together never write the shared cache at once."""
    here = os.path.normcase(os.path.abspath(os.getcwd()))
    if os.path.normcase(os.path.join(".claude", "worktrees")) in here:
        sys.exit("scryfall.py --refresh is the coordinator's, run from the main checkout, not from a pool slot")
    global _index
    _download("oracle-cards")
    _download("rulings")
    for old in os.listdir(CACHE):
        if old.startswith("by-name.json") and old.endswith(".part"):
            os.remove(os.path.join(CACHE, old))
    _index = None
    print(f"{len(_load(write=True))} names indexed in {CACHE}")


def card(name):
    return _load().get(name.lower()) or _direct_card(name)


def _normalize(d, rulings):
    faces = d.get("card_faces") or [d]
    return {
        "name": d["name"],
        "layout": d.get("layout", ""),
        "faces": [{key: f.get(key, d.get(key, "") if key in ("mana_cost", "type_line") and len(faces) == 1 else "")
                   for key in FACE_KEYS} | {"name": f["name"], "oracle_text": f.get("oracle_text", "")}
                  for f in faces],
        "rulings": sorted(rulings, key=lambda r: r["date"]),
    }


def _api(url):
    """One direct Scryfall API request."""
    time.sleep(0.1)
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=HEADERS), timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        sys.exit(f"Scryfall: {error}")
    except urllib.error.URLError as error:
        sys.exit(f"Scryfall: {error}")


def _direct_card(name):
    d = _api("https://api.scryfall.com/cards/named?" + urllib.parse.urlencode({"exact": name}))
    if d is None:
        return None
    rulings = _api(d["rulings_uri"])
    return _normalize(d, [{"date": r["published_at"], "text": r["comment"]}
                          for r in rulings["data"]] if rulings else [])


def search_text(pattern):
    """(name, matching line) for every printed card (paper, not playtest, Alchemy or un-set) whose Oracle text matches
    the case-insensitive regex, on any face; each card once, sorted by name."""
    import re
    rx = re.compile(pattern, re.IGNORECASE)
    hits = {}
    for d in oracle_cards():
        if d.get("set_type") in ("funny", "playtest", "memorabilia", "alchemy") or d["name"] in hits:
            continue
        for f in d.get("card_faces") or [d]:
            line = next((l for l in f.get("oracle_text", "").split("\n") if rx.search(l)), None)
            if line is not None:
                hits[d["name"]] = line
                break
    return sorted(hits.items())


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    if sys.argv[1] == "--refresh":
        refresh()
        return
    if sys.argv[1] == "--text":
        if len(sys.argv) < 3:
            sys.exit("--text needs a regex")
        hits = search_text(sys.argv[2])
        for name, line in hits:
            print(f"{name}: {line}")
        print(f"{len(hits)} printed card{'' if len(hits) == 1 else 's'}")
        return
    found = card(sys.argv[1])
    if found is None:
        sys.exit(f"no card named {sys.argv[1]!r} on Scryfall")
    if found.get("layout") not in (None, "", "normal"):
        print(f"layout: {found['layout']}\n")
    for face in found["faces"]:
        stats = "/".join(str(face[k]) for k in ("power", "toughness") if face.get(k))
        stats = stats or (f"loyalty {face['loyalty']}" if face.get("loyalty") else f"defense {face['defense']}" if face.get("defense") else "")
        header = "  ".join(x for x in (face["name"], face.get("mana_cost", ""), face.get("type_line", ""), stats) if x)
        indicator = f"\ncolor indicator: {''.join(face['color_indicator'])}" if face.get("color_indicator") else ""
        print(f"{header}{indicator}\n{face['oracle_text']}\n")
    for r in found["rulings"]:
        print(f"{r['date']}  {r['text']}")


if __name__ == "__main__":
    main()
