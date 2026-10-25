"""Retention of the expert-iteration loop's generations (docs/ai-player-plan.md, "Phase 4").

    python -m cantrip_ai.retention --root ../../ai-data [--keep 8] [--pin gen3 ...] [--dry-run]

A generation is a directory `genN` under the root (its self-play data, its README and `generation.json`) and its
model, `models/genN`. The newest `--keep` generations are kept whole. An older one is retired: its data and its model
are deleted, its README and `generation.json` stay (what it was, and what its gate said), and a `RETIRED` file says
when. A generation is pinned, and never retired, when its directory holds a `PINNED` file or it is named by `--pin`;
gen0 (the tactical bot's games, the imitation data everything started from) is always pinned. Champions are frozen
copies under `champions/`, which retention never touches, so a champion's model outlives its generation.
"""

from __future__ import annotations

import argparse
import re
import shutil
import time
from pathlib import Path

GENERATION = re.compile(r"^gen(\d+)$")
KEPT_FILES = ("README.md", "generation.json", "PINNED", "RETIRED")
ALWAYS_PINNED = ("gen0",)


def generations(root: Path) -> list[tuple[int, Path]]:
    """The generations under `root`, oldest first."""
    found = []
    for p in root.iterdir() if root.exists() else []:
        m = GENERATION.match(p.name)
        if m and p.is_dir():
            found.append((int(m.group(1)), p))
    return sorted(found)


def pinned(path: Path, pins: set[str]) -> bool:
    return path.name in pins or path.name in ALWAYS_PINNED or (path / "PINNED").exists()


def plan(root: Path, keep: int, pins: set[str]) -> list[Path]:
    """What retention would delete: every path of a retired generation but its record, and its model."""
    gens = generations(root)
    old = gens[: max(len(gens) - keep, 0)]
    doomed: list[Path] = []
    for _, g in old:
        if pinned(g, pins):
            continue
        doomed.extend(sorted(p for p in g.iterdir() if p.name not in KEPT_FILES))
        model = root / "models" / g.name
        if model.exists():
            doomed.append(model)
    return doomed


def retire(root: Path, keep: int = 8, pins: set[str] | None = None, dry_run: bool = False, log=print) -> list[Path]:
    root = Path(root)
    pins = set(pins or ())
    doomed = plan(root, keep, pins)
    for p in doomed:
        log(f"{'would delete' if dry_run else 'deleting'} {p}")
        if dry_run:
            continue
        if p.is_dir() and not p.is_symlink():
            shutil.rmtree(p)
        else:
            p.unlink()
    if not dry_run:
        for g in {p.parent for p in doomed if p.parent.parent == root}:
            (g / "RETIRED").write_text(f"retired {time.strftime('%Y-%m-%d %H:%M:%S')}: data and model deleted, kept the last {keep} generations\n")
    return doomed


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    a.add_argument("--root", required=True, help="the ai-data directory")
    a.add_argument("--keep", type=int, default=8)
    a.add_argument("--pin", nargs="*", default=[], help="generations never retired (gen0 always is)")
    a.add_argument("--dry-run", action="store_true")
    args = a.parse_args(argv)
    doomed = retire(Path(args.root), args.keep, set(args.pin), args.dry_run)
    print(f"{len(doomed)} path(s) {'to delete' if args.dry_run else 'deleted'}")


if __name__ == "__main__":
    main()
