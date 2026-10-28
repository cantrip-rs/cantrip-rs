"""Which lines of a card's Rust file its scenarios never execute.

    python tools/maintenance/card-coverage.py "Card Name"
    tools\\card-coverage.cmd "Card Name"

Runs the card's scenarios (`corpus --card`, every scenario whose decks hold the card) under cargo-llvm-cov, then reads
the line coverage of crates/cantrip-cards/src/cards/<card>.rs. Prints each line that never ran, with its text, and
exits 1 if there is one, 0 when every line ran. A card with no Rust file has nothing to cover. Needs cargo-llvm-cov
(`rustup component add llvm-tools-preview`, `cargo install cargo-llvm-cov --locked`). The add-cards skill requires
every line covered.
"""

import os
import re
import subprocess
import sys
import tempfile
import unicodedata

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
CARDS = os.path.join(ROOT, "crates", "cantrip-cards", "src", "cards")


def file_stem(name):
    """The card's Rust file name: new-card.py's rule (accents folded, other non-alphanumerics to '_')."""
    folded = unicodedata.normalize("NFKD", name)
    folded = "".join(c for c in folded if not unicodedata.combining(c))
    return re.sub(r"[^a-z0-9]+", "_", folded.replace("'", "").lower()).strip("_")


def card_file(name):
    for stem in (file_stem(name), re.sub(r"[^a-z0-9]+", "_", name.replace("'", "").lower()).strip("_")):
        path = os.path.join(CARDS, stem + ".rs")
        if os.path.isfile(path):
            return path
    return None


def const_fn_lines(source):
    """The line numbers inside `const fn` items: they run at compile time, so llvm-cov counts them as never run."""
    lines, i = set(), 0
    while i < len(source):
        if re.match(r"\s*(pub(\([^)]*\))?\s+)?const\s+fn\s", source[i]):
            depth, started, j = 0, False, i
            while j < len(source):
                code = re.sub(r'"(\\.|[^"\\])*"', "", source[j]).split("//")[0]
                depth += code.count("{") - code.count("}")
                started = started or "{" in code
                lines.add(j + 1)
                if started and depth <= 0:
                    break
                j += 1
            i = j
        i += 1
    return lines


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    name = sys.argv[1]
    path = card_file(name)
    if path is None:
        print(f"{name}: no Rust file in {os.path.relpath(CARDS, ROOT)} (a card of core keywords only needs none).")
        return 0
    lcov = os.path.join(tempfile.mkdtemp(prefix="card-coverage-"), "card.lcov")
    run = subprocess.run(
        ["cargo", "llvm-cov", "run", "--profile", "cards", "-p", "cantrip-script", "--bin", "corpus",
         "--no-report", "--", "--card", name],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    summary = [l for l in run.stdout.splitlines() if "scenarios:" in l]
    if run.returncode == 0:
        # The report is a step of its own: since cargo-llvm-cov 0.8, a run's report holds only the package it ran
        # (cantrip-script), never the card's crate. `report --workspace` (0.9) holds every member; an older version
        # takes no --workspace and holds them all without it.
        for scope in (["--workspace"], []):
            run = subprocess.run(
                ["cargo", "llvm-cov", "report", "--profile", "cards", *scope, "--lcov", "--output-path", lcov],
                cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
            if run.returncode == 0:
                break
    if run.returncode != 0 or not os.path.isfile(lcov):
        sys.stdout.write(run.stdout[-4000:] + run.stderr[-4000:])
        sys.exit("card-coverage: the coverage run failed (is cargo-llvm-cov installed?)")
    target = os.path.normcase(os.path.abspath(path))
    hits, current = {}, None
    for line in open(lcov, encoding="utf-8"):
        if line.startswith("SF:"):
            named = line[3:].strip()
            if named.startswith("\\\\?\\"):
                named = named[4:]  # llvm-cov writes Windows paths in their long form, \\?\D:\...
            current = os.path.normcase(os.path.abspath(named)) == target
        elif current and line.startswith("DA:"):
            number, count = line[3:].strip().split(",")[:2]
            hits[int(number)] = hits.get(int(number), 0) + int(count)
    source = open(path, encoding="utf-8").read().splitlines()
    exempt = const_fn_lines(source)
    hits = {n: c for n, c in hits.items() if n not in exempt}
    missed = sorted(n for n, c in hits.items() if c == 0)
    rel = os.path.relpath(path, ROOT)
    if summary:
        print(summary[-1].strip())
    if not hits:
        print(f"{rel}: no instrumented lines ran or exist - does any scenario hold {name}?")
        return 1
    if not missed:
        print(f"{rel}: every line ran ({len(hits)} instrumented lines).")
        return 0
    print(f"{rel}: {len(missed)} of {len(hits)} instrumented lines never ran:")
    for n in missed:
        print(f"  {n:4}  {source[n - 1].rstrip() if n <= len(source) else ''}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
