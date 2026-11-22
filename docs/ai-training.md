# Training the AI player

How to train the AI player's network on any machine: an NVIDIA GPU (CUDA), an AMD GPU (ROCm), or the CPU alone. The
design is [ai-player-plan.md](ai-player-plan.md) ("Training", Phases 2-4); the AI host and self-play binary are
[ai-host.md](ai-host.md). This is the runbook.

Training runs in **generations** of expert iteration. The current champion plays itself with search, a new network is
trained on those games, and the new network replaces the champion only if it beats it in a gate. A GPU speeds up the
training stage, and on an NVIDIA GPU self-play, the parity check and the gate as well, through the inference server
(below). Without it they run the network with ONNX Runtime on the CPU, so a machine with many cores matters as much as
a GPU.

Models and training data never go in the repository. They live in a data directory beside it, `ai-data/` in this
document (any path works; the driver takes `--root`):

```
<parent>/
  cantrip-rs/   the repository
  ai-data/      shards, models and champions (created by the steps below)
```

## What you need

| | Needed for | Notes |
|---|---|---|
| Rust (`rustup`; the repository's `rust-toolchain.toml` picks the version) | build, self-play, parity | the `ort` feature downloads ONNX Runtime the first time it builds |
| .NET SDK 8 or later | the gate (BotLab) and recording gen0 | the driver runs `CANTRIP_DOTNET` if it is set, else the `dotnet` on PATH, else `nix shell nixpkgs#dotnet-sdk_10` |
| Python 3.11 or later with PyTorch, onnx, onnxruntime, numpy and zstandard | training and export | see "Python environment" below; `ai/pyproject.toml` lists the packages |
| A GPU (optional) | training only | CUDA or ROCm PyTorch; training falls back to the CPU |

## Python environment

Pick **one** way of setting it up. Each gives the same `python` with the same packages; only the PyTorch build differs.

### pip or uv (any OS)

Install the PyTorch build for your hardware first, then the rest. PyTorch's own selector
(<https://pytorch.org/get-started/locally/>) gives the right index URL for your driver; the ones below are examples.

```bash
cd cantrip-rs
python -m venv .venv && . .venv/bin/activate       # Windows: .venv\Scripts\activate

# one of:
pip install torch --index-url https://download.pytorch.org/whl/cpu        # CPU only
pip install torch --index-url https://download.pytorch.org/whl/cu126      # NVIDIA, CUDA 12.6
pip install torch --index-url https://download.pytorch.org/whl/rocm6.3    # AMD, ROCm 6.3 (Linux)

pip install "onnx>=1.16" "onnxruntime>=1.18" "numpy>=1.26" "zstandard>=0.22"
```

With uv, `uv venv`, then `uv pip install torch --index-url ...` as above, then `uv pip install -r ai/pyproject.toml`.

### Nix

| Shell | PyTorch |
|---|---|
| `nix-shell ai/shell.nix` | CPU (it also brings cargo, rustc and node) |
| `nix-shell ai/shell-cuda.nix` | CUDA (`torch-bin`, the prebuilt wheel; unfree, allowed by the shell) |
| `nix-shell ai/shell-rocm.nix` | ROCm (`torchWithRocm`; the ROCm user space comes with the shell) |

The GPU shells leave cargo, rustc and dotnet to the host. Run the commands below inside the shell
(`nix-shell ai/shell-cuda.nix --run '...'`), or enter it first.

### Docker (training and export only)

`ai/Dockerfile.cuda` and `ai/Dockerfile.rocm` are the train/export half. The driver's other stages need cargo and dotnet,
so the full loop runs on the host; a container is for training a model from data already recorded:

```bash
docker build -f ai/Dockerfile.cuda -t cantrip-ai-cuda ai
docker run --rm -it --device nvidia.com/gpu=all -v "$PWD/..:/work" -w /work/cantrip-rs \
  -e PYTHONPATH=/work/cantrip-rs/ai cantrip-ai-cuda \
  python -m cantrip_ai.train --data /work/ai-data/gen1/selfplay --out /work/ai-data/models/try --device cuda

docker build -f ai/Dockerfile.rocm -t cantrip-ai-rocm ai
docker run --rm -it --device /dev/kfd --device /dev/dri --group-add video --security-opt seccomp=unconfined \
  -v "$PWD/..:/work" -w /work/cantrip-rs -e PYTHONPATH=/work/cantrip-rs/ai cantrip-ai-rocm \
  python -m cantrip_ai.train --data /work/ai-data/gen1/selfplay --out /work/ai-data/models/try --device cuda
```

### Check it

```bash
python -c "import torch; print(torch.__version__, torch.cuda.is_available() and torch.cuda.get_device_name(0))"
python -m unittest discover -s ai/tests
```

On a ROCm build, `torch.cuda` is HIP, so `cuda` is the device name on AMD as well. `False` means training will run on
the CPU.

## Choosing the device

`train.py` and the generation driver take `--device`:

| `--device` | Uses |
|---|---|
| `auto` (default) | `cuda` when `torch.cuda.is_available()` (an NVIDIA GPU, or an AMD GPU on a ROCm build), else `cpu` |
| `cuda` | the first GPU, CUDA or ROCm; `cuda:1` for the second |
| `cpu` | the CPU; `--train-args '--threads N'` sets its threads (default: all but two) |

Checkpoints are saved on the CPU whatever the device, so a model trained on one machine exports, loads in Rust and
resumes on any other.

## Cards

Nothing about cards is checked in. A position names each card by its row of the **card index**, the engine's own cards,
so every card the engine has can be recorded, and each shard says which index it used. The **text vectors** a row
starts from (TF-IDF over the card's Oracle text, reduced to 64 numbers) are made when a network is trained:

- a fresh network (generation 0) fits them to the cards its shards name;
- a fine-tuned one keeps its checkpoint's rows exactly and places each card new to it in the same basis, so the cards
  upstream adds since the last generation get rows in the next one;
- the checkpoint keeps the names and the basis, and export writes the names as the model's `cards.json`. Until a model
  is trained with a card, the card plays as the unknown card.

`python -m cantrip_ai.embed` prints how many cards have text and the size of the basis fitted to them.

## Windows

Everything below runs on Windows too, from PowerShell or `cmd`, with these differences:

- The scripts are `tools\bot-duel.cmd` for `tools/bot-duel.sh`, and paths take either slash.
- The Python environment is a venv whose `python.exe` runs the driver (`.venv\Scripts\python.exe -m cantrip_ai.generation
  ...`); the driver runs `train`, `export` and the server with the same interpreter.
- `champions/current` is a symlink where Windows allows one (Developer Mode, or an administrator), and otherwise a
  directory junction, which any user can make. Both are read the same way.
- `cantrip_ai.train --workers N` works on Windows as on Linux. Windows starts a worker by spawning, not forking, so
  a worker would be handed its own copy of the loaded data (14 GB for the six-deck gen0, 6 M samples): `train` moves
  the data into named shared memory first (`dataset.share`), and every worker reads the one copy. Measured on that
  gen0 (RTX 4080, 32 threads, batch 256): `--workers 0` trains 5,300 samples/s with the GPU about 35% busy;
  `--workers 4` trains 13,700 (67% busy) in the same memory, about 23 GB; `--workers 8` adds nothing. The CPU alone
  (30 threads) trains 885. Under `fork` (Linux) nothing is moved: the workers share the parent's memory already.
- `cantrip_ai.train --load-workers N` (default: 16, or the logical cores if fewer) reads the shards in that many
  spawned processes as they load; `0` or `1` reads them in the trainer's own. The data loaded is the same either way,
  byte for byte: what depends on the order (the training index, game numbers, a weight's choice of games,
  `--max-games`) is settled in order by the trainer (`dataset.load`). A game passed over by `--decks` or a weight below
  1 is skipped unread, which is most of an imitation corpus at `--imitation-weight 0.1`.

## The GPU inference server

`python -m cantrip_ai.serve` (`ai/cantrip_ai/serve.py`) holds the network on the GPU and evaluates the search's leaves
for every self-play lane and every AI host at once, in batches (docs/ai-player-plan.md, "Decisions"; the client is
`--backend remote`, docs/ai-host.md). The generation driver starts and stops it by itself (`--gpu-server`, below); to
run it by hand, from `ai/`:

```bash
python -m cantrip_ai.serve --model ../../ai-data/champions/gen1 [--model ../../ai-data/models/gen2] --port 7710
```

- **The model directory** is export's, with `model.pt`, the weights the server runs, which export writes beside the ONNX
  graphs. A model exported before that gets it with `python -m cantrip_ai.export --checkpoint DIR/train/checkpoint.pt
  --out DIR --weights-only`, which changes nothing else there. At start the server runs the model's references
  (`reference/`) through its own path and refuses a model that is not within export's tolerances.
- **Clients**: `cantrip-selfplay --backend remote --server 127.0.0.1:7710`, `cantrip-ai-host --backend remote --server
  ...`, and BotLab's `--ai-server 127.0.0.1:7710` for its neural seats. A client names its model by its directory's
  `manifest.json`, so the server must have been started with that model.
- **`--device`**: `auto` (CUDA, or ROCm's HIP, else the CPU), `cuda`, `cuda:1` or `cpu`.
- **Batching**: each queue (a model and a token bucket) runs when it holds `--batch` rows, when every client is waiting
  on it, or when its oldest row has waited `--flush-ms` (2). The defaults, measured on an RTX 4080: `--batch
  32:256,64:128,96:96,128:64` (a bucket not named keeps its default) and `--pipelines 2`. A row runs at the smallest of
  the model's buckets that holds its real tokens and every token they point at: 96 is a serving bucket only (positions
  are encoded at 32, 64 or 128, and `encoder_hash` does not name it), which a model has when its manifest names a T96
  graph (export writes one; `export --add-graphs` adds it to an older model).
- **Reproducible.** Every batch is padded to its bucket's fixed size and the kernels are deterministic, so a position's
  outputs do not depend on what it was batched with, and self-play at a seed plays the same games on every run through
  the same server settings (batch sizes, precision, GPU and PyTorch). `--precision` is `fp32` by default, held to
  export's 1e-3 on every output (the life swing's
  1e-3 relative to its size above 50 points). Below it the references are held to what the search uses (`serve.REDUCED`,
  `eval::reference::PolicyDrift`): value and dead-next-turn within 1e-2; the policy, over the references with two legal
  options or more, by the total-variation distance of its legal probabilities from PyTorch's - mean below 2e-4, 99th
  percentile below 5e-3, largest below 5e-2 - with no best option changed where PyTorch's top two are 0.02 apart or more;
  the life swing (which no search reads) within half a point; the same best option on 99.5%. (Until 2026-11-13 the
  policy was held to each option's largest probability change, 1e-2: that is set by the closest tie among the
  references and moves with batching, and failed a model whose policy moved 1e-4 on average.) On the templates line's
  gen0-gen3 (the server's check, and its graphs under ORT), fp16's mean is 4e-5 to 1.3e-4 and its 99th percentile
  0.0007-0.0014, tf32's mean 3e-5 to 7e-5; bf16's are 5.4e-4 to 8.1e-4 and 0.008-0.018, and fail. Measured on gen0's 512 references, the largest drift:

  | Precision | Value | Life swing | Legal logits | Option probability | Same best |
  | --- | --- | --- | --- | --- | --- |
  | fp32 | 3e-6 | 1e-4 | 4e-5 | 2e-6 | 512/512 |
  | tf32 | 2e-3 | 0.08 | 0.04 | 4e-3 | 512/512 |
  | fp16 | 4e-3 | 0.12 | 0.08 | 4e-3 | 512/512 |
  | bf16 | 3e-2 | 0.63 | 0.98 | 3e-2 | 511/512 (refused) |

  fp16 and tf32 pass on gen0, gen1 and the bench model (and on the templates line's gen0-gen3 under the policy bar of
  2026-11-13); bf16 (an 8-bit mantissa) is refused. Each precision is
  reproducible bit for bit in itself, not across precisions.
- **Lanes**: a lane spends most of its time waiting on the server, so give self-play more lanes than threads. On 32
  threads and an RTX 4080, 16 lanes made 11.8 k evaluations a second, 32 made 17.8 k, 64 made 20.3 k and 128 made
  21.1 k, against 3.3-3.7 k on the CPU. Self-play now finds its own lane count (docs/ai-host.md, "The governor"): the
  driver's `--lanes` is a ceiling (96 by default) and the lanes grow until throughput stops repaying them, the CPU is
  busy past `--selfplay-cpu-ceiling` or memory admission defers; `--no-selfplay-autoscale` plays every lane, and its
  default with the server is two lanes a thread. The server prints
  its throughput every `--log-every` seconds (60): rows a second, how full its batches are, and how long a request
  waits.
- `--priority high` (the default) raises the server above the lanes that feed it where the OS allows it.
- **Native** (NVIDIA): `cantrip-infer` serves the same protocol without Python (docs/ai-host.md, "The native inference
  server"); the driver starts it instead of serve.py with `--server-impl native` (and `--ort-dylib`, or
  `ORT_DYLIB_PATH`, naming onnxruntime-gpu 1.24's `onnxruntime.dll`; CUDA's libraries come from this PyTorch's
  `torch/lib`), passing `--server-args` to it (`--precision fp16`, `--batch`, `--pipelines`, `--flush-ms`). Self-play
  can also run the engine in its own process (`cantrip-selfplay --backend gpu`). `model_parity` checks a fp16 server
  with `CANTRIP_AI_SERVER_REDUCED=1`.
- **AMD**: a ROCm build of PyTorch serves on the GPU as `cuda`, but the server has not been run on AMD hardware yet, so
  the driver's `--gpu-server auto` leaves it off on ROCm; `--gpu-server on` asks for it. Check it first with the parity
  stage (`model_parity` runs through the server when `CANTRIP_AI_SERVER` names it).

## Bootstrap: generation 0

Expert iteration starts from a champion. Generation 0 is a network that imitates the tactical bot: its games are
recorded as shards, a network is trained on them, and the driver makes that model the first champion. Run from the
repository root; `tools/bot-duel.sh` is `tools\bot-duel.cmd` on Windows.

```bash
# 1. Record tactical-bot games (about 2 games a second on 16 cores). Each game its own shuffle: the bot is deterministic.
tools/bot-duel.sh --a tactical --b tactical --deck rw-aggro --deck ub-reanimator --mirrors --each-shuffle \
  --games 12000 --parallel 16 --ai-record ../ai-data/gen0/matchup

# 2. Train generation 0 and export it. The driver expects the model at ai-data/models/gen0, its training under train/.
cd ai
python -m cantrip_ai.train --data ../../ai-data/gen0/matchup --out ../../ai-data/models/gen0/train --device auto
python -m cantrip_ai.export --checkpoint ../../ai-data/models/gen0/train/checkpoint.pt --out ../../ai-data/models/gen0
```

`python ai/shards.py --check DIR` checks that every recorded answer is legal. One epoch of 12,000 games is about 3.8 M
samples: about 2.5 hours on 10 CPU threads, and under an hour on a GPU. The first generation copies `models/gen0` to
`champions/gen0` and makes it current.

## Running generations

From `ai/`:

```bash
python -m cantrip_ai.generation --root ../../ai-data --device auto
```

Without `--generation`, the driver resumes the latest generation that was started but not finished, or starts the
next one. Its stages (with the GPU server, self-play, parity, the duels and the gate each run beside a server of their
own, started and stopped by the driver; the ones marked optional run only when asked for):

| Stage | What it does |
|---|---|
| build | `cantrip-ai-host` and `cantrip-selfplay` (release, `--features ort`), the native host, and BotLab (and `cantrip-infer` with the native server or `--workers`); on every run while the working tree is the generation's engine, and what it builds is kept as the generation's own ("The engine a generation runs") |
| preflight (not a stage: after the build, every run) | room on the disk and in memory, the workers reached, the champion's parity with the serving options ("Preflight") |
| selfplay | the champion plays itself with search (`--games`, `--simulations`, `--lanes`), shards to `genN/selfplay`; with `--workers`, on other machines too ("Distributed self-play") |
| heldout (optional, once) | `--heldout-games N` chooses the permanent held-out set: whole shards of this generation's self-play, before any network trains on them, moved to `ROOT/heldout` (`--heldout-dir`) with `HELDOUT.json` |
| train | fine-tunes the champion on the window of recent generations (`--window`, `--older-weight`, `--imitation-weight`); the window never holds the held-out set |
| export | ONNX models and the manifest, to `models/genN` |
| parity | the Rust evaluators against PyTorch on held-out positions, and with the GPU server, the server too (it checks the references itself as it starts); the first serving option that passes for every model the later servers hold is the generation's ("Parity, and what a generation serves with") |
| gate | the candidate against the champion in BotLab on self-play's metagame pairings, scored by pair, by `--accept` (below) over at most `--gate-games` (6,000); with `--workers`, on other machines too ("Distributed duels"), as are the ladder and the miner's replays; recorded by matchup too ("Reading a generation") |
| promote | an accepted candidate goes to `champions/genN` and becomes `champions/current` |
| mine, mined (unless `--no-mine`) | the pair miner (`mining.py`): the gate's decisive pairs replayed, judged by deep searches and written as policy targets, `genN/mine/targets`, which the next generations train on. **Data-producing**: a failure stops the generation ("When a stage fails") |
| evaluate (when a held-out set exists) | `value_eval` on the held-out set, candidate and champion: value MSE and ECE, policy CE and top-1 against its full searches' targets; the two paired by game, and each network's mean value by the seat's deck ("Reading a generation") |
| ladder (optional, after a promotion) | `--ladder-every N`: after every N-th promotion, the new champion against fixed anchors, recorded in `ROOT/ladder.json` and `ladder.md` (a generation run again replaces its rows) and by matchup in the generation's record; a generation that promoted nothing plays none |
| retire | deletes the shards and models of all but the last `--keep` generations (gen0 and pinned ones are kept) |

#### When a stage fails

- A stage that decides or makes something (build to promote, retire) ends the driver with the error, the tail of its
  log, a `failed` status ("Watching a run") and a notification. Running the driver again resumes at that stage.
- The **measurements** (evaluate, ladder) run after the promotion and decide nothing: one that fails is recorded as
  failed (`"failed"` in `generation.json`, a row of the README), listed in the README's WARNING block, told, and the
  generation goes on to the next stage.
- The **data-producing** stages (mine, mined) make the next generations' mined targets. One that fails **stops the
  generation**, saying that the generations after it would train without them: fix the cause and run the driver
  again. `--allow-data-failure` records the failure as a measurement's is and goes on; the README's WARNING block
  then says the targets are missing, and so does the README of each generation that trains without them. A
  generation whose mining window holds an earlier generation with a failed mine or mined stage refuses to run
  without `--allow-data-failure` - at its preflight, before a game is played, and again at training. A failure
  recorded by a driver from before this one (a record with no `engine`) is past mending and refuses nothing: it is
  flagged in the README and told once ("gen13's mined targets were lost before the hardened driver; training without
  them"). (gen13 of the templates line failed `mine verify`, was "recorded and passed over", and gen14 trained
  without its targets with nothing said: that is what this is for.)
- Training's data loader out of memory (its workers each attach the window; the trainer can hang on workers that are
  gone) is the one failure the driver gets past by itself: training is run again from its last checkpoint with half
  the loader's workers, down to none, `--train-oom-retries` times (2), each told as `recovered`. A trainer whose log
  says it and that has not exited a minute later is killed first.

The driver
runs no network-only duel (it did, as a `netduel` stage, until 2026-11-21: it decided nothing and no reading of a
generation came to rest on it); an older generation's record that holds one still prints it, and "A network-only
duel" below plays one by hand.

A rerun skips the stages `genN/generation.json` records as done, so an interrupted generation resumes. Self-play keeps
the shards it closed and plays only the games they don't hold. Training continues from its last checkpoint (every 200
steps), with the optimizer, step and epoch. A gate that finished but wasn't recorded is read back from its log, when
the log is this run's gate: the same champion and seat, the same acceptance rule (its stop line, with the early stop's
z and minimums) and the same pairing mode.

To run generation after generation, use the supervisor ("The loop", below): it watches each one, tells you what
happens, and changes the code only between two.

What a generation leaves in `ai-data/`:

- `genN/README.md`: what is flagged, the stage table and its numbers, then the gate by matchup and the held-out set
  by seat (the first thing to read: "Reading a generation", below);
- `genN/logs/{selfplay,train,export,parity,gate}.log`: the full output (`tail -f` them to watch), and with the GPU
  server `server-{selfplay,parity,gate}.log`, its throughput; `evaluate.log` and `ladder-*.log` for the
  measurements that ran, `genN/*-pairings.txt` the matchups each duel played, and `logs/*-results.tsv` every game
  of each duel;
- `genN/generation.json`: the same numbers, machine-readable;
- `models/genN/`: the exported ONNX model and its `manifest.json`;
- `champions/current`, a symlink to the champion in force, and `champions/history.json`, every gate (paired fields
  included);
- `heldout/` (once chosen) and `ladder.json` / `ladder.md` (when the ladder runs).

### Watching a run

While it works, the driver keeps `ROOT/genN/status.json` (`status.py`): the stage and when it began, the driver's pid,
when that process started (a pid is reused; the two together are one process) and host, how long the stage took in
the generations before (whole runs only: a resumed stage's time is its last part's), a progress counter and when it
last moved. The counter is read from what the stages write - the games in the distributed ledgers, self-play's
closed shards, the bytes of the stage logs, training's files - never from the words of a log. A status that cannot
be written never fails a run.

```bash
python -m cantrip_ai.status --root ../../ai-data          # exit 0 running/finished, 1 failed, 2 stalled, 3 dead, 4 stopped
```

says **running**; **stalled** (the process lives and nothing has moved for a quarter of the stage's usual time, at
least 15 minutes - `--stall-factor`, `--stall-floor` - or for an hour with no earlier generation to go by; or the
driver has stopped writing the file); **dead** (the file says running and that process is gone - a process that
exists and may not be asked about counts as alive); **failed**, with
the error; **stopped** (asked to: Ctrl-C, `--stop`); or **finished**; then the supervisor's own state and the tail of
the stage's log. `--json` for code.

**Notifications** (`notify.py`). The driver tells a stage that failed (`stage-failed`), a measurement or an allowed
data stage that failed and was passed over (`measurement-failed`, `data-failed`), anything the README's WARNING block
flags (`warning`: a matchup far below a half, a seat misvalued, missing mined targets, a serving fallback, a worker
not reached), a failure it got past (`recovered`), the gate's `verdict`, and a `finished` generation. The supervisor
tells what the driver cannot: `stalled`, `resumed`, `dead`, `retry`, `held`, `stopped`. Each is one short message and a
JSON payload:

```json
{"event": "stage-failed", "message": "gen14 FAILED at parity: parity (candidate): no serving option passes for gen14, gen13; tried ...",
 "root": "../ai-data", "generation": 14, "stage": "parity", "host": "trainer", "at": "2026-11-21 14:15:02",
 "details": {"error": "... the last 1,500 characters of the error ...", "transient": null}}
```

Where they go is this machine's business and never the repository's:

- `CANTRIP_AI_NOTIFY_HOOK`: a command line, run through the shell (`cmd.exe` on Windows) for each one with the message in
  `CANTRIP_AI_MESSAGE`, the event in `CANTRIP_AI_EVENT` and the payload on its standard input;
- `CANTRIP_AI_NOTIFY_WEBHOOK`: a URL the payload is POSTed to as JSON with a `content` field holding the message - a
  Discord webhook URL works as it is;
- or both in a file, `{"hook": "...", "webhook": "...", "events": ["stage-failed", "dead", ...]}`: `notify.json` in
  `%LOCALAPPDATA%\cantrip-ai` (Windows) or `~/.config/cantrip-ai`, or wherever `CANTRIP_AI_NOTIFY` says. `events`
  limits what is sent.

Every notification is also a line of `ROOT/notifications.jsonl`, sent or not, with what each channel answered.
`python -m cantrip_ai.notify --root ROOT --test "hello"` sends one and says where it went. A notification never
fails or delays a run.

### The loop

```bash
python -m cantrip_ai.loop --root ../../ai-data --update origin/main
```

runs generations one after another with the driver's arguments in `ROOT/generation.args` (`--args-file` names
another; shell words on any number of lines, `#` comments, Windows paths with forward slashes; arguments after `--`
are added; a line with a backslash outside quotes is told, since the shell's rules drop it), each generation's
output in `ROOT/genN-driver.log`. Start it detached (`nohup ... &`, a service, a terminal that stays open); one
supervisor a root (`ROOT/loop.lock`: a second one refuses to start). At each turn it

1. ends if `ROOT/LOOP.stop` exists, and waits while `ROOT/LOOP.pause` exists;
2. attaches to a driver that is already running the generation, and does nothing else beside it: no update, no
   preflight. A driver of this root that runs and writes no status file (one from before the status file; found in
   the process table) is not attached to and not run beside either: the loop is held until it has ended. The driver
   itself refuses to start beside a live driver of the same generation;
3. with `--update REMOTE/BRANCH`, and only between generations - never while the next one is begun and not
   finished - fetches and fast-forwards the working tree to it: never a merge, never over local changes; without
   `--update` the code is never touched;
4. runs the driver's `--preflight` ("Preflight"), which builds and checks everything the generation needs;
5. runs the generation and watches it.

Nothing the supervisor runs can hang it unnoticed: a git command is given up after two minutes (a fetch after ten;
the code then stays as it is) and a preflight after `--preflight-timeout` (two hours; the loop is then held), each
killed with what it started and told; the generation itself is watched for stalls.

A failure the driver names as transient (self-play stopped by its memory governor, an inference server that did not
start - but never one that refused a model for parity: `status.TRANSIENT`) is run again, `--retries` (2) times a
generation, after `--retry-wait` (300 s); so is a preflight that failed that way. Anything
else - a failed stage, a failed preflight, an update that is no fast-forward, a driver that died or was stopped -
**holds** the loop: the reason is written into `ROOT/LOOP.pause` and told, and the loop waits there. Remove the file
and it goes on, resuming the generation where it stopped; `LOOP.stop` ends it instead. `python -m cantrip_ai.status`
shows the loop's state (`ROOT/loop.json`) beside the generation's; `ROOT/loop.log` is its log.

**A generation held at its first preflight counts as begun** (its record exists), so `--update` no longer reaches
it: the code is updated only between generations. A fix for what held it is merged into the training tree by hand
(`git merge --ff-only`), then `LOOP.pause` removed. If the fix changes the engine's sources and nothing but the build
has run, remove the `genN` directory as well (the generation then starts afresh and pins the new engine), or pass
`--repin-engine` once.

To land code under a running loop: `touch ROOT/LOOP.pause`, wait for the generation to finish (`status` says
`loop: paused`), change the working tree, remove the file - or let `--update` do it. Never change the tree while a
generation runs; if it happens anyway the generation keeps the engine it pinned ("The engine a generation runs"), but
its Python is the tree's.

A supervisor started while a driver is already running the generation attaches to it rather than starting a second.
The process that loops is small and loads nothing that changes; each step is a fresh process, so updated code is in
force from the step after the update.

### Preflight

After the build, before anything is played, the driver checks (`--no-preflight` skips it):

- **the disk**: `--min-free-disk` GB (5) free under the root, and at least 1.2 times the last generation's size;
- **memory**, before training: `--min-free-memory` GB (4) available, and at least the training window's size on disk;
- **the workers** `--workers` names: each leased and reached through `cantrip-workers`. One that is not is a
  warning (the stages are played without it) unless `--require-workers`;
- **the champion's parity**, with the GPU server: the champion served with the serving options, falling back as the
  parity stage does ("Parity, and what a generation serves with"); self-play serves with what passed.

- **the mined targets** the mining window should hold ("When a stage fails"): an earlier generation whose mine or
  mined stage failed under this driver refuses here, unless `--allow-data-failure`.

A problem ends the driver before any stage runs. `generation.py ... --preflight` does only this: it builds, makes
every check for real, prints the plan as `--dry-run` does and runs nothing else (its logs under `ROOT/preflight/`).
An argument the driver does not know fails it at once, which is how the supervisor finds a stale argument file
before a generation and not during one. `--dry-run` makes the checks that start nothing (disk, memory, the pool).

### The engine a generation runs

A recording replays only with the engine that made it: the pair miner stands the gate's games in again position by
position, and one card's change makes a replay another game (`MismatchedPositions`). So each generation names its
engine once, when it starts - `engine` in `generation.json`: the commit and a hash of what the binaries are built
from, the Cargo workspace and what its build scripts read, and `dotnet/` - and keeps it (`engine.py`):

- after the build, what that build made is copied to `ROOT/builds/<id>/` (the Rust binaries, BotLab, BotLab
  published for the workers it published for, and the source tar a Linux worker builds the same engine from), as a
  Linux worker keeps its builds under `builds/<sha>`; every stage runs those files, and every stage's record carries
  the id. Only what this run's build stage built is taken - a binary the tree holds from some earlier build is no
  part of the engine. A kept file is replaced when the same source, built again, gives another, until a stage
  of the generation has begun; from then on what is pinned stays (the generation's ledgers name the binaries its
  games began with) and the difference is logged. `--no-build` builds nothing and so pins nothing: the stages then
  run the tree's files for as long as the tree is that source, their records name no engine, and the mine stage says
  that it cannot hold the gate's games to one (its verify step still leaves out every pair whose replay differs);
- a generation resumed after the working tree moved to another source says so, builds nothing, and goes on with its
  pinned build; if that build lacks something it needs, the driver stops and names the commit to check out;
- `--repin-engine` adopts the tree as it is now instead. The mine stage then refuses a gate another engine played
  (and its own recordings of another engine are made again): play the gate again, or `--no-mine`;
- retention deletes the builds no kept generation names; a pinned generation's and a champion's are kept.

Known limits of the pin, none of them reached while the loop alone moves the tree:

- **The Python that drives the stages** is the working tree's (`ai/`), not pinned.
- **The decklists and `content/`** a stage reads at run time (`--decks-dir`, BotLab's and the hosts' card data) are
  read from where they live; the id covers them as sources, the pinned build does not hold a copy.
- **The toolchains** (rustc, the .NET SDK, onnxruntime, CUDA) are not in the id: the same source built by another
  compiler has the same id.
- **The parity stage's Rust test** is built by `cargo test` from the working tree, not from the pinned build (it
  checks a model against its references; it replays nothing).

### Parity, and what a generation serves with

Below fp32 a GPU's rounding moves a network's outputs, by an amount that differs from network to network. The
server checks every model it holds against the model's references as it starts and refuses to start on one outside
the tolerance; the parity stage then runs the Rust test through it. The tolerance is never loosened. Instead the
check tries serving options in order - `--server-args` as configured, then each `--parity-fallback` in place of its
precision flags, by default

| option | measured (RTX 4080, T64, rows/s) |
|---|---|
| the configured flags (the templates line: `--precision fp16`) | 32.3k |
| `--precision fp16 --fp16-graph residual --inputs-tf32` | 34.7k |
| `--precision fp16 --fp16-graph attention --inputs-tf32` | 26.2k |
| `--precision fp16 --fp16-graph products` | 21.3k |
| `--precision fp32` | 15.1k |

- and the first that passes **for every model the generation's later servers hold together** (the candidate and the
champion, which the gate's and the miner's servers serve side by side, and the ladder's anchors), **in every server
that will hold them** (the generation's own, `--server-args`, and the one each host of a distributed duel runs,
`--duel-server-args`: the gate never serves flags that were not checked) is the generation's: `serving` in
`generation.json`, with what failed before it. The gate, the ladder, the miner's replays and judgements and the
workers' servers all serve with it; a fallback is flagged in the README and told. Models pass different options
(under the earlier bar, the maximum over the references, gen14 passed `residual` with TF32 inputs on one GPU and
gen13 failed it), which is why the check holds them together. With `--no-parity-fallback`, or when no option passes,
the stage fails as before. A worker whose own `server_args` name a precision keeps the rest of its arguments and
takes the fallback's precision.

Only a parity failure moves on to the next option: the server saying a model is not its references, or the test's
assertion on the served evaluator. A server that does not start for any other reason - the GPU out of memory, a
library missing, a timeout - fails the stage as that failure (the supervisor retries it), and is never answered
with a slower option for a whole generation.

### Running your own line

Everything above comes with the repository; a line of your own needs three things outside it.

1. **The arguments**, in `ROOT/generation.args` - the driver's options for every generation of the line, e.g.

   ```
   # my line: decklist mode, 3,000 games a generation
   --mode decklist --decks-dir ../ai-data/decks --games 3000 --simulations 64
   --device auto --window 4 --ladder-every 1
   --train-args "--batch 128"
   ```

   The supervisor's preflight checks them before every generation; an argument the driver does not know holds the
   loop at once.
2. **Where to be told** ("Watching a run"): a Discord webhook URL in `CANTRIP_AI_NOTIFY_WEBHOOK`, or a script in
   `CANTRIP_AI_NOTIFY_HOOK`, or both in the per-user `notify.json`; then
   `python -m cantrip_ai.notify --root ROOT --test "hello"`. With neither, everything is still in
   `ROOT/notifications.jsonl` and `python -m cantrip_ai.status`.
3. **The loop**, from `ai/`, detached: `python -m cantrip_ai.loop --root ROOT --update origin/main` (leave
   `--update` out to keep the code where it is; point it at your own branch to train on your own changes). Train from
   a working tree nobody edits - a second clone or `git worktree` - so that `--update` is the only thing that moves it.

Then: `python -m cantrip_ai.status --root ROOT` to look; `touch ROOT/LOOP.pause` to hold it at the next boundary;
`touch ROOT/LOOP.stop` to end it there; `python -m cantrip_ai.generation --root ROOT --stop` to stop a running
self-play or duel gracefully now (the loop is then held, and resumes the generation when `LOOP.pause` is removed).

### Testing the driver

`python -m unittest discover -s tests` (from `ai/`) runs the driver's unit tests: no game is played, nothing is
served, nothing waits on time. Before landing any change to the driver, the miner, the distributed code or the
supervisor, also run the end-to-end test:

```bash
CANTRIP_AI_E2E=1 python -m unittest tests.test_e2e_generation     # minutes, CPU only; CARGO_BUILD_JOBS bounds the build
```

It records a few games, trains a toy generation 0, and runs two real `--tiny` generations with the real binaries:
every stage, mining included, the distributed path through the worker manager's `local` machine (`--workers local`:
real ledgers, grants and hosts' results files), the first under the supervisor, on a root seeded with a scrubbed
copy of a real line's shape (`ai/tests/fixtures/line`: a champions history with labelled generations, a distributed
gate's results with their `#` header lines). It is skipped without `CANTRIP_AI_E2E` because it builds the engine;
`CANTRIP_AI_E2E_KEEP=DIR` leaves its root there to look at. Unit tests run the driver with its processes replaced;
this is the test that runs what they replace.

### Reading a generation

A generation's totals can all look level while it is going wrong. Three candidates in a row once lost their gates at
47 to 49% with level held-out numbers, because the value head had collapsed for one deck's seat: it valued a seat
that wins two games in three at log-odds near -5. Two readings showed it, and the driver now makes both every
generation (`ai/cantrip_ai/reports.py`), in the stage's record in `generation.json`, in the driver's log, and in the
README:

- **The gate by matchup** (and each ladder duel): from the duel's results file (`genN/logs/gate-results.tsv`, one
  machine's or the hosts' merged), the candidate's paired share in each pairing of decks, whichever seat holds which
  (the candidate plays both sides of a pair's deal), in the mirrors together and in the rest together: the pairs, the
  share and its 95% half-width, its z against a half, and the pairs lost outright, split and won outright. A pair is
  one observation, so the error is the pairs' own spread. A matchup more than 3 standard errors below a half is
  flagged (`LOW`); where every pair of a matchup ended alike there is no spread, and it is flagged from 10 pairs
  below a half. In the regression above the collapsed deck's mirror stood at 34 to 42% while everything else was
  level.
- **The held-out set by seat** (the evaluate stage): over the positions each deck's seat answered, the mean outcome
  for that seat and each network's mean value (both from -1 to 1), and for a network with a heuristic head
  (`train.py --residual`) that head's mean log-odds and the residual's. A network whose mean value for a seat is more
  than 0.15 from the seat's mean outcome is flagged (`OFF`). The evaluate row also gives the candidate less the
  champion on the same games, **paired by game**, for the value's MSE (every position) and the policy's CE (the
  full-search targets), each with a 95% half-width from the games' own differences (a game's positions are not
  independent): below zero the candidate is the better. `genN/heldout-eval.json` keeps each game's totals
  (`by_game`), and `python -m cantrip_ai.value_eval` prints the same tables for any checkpoints.

Whatever is flagged is listed first in the README, above the stage table, under `WARNING`, and logged as `WARNING:`
lines as the stage ends; the tables are below the stage table. A flag is a reading, not a verdict: it stops nothing
and promotes nothing. Read a flagged matchup beside that deck's seat in the held-out table: a seat valued far below
its outcomes plays its games as if they were lost. Neither reading costs a game: the first is arithmetic on the
gate's results, the second comes out of the pass over the held-out set the evaluate stage makes anyway.

### Distributed self-play

`--workers REQUEST` plays a generation's self-play on this machine and on other Windows or Linux machines, with
nothing run by hand on them (`ai/cantrip_ai/distributed.py`). Off by default; with it off, the driver runs self-play as above.

**The workers** are machines of the worker manager's pool (docs/workers.md), and `REQUEST` is what the driver asks
it for: `all` (every machine of the pool that is free), machines by name (`worker1,worker2`), or what a machine has to
have (`os=linux,gpu,rent`). The driver leases them for each distributed stage - self-play, each duel, the judgements -
as `generation:<root tag>`, renews the leases while the stage runs and releases them at its end
(`distributed.Leases`), so between stages a machine is free for others and a rented box nobody holds is destroyed. A
stage is played on what can be leased when it starts: a machine someone else holds, or a box that could not be
rented, is logged and done without (only a machine the pool does not have at all stops the run). It
knows a worker by its name and its root alone: every command it runs on one and every file it copies is a
`cantrip-workers exec`, `push` or `pull` under the worker's lease, and the addresses, the key and the vast.ai account
stay in the pool, outside the repository. A worker whose lease ended is one that cannot be reached (below).

A lease hands back the machine's `name` (in the logs, the shards' names and `HOSTS.json`), `os` (`windows` or `linux`:
"A Linux worker" below) and `root` (where everything of the driver's goes on it), and whatever the pool keeps under
the machine's `worker` for the driver: `lanes`
(its lane ceiling, `--worker-lanes` 256 otherwise: its governor scales below it to its first limiter), `device` (its
CUDA device, 0), `server_args` (its `cantrip-infer`'s; otherwise `--worker-server-args`, which defaults to
`--server-args` with `--server-impl native` and else to `--precision fp16 --pipelines 1`, the native server's settings
for self-play; `--shapes` is refused, since smaller batch shapes make a game depend on its batch companions),
`duel_parallel` (its games at once in a distributed duel: "Distributed duels" below), `judge_lanes` (its lanes in the
pair miner's judgements: "Distributed judgements"), `backend` (`gpu`, the default:
`cantrip-infer` on its GPU; or `tract`/`ort`: no server, the CPU). A workers file of before the pool is moved over
once with `cantrip-workers import` (docs/workers.md, "Moving an old workers file over").

**Setting up a worker, once** (Windows 10/11, as an administrator on it):

1. OpenSSH Server: `Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0`, then
   `Set-Service sshd -StartupType Automatic; Start-Service sshd` (the installer opens the firewall's port 22). Its
   default shell stays `cmd`: the driver pipes PowerShell scripts to it.
2. A local administrator account for the driver (`net user worker * /add`, `net localgroup Administrators worker /add`).
   Use a local account, not a domain one: an SSH key logon of a domain account asks the domain controller each time,
   and fails whenever the controller is unreachable (a laptop off the network, a VPN down), while a local account's
   logon is the machine's own.
3. Its key: an administrator's authorized keys are not read from the account's `.ssh` but from
   `C:\ProgramData\ssh\administrators_authorized_keys`, which sshd ignores unless only Administrators and SYSTEM can
   read it: put the public key there, then
   `icacls C:\ProgramData\ssh\administrators_authorized_keys /inheritance:r /grant "Administrators:F" /grant "SYSTEM:F"`.
   Then add the machine to the pool (docs/workers.md) and check from the driver's machine:
   `cantrip-workers lease NAME --holder me --purpose check`, `cantrip-workers exec LEASE -- hostname`,
   `cantrip-workers release LEASE`.
4. An NVIDIA driver recent enough for CUDA 12 (the bundle carries CUDA's and cuDNN's libraries; nothing else is
   installed), and the Visual C++ runtime (`vcruntime140.dll`, `msvcp140.dll`; present on most machines).

**What the driver pushes**, to `ROOT_REMOTE` (the worker's `root`), with `cantrip-workers push`, each file once: a bundle of what a worker
needs - `cantrip-selfplay.exe` and the DLLs beside it, `cantrip-infer.exe` (built with `--features gpu`; the build stage
builds it whenever `--workers` is given), onnxruntime-gpu's three DLLs (beside `--ort-dylib`), the seven CUDA libraries
`cantrip-infer` loads (`distributed.CUDA_DLLS`: cudart, cuBLAS, cuBLASLt, cuFFT, and cuDNN's front, graph and ops parts,
about 1.2 GB of PyTorch's `torch/lib`, which is `CANTRIP_CUDA_LIBS` or the driver's PyTorch's: the modules a running
server holds, and enough on a worker, where the references pass at fp16 and self-play runs for hours; `--worker-cuda-dlls`
names another set), the champion's model
directory (its graphs, manifest, cards and references; not its checkpoint), the decklists, and the generation's
pairings file. Every file is stored once as `ROOT_REMOTE/blobs/<sha256>` (checked by its hash on arrival), so what is
there already - the CUDA libraries after the first generation - is never sent again, and a generation's bundle is
`ROOT_REMOTE/bundle/<hash>/` made of hard links to them, its `MANIFEST.json` written last. Each generation's champion
stays in its own bundle; the last two bundles are kept and the blobs no kept bundle names are removed. The local hashes
are cached in `ROOT/dist/hashes.json`. The push (and a sweep that stops any run under this root's tag the ledger does
not know: a coordinator's orphans) happens before any attempt starts, so it holds up no host's grants.

**How a worker runs.** Each run on a worker (an attempt) is
`ROOT_REMOTE/gens/<root tag>/genN/<name>/<attempt>-<nonce>/` (the root tag is the generation root's name and a hash of
its path, so a scratch root never meets the real one; the nonce, so a directory is never an earlier one's). The driver
writes its `run.ps1`, `grant.txt` and `heartbeat` there, `job.json` last, and starts `run.ps1` with
`Win32_Process.Create` (CIM): Windows' OpenSSH puts a session's processes in a job object it kills on disconnect, and a
process WMI creates is outside it, so the run lives on after the SSH session ends. `run.ps1` starts `cantrip-infer` on
a free port, then `cantrip-selfplay --backend remote` on it with this machine's search settings, the governor on and
the worker's lane ceiling, records each process's id, start time and image in `pids.json` (written whole), and, however
it ends (`try`/`finally`), the self-play's exit code in `exit.json`, the server stopped. A process counts as alive, and
is killed, only when all three match: an id Windows reused is another process. A poll reads liveness first and
`exit.json` after, so a run that has just exited is never taken for a crash. Its logs are `infer.log` and
`selfplay.log` there (copied to `genN/dist/logs` when the attempt ends).

**A Linux worker** (`"os": "linux"` in the pool, and every rented box; `distributed.LinuxSshHost`) is a GPU machine or container
reached over SSH as a user that can install packages (root on a rented container), with an NVIDIA driver for CUDA 12 and
nothing else prepared: a fresh Ubuntu image will do. It keeps the same tree under its `root` and the same attempt files
as a Windows worker; its scripts are Python, piped to `python3 -` (installed if missing). Its bundle carries the
engine's source in place of binaries and libraries - `ROOT/dist/src/src-<hash>.tar.gz`, git's tracked files under
`Cargo.toml`, `Cargo.lock`, `crates/`, `content/` and `scenarios/` as this working tree has them (so what this machine
built), made the same way every time and remade only when a file's size or time moved - and the worker makes itself
ready in turn: once, its toolchain and libraries (`distributed.LINUX_SETUP`: Rust, a C compiler, onnxruntime-gpu with
cuDNN, cuRAND, cuFFT, cuBLAS and cudart in a venv under `root`; `ROOT_REMOTE/env.json` written once onnxruntime's CUDA
provider resolves every library, `setup.log`), then for each new source its build (`LINUX_BUILD`: `cargo build
--release`, cantrip-selfplay and cantrip-explore with `--features ort`, cantrip-infer with `--features gpu`, into
`ROOT_REMOTE/builds/<sha>/`, cargo's `target/` shared so a change rebuilds only what it touches; `build.log` there),
the binaries linked into the bundle's `bin/` before its manifest. Both run detached, and until they are done the worker
is "not ready" (the reason and the log's last line in the driver's log) and asked again every few minutes, so the other
hosts play on meanwhile: a first setup and build take ten to twenty minutes. Its attempts' supervisor is `run.py`,
started in a session of its own (`setsid`), which outlives the SSH session as `Win32_Process.Create`'s does on Windows;
a process is recorded by its id, its start (`/proc/<pid>/stat`) and its executable. The shards' header names no
binary, so a Linux build's games are the same as this machine's. A rented machine's effective CPUs are its CPU quota
(`/sys/fs/cgroup/cpu/cpu.cfs_quota_us` over the period, or `cpu.max`), not `nproc`: self-play scales to the first
limiter, and on a slice of a large host that is usually the CPU. Its duels and the pair miner's judgements run there
too ("Distributed duels", "Distributed judgements").

**A Linux worker's cores.** Its supervisor pins self-play to one hardware thread on each of as many physical cores as
the quota's whole CPUs allow, less three, and the server to both threads of those three (`distributed.pin_plan`; the
plan is the attempt's `pin.json`, and nothing is pinned where the topology cannot be read). A lane waits on the server
for a third of its time or more, so a box needs more lanes than cores; but two lanes on one core's sibling threads make
little more than one, and a quota counts thread-seconds, so lanes Linux places itself double up on siblings while
other cores idle, spend the quota for half the work, and get the container throttled, the server with it (measured on a
5090 with a 64-core EPYC: 2,533 requests a second as Linux placed 60 lanes, 3,246 with 100 lanes on one thread each of
58 cores, and 3,974 with the changes below too). The lanes are then `distributed.LANES_PER_CORE` (1.7) a pinned core,
all from the start; a worker's `lanes` sets them instead, `lanes_per_core` the factor, and `"pin": false` turns the
pinning off. Self-play also runs with `MIMALLOC_PURGE_DELAY=-1`: mimalloc otherwise returns each full search's tree to
the system and faults it back (about 250,000 faults a second over a box, 9% of its CPU). A worker's `selfplay_args`
(a list) are added to its self-play's arguments. When renting, count physical cores and the quota, not threads: a
5090 can serve more than 64 cores feed it, and a GPU's utilisation is no load signal (the server runs its fixed-shape
batches whatever arrives).

**The lease.** Every self-play the driver starts, local or remote, holds a lease (`cantrip-selfplay --lease`): a
`heartbeat` file the driver touches at each poll. Once it has not been touched for `--dist-lease` seconds (900), the
self-play stops gracefully on its own, so a driver that went away leaves nothing playing for long; one that comes back
within the lease re-attaches to its runs.

**The work split.** Game i is dealt from `seed + i` with its pairing, seats and first player from i alone, so it is the
same game on any host. Every self-play process plays only the indices its grant file grants (`cantrip-selfplay
--grant`), as the driver adds to it, until it says `end`. The driver's ledger, `genN/dist/ledger.json`, holds who was
granted what and which shards were accepted, and three rules make every index 0..N-1 played into `genN/selfplay`
exactly once, the same set of games as one machine plays:

1. an index is granted only while it is pending: not in an accepted shard, not a runaway, not dropped, and owned by no
   attempt, so at any moment at most one attempt owns it;
2. a shard is accepted only if its header is the one this binary and model write (`cantrip-selfplay --print-header`,
   asked before any game, never learnt from a shard) and every game in it is of an index its attempt owns and no
   accepted shard holds; it is moved into `genN/selfplay` first and recorded after, so an accepted shard is always
   there. Anything else is rejected whole and moved to `genN/dist/rejected`, never trained on, and counts against its
   host;
3. an attempt's unplayed indices go back to pending only when it is over: stopped (exit 3) or finished (exit 0) with
   its closed shards pulled, crashed with its `.part`s recovered there (`cantrip-selfplay --recover`) and pulled, or
   given up: unreachable past `--worker-timeout` (10 minutes), its recovery failing that long, or its launch never
   having started a process (no `job.json` after three minutes). What a given-up attempt sends later is rejected by
   rule 2. A grant never takes an index back from a live process.

A finished attempt's indices that are in none of its shards are games that failed: each goes back to pending, and one
that has failed on two attempts is dropped, as one run drops it; runaways are accounted as one run accounts them. A run
dropping games fast (three or more, and over a fifth of its games: its server gone, say) is stopped, fed no more, and
counted against its host; a host with three counts against it (crashes, losses, failed launches, dropping runs and
rejected shards) gets no new attempt this generation. A host whose attempt was given up while it may still run gets no
new attempt until that one is seen dead (killed, once it answers), so two servers never share its GPU. Grants go out on
demand, lowest index first, in chunks of `--dist-chunk` (the shard size): each running attempt is kept a backlog of
its lanes plus `--dist-lookahead` hours (0.05) of its measured rate, so a fast machine is never idle while a slow one
holds a long queue; at the end each has at most its backlog left. The ledger is written before a grant file, grant
files whole, and each poll compares a worker's grant file with the ledger's and writes it again when they differ, so a
write that failed (the closing `end` too) is made good, and an attempt whose grant was not written gets no more until
it is. A restarted driver picks up where it was: it re-reads the ledger, checks that every accepted shard is still in
`genN/selfplay` (one still in `genN/dist/incoming` is moved in; one that is nowhere is un-accepted, its games played
again), adopts any shard it had not recorded, and re-attaches to every run still going, local or remote. A lock,
`genN/dist/ledger.lock`, keeps a second driver off the same generation.

**Identity.** The ledger records what the self-play is: the model's graphs, the games and seed, the mode, every search
setting (simulations, full simulations and share, batches, sampled, temperature, the `--search` and `--full-search`
keys), the pairings and the decks (by content), and for each host its settings: a worker's bundle hash, backend,
server arguments and precision; this machine's `cantrip-selfplay` and its server's. A driver run again with any of these
changed is refused (a new host may join); run it as it began, or remove the generation's `selfplay/` and `dist/` to start
again. A generation whose self-play is distributed is not finished without `--workers` either: `--abandon-workers`
finishes it on this machine alone (stop the workers first).

A new attempt is granted one chunk to start with, and its backlog grows with its lanes as its governor scales them.

**Collecting.** Every `--dist-poll` seconds (30) the driver reads each attempt's status (`selfplay-status.json`), its
closed shards, runaways and exit; it pulls each new shard with `cantrip-workers pull`, reads it whole (every game, its seed in the
generation's range, its header this self-play's), checks rule 2, and moves it into `genN/selfplay` as
`<worker>-<name>`, where the training window reads it. Every few minutes it logs each host's games, games an hour,
lanes allowed and running, its limiter and its backlog. `genN/selfplay/HOSTS.json` records which host (and GPU) played
each shard's games: a different GPU can round fp16 differently, so a game replays exactly only on the machine that
played it, and its server's arguments and precision with it. `generation.json`'s self-play stage gains `hosts` (each
host's games, games an hour, lanes and limiter) and `attempts`, and the README's row names each host's part. A worker's
shards are deleted there once accepted (at the next poll). The worker manager's calls run in process groups of their own, so a Ctrl-C
never breaks a transfer: the interrupt waits for it, and a fetch or a stop that fails is tried again at the next poll.

A game reaches the driver only when its lane's shard closes (`--selfplay-shard-games`, 25 at gen5's settings), so
with 60-odd lanes a worker's first shards come in after a lane's 25 games (about half an hour at gen5's budget), and
then steadily; a stop closes every shard. Smaller shards bring them sooner, at more files.

**Measured** (2026-11-15, the templates line's gen4 champion at gen5's self-play settings - @64, full searches @4096 on
an eighth, Worlds=1 and Gumbel, fp16 - a 64-core worker with an RTX 4080 alone): its governor grew
to 62 lanes in about 25 minutes and held there, limited by the throughput plateau (the GPU), the system's CPU 38-41%
busy; at steady state 1,800 to 2,000 games an hour (565 to 744 games over six minutes), against this machine's 850-880
an hour at 42 lanes, CPU-bound. The on-demand grants follow each host's rate without a setting.

**Stopping.** Ctrl-C, or `python -m cantrip_ai.generation --root ROOT --stop` from another terminal (it writes
`genN/selfplay/selfplay.stop`, which this machine's self-play reads, and `genN/dist/stop`, which a distributed driver
reads: a file apart from the one a local run is stopped through, so the driver stopping one run that drops games is
never taken for a stop of them all), stops every host gracefully: the driver writes each worker's stop file, every
self-play finishes its games in flight and closes its shards, the driver pulls them, and exits with the stage not
done; run it again to resume. A second Ctrl-C (or `--stop-now`) stops the games in flight too; a third kills the local
self-play. `--no-selfplay-local` leaves this machine out of the self-play.

```bash
python -m cantrip_ai.generation --root ../../ai-data ... --workers all
python -m cantrip_ai.generation --root ../../ai-data --stop     # from another terminal
```

### Distributed duels

With `--workers`, the gate, the ladder's duels and the pair miner's replays are split across this
machine and the workers too (`ai/cantrip_ai/duels.py`; `--no-dist-duels` keeps them here, `--no-duel-local` leaves this
machine out of them). A duel's game is a function of its number and the duel's settings (its deal from `--seed` and
the number, its seats and decks from the number, its searches seeded by the number), so a duel is split as self-play
is, by the same coordinator, ledger, bundles and supervisors: each host runs BotLab's grant mode (`duel --grant FILE
--out DIR`, the grant file self-play's format) beside an inference server of its own, and returns its results in
closed files of a few games each (`*.results.tsv`, a line a game, each headed by the duel's header). The driver asks
BotLab for that header before any game (`duel --print-header`: the seats, the seeds, the decks, the pairings and the
models by content, no path) and accepts a file only if it carries it and its games are its host's to play. Pairs
(games 2k and 2k + 1) are granted whole, so each pair is played on one host. A game that does not finish is played
once more elsewhere, and a game that fails twice fails the duel, as it fails one machine's.

At the end the results are merged into `genN/logs/NAME-results.tsv` (the file one machine's `--results` writes, in
the order the games came in) and BotLab summarises it: `duel --summarize FILE` counts each line as a duel counts a game
as it ends, with the same code, and prints the same lines (all but the time an answer took, which no results file
holds), which go to `genN/logs/NAME.log`, where the driver reads the verdict as before; a last line names each host's
part. Played whole on one machine or split across several, a duel's games and summary are the same where the hosts
compute alike: `ai/tests/test_duels.py`'s `WholeAndSplitTest` plays a duel whole and split across this machine and a
worker (on the loopback) and checks it. A different GPU can round differently, so a game's evaluations, and so its
play, are exactly reproducible on the hardware that played it; every host of a duel runs `cantrip-infer` with the
same arguments (`--duel-server-args`; by default `--server-args` with `--server-impl native`, else `cantrip-infer`'s
own, fp32; `--shapes` refused), this machine's too, and the ledger's identity holds them.

A sequential stop (`--accept sprt`, `sprt-paired`, or `above-half` with `--gate-early-stop`) is the driver's: after each
poll that brought results it summarises them so far, in the order they came, and once a test has its verdict nothing
more is granted, every host is asked to stop starting games (its stop file), and the games in play finish and count,
as one machine's duel stops; the rest are skipped. The summary replays the stop over the merged lines, so its verdict
line is the driver's.

Stopping: `generation.py --stop` (or `--stop-now`) also writes `genN/dist/stop`, which the running duel's (or
judgement's) driver reads, a file apart from each host's own stop file (which the driver writes to close a duel or to
stop a host dropping games), so neither is taken for the other; the driver removes it when it starts a duel again. A
duel's ledger that holds a live attempt of a host the run no longer has (a worker the request no longer leases, this
machine with `--no-duel-local`) gives it up as it starts: its games go to the hosts there are, and what it sends later
is rejected.

The pair miner's replays of a distributed gate are distributed by the gate: each game is played again on the host
that played it in the gate (its ledger says which), with the recorder on (`--ai-record` in grant mode records into the
host's output, `RUN.replay/`, a directory with the results and the recordings closed together once the recorder has
written them, so no result comes home without its recording); a host that is gone (failed three times, out of reach
past `--worker-timeout`, or still setting itself up or building after two hours: a Linux worker reports that while it
does, and that is not being out of reach) has its games replayed by the others, and the miner's check of every replay against
the gate keeps or leaves out their pairs as it does any replay.

A worker's duel bundle is the self-play bundle's kind, pushed the same way, each file once: BotLab published
self-contained in one file (`dotnet publish -r win-x64` or `linux-x64`, by the build stage when `--workers` is given,
into `dotnet/tools/Cantrip.BotLab/bin/publish/`; with `--no-build` the driver stops with a word if it is not there), the native host (and the recording one for replays), the AI host,
`cantrip-infer` and its libraries, the duel's models, decklists and pairings; BotLab finds the bundle as its root by
`CANTRIP_ROOT`. A Linux worker builds the AI host, the native host and `cantrip-infer` from the bundle's source (and the
recording native host for replays) as it builds self-play's binaries, each once a source; a bundle counts as there
only with every binary its run needs (a replay's bundle is its gate's, which had no recording host). A worker's attempts of a
duel are `gens/<root tag>/genN/<name>/<duel>-<attempt>-<nonce>/`, and a duel's ledger is `genN/dist/<duel>/ledger.json`
(results in `genN/dist/<duel>/results/`): a restarted driver re-attaches as self-play's does, and a duel already
complete is only merged and summarised again. A worker plays `--worker-duel-parallel` (32) games at once, or its own
`duel_parallel`; on Linux, pinned as self-play is, `lanes_per_core` a pinned core. A duel's games without a decks
directory (`--decks-dir ''`, the fixed slice) are played here.

### Distributed judgements

With `--workers`, the pair miner's deep judgements (`cantrip-explore mine-judge`, `mining.judge_runs`: the forks twice
at the verdict budget, once deeper for targets, and the losers' decisions) are made within the `mine` stage across this
machine and every worker (`mining.judge_distributed`; `--no-judge-local` leaves this machine out), each run in turn
and each run's plan split among the hosts by game (a game's positions together; the games numbered by their recorded
directory and seed, as `mine-judge` sorts them), by the same coordinator as self-play and the duels, with a `Kind` of
its own (`mining.JUDGE`). A host's part is `mine-judge --grant FILE`: the plan's games the grant names (read once),
judged beside an inference server of its own (`--duel-server-args`, every host's the same), written into
`ATTEMPT.judged.part` and renamed `ATTEMPT.judged` once whole, with `items.txt` naming its games; the driver takes a
part whole, its header (the search's simulations, worlds, seed and targets, from its report) held to the run's, and a
part that lost a position to its inference server is refused and its games judged again. A position's judgement is
seeded by the judge's seed, its game and its decision, so a game judged in any part is judged the same (on the same
hardware): `cantrip-ai-host`'s `mine` test splits a plan in two parts and checks the judgements are the whole run's. The
parts are about a third of a host's share each, so a fast host takes more of them and a lost one costs its part
only. The accepted parts are merged into `genN/mine/judge/<run>/` as one machine writes it (`judged.jsonl` by
position id, the targets' shards under `<split>/<directory>/` named after their part), and the `mined` stage goes on
from there as before. A worker's bundle is the judge's (`cantrip-explore`, `cantrip-infer` and their libraries, or
on Linux the source it builds them from), the judge network, the decklists, and the plans and recordings (shards and
traces) under `data/`; a worker judges with `--worker-judge-lanes` lanes (4 x `--mine-lanes`, the four runs' lanes
when one worker made them all at once before), or its own `judge_lanes`; this machine with `--mine-local-lanes`. A
run's ledger is `genN/dist/judge-<run>/ledger.json`; a ledger of another plan (the stage made again) is started over,
as are the replays' of another selection. Without `--workers` the judgements are made here, as before.

### The gate, by pair

The gate's games come in pairs (games 2k and 2k+1). With `--gate-pairings metagame` (the default) each pair is the
next line of `genN/gate-pairings.txt`, drawn by metagame share as self-play's are (from the decks of `--decks-dir`
with at least 30 cards, as BotLab's `--decks` takes them), and both games deal that line's first
deck to seat 1 and its second to seat 2 from one shuffle, with the players exchanged: a true duplicate, so what the
deal gives a seat it gives each network once. `--gate-pairings mirrors` plays the decks as a pool of mirrors (pairs of
those are duplicates too). BotLab's `--pairings-keep-decks` keeps its old pairing, in which the decks moved with the
players (not a duplicate).

A pair scores the candidate 0, 0.5, 1, 1.5 or 2 points (a draw a half). The gate prints the five counts (the
pentanomial), the share of pairs split 1-1, the paired share (the mean of the pairs' points, halved) and its 95%
interval, `1.96 sqrt(var / N)` over N pairs with `var` the variance of a pair's halved points. Two networks that play
alike split most pairs, and a split pair says nothing about which is better, so the paired interval is narrower than
the unpaired one by as much as the pairs split (the gate prints the ratio of the two variances). The unpaired lines
are printed as before.

| `--accept` | promotes when | stops early |
|---|---|---|
| `sprt` | the unpaired SPRT accepts H1 (`--gate-h0` 50%, `--gate-h1` 52%, `--gate-alpha` and `--gate-beta` 0.05) | at its verdict |
| `sprt-paired` | the pentanomial SPRT accepts H1 (the same hypotheses, by pair) | at its verdict |
| `ahead` | the unpaired share is above a half by more than its margin | no |
| `above-half` (default) | the paired share is above a half ("anything over 50% is a win", the project's rule), decided from the five pair counts, not the printed share (an older log with no pairs: wins above losses) | with `--gate-early-stop`, once the paired interval excludes 50% either way (`--gate-stop-z` 2.58, after `--gate-stop-min-pairs` 100 pairs of which `--gate-stop-min-decisive` 30 did not split 1-1; BotLab's `--interval-z`, `--interval-min-pairs`, `--interval-min-decisive`) |
| `always` | always, as AlphaZero trained: no gate is played unless `--gate-measure` asks for it, as a measurement | no |

The pentanomial SPRT is the generalized SPRT chess engine testing (fishtest) uses for paired games: its LLR is N times
the mean log-ratio of the two most likely distributions on the five outcomes whose means are H1 and H0 (each in closed
form up to one root, `p_i = p̂_i / (1 + θ (x_i - s))`), with each count taken as at least 0.001, against Wald's bounds
`ln(β / (1 - α))` and `ln((1 - β) / α)`. A test that stops at its bound reads the step high at the stop. An interval
looked at after every pair is crossed by chance more often than its nominal rate, so the early stop's interval is 99%
(z 2.58) and waits for 30 pairs that did not split: the interval's normal approximation rests on those, and 96 split
pairs with 4 won are 52.0% +/- 1.9% at 95%, which would otherwise stop and promote on four pairs.
`--gate-early-stop` is a stop for clear results, not a 95% test.

### Measuring the parts

The gate compares a candidate with its champion. These say more, more cheaply:

- The permanent held-out set: choose it once, with `--heldout-games 1000` on the generation whose self-play it should
  come from (whole shards are moved, so the set is at least that many games); from then on every generation's
  `evaluate` stage scores the candidate and the champion on the same positions (`genN/heldout-eval.json`, the README's
  evaluate row). A set made by hand is named with `--heldout-dir DIR`; the window leaves that directory out, but a copy
  of shards still in a generation's `selfplay` would be trained on, so move them, as the driver does. To have
  `train.py` score the set during training as well, pass it through `--train-args '--eval-data ROOT/heldout'` (in
  place of `pi/eval`: `--eval-data` takes one list).
- The anchor ladder: `--ladder-every 1` plays every new champion against `--ladder-anchors` (gen0 and the
  tactical bot by default; any champion or model by name, a model directory, or a BotLab seat) for `--ladder-games`
  (1,000) at `--ladder-budget` (the self-play budget), and every `--ladder-play-every`-th time also at
  `--ladder-play-budget` (250ms, the play budget). The rows go to `ROOT/ladder.json` and `ladder.md`: a scale that
  does not move with the champion. Only a generation that promoted its candidate plays it: a rejected one would
  measure the champion an earlier ladder measured. `--ladder-every N` counts promotions (`champions/history.json`),
  not generations: the ladder after every N-th new champion. A resumed generation whose ladder is recorded keeps it.
  Each duel is also recorded by matchup ("Reading a generation").

#### A network-only duel

The two networks with no search, by pair: tens of thousands of games an hour, which resolves steps of about 1%. It
says how the networks' own answers compare, apart from what search makes of them. No stage plays it; when a
diagnosis wants one, play it by hand with BotLab (a seat named with no budget answers from the network alone;
`tools/bot-duel.sh` is `tools\bot-duel.cmd` on Windows):

```bash
tools/bot-duel.sh --a neural:decklist --b neural:decklist \
  --model ../ai-data/models/gen12 --model-b ../ai-data/champions/gen9 \
  --decks decks/standard --ai-prior decks/standard --pairings ../ai-data/gen12/gate-pairings.txt \
  --games 20000 --parallel 16 --seed 126000000 --results netduel-results.tsv
```

`--a` plays `--model` and `--b` `--model-b`; the seat is the generation's information mode (`neural` for full,
`neural:decklist`, `neural:hidden`), the same for both. `--pairings` makes each pair of games one deal with the
players exchanged, as the gate's are; any pairings file serves (the gate's is as long as the gate, and BotLab goes
round it again), or leave it out with `--mirrors` for a pool of mirrors. The summary's `Paired:` line is the result,
and `--results` keeps every game.

The fixed evaluation set `train.py` scores (`--eval-data`, which the fingerprint line points at `pi/eval`: the
tactical bot's games) measures the policy's distance from the tactical bot. Its policy CE is expected to rise as the
policy improves on the bot and leaves it, and that is not a regression by itself (gen3's 0.084 against gen1's 0.076
was read as "worse"); the README's train row says so beside it. Read the policy's progress on the held-out set's
full-search targets instead.

On a terminal the driver streams the running stage's progress; `--progress off` keeps it quiet and `--progress on`
forces it when piped.

### Common variations

```bash
--dry-run                          # print every stage's command and the window; run nothing
--games 2400                       # a smaller generation (default 5,000)
--lanes 8 --gate-parallel 8        # games at once: self-play's a ceiling it scales under (default 96; with
                                   #   --no-selfplay-autoscale one a CPU thread, two with the GPU server)
--selfplay-memory-budget 40        # self-play's governor (docs/ai-host.md, "The governor"): a private-memory cap (none),
--selfplay-memory-reserve 6        #   available memory kept free (6 GB), commit headroom kept free (6 GB), and the
--selfplay-memory-commit-reserve 6 #   floor that stops it gracefully (2 GB); a stopped self-play exits 3, the driver
--selfplay-memory-floor 2          #   says so and exits with the stage not done, and running it again resumes;
                                   #   Ctrl-C waits for self-play to close its shards (twice stops its games in
                                   #   flight, three times kills it), and the inference server, in a process group
                                   #   of its own, keeps serving them meanwhile
--selfplay-scale-window 45 --selfplay-scale-min-gain 0.2 --selfplay-cpu-ceiling 0.95   # how it scales its lanes
--gpu-server on|off                # the inference server for self-play, parity and the gate (default: on with an NVIDIA GPU)
--server-args '--flush-ms 1'       # anything else for serve.py
--train-steps 8000                 # more training (default from --train-hours at --train-rate)
--train-args '--batch 128'         # anything else for train.py
--decks-dir ''                     # the fixed rw-aggro/ub-reanimator slice instead of decks/standard
--older-weight 0 --imitation-weight 0   # train on this generation's self-play only
--mode hidden                      # an information mode other than full (hidden needs --decks-dir's prior)
--retrain 'value overfit'          # run a rejected generation again from training, keeping its self-play
--gate-early-stop                  # the default rule (above-half, paired) stopping once its 99% interval excludes 50%
--accept sprt                      # the unpaired SPRT (the default before 2026-11-13)
--accept sprt-paired --gate-h1 0.51     # the pentanomial SPRT
--accept always --gate-measure     # promote every candidate; the gate as a measurement only
--heldout-games 1000               # choose the permanent held-out set from this generation's self-play (once)
--ladder-every 1 --ladder-play-every 3   # each new champion against gen0 and the tactical bot; at 250ms every 3rd of them
--target-uses 4 --search-target-repeat 8   # steps by the search targets' exposure (Training options, below)
--schedule constant --carry-optimizer      # continuous training from the champion's optimizer state
--train-args '--anchor {champion} --anchor-kl 0.05'   # {champion} is the champion's checkpoint
```

### Training options

Options from the design review of 2026-11-13. Every one is off by default, and with all of them off training is what
it was. `train.py` takes each; the driver takes the ones marked "driver" itself and passes the others through
`--train-args`.

**Where a policy target came from.** The dataset keeps, per sample, whether its visits are a search's root visits
(`searched`: searched, not searched, or recorded, for a game with no search in it, the imitation corpus), whether its
decision was a fast search (`fast`), and the root's value of each option it measured (`q_offset`/`q_token`/`q_value`;
the column names and types are in `ai/cantrip_ai/dataset.py`'s docstring). Shard format 2 records both per sample
(`ai/shards.py` reads them as `Sample.searched` and `Sample.option_q`: the search's own flag, and its root options'
values); in a format 1 shard, which says neither, a sub-step counts as searched when its decision has a root value,
which also takes a sub-step the search never reached for a searched one. A **search target**
is a policy target (more than one legal option, not a fast search's) whose visits are a search's. The training log
says how many there are and whether they were flagged per sample; held-out evaluations report `search_choices`,
`search_policy_ce` and `search_top1`, the policy against the search targets alone.

| Option | Default | What it does |
|---|---|---|
| `--search-target-repeat R` (driver) | 1 | each search target dealt R times an epoch (the fraction by chance); the copies beyond the first train the policy alone: the value and the auxiliary losses skip them, so the value's exposure is not raised with the policy's |
| `--target-uses U` | off | the run's length in uses of each search target: U / R epochs. The driver's `--target-uses K` gives this generation K / (1 + (`--window` - 1) x `--older-weight`), so a target is used about K times in its life in the window, and drops `--train-hours`; `--train-steps` still wins |
| `--imitation-policy-weight W` (driver) | 1 | the policy loss's weight on samples of games with no search (the tactical bot's). The policy loss is the weighted mean over the weights' sum, so 0 takes the imitation corpus out of the policy (the loss is the self-played rows' alone), keeping its value targets |
| `--anchor CKPT --anchor-kl W` | off | KL(champion's policy, network's) at weight W on self-played positions with a choice that are no search target (the network's own answers, the fast searches'): the policy is held to the champion where no search says otherwise, and the anchor moves with the champion |
| `--surprise-weight S` | 0 | KataGo's policy surprise weighting on the search targets: weight (1 - S) + S x min(KL(visits, prior) / its running mean, 10). The running mean is over the targets (a batch counts by the targets it holds) and is used once it is over 64 of them (the targets weigh 1 before), so a first batch's few cannot set it; the cap keeps one surprising position from taking a batch's loss. The prior is `--anchor`'s (the network that played the games) when given, else the network's own |
| `--value-mix-full M`, `--value-mix-fast M` (driver) | `--value-mix` | the root value's share of the value target on a full search's positions (rooted, a policy target) and on a fast search's (rooted, no policy target); the review suggests 0.75 and 0.5. `--td-lambda` still replaces any mix. `--distill-mix D` mixes the distilled perfect-information value into the target the mix made, where one was distilled: (1 - D) x the mix's target + D x the distilled value |
| `--value-games-cap C` | off | a game's positions count in the value loss as at most C positions do an epoch: each weighted min(1, C / the game's samples dealt an epoch). A cap, not a reweighting: the value's exposure to a long game falls; with C below every game's length it is 1 / n weighting |
| `--value-loss bce` | `mse` | a binary cross-entropy on the value's win logit (value = tanh(logit / 2)) against the outcome's win probability (a draw is a half), in place of the squared error on tanh. Held-out value MSE is reported either way |
| `--future-weight W` | 0 | the future head (`targets.future`: life, power and hand a turn or two on), added to a checkpoint or a fresh network that has none; about 0.05 is a small weight |
| `--fate-weight W` | 0 | per-token fates (`targets.fates`, a training-only head): for a permanent, the share of its card's permanents under its controller still on the battlefield two turns on; for a card in the seat's own hand, the share of its copies there that have left the hand. The encoding gives an object no identity, so copies of one card are counted together, and "cast" cannot be told from "played" or "discarded": leaving the hand is what a later position shows |
| `--q-weight W` | 0 | a per-option Q head (`model.Config.q_head`, training-only, not in the graphs) trained towards the search's per-option root values where the data has them |
| `--policy-smoothing E` | 0 | E of each policy target spread evenly over the legal options |
| `--entropy-weight H` | 0 | the policy's entropy on its targets, rewarded at H; with smoothing, it keeps the policy from 1 - 1e-8 |
| `--distill-from CKPT` | off | grow a network by distillation: every position with a choice is a policy target towards the teacher's distribution, and the value's target is the teacher's value (`--distill-share S`, default 1, mixes them with the recorded targets). Train a new size with `--width` etc. and no `--init` |
| `--ema D` | 0 | an exponential moving average of the weights at decay D (0.999); the checkpoint's `state`, which export, the gate and the next generation read, is the average, its `train_state` the trained weights. Evaluations score the average. Its decay warms up, min(D, (1 + t) / (10 + t)) at its t-th update; an average carried from a checkpoint continues its count (`ema_updates`) |
| `--schedule constant` (driver) | `cosine` | the rate held after the warm-up instead of a cosine to zero |
| `--init-optimizer` (driver: `--carry-optimizer`) | off | continuous training: `--init`'s AdamW state carried by parameter name (a parameter that grew keeps its old part's moments), and no warm-up (the moments are already at the rate). The weights: with `--ema` too, the champion's unaveraged weights (`train_state`) are trained on and its average (`state`) continued; without `--ema`, the run starts from the gated model (`state`, the average where the champion kept one), as a fine-tuned run does. A resumed run (`--resume`) always continues its own unaveraged weights, and carries its optimizer by name when its parameters changed (a head added) |
| `--log-nums`, `--nums-clip C` | off; 8 fresh, 0 widened | the network reads each number clipped to +/- C (0: unclipped) beside sign(x) log(1 + abs(x)), in its input stage (`model.Config.nums_log`). A checkpoint without it is widened onto an unclipped linear copy (C 0 unless given), so it computes what it did for every input; a fresh network clips at 8. The graph gains `Log` (now on export's operator list); the encoding and its hash are unchanged |

A checkpoint written with these options keeps `param_names` (the optimizer's groups by name) and, under `--ema`,
`train_state`. One without the new configuration keys loads as before: every new head and reading is off.

Two notes. The window keeps an older generation's self-play at its weight by choosing its games again each
generation (`dataset._keep`, seeded by the generation), so a sample's life in the window is not the weights' product
for any one sample, only on average; `--target-uses` counts on the average. And the heuristic head (`--residual`) is
trained wherever a game notes its heuristic (`heuristic_values` in the game's meta, self-played games included, or a
reader's `Sample.heuristic`): self-play shards that note one train it on self-play positions with no change here.

`--tiny` is the end-to-end smoke test, at toy scale (a handful of games at 8 simulations, a few training steps, a
4-game gate). Run it against a copy of the data, so it can't touch the real champions:

```bash
cp -a ../../ai-data /tmp/ai-tiny-data
python -m cantrip_ai.generation --root /tmp/ai-tiny-data --tiny --no-build
```

`--tiny` still loads the whole training window, so it takes minutes, not seconds.

## Reanalyse: deep searches where they pay

`cantrip-reanalyse` (`crates/cantrip-ai-host/src/reanalyse.rs`) searches chosen positions of self-played games again
with the newest network, deeply and in bulk, and writes them as shards of search targets (MuZero's Reanalyse; the
review of 2026-11-13, item 8). The games need their traces: run self-play with `--traces` (the driver's
`--selfplay-traces`, off by default).

```bash
cantrip-reanalyse --selfplay ../../ai-data/gen4/selfplay --model ../../ai-data/models/gen4 --out ../../ai-data/gen4/reanalysed \
    --positions 20000 --simulations 4096 --server 127.0.0.1:7781
```

| Flag | Default | Meaning |
|---|---|---|
| `--selfplay DIR` | (required, repeatable) | self-play directories: their shards and `traces/` |
| `--model DIR` | (required) | the network that searches, and whose prior chooses |
| `--out DIR` | (required) | the shards, `reanalyse-report.json` and `reanalyse-positions.jsonl` (each chosen position and why) |
| `--positions N` | 1000 | positions searched in all |
| `--random-share P` | 0.25 | the share of them drawn uniformly from every candidate, so the choice is not all bias |
| `--flat P` | 0.6 | a position whose likeliest option has less prior than this is chosen (`flat`) |
| `--swing V` | 0.3 | a seat's root value moving this much between two searched decisions of a turn chooses both (`swing`) |
| `--before-swing N` | 3 | the loser's searched decisions taken up to its largest fall of root value (`loser`) |
| `--simulations N`, `--worlds N`, `--batch N` | 4096, 8, 32 | the deep search's budget, worlds and batch |
| `--lanes N`, `--seed N` | the cores, fixed | |
| `--backend ort\|tract\|remote`, `--server HOST:PORT`, `--threads N` | | as self-play's |
| `--prior DIR` | `decks/` | Hidden mode's prior |
| `--starts DIR` | | the scenarios of games self-play began from `--starts` positions |

A candidate is a decision of the learner (`GameMeta::learners`) with a choice that no full search answered. Besides
`flat`, `swing`, `loser` and `random`, a fast search whose most visited option is not the network's likeliest is
`fast-differs`. The prior criteria need the shard's card index to be the build's; otherwise only the value criteria
and the random share choose. Positions with more reasons go first.

Each game is stood in again from its trace with each seat's payment state and cancel guard as self-play kept them, and
each chosen position searched afresh. A shard of it holds only the chosen decisions' samples (their labels the game's
own, their visits and q the deep search's, `searched` set), the meta says `reanalysed`, and its decisions are in
`root_values` with no `fast_decisions`, so a training run reads them as full-search policy targets. Their positions
are already in the self-play shard as value targets, so the loader reads a reanalysed game's samples as policy targets
alone: no value, auxiliary, belief or heuristic loss, the mask the policy-only copies of `--search-target-repeat` use
(`dataset.unmark`). The meta keeps the source game's seed, start, decks and names, and the held-out split puts a
reanalysed game on its source's side (`dataset.split`, `dataset.source_key`), so a held-out game never trains the
policy through its reanalysed copy. A replayed position
whose encoding is not the shard's means the trace is not the game; the game's positions are dropped and counted
(`MismatchedPositions`). `AgreeShare` is how often the deep search's most visited option is the recorded one.

The imitation corpus is reanalysed the same way once it is recorded with traces (`cantrip-native-host
--ai-record-traces`, or `CANTRIP_AI_TRACES=1` in the bot lab's environment; docs/host-protocol.md). A table's player
pays some costs by tapping a land itself, which the factoring cannot say, so such a trace is cut short there: only the
decisions before the cut are candidates, and their samples keep the shard's labels (outcome, life swing, death next
turn, the opponents' hands) and answer, since the replay ends before the game did (`TracesCutShort` in the report).
A corpus recorded without traces cannot be stood in again.

## Exploration: what the policy excludes

`cantrip-explore` (`crates/cantrip-ai-host/src/explore.rs`) searches with a prior floor at the root (the review of
2026-11-13, item 9), by the search's own options: the audit is `search::audit_shortcut` (`RootFloor` under every root
option, `(1 - f) P + f / options`, the root's shortcut off), and verify floors the nominated action alone
(`search::Config::root_favour`: `(1 - f) P + f` on X, shared among the whole answers through it by their priors).
`floored` / `ActionSearchPrior` say the floor was applied (the search's `RootOption::prior` against `net_prior`).

**The shortcut audit.** Positions the priority shortcut passed without a search (every option but passing under the
shortcut's bound of the prior), drawn at random from traced self-play games (`--traces`), each searched in full with
the floor and the root's shortcut off. A line per position goes to `--out`; the summary counts how often the best excluded
option (with `--min-visits` visits) beats passing by `--margin` in value (`Wins`, `WinShare` with a 95% Wilson
interval), and how often the search would play an excluded option (`ChosenExcluded`). Rare, and the shortcut is fine;
common, and it hides plays.

```bash
cantrip-explore audit --traces ../../ai-data/gen4/selfplay --model ../../ai-data/models/gen4 --out audit.jsonl \
    --positions 2000 --server 127.0.0.1:7781
```

| Flag | Default | Meaning |
|---|---|---|
| `--traces DIR` | (required) | a directory of traces, or a self-play directory holding `traces/` |
| `--positions N` | 2000 | shortcut positions to audit |
| `--floor F` | 0.2 | the floor's share of the prior, spread over every root option |
| `--margin M`, `--min-visits N` | 0.05, 8 | what counts as an excluded option winning |
| `--shortcut P` | the search's (0.01) | the bound positions are found with (and the search's below the root): a larger one asks what a wider shortcut would hide |
| `--simulations N`, `--worlds N`, `--mode M` | 4096, 8, decklist | the audit's search |
| `--starts DIR` | | the scenarios of games begun at `--starts` positions |

**Verifying a nomination.** The verification step of the people-and-language-models pipeline: a position (a
`.cantrip` scenario's, as self-play's `--starts` takes it: once its last line is played) and an action X nominated
there, named by its words (`--action "lightning bolt"`, matched against each option completed into a whole answer by
the network) or by its place (`--action "#2"`). The search floors X's prior (`--floor`, 0.2) over `--worlds` (16)
worlds, and compares X with the search's most visited other option world by world - the same deal for both, a paired
comparison - giving the mean difference of their values with a 95% interval (Student's t) and a verdict: `better`
(the interval above 0), `worse` or `undecided`. A confirmed play is a policy target and a starting position for
self-play (`--starts`); a refuted one is kept as a regression case.

```bash
cantrip-explore verify --scenario positions/hold-the-bolt.cantrip --action "lightning bolt" --model ../../ai-data/models/gen4 \
    --worlds 32 --simulations 8192 --out verdict.json
```

Both take `--backend ort|tract|remote`, `--server HOST:PORT`, `--threads N`, `--prior DIR`, `--seed N` and `--batch N`.

## What to expect

Measured on one machine (16 CPU threads, an AMD RX 7900 XTX on ROCm, the standard decks, 64 simulations). Scale the
self-play and gate rows by your core count, and the training row by your GPU, or by about 4x slower on a 16-thread
CPU.

| Stage | Measured | A 4,800-game generation |
|---|---|---|
| self-play | 700-1,200 games an hour on 16 lanes | 5-8 h |
| self-play, through the GPU server | 20 k evaluations a second on 64 lanes (32 threads, an RTX 4080), against 3.3-3.7 k on the same CPU with ort: about 5.5x | |
| train | 1,700 samples a second on the GPU (445 on 10 CPU threads) | about 15 min on the GPU |
| export, parity | seconds, a couple of minutes | |
| gate | neural against neural at 64 simulations | about 1.3 h at the unpaired SPRT's expected 3,700 games at H1 52%; at most 2.1 h for 6,000 (47 games a minute with the GPU server); a paired test, less by as much as the pairs split |

## Troubleshooting

- **A run went quiet.** `python -m cantrip_ai.status --root ROOT` says whether it is running, stalled, dead or
  failed, with the error and the stage's log ("Watching a run"). Under the supervisor, `ROOT/LOOP.pause` holds the
  reason the loop was held; remove it to go on.
- **`... was made with other settings`.** A distributed stage's ledger was started with other settings than this
  run's (another `--server-args`, say). A ledger that holds no games yet is replaced by itself, with a log line; one
  that holds games is kept and refused: run with the settings it began with, or remove that stage's `genN/dist/NAME`
  to play it again.
- **`this generation's engine is ...`.** The working tree changed under a generation and its pinned build lacks a
  binary: check out the commit the message names and run again, or `--repin-engine` ("The engine a generation runs").
- **`parity (...): no serving option passes`.** Every option was tried against every model the server holds; the
  attempts' logs are `genN/logs/parity-tryK.log` and `server-candidate-tryK.log`. Add an option with
  `--parity-fallback`, never a looser tolerance.
- **`torch.cuda.is_available()` is False.** NVIDIA: `nvidia-smi` should list the card, and the PyTorch build's CUDA
  version must be one the driver supports. AMD: `rocminfo | grep gfx` should list it. A consumer card ROCm doesn't
  list may need `HSA_OVERRIDE_GFX_VERSION` (`11.0.0` for RDNA3, `10.3.0` for RDNA2). Check that the CPU build of torch
  isn't the one installed (`torch.version.cuda` and `torch.version.hip` are both None on it).
- **Out of GPU memory.** Lower the batch: `--train-args '--batch 128'` (default 256).
- **No dotnet.** Install the .NET SDK, or set `CANTRIP_DOTNET` to the command that runs it, e.g.
  `CANTRIP_DOTNET='nix shell nixpkgs#dotnet-sdk_10 -c dotnet'`.
- **The gate won't load a deck.** BotLab keeps decklists of at least 30 cards. `cantrip-botlab` reports a malformed
  list, and self-play reports the same file through `load_deck`.
- **`... names no card index`.** The shard was recorded before shards named their card index. Put the `cards.json`
  of the index it names (its header's `text_hash`) in the data directory, `ai-data/cards.json`; any directory above
  the shard works. A model directory's own `cards.json` serves an old checkpoint the same way.
- **A model refuses to load: schema or encoder hash.** The model was trained against another observation encoding
  (`ai/encoding.json`, `cantrip_obs::encode::encoder_hash`). Retrain it; a newer card set alone is accepted.
