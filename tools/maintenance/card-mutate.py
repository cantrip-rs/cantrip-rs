"""Breaks a card's code one small change at a time and runs its scenarios against each: every change no scenario
catches is a decision of the card's own code that no expectation would catch being wrong (the add-cards skill's
"break the card", step 5, done by the machine).

    python tools/maintenance/card-mutate.py "Card Name"            # every change, one build and run each
    python tools/maintenance/card-mutate.py "Card Name" --list     # the changes it would try, nothing built
    python tools/maintenance/card-mutate.py "Card Name" --lines 12-30 --max 20
    python tools/maintenance/card-mutate.py "Card Name" --file crates/cantrip-mechanics/src/tokens.rs --lines 40-60
    python tools/maintenance/card-mutate.py "Card Name" --whole    # with every card in the build, not only those played

The changes: a number one more and one less; `<` and `<=`, `>` and `>=` swapped, `==` and `!=`; `&&` and `||`;
`true` and `false`; a `!` dropped; `+` and `-`; a statement that calls the game left out. Each is built and the card's
scenarios run (`corpus -- --card`): one that fails is CAUGHT, one that passes SURVIVED (a missing expectation - or an
equivalent change, which the report names as such when you have read it), one that does not compile is skipped. The
file is restored after every change and at exit; it should be committed or staged first all the same
(`git checkout -- <file>` restores it).

A build holds only the cards the card's scenarios play (the `mutate` profile, with `CANTRIP_CARDS_ONLY` naming them, which
crates/cantrip-cards/build.rs reads): seconds a change, where a build of every card takes most of a minute. The profile keeps
its own place under `target/`, so it costs the `cards` build nothing; the first run in a tree builds the engine for it. A card
reached some way the scan cannot see fails the unchanged run, and that run then says so and builds every card, as `--whole` does.
"""

import argparse
import importlib.util
import os
import re
import subprocess
import sys
import time

sys.stdout.reconfigure(encoding="utf-8")

_spec = importlib.util.spec_from_file_location("new_card", os.path.join(os.path.dirname(__file__), "new-card.py"))
new_card = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(new_card)

ROOT = new_card.ROOT
PROFILE = "mutate"
FEW_CARDS = os.path.join(ROOT, "target", "card-mutate-cards.txt")

# (name, pattern, replacement): each match on a code line is one change.
SWAPS = [
    # A comparison has a space on each side; a generic (`Vec<ObjectId>`, `Result<(), Suspend> {`) does not.
    ("<= to <", r"(?<=\s)<=(?=\s)", "<"),
    ("< to <=", r"(?<=\s)<(?=\s)", "<="),
    (">= to >", r"(?<=\s)>=(?=\s)", ">"),
    ("> to >=", r"(?<=\s)>(?=\s)", ">="),
    ("== to !=", r"==", "!="),
    ("!= to ==", r"!=", "=="),
    ("&& to ||", r"&&", "||"),
    ("|| to &&", r"(?<=\s)\|\|(?=\s)", "&&"),
    ("true to false", r"\btrue\b", "false"),
    ("false to true", r"\bfalse\b", "true"),
    ("! dropped", r"(?<![\w!=)\]])!(?=[\w(])", ""),
    ("+ to -", r"(?<=\s)\+(?=\s)", "-"),
    ("- to +", r"(?<=\s)-(?=\s)", "+"),
]

NUMBER = re.compile(r"(?<![\w.#])(\d+)(?![\w.])")
# A statement that does something to the game: `g.draw(you, 1)?;`, `effects::pump(g, ...);`.
CALL_STATEMENT = re.compile(r"^(\s*)(?!let\b|return\b|Ok\(|Err\()((?:g|game)\.\w+|\w+::\w+(?:::\w+)*)\(.*\)\??;\s*$")


def code_spans(line):
    """The parts of a line outside string literals and comments: (start, end) pairs."""
    spans, start, i, quote = [], 0, 0, None
    while i < len(line):
        c = line[i]
        if quote:
            if c == "\\":
                i += 2
                continue
            if c == quote:
                quote, start = None, i + 1
        elif c == '"':
            spans.append((start, i))
            quote = c
        elif line.startswith("//", i):
            spans.append((start, i))
            return spans
        i += 1
    if not quote:
        spans.append((start, len(line)))
    return spans


def mutants(lines, wanted):
    """Every change to the code lines in `wanted`: (line number, what, new line)."""
    out = []
    for n, line in enumerate(lines, 1):
        if n not in wanted:
            continue
        stripped = line.strip()
        if not stripped or stripped.startswith(("//", "use ", "#[", "pub static", "static ", "const ", "pub const")) \
                or "oracle::" in line:
            continue
        spans = code_spans(line)

        def inside(m):
            return any(a <= m.start() and m.end() <= b for a, b in spans)

        for m in NUMBER.finditer(line):
            if not inside(m) or re.search(r"(?:as\s+u\d+|\{|::)\s*$", line[:m.start()]):
                continue
            value = int(m.group(1))
            for new in (value + 1, value - 1):
                if new >= 0:
                    out.append((n, f"{value} to {new}", line[:m.start()] + str(new) + line[m.end():]))
        for what, pattern, replacement in SWAPS:
            for m in re.finditer(pattern, line):
                if inside(m) and "->" not in line[max(0, m.start() - 1):m.end() + 1] and "=>" not in line[max(0, m.start() - 1):m.end() + 1]:
                    out.append((n, what, line[:m.start()] + replacement + line[m.end():]))
        m = CALL_STATEMENT.match(line)
        if m and inside(re.search(re.escape(m.group(2)), line)):
            out.append((n, "statement left out", m.group(1) + "// (left out by card-mutate)"))
    return out


def parse_lines(text, count):
    if not text:
        return set(range(1, count + 1))
    wanted = set()
    for part in text.split(","):
        a, _, b = part.partition("-")
        wanted.update(range(int(a), int(b or a) + 1))
    return wanted


def deck_cards(text):
    """The cards a scenario's decks, sideboards and companions hold, as its parser reads them."""
    names, reading = [], False
    for line in text.split("[Actions]", 1)[0].splitlines():
        line = line.strip()
        if line.startswith("["):
            reading = line.upper().endswith((" DECK]", " SIDEBOARD]", " COMPANION]"))
        elif reading and line and not line.startswith("#"):
            names.append(re.sub(r"^\d+\s+|\s+[xX]\d+$", "", line).strip())
    return names


def shared_definitions():
    """A card file for each definition the mechanics crate shares, a token or a named emblem, which comes into the card table
    only in some card's `EXTRA`: without one the game stops at it, by a route no deck names."""
    providers = {}
    for name in sorted(os.listdir(new_card.CARDS)):
        with open(os.path.join(new_card.CARDS, name), encoding="utf-8") as f:
            found = re.search(r"pub static EXTRA:[^=]*=(.*?);\s*$", f.read(), re.S | re.M)
        for shared in re.findall(r"&(\w+(?:::\w+)+)", found[1]) if found else ():
            providers.setdefault(shared, name[:-3])
    return set(providers.values())


def played(card):
    """The card files it takes to play this card's scenarios: their decks' cards, the files those name, and the definitions
    any of them may reach."""
    stems = {new_card.file_stem(card)}
    wanted = card.lower()
    for name in os.listdir(new_card.SCRIPTS):
        if name.endswith(".cantrip"):
            with open(os.path.join(new_card.SCRIPTS, name), encoding="utf-8") as f:
                decks = deck_cards(f.read())
            if wanted in (deck.lower() for deck in decks):
                stems.update(new_card.file_stem(deck) for deck in decks)
    exists = lambda stem: os.path.exists(os.path.join(new_card.CARDS, stem + ".rs"))
    stems = {stem for stem in stems if exists(stem)} | shared_definitions()
    todo = list(stems)
    while todo:
        with open(os.path.join(new_card.CARDS, todo.pop() + ".rs"), encoding="utf-8") as f:
            text = f.read()
        for stem in re.findall(r"\b(?:super|crate)::(?:r#)?(\w+)::", text):
            if stem not in stems and exists(stem):
                stems.add(stem)
                todo.append(stem)
    return stems


def run_scenarios(card, few=False):
    """'caught', 'survived' or 'no build', with the command's output; 'cards missing' if a build of few cards lacks one."""
    proc = subprocess.run(
        ["cargo", "run", "--profile", PROFILE if few else "cards", "-q", "-p", "cantrip-script", "--bin", "corpus", "--", "--card", card, "--brief"],
        cwd=ROOT, env=few_cards_env() if few else None, capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = proc.stdout + proc.stderr
    if "error[" in out or "could not compile" in out:
        return "no build", out
    if re.search(r"\b0 pass, 0 fail\b", out):
        return "no scenarios", out
    if few and not re.search(r"\b0 need cards", out):
        return "cards missing", out
    return ("survived" if proc.returncode == 0 else "caught"), out


def few_cards_env():
    return dict(os.environ, CANTRIP_CARDS_ONLY=FEW_CARDS)


def name_few_cards(card):
    """Writes the list a build of few cards reads, only if it changed, since a change rebuilds them."""
    text = "\n".join(sorted(played(card))) + "\n"
    os.makedirs(os.path.dirname(FEW_CARDS), exist_ok=True)
    if not os.path.exists(FEW_CARDS) or open(FEW_CARDS, encoding="utf-8").read() != text:
        with open(FEW_CARDS, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("card")
    parser.add_argument("--file", help="the file to change, when not the card's own (a shared mechanic it uses)")
    parser.add_argument("--lines", help="only these lines, as 12-30,41")
    parser.add_argument("--max", type=int, default=0, help="at most this many changes")
    parser.add_argument("--list", action="store_true", help="list the changes and build nothing")
    parser.add_argument("--whole", action="store_true", help="build every card, not only those the card's scenarios play")
    args = parser.parse_args()

    path = os.path.join(ROOT, args.file) if args.file else os.path.join(new_card.CARDS, new_card.file_stem(args.card) + ".rs")
    if not os.path.exists(path):
        sys.exit(f"no file {os.path.relpath(path, ROOT)} (a card of characteristics alone has no code to break)")
    rel = os.path.relpath(path, ROOT).replace("\\", "/")
    original = open(path, encoding="utf-8", newline="").read()
    stamp = os.stat(path)
    newline = "\r\n" if "\r\n" in original else "\n"
    lines = original.split(newline)
    found = mutants(lines, parse_lines(args.lines, len(lines)))
    if args.max:
        found = found[:args.max]
    if args.list:
        for n, what, new in found:
            print(f"{rel}:{n}: {what}\n    {new.strip()}")
        print(f"{len(found)} changes")
        return 0

    dirty = subprocess.run(["git", "diff", "--quiet", "--", path], cwd=ROOT).returncode != 0
    if dirty:
        print(f"note: {rel} has unstaged changes; it is restored to them after each change")
    print(f"the unchanged card first: its scenarios must pass")
    few = not args.whole
    if few:
        name_few_cards(args.card)
        verdict, out = run_scenarios(args.card, few=True)
        if verdict != "survived":
            print(f"a build of only the cards it plays gives {verdict}: building every card instead")
            few = False
    if not few:
        verdict, out = run_scenarios(args.card)
    if verdict != "survived":
        print(out[-3000:])
        sys.exit(f"{args.card}'s scenarios do not pass as the card stands ({verdict}): fix that first")

    survived, results = [], {"caught": 0, "survived": 0, "no build": 0}
    started = time.time()
    try:
        for i, (n, what, new) in enumerate(found, 1):
            changed = lines[:n - 1] + [new] + lines[n:]
            with open(path, "w", encoding="utf-8", newline="") as f:
                f.write(newline.join(changed))
            verdict, _ = run_scenarios(args.card, few)
            results[verdict if verdict in results else "no build"] += 1
            mark = {"caught": "caught", "survived": "SURVIVED", "no build": "no build"}.get(verdict, verdict)
            print(f"[{i}/{len(found)}] {rel}:{n} {what}: {mark}", flush=True)
            if verdict == "survived":
                survived.append((n, what, new))
    finally:
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(original)
        # The corpus binary was last built from a mutant: build it again from the restored file, so a scenario run
        # that uses the binary directly afterwards tests the card as it stands.
        subprocess.run(["cargo", "build", "--profile", PROFILE if few else "cards", "-q", "-p", "cantrip-script", "--bin", "corpus"],
                       cwd=ROOT, env=few_cards_env() if few else None, capture_output=True)
        # The `cards` build never saw a mutant, so with its time back the file costs it no rebuild either.
        if few:
            os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))

    minutes = (time.time() - started) / 60
    print(f"\n{results['caught']} caught, {results['survived']} survived, {results['no build']} did not build"
          f" ({minutes:.1f} min); {rel} restored")
    for n, what, new in survived:
        print(f"SURVIVED {rel}:{n}: {what}\n    was: {lines[n - 1].strip()}\n    now: {new.strip()}")
    if survived:
        print("\nEach survivor is an expectation missing - or a change that means the same thing (say so in the report).")
    return 1 if survived else 0


if __name__ == "__main__":
    sys.exit(main())
