"""Everything there is to read before porting a card to the native engine, in one call, and its file to start from.

    python tools/maintenance/new-card.py "Mind Stone" [--no-write]

Writes the card's content JSON from Scryfall when content/ has none (a single face, a double-faced card, an adventure, a
prototype card;
any other layout is written by hand), then prints the card's content JSON, the scenarios that use it and, for each of its abilities, the implemented cards that
already do it (ability-index.py), and writes
crates/cantrip-cards/src/cards/<card_name>.rs over the card's generated constant if the
file does not exist yet. Then the loop is: write the behavior, and

    cargo run --profile cards -p cantrip-script --bin corpus -- --card "Mind Stone"

which runs only that card's scenarios and prints the full report of any that fail (docs/adding-cards.md).
"""

import importlib.util
import json
import os
import re
import sys

import subprocess
import unicodedata

import cr
import scryfall

# Oracle text carries characters such as U+2212 that the Windows console code page cannot print.
sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")


def _root():
    """The repository the command runs in, so a worktree's run writes into that worktree whichever copy of the script
    was named; else the script's own repository."""
    try:
        out = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip()
        if out:
            return os.path.normpath(out)
    except (OSError, subprocess.CalledProcessError):
        pass
    return os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))


ROOT = _root()
CONTENT = os.path.join(ROOT, "content")
SCRIPTS = os.path.join(ROOT, "scenarios")
CARDS = os.path.join(ROOT, "crates", "cantrip-cards", "src", "cards")
GAME_RS = os.path.join(ROOT, "crates", "cantrip-api", "src", "game.rs")


def ident(name):
    """The generated oracle constant (crates/cantrip-oracle/build.rs)."""
    s = re.sub(r"[^A-Za-z0-9]+", "_", name.replace("'", "")).strip("_").upper()
    return s if not s[0].isdigit() else "_" + s


def file_stem(name):
    return re.sub(r"[^a-z0-9]+", "_", name.replace("'", "").lower()).strip("_")



def find_card(name):
    for dataset in sorted(os.listdir(CONTENT)):
        folder = os.path.join(CONTENT, dataset, "cards")
        if not os.path.isdir(folder):
            continue
        for file in os.listdir(folder):
            if not file.endswith(".json"):
                continue
            path = os.path.join(folder, file)
            text = open(path, encoding="utf-8-sig").read()
            body = "\n".join(line for line in text.split("\n") if not line.strip().startswith("//"))
            try:
                card = json.loads(body)
            except ValueError:
                continue
            if card.get("name", "").lower() == name.lower():
                return dataset, path, text, card
    return None


def _abilities(index, text, names):
    """A face's abilities as compared with Scryfall's: one kind of quote and dash, case aside."""
    text = text.replace("’", "'").replace("‘", "'").replace("“", '"').replace("”", '"').replace("−", "-")
    text = re.sub(r"(?<=\S) - (?=\S)|\s*-\s*$", lambda m: " — " if m.group(0).strip() else " —", text, flags=re.M)
    return sorted(index.template(part, "exact") for ab in index.split_abilities(text, names) for _, part in ab.parts)


def scryfall_report(card, index):
    """Whether each face's Oracle text in content/ is Scryfall's current text, and the card's rulings."""
    found = scryfall.card(card["name"])
    if found is None:
        return f"  not found on Scryfall (or Scryfall unreachable): check the Oracle text and rulings by hand"
    theirs = {f["name"].lower(): f["oracle_text"] for f in found["faces"]}
    if len(found["faces"]) == 1:
        theirs[found["name"].lower()] = found["faces"][0]["oracle_text"]
    lines = []
    names = [n for n, _ in index.card_texts(card)] + [card["name"]]
    for face, mine in index.card_texts(card):
        current = theirs.get((face or card["name"]).lower())
        if current is None:
            if "//" not in card["name"]:  # a split card's halves are faces of the whole card's name
                lines.append(f"  {face}: no face of that name on Scryfall")
        # Compared ability by ability: the card's name and "this creature" are one (Oracle updates move between them),
        # reminder text is no rule, and "Flying, lifelink" is "Flying" and "Lifelink" on lines of their own.
        elif _abilities(index, mine, names) != _abilities(index, current, names):
            lines.append(f"  {face}: DIFFERS from Scryfall's current Oracle text - fix the content JSON first:")
            lines.append("    " + current.replace("\n", "\n    "))
    if not lines:
        lines.append("  Oracle text matches Scryfall's")
    rulings = found["rulings"]
    lines.append(f"  {len(rulings)} ruling{'' if len(rulings) == 1 else 's'}{':' if rulings else ''}")
    for r in rulings:
        lines.append(f"  - {r['date']}  {r['text']}")
    return "\n".join(lines)


# A cost the card's own file declares (an activated ability's, or a keyword's like flashback or cycling): reviews kept
# finding one a cheaper cost would still pass, because the mana sat in the pool and nothing checked what was left.
COST_POINT = ("the cost '{m}': paid exactly - the pool checked empty right after paying, so one less fails - and each"
              " colored symbol pinned (refused with one too few sources of that color, enough in all)")

# The words of Oracle text a scenario most often fails to pin down, each with what pins it. Found by what reviews sent
# back: an amount no expectation checked, one player's side never shown, a value read after an effect changed it.
TEST_POINTS = [
    (r"\b(?:\d+|one|two|three|four|five|six|seven|x)\b(?! or more| or less| or greater)|\{\d+\}",
     "the amount {m}: an expectation that a value one off would fail (paying exactly, dealing exactly)"),
    (r"\b(?:\d+|one|two|three|four|five|six|seven|x) or (?:more|less|greater|fewer)\b",
     "the limit '{m}': an object exactly at it qualifies, one just past it doesn't"),
    (r"\bup to (?:one|two|three|four|five|x|\d+)\b", "'{m}': fewer than the most (or none) as well as the most"),
    (r"[+-][\dxX]+/\+0\b|\+0/[+-][\dxX]+",
     "'{m}': the unchanged half checked too (power and toughness both expected), so a pump of the wrong half fails"),
    (r"\btarget\b", "'target': one of your own (or you) accepted as a target"),
    (r"\btarget\b", "'target': one of an opponent's (or the opponent) accepted as a target"),
    (r"\btarget\b", "'target': one just outside the filter refused"),
    (r"\btarget (?:spell|permanent|card)\b",
     "'{m}': targets of different kinds the word covers (a creature spell and a noncreature one for 'spell'), so a filter"
     " narrower than the text fails"),
    (r"\bwhenever\b[^,]*\b(?:dies|die|sacrifices?|leaves?|put into a graveyard|is exiled)\b",
     "'{m}': several at once trigger it as many times, and this permanent leaving in the same event still sees them"),
    (r"\b(?:return|exile|sacrifice|put) (?:it|them|that \w+|those \w+)\b[^.]*\bat the beginning of\b",
     "'{m}': the delayed part touches only the same object(s), and only while still where the text left them (CR 603.7c, 400.7)"),
    (r"\b(?:each|all)\b(?! (?:opponent|player))", "'{m}': both players' objects it could apply to, so one side left out fails"),
    (r"\byou control\b|\ban opponent controls\b|\byour opponents? control\b", "'{m}': the other side's object must not count"),
    (r"\b(?:from )?your (?:graveyard|hand|library)\b|\b(?:from )?an opponent's (?:graveyard|hand|library)\b",
     "'{m}': a matching card in the other player's zone at that moment, which must stay where it is"),
    (r"\b(?:another|other)\b", "'{m}': the object itself must not count (it is there, and would match)"),
    (r"\bthen\b", "'{m}': the order shown - a case where the steps in the other order end differently"),
    (r"\b(?:this turn|until end of turn|until (?:the end of )?your next turn|next turn|last turn)\b",
     "'{m}': the same thing on another turn, and the effect checked after it should have ended"),
    (r"\binstead\b", "'instead': the replaced event must not also happen"),
    (r"\bunless\b[^.]*\bpays?\b", "'unless ... pays': paid exactly (the effect doesn't happen) and unpayable (it does)"),
    (r"\bcosts? \{[^}]+\}(?:\{[^}]+\})* (?:less|more)\b",
     "the cost change: the pool checked right after the cast, both with and without the condition"),
    (r"^[^:\"]*\{[^}]+\}[^:\"]*(?=:)", COST_POINT),
    (r"\bas an additional cost to cast\b[^.]*",
     "'{m}': the cast refused when it can't be paid, with the mana there to pay the rest (a refusal for want of"
     " mana proves nothing), beside the allowed cast"),
    (r"\b(?:sacrificed|exiled|destroyed|discarded|that) (?:creature|permanent|card)'s (?:power|toughness|mana value)\b",
     "'{m}': read as it last existed - change it with an effect first and show the result follows"),
    (r"\bcreate (?:two|three|four|five|x|\d+)\b|\bfor each\b", "'{m}': all made / done at once, as one event (never a loop)"),
    (r"\b\d+/\d+ [a-z ]*?creature tokens?\b|\b(?:Treasure|Food|Clue|Blood|Map) tokens?\b",
     "the token '{m}': if `scryfall.py --text` finds another printed card making it exactly, it is defined once in"
     " cantrip-mechanics `tokens` (or already is there) - never in the card file"),
    (r"\bmay\b", "'may': both answers"),
    (r"\b(?:combat )?damage to (?:a player|an opponent|target player|each opponent)\b",
     "'{m}': damage to a planeswalker (or a creature) must not count"),
    (r"\b(?:white|blue|black|red|green|colorless|multicolored|monocolored)\b",
     "the color '{m}': an object of another color refused, one of two colors where it matters"),
]


def test_points(abilities):
    """The test points of a card's abilities: for each word above its text uses, what an expectation must pin down."""
    out, seen = [], set()
    for i, ab in enumerate(abilities, 1):
        text = re.sub(r"\([^)]*\)", "", ab.text)
        if ab.keyword is not None:
            lines = [f"  [{i}] {ab.text}: shown at least once (a keyword's file entry runs no lines)"]
            cost = re.search(r"\{[^}]+\}(?:\{[^}]+\})*", text)
            if cost:  # flashback {7}{G}{G}{G}, cycling {2}: the card's file declares the cost, so it is the card's to pin
                lines.append(f"  [{i}] " + COST_POINT.format(m=cost.group(0)))
            for line in lines:
                if line not in seen:
                    seen.add(line)
                    out.append(line)
            continue
        before = len(out)
        for pattern, what in TEST_POINTS:
            for m in dict.fromkeys(x.group(0) for x in re.finditer(pattern, text, re.I)):
                line = f"  [{i}] " + what.format(m=m)
                if line not in seen:
                    seen.add(line)
                    out.append(line)
        if len(out) == before:  # an ability no word above names still has to be seen working
            out.append(f"  [{i}] {ab.text[:80]}: shown working (a mana ability or other static entry runs no lines)")
    return "\n".join(out)


# Keyword actions the core does in its own procedures (casting, activating, searching ...): their number is enough.
BASIC_ACTIONS = {"activate", "attach", "cast", "counter", "create", "destroy", "discard", "double", "exchange", "exile",
                 "play", "reveal", "sacrifice", "search", "shuffle", "tap and untap", "transform", "fight", "mill"}


def _has_verb(stem):
    """Whether cantrip-api's Game has a verb of that name ("scry", "sacrifice")."""
    try:
        return re.search(r"\bfn " + re.escape(stem) + r"\b", open(GAME_RS, encoding="utf-8").read()) is not None
    except OSError:
        return False


def cr_report(card, abilities, index):
    """The CR rule of each keyword and keyword action on the card: the whole rule when nothing in the engine does it yet
    (the rule is the specification), its number when the core or cantrip-mechanics already does."""
    core = index.core_keywords()
    lines, seen = [], set()
    for ab in abilities:
        if ab.keyword is None:
            continue
        found = cr.section(ab.keyword)
        if found is None or found[0] in seen:
            continue
        seen.add(found[0])
        number, title = found
        if ab.keyword in core:
            lines.append(f"  {number} {title}: core keyword")
        elif index.mechanics_for(ab.keyword):
            lines.append(f"  {number} {title}: {index.mechanics_for(ab.keyword)}")
        else:
            lines.append(f"  {number} {title}: no module yet - its rule:\n    " + cr.rule(number).replace("\n", "\n    "))
    names = [n for n, _ in index.card_texts(card)] + [card["name"]]
    text = index.clean("\n".join(t for _, t in index.card_texts(card)), names)  # "Enduring Courage" endures nothing
    for number, title in cr.terms_in(text):
        if number in seen:
            continue
        stem = index.file_stem(title)
        home = f"g.{stem}" if _has_verb(stem) else index.mechanics_for(title)
        if title.lower() in BASIC_ACTIONS:
            lines.append(f"  {number} {title}" + (f": {home}" if home else ""))
        elif home:
            lines.append(f"  {number} {title}: {home}")
        else:
            lines.append(f"  {number} {title}: no verb or module of that name - its rule:\n    " + cr.rule(number).replace("\n", "\n    "))
    return "\n".join(lines) or "  none"


# Scryfall layouts a content file says as one face, a back face or an adventure; the rest are written by hand.
SINGLE = {"normal", "leveler", "class", "saga", "case", "mutate"}
COLOR_NAMES = {"W": "White", "U": "Blue", "B": "Black", "R": "Red", "G": "Green"}


def _core_keywords():
    """{"first strike": "FirstStrike", "daybound": "Daybound", ...}: every core keyword (crates/cantrip-oracle/build.rs's
    KEYWORDS) by the words Oracle prints it as; the ones no keyword line says (CantBeBlocked) never match."""
    src = open(os.path.join(ROOT, "crates", "cantrip-oracle", "build.rs"), encoding="utf-8").read()
    names = re.findall(r'"([^"]+)"', re.search(r"const KEYWORDS[^=]*=\s*\[(.*?)\];", src, re.S).group(1))
    return {re.sub(r"(?<!^)(?=[A-Z])", " ", n).lower(): n for n in names}


def _stat(value):
    return int(value) if re.fullmatch(r"-?\d+", value or "") else value


def _face(face, names, index, core):
    """One face as a content file has it: its characteristics, and the core keywords its Oracle text has."""
    out = {"name": face["name"]}
    if face.get("mana_cost"):
        out["manaCost"] = face["mana_cost"]
    if face.get("color_indicator"):
        out["colorIndicator"] = [COLOR_NAMES[c] for c in face["color_indicator"]]
    out["type"] = face["type_line"]
    # A station card's [P/T] box is printed in its striation (CR 721.2b) and is no characteristic until it has the counters
    # (CR 721.2c): kept as the box it is, for the client to draw there, never as power and toughness.
    station = re.search(r"^\d+\+ \|", face.get("oracle_text", ""), re.M) and face.get("power") and face.get("toughness")
    if station:
        out["stationPt"] = f"{face['power']}/{face['toughness']}"
    for key in ("power", "toughness", "loyalty", "defense"):
        if face.get(key) and not (station and key in ("power", "toughness")):
            out[key] = _stat(face[key])
    out["oracleText"] = face["oracle_text"]
    words = [ab.keyword for ab in index.split_abilities(face["oracle_text"], names) if ab.keyword]
    keywords = [core[w] for w in dict.fromkeys(words) if w in core]
    # Whole lines the core plays as a keyword of its own (every card in content that prints one has it).
    for line in index.clean(face["oracle_text"], names).split("\n"):
        for pattern, keyword in index.TEXT_KEYWORDS:
            if re.fullmatch(pattern, line.strip(), re.I) and keyword not in keywords:
                keywords.append(keyword)
        # Toxic N (CR 702.164a): a core keyword with its number, "Toxic 2", alone or in a keyword list ("Menace, toxic 1"),
        # each part as crates/cantrip-oracle/build.rs reads it.
        for part in re.split(r"[,;]", line):
            toxic = re.fullmatch(r"toxic (\d+)", part.strip(), re.I)
            if toxic:
                keywords.append(f"Toxic {toxic.group(1)}")
    if keywords:
        out["keywords"] = keywords
    for flag in ("convoke", "delve", "improvise", "assist"):
        if flag in words:
            out[flag] = True
    return out


def draft_content(name, index):
    """Writes content/mtg/cards/<slug>.json for a card new to content, from Scryfall's current characteristics; for a
    layout a file can't be written from directly (split, omen, prototype ...), prints what Scryfall has and exits."""
    found = scryfall.card(name)
    if found is None:
        sys.exit(f"no card named {name!r} in content/ or on Scryfall; spell it as printed")
    faces, layout = found["faces"], found.get("layout", "normal")
    names = [f["name"] for f in faces] + [found["name"]]
    core = _core_keywords()
    if layout in SINGLE and len(faces) == 1:
        card = _face(faces[0], names, index, core)
    elif layout in ("transform", "modal_dfc") and len(faces) == 2:
        card = _face(faces[0], names, index, core)
        if layout == "modal_dfc" and "Land" in faces[1]["type_line"].split(" — ")[0].split():
            card["backFacePlayableAsLand"] = True  # CR 712.12: a modal double-faced card may be played as its back face
        card["backFace"] = _face(faces[1], names, index, core)
    elif layout == "prototype" and len(faces) == 1:
        # A prototype card (CR 718): its inset frame's mana cost, power and toughness are printed only in its keyword line,
        # "Prototype {2}{R} — 3/2", which crates/cantrip-oracle/build.rs checks the block against.
        card = _face(faces[0], names, index, core)
        line = re.search(r"^Prototype ((?:\{[^}]+\})+) — (-?\d+)/(-?\d+)", faces[0]["oracle_text"], re.M)
        if not line:
            sys.exit(f"{found['name']}: a prototype card whose Oracle text has no \"Prototype {{cost}} — P/T\" line")
        card["prototype"] = {"manaCost": line.group(1), "power": int(line.group(2)), "toughness": int(line.group(3))}
    elif layout == "adventure" and len(faces) == 2:
        # Scryfall lays an Omen card (CR 720) out as an adventure: its second face's subtype says which it is.
        kind = "omen" if "Omen" in faces[1]["type_line"].split(" — ")[-1].split() else "adventure"
        card = _face(faces[0], names, index, core) | {kind: _face(faces[1], names, index, core)}
    else:
        print(f"{found['name']} is a {layout} card: write content/mtg/cards/<name>.json by hand (the add-cards skill, step 2),")
        print("then run new-card.py again. What Scryfall has:\n")
        for face in faces:
            print(f"{face['name']}  {face.get('mana_cost', '')}  {face.get('type_line', '')}\n{face['oracle_text']}\n")
        sys.exit(1)
    slug = re.sub(r"-+", "-", re.sub(r"[^a-z0-9]+", "-", unicodedata.normalize("NFKD", card["name"])
                                     .encode("ascii", "ignore").decode().lower().replace("'", ""))).strip("-")
    path = os.path.join(CONTENT, "mtg", "cards", slug + ".json")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(card, indent=2, ensure_ascii=False) + "\n")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        sys.exit(__doc__)
    found = find_card(args[0])
    spec = importlib.util.spec_from_file_location("ability_index", os.path.join(os.path.dirname(__file__), "ability-index.py"))
    index = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(index)
    drafted = False
    if found is None:
        if "--no-write" in sys.argv:
            sys.exit(f"no card named {args[0]!r} in content/ (run without --no-write to write it from Scryfall)")
        draft_content(args[0], index)
        found = find_card(args[0])
        drafted = True
    dataset, path, text, card = found
    name = card["name"]

    print(f"=== {name}  ({os.path.relpath(path, ROOT)})  oracle::{ident(name)}")
    if drafted:
        print("    written just now from Scryfall: check it against the card (the core keywords besides Oracle's -")
        print("    CantBeBlocked, CantBeCountered ... in crates/cantrip-oracle/build.rs - are yours to add where the text says so)")
    print(text.strip())

    notes = os.path.join(ROOT, "docs", "card-notes", file_stem(name) + ".md")
    if os.path.exists(notes):
        print(f"\n=== {os.path.relpath(notes, ROOT)}: the owner's design for this card - follow it, and delete the file")
        print("    in the card's commit")
        print(open(notes, encoding="utf-8").read().strip())

    print("\n=== Scenarios naming it")
    pattern = re.compile(r"^\s*(?:\d+\s+)?" + re.escape(name) + r"(?:\s+x\d+)?\s*$", re.M | re.I)
    for file in sorted(os.listdir(SCRIPTS)):
        if file.endswith(".cantrip") and pattern.search(open(os.path.join(SCRIPTS, file), encoding="utf-8-sig").read()):
            print(f"  scenarios/{file}")

    abilities = index.card_abilities(card)

    print("\n=== Scryfall: current Oracle text and official rulings (tools/maintenance/scryfall.py)")
    print(scryfall_report(card, index))

    print("\n=== Comprehensive Rules: the card's keywords and keyword actions (tools/maintenance/cr.py <number or name>)")
    print(cr_report(card, abilities, index))

    print("\n=== Prior art: the implemented cards that already do each ability (tools/maintenance/ability-index.py)")
    print("    Read each listed card's code for the line. Where it already does what yours would - the same steps, only")
    print("    its numbers, its verb at the end or its own object left out differing - that part is not written again: it")
    print("    moves into one shared function in cantrip-mechanics, in a `Mechanics:` commit before the card, and both")
    print("    cards call it (SKILL.md, \"Share what is the same\"). Reviews send back every card that skips this.")
    print(index.report(index.Index(exclude=[name]), abilities) or "  (no rules text)")

    print("\n=== Test points: each needs an expectation that fails if the card gets it wrong (list the line that catches")
    print("    each in your report; the reviewer checks the list). A refusal (`! ...`) pins nothing unless the action could")
    print("    otherwise be taken right there: pass priority first if a trigger may be on the stack, and empty the pool of")
    print("    any mana that could pay instead (docs/known-issues.md KI-133)")
    print(test_points(abilities) or "  none beyond showing each ability")

    target = os.path.join(CARDS, file_stem(name) + ".rs")
    if os.path.exists(target):
        print(f"\n=== {os.path.relpath(target, ROOT)} exists:")
        print(open(target, encoding="utf-8").read().strip())
    elif "--no-write" not in sys.argv:
        with open(target, "w", encoding="utf-8", newline="\n") as f:
            f.write(
                "use cantrip_api::*;\nuse cantrip_oracle as oracle;\n\n"
                f"pub static CARD: CardDef = CardDef {{ ..oracle::{ident(name)} }};\n"
            )
        print(f"\n=== wrote {os.path.relpath(target, ROOT)} (characteristics only; add the behavior)")


if __name__ == "__main__":
    main()
