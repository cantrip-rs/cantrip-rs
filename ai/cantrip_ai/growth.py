"""The network's growth, a generation at a time: each generation's size, throughput and training, and how fast each run
converged by the samples it saw, so sizes are compared at equal data (when to stop growing the network).

    python -m cantrip_ai.growth --root ../../ai-data/templates/fp [--models DIR ...] [--marks 2M,5M,10M] [--out FILE.md]

Read only. For each generation `ROOT/genN` (`retention.generations`): its record `generation.json` (`generation.py`),
self-play's own report `genN/selfplay/selfplay-report.json` (preferred for throughput when present; else the record's
selfplay stage), the inference server's last summary line in `genN/logs/server-selfplay.log` (cantrip-infer's `infer:`
or serve.py's `serve:`), and its training under `ROOT/models/genN/train` (`metrics.json` and `train.log`, as train.py
writes them). Each `--models DIR` is a training run of its own (an imitation model, a step up the size ladder): its
`DIR/train/metrics.json`, or `DIR/metrics.json` for a bare training directory, with `train.log` beside it, else
`DIR/train.out`, else `DIR.out`. A number a record does not have is shown as "-".

The convergence tables give the policy cross-entropy and value MSE at each mark of samples seen, linearly interpolated
between the two evaluations around it; a mark past a run's last evaluation is "-". A run trained with `--eval-data`
is read on its fixed evaluation set (`fixed_eval`): the same positions throughout the run, and in every run given the
same set and seed, so such runs compare directly. A run without one is read on its held-out set (`held_out`), which
train.py holds out of the data chunk it is training on, so the set can change as a run moves through its chunks, and
runs on the same data and chunk schedule meet the same sets at the same marks. Each row says which set it is read on
and that set's number of choices (the last evaluation's).
"""

from __future__ import annotations

import argparse
import ast
import json
import re
import statistics
import sys
from pathlib import Path

from . import retention

DEFAULT_MARKS = (1e6, 2e6, 5e6, 10e6, 20e6, 40e6, 80e6, 160e6, 320e6)

STEP = re.compile(r"^step (\d+)/(\d+)\b.*?([\d.]+) samples/s(?:.*?peak ([\d.]+) GiB)?")
MODEL = re.compile(r"^model (\{.*\}): (\d+) parameters")
PRECISION = re.compile(r"^precision (\S+)")
# cantrip-infer: "infer: ... rows (16569 a second) ..., 70% of the padded rows real; workers busy 108%, ..."
# serve.py:      "serve: ... rows (9000 a second) ..., 70% of the padded rows real; a batch running 80% of the time; ..."
SERVER = re.compile(r"^(?:infer|serve): \d+ requests, \d+ rows \(([\d.]+) a second\).*?([\d.]+)% of the padded rows real"
                    r"(?:; workers busy ([\d.]+)%|; a batch running ([\d.]+)% of the time)?", re.M)


# --- Reading ------------------------------------------------------------------------------------------------------


def read_json(path: Path) -> dict | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def server_summary(path: Path) -> dict | None:
    """The inference server's last summary line: rows a second, the share of padded rows real, workers busy."""
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    found = None
    for found in SERVER.finditer(text):
        pass
    if found is None:
        return None
    busy = found.group(3) or found.group(4)
    return dict(rows_per_second=float(found.group(1)), real=float(found.group(2)) / 100,
                busy=float(busy) / 100 if busy is not None else None)


def train_log(path: Path | None) -> dict:
    """What train.py's log says: the model's size, the precision, each step line's rate and the peak memory."""
    out: dict = dict(rates=[], peak=None)
    if path is None:
        return out
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return out
    for line in lines:
        if m := STEP.match(line):
            out["rates"].append(float(m.group(3)))
            if m.group(4) is not None:
                out["peak"] = max(out["peak"] or 0.0, float(m.group(4)))
        elif m := MODEL.match(line):
            try:
                config = ast.literal_eval(m.group(1))
                out["size"] = f"{config['width']}x{config['layers']}"
            except (ValueError, SyntaxError, KeyError, TypeError):
                pass
            out["parameters"] = int(m.group(2))
        elif m := PRECISION.match(line):
            out["precision"] = m.group(1)
    return out


def training_files(directory: Path) -> tuple[Path | None, Path | None]:
    """A training run's metrics.json and its log: under `train/`, or directly in a bare training directory."""
    for d in (directory / "train", directory):
        if (d / "metrics.json").exists():
            metrics = d / "metrics.json"
            for log in (d / "train.log", directory / "train.out", directory.with_name(directory.name + ".out")):
                if log.exists():
                    return metrics, log
            return metrics, None
    for log in (directory / "train" / "train.log", directory / "train.log", directory / "train.out"):
        if log.exists():
            return None, log
    return None, None


def training(directory: Path) -> dict | None:
    """A training run: its history of evaluations by samples seen (`curve`: on its fixed evaluation set where it has
    one, else held out; `curve_set` says which), and what its log says."""
    metrics_path, log_path = training_files(directory)
    if metrics_path is None and log_path is None:
        return None
    metrics = read_json(metrics_path) if metrics_path else None
    history = (metrics or {}).get("history") or []
    log = train_log(log_path)
    last = history[-1] if history else {}

    def evaluations(key):
        return [r for r in history if isinstance(r.get(key), dict) and r[key] and "seen" in r]

    held = evaluations("held_out")
    fixed = evaluations("fixed_eval")
    key, points = ("fixed_eval", fixed) if fixed else ("held_out", held)
    return dict(
        steps=last.get("step"), seen=last.get("seen"), seconds=last.get("seconds"),
        final=(metrics or {}).get("final"), held_out=held[-1]["held_out"] if held else None,
        fixed_eval=fixed[-1]["fixed_eval"] if fixed else None,
        curve=[(r["seen"], r[key]) for r in points], curve_set="fixed" if fixed else "held out",
        rate=statistics.median(log["rates"]) if log["rates"] else None, peak=log["peak"],
        size=log.get("size"), parameters=log.get("parameters"), precision=log.get("precision"))


def generation(root: Path, n: int, directory: Path) -> dict:
    """One generation's row: its record, self-play's report, the server's summary, its training."""
    record = read_json(directory / "generation.json") or {}
    stages = record.get("stages") or {}
    sp, tr = stages.get("selfplay") or {}, stages.get("train") or {}
    report = read_json(directory / "selfplay" / "selfplay-report.json")
    run = training(root / "models" / f"gen{n}")
    row = dict(name=f"gen{n}", stages=[k for k, v in stages.items() if isinstance(v, dict) and v.get("done")],
               finished=bool(record.get("finished")), training=run)
    # Self-play: its own report first, the record's stage else.
    if report is not None:
        row.update(games=report.get("Games"), games_per_hour=report.get("GamesPerHour"),
                   searched_per_second=report.get("SearchedPerSecond"), evaluations_per_second=report.get("EvaluationsPerSecond"),
                   runaways=report.get("Runaways"), capped=report.get("Capped"))
    else:
        row.update(games=sp.get("games"), games_per_hour=sp.get("games_per_hour"), searched_per_second=sp.get("searched_per_second"),
                   evaluations_per_second=sp.get("evaluations_per_second"), runaways=sp.get("runaways"), capped=sp.get("capped"))
    row["server"] = server_summary(directory / "logs" / "server-selfplay.log")
    # Training: the record's numbers first, the log's and the metrics' else.
    run = run or {}
    held = tr.get("held_out") if isinstance(tr.get("held_out"), dict) else run.get("held_out")
    row.update(size=tr.get("size") or run.get("size"), parameters=tr.get("parameters") or run.get("parameters"),
               precision=tr.get("precision") or run.get("precision"), steps=tr.get("steps", run.get("steps")),
               seen=tr.get("seen", run.get("seen")), rate=run.get("rate"), peak=run.get("peak"),
               train_seconds=tr.get("seconds_training", run.get("seconds")), held_out=held)
    gate, promote = stages.get("gate"), stages.get("promote")
    row["gate"] = gate if isinstance(gate, dict) else None
    row["accepted"] = promote.get("accepted") if isinstance(promote, dict) else None
    return row


def model_run(directory: Path) -> dict:
    """A `--models` training run, as a row of the same table (no self-play, no gate)."""
    run = training(directory) or {}
    name = directory.name
    if name.startswith("train") and directory.parent.name:
        name = f"{directory.parent.name}/{name}"
    return dict(name=name, stages=[], finished=run.get("final"), training=run or None, size=run.get("size"),
                parameters=run.get("parameters"), precision=run.get("precision"), steps=run.get("steps"), seen=run.get("seen"),
                rate=run.get("rate"), peak=run.get("peak"), train_seconds=run.get("seconds"), held_out=run.get("held_out"),
                server=None, gate=None, accepted=None)


# --- The curves ---------------------------------------------------------------------------------------------------


def at_mark(curve: list[tuple[float, dict]], mark: float, key: str) -> float | None:
    """A held-out metric at `mark` samples seen, linearly between the evaluations around it; None past the last."""
    points = [(seen, h[key]) for seen, h in curve if isinstance(h.get(key), (int, float))]
    for (s0, v0), (s1, v1) in zip(points, points[1:]):
        if s0 <= mark <= s1:
            return v0 if s1 == s0 else v0 + (v1 - v0) * (mark - s0) / (s1 - s0)
    if points and points[0][0] == mark:
        return points[0][1]
    return None


def parse_count(text: str) -> float:
    text = text.strip().upper()
    scale = {"K": 1e3, "M": 1e6, "G": 1e9, "B": 1e9}.get(text[-1:], 1)
    return float(text[:-1] if scale != 1 else text) * scale


# --- Writing ------------------------------------------------------------------------------------------------------


def fmt(v, spec="{:.3f}") -> str:
    return "-" if v is None else spec.format(v)


def count(v) -> str:
    if v is None:
        return "-"
    for scale, suffix in ((1e9, "G"), (1e6, "M"), (1e3, "k")):
        if abs(v) >= scale:
            return f"{v / scale:.3g}{suffix}"
    return f"{v:g}"


def gate_text(g: dict | None) -> str:
    if not g:
        return "-"
    share, margin = g.get("share"), g.get("margin")
    score = f"{share:.1%} +/- {margin:.1%}" if share is not None and margin is not None else fmt(share, "{:.1%}")
    return f"{score}, {fmt(g.get('games'), '{}')} games, {g.get('verdict') or '-'}"


def table(rows: list[dict]) -> list[str]:
    head = ["run", "stages done", "size", "parameters", "precision", "train steps", "train samples/s", "peak GiB", "train h",
            "policy CE", "value MSE", "top1", "self-play games/h", "searched/s", "server rows/s", "real rows", "server busy",
            "runaways", "gate (score, games, verdict)", "accepted"]
    lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    for r in rows:
        h = r.get("held_out") or {}
        s = r.get("server") or {}
        stages = ", ".join(r["stages"]) if r["stages"] else "-"
        if r["stages"] and r.get("finished"):
            stages = "finished"
        accepted = "-" if r.get("accepted") is None else ("yes" if r["accepted"] else "no")
        cells = [r["name"], stages, r.get("size") or "-", count(r.get("parameters")), r.get("precision") or "-",
                 fmt(r.get("steps"), "{}"), fmt(r.get("rate"), "{:.0f}"), fmt(r.get("peak"), "{:.1f}"),
                 fmt(None if r.get("train_seconds") is None else r["train_seconds"] / 3600, "{:.2f}"),
                 fmt(h.get("policy_ce"), "{:.4f}"), fmt(h.get("value_mse"), "{:.4f}"), fmt(h.get("top1"), "{:.4f}"),
                 fmt(r.get("games_per_hour"), "{:.0f}"), fmt(r.get("searched_per_second"), "{:.1f}"),
                 fmt(s.get("rows_per_second"), "{:.0f}"), fmt(s.get("real"), "{:.0%}"), fmt(s.get("busy"), "{:.0%}"),
                 fmt(r.get("runaways"), "{}"), gate_text(r.get("gate")), accepted]
        lines.append("| " + " | ".join(cells) + " |")
    return lines


def convergence(rows: list[dict], marks: list[float], key: str) -> list[str]:
    runs = [r for r in rows if r.get("training") and r["training"].get("curve")]
    if not runs:
        return ["No training run has a held-out evaluation yet."]
    reached = [m for m in marks if any(at_mark(r["training"]["curve"], m, key) is not None for r in runs)]
    head = ["run", "size", "set", "choices (last)", *(count(m) for m in reached), "at the end (seen)"]
    lines = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    for r in runs:
        curve = r["training"]["curve"]
        seen, last = curve[-1]
        cells = [r["name"], r.get("size") or "-", r["training"].get("curve_set") or "held out", fmt(last.get("choices"), "{}"),
                 *(fmt(at_mark(curve, m, key), "{:.4f}") for m in reached),
                 f"{fmt(last.get(key), '{:.4f}')} ({count(seen)})"]
        lines.append("| " + " | ".join(cells) + " |")
    return lines


def report(root: Path, models: list[Path], marks: list[float] | None = None) -> str:
    marks = sorted(marks or DEFAULT_MARKS)
    rows = [model_run(d) for d in models]
    rows += [generation(root, n, d) for n, d in retention.generations(root)]
    out = [f"# Growth: {root}", "",
           "Each generation's network, its training and its self-play, and each `--models` run's training. Training "
           "samples/s is the median of the log's step lines; the held-out numbers are the last evaluation; self-play's "
           "from its report (else the generation's record); the server's from the last summary line of "
           "`logs/server-selfplay.log` (rows a second, the share of padded rows real, workers busy).", "",
           *table(rows), "",
           "## Convergence by samples seen", "",
           "Each run's numbers at each mark of samples seen, linearly interpolated between the evaluations around it "
           "(\"-\" outside a run's evaluations; a mark no run has is left out). A generation's samples are counted from its "
           "own start, not its initialisation's. The set column says what a run is read on: \"fixed\", its fixed "
           "evaluation set (train.py `--eval-data`), the same positions throughout, so runs on the same set and seed "
           "compare directly; \"held out\", the games held out of the data chunk being trained on, so such runs compare "
           "only on the same data and chunk schedule. The set's choices are the last evaluation's.", "",
           "### Policy cross-entropy", "", *convergence(rows, marks, "policy_ce"), "",
           "### Value MSE", "", *convergence(rows, marks, "value_mse"), ""]
    return "\n".join(out)


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    a.add_argument("--root", required=True, help="the generations' root (genN/, models/genN/)")
    a.add_argument("--models", nargs="*", default=[], help="training runs of their own (a model directory or a bare training directory)")
    a.add_argument("--marks", help="samples-seen marks for the convergence tables, e.g. 2M,5M,10M (default 1M ... 320M)")
    a.add_argument("--out", help="also write the report to this Markdown file")
    args = a.parse_args(argv)
    marks = [parse_count(m) for m in args.marks.split(",") if m.strip()] if args.marks else None
    text = report(Path(args.root), [Path(m) for m in args.models], marks)
    sys.stdout.write(text)
    if args.out:
        Path(args.out).write_bytes(text.encode("utf-8"))


if __name__ == "__main__":
    main()
