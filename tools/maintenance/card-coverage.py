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
import glob
import json
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
    metadata = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if metadata.returncode != 0:
        sys.stdout.write(metadata.stdout[-4000:] + metadata.stderr[-4000:])
        sys.exit("card-coverage: cargo metadata failed")
    target_dir = json.loads(metadata.stdout)["target_directory"]
    coverage_dir = os.path.join(target_dir, "llvm-cov-target")
    clean = subprocess.run(["cargo", "llvm-cov", "clean", "--workspace"], cwd=ROOT, capture_output=True, text=True)
    if clean.returncode != 0:
        sys.stdout.write(clean.stdout[-4000:] + clean.stderr[-4000:])
        sys.exit("card-coverage: could not clear previous coverage profiles")
    run = subprocess.run(
        ["cargo", "llvm-cov", "run", "--profile", "cards", "--dep-coverage", "cantrip-cards", "-p", "cantrip-script", "--bin", "corpus",
         "--no-report", "--", "--card", name],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    summary = [l for l in run.stdout.splitlines() if "scenarios:" in l]
    if run.returncode == 0:
        host = subprocess.run(["rustc", "-vV"], cwd=ROOT, capture_output=True, text=True).stdout
        triple = next((line.removeprefix("host: ") for line in host.splitlines() if line.startswith("host: ")), None)
        sysroot = subprocess.run(["rustc", "--print", "sysroot"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
        extension = ".exe" if os.name == "nt" else ""
        tool_dir = os.path.join(sysroot, "lib", "rustlib", triple or "", "bin")
        llvm_profdata = os.path.join(tool_dir, "llvm-profdata" + extension)
        llvm_cov = os.path.join(tool_dir, "llvm-cov" + extension)
        raw = glob.glob(os.path.join(coverage_dir, "*.profraw"))
        profile = os.path.join(coverage_dir, "card-coverage.profdata")
        binary = os.path.join(coverage_dir, "cards", "corpus" + extension)
        lcov = os.path.join(tempfile.mkdtemp(prefix="card-coverage-"), "card.lcov")
        if not raw or not os.path.isfile(llvm_profdata) or not os.path.isfile(llvm_cov) or not os.path.isfile(binary):
            run.returncode = 1
            run.stderr += "\ncard-coverage: llvm-tools-preview is not installed or the instrumented corpus/profile is missing"
        else:
            merge = subprocess.run([llvm_profdata, "merge", "-sparse", *raw, "-o", profile], cwd=ROOT, capture_output=True, text=True)
            if merge.returncode == 0:
                with open(lcov, "w", encoding="utf-8") as output:
                    run = subprocess.run([llvm_cov, "export", "-format=lcov", f"-instr-profile={profile}", binary],
                        cwd=ROOT, stdout=output, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace")
            else:
                run = merge
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
        passed = re.search(r"(\d+) pass,", summary[-1]) if summary else None
        if passed and int(passed.group(1)) > 0 and not re.search(r"\bfn\s+\w+\s*\(", "\n".join(source)):
            print(f"{rel}: no executable card-specific Rust lines; the focused scenarios passed and exercise its declared abilities")
            return 0
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
