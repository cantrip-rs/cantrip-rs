"""The cards Commander plays, from EDHREC: its top cards, its top commanders, and each of those commanders' top cards.

    python tools/maintenance/edhrec-scrape.py [--commanders 100] [--top-pages 1] [--window year|month|week]

Reads EDHREC's own JSON (json.edhrec.com, what edhrec.com's pages are drawn from), politely and cached in
target/format-cache the way format-scrape.py does, so a second run asks for nothing (clear the cache to scrape again):

    top          edhrec.com/top: the most-played cards of the window, 100 a page (--top-pages of them); a card's share
                 is the percent of the decks that could play it (its colour identity allows it) that do.
    commanders   edhrec.com/commanders: the most-built commanders of the window; the first --commanders are read.
    each         edhrec.com/commanders/<commander>: the cards that commander's decks play most, by type, with the
                 decks that play each.

The window is EDHREC's tab: `year` is "Past 2 Years", the pages' default. Every name is checked against Scryfall and
written as the card files spell it, and the result is docs/formats/commander.json, best-played first, in the shape
format-scrape.py writes - the list `card-issues.py sync commander` makes issues from. For each card:

    top          its share on edhrec.com/top (0 when it is not on the pages read)
    commanders   the percent of the read commanders' decks that play it (a commander's own decks count for it)
    decks        those decks, and `among` the number of the read commanders whose page lists it
    score        the larger of the two shares: a card is as played as its best showing

Basic lands are left out, as format-scrape.py leaves them.
"""

import argparse
import datetime
import importlib.util
import json
import os
import sys

sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")
sys.dont_write_bytecode = True  # no __pycache__ left in tools/maintenance

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
OUT = os.path.join(ROOT, "docs", "formats")
BASE = "https://json.edhrec.com/pages/"
WINDOWS = {"year": "Past 2 Years", "month": "Past Month", "week": "Past Week"}


def load(name):
    """A sibling script (their names have hyphens) as a module."""
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


scrape = load("format-scrape")


def page(path):
    return json.loads(scrape.fetch(BASE + path))


def listed(path, pages):
    """The cardviews of a paged EDHREC list: the first page, then its `more` pages, up to `pages` of them."""
    data = page(path)["container"]["json_dict"]["cardlists"][0]
    views, more = list(data["cardviews"]), data.get("more")
    for _ in range(pages - 1):
        if not more:
            break
        data = page(more)
        views += data["cardviews"]
        more = data.get("more")
    return views


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--commanders", type=int, default=100, help="how many of the top commanders are read")
    parser.add_argument("--top-pages", type=int, default=1, help="pages of edhrec.com/top read, 100 cards each")
    parser.add_argument("--window", choices=sorted(WINDOWS), default="year", help="EDHREC's time window")
    args = parser.parse_args()

    top = {}
    for view in listed(f"top/{args.window}.json", args.top_pages):
        if view.get("potential_decks"):
            top[view["name"]] = 100 * view["num_decks"] / view["potential_decks"]
    top_read = len(top)
    print(f"edhrec.com/top: {top_read} cards", file=sys.stderr)

    commanders = listed(f"commanders/{args.window}.json", (args.commanders + 99) // 100)[:args.commanders]
    decks, among = {}, {}
    total = 0
    for index, commander in enumerate(commanders, 1):
        built = commander["num_decks"]
        total += built
        decks[commander["name"]] = decks.get(commander["name"], 0) + built
        data = page(f"commanders/{commander['sanitized']}.json")
        seen = set()
        for cardlist in data["container"]["json_dict"]["cardlists"]:
            for view in cardlist["cardviews"]:
                if view["name"] in seen:  # a card in "Top Cards" and again under its type counts once
                    continue
                seen.add(view["name"])
                decks[view["name"]] = decks.get(view["name"], 0) + view["num_decks"]
                among[view["name"]] = among.get(view["name"], 0) + 1
        print(f"[{index}/{len(commanders)}] {commander['name']}: {len(seen)} cards", file=sys.stderr)
    if not top or not total:
        sys.exit("EDHREC returned nothing; its pages may have changed")

    card_list = load("card-list")
    names = sorted(set(top) | set(decks))
    found, unknown = card_list.resolve(names)
    # A pair of partner commanders is one name on EDHREC ("Frodo, Adventurous Hobbit // Sam, Loyal Attendant"), which
    # Scryfall does not know as one card: each of the two is played as often as the pair.
    pairs = {name: name.split(" // ") for name in unknown if " // " in name}
    if pairs:
        halves, unknown = card_list.resolve(sorted({half for both in pairs.values() for half in both}))
        for name, both in pairs.items():
            for half in both:
                if half in halves:
                    found[half] = halves[half]
                    if name in top:
                        top[half] = max(top.get(half, 0.0), top[name])
                    decks[half] = decks.get(half, 0) + decks.get(name, 0)
                    among[half] = among.get(half, 0) + among.get(name, 0)
        unknown += [name for name in names if name not in found and name not in pairs]
    omenpaths = card_list.cubecobra()
    cards = {}
    for name, card in found.items():
        if card.get("layout") in card_list.NOT_CARDS:
            continue
        spelled = card_list.card_name(card, omenpaths)
        if spelled in scrape.BASICS:
            continue
        entry = cards.setdefault(spelled, {"name": spelled, "top": 0.0, "commanders": 0.0, "decks": 0, "among": 0})
        entry["top"] = max(entry["top"], top.get(name, 0.0))
        entry["decks"] += decks.get(name, 0)
        entry["among"] += among.get(name, 0)
    for entry in cards.values():
        entry["commanders"] = round(100 * entry["decks"] / total, 2)
        entry["top"] = round(entry["top"], 2)
        entry["score"] = max(entry["top"], entry["commanders"])
    ranked = sorted(cards.values(), key=lambda c: (-c["score"], -c["decks"], c["name"]))

    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "commander.json")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump({
            "format": "commander",
            "generated": datetime.date.today().isoformat(),
            "sources": {
                "EDHREC": {"window": WINDOWS[args.window], "topCards": top_read, "commanders": len(commanders),
                           "commanderDecks": total},
            },
            "cards": ranked,
        }, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"{len(ranked)} cards to {os.path.relpath(path, ROOT)}")
    for name in unknown:
        print(f"unknown card: {name}", file=sys.stderr)


if __name__ == "__main__":
    main()
