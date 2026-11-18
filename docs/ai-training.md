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
| build | `cantrip-ai-host` and `cantrip-selfplay` (release, `--features ort`), the native host, and BotLab (and `cantrip-infer` with the native server or `--workers`); on every run, so a resumed run never uses a stale binary |
| selfplay | the champion plays itself with search (`--games`, `--simulations`, `--lanes`), shards to `genN/selfplay`; with `--workers`, on other machines too ("Distributed self-play") |
| heldout (optional, once) | `--heldout-games N` chooses the permanent held-out set: whole shards of this generation's self-play, before any network trains on them, moved to `ROOT/heldout` (`--heldout-dir`) with `HELDOUT.json` |
| train | fine-tunes the champion on the window of recent generations (`--window`, `--older-weight`, `--imitation-weight`); the window never holds the held-out set |
| export | ONNX models and the manifest, to `models/genN` |
| parity | the Rust evaluators against PyTorch on held-out positions, and with the GPU server, the server too (it checks the references itself as it starts) |
| gate | the candidate against the champion in BotLab on self-play's metagame pairings, scored by pair, by `--accept` (below) over at most `--gate-games` (6,000) |
| promote | an accepted candidate goes to `champions/genN` and becomes `champions/current` |
| netduel (optional) | `--net-duel-games N`: the candidate's network against the champion's, no search, by pair: a measurement |
| evaluate (when a held-out set exists) | `value_eval` on the held-out set, candidate and champion: value MSE and ECE, policy CE and top-1 against its full searches' targets |
| ladder (optional) | `--ladder-every N`: the champion against fixed anchors, recorded in `ROOT/ladder.json` and `ladder.md` (a generation run again replaces its rows) |
| retire | deletes the shards and models of all but the last `--keep` generations (gen0 and pinned ones are kept) |

The measurements (netduel, evaluate, ladder) run after the promotion and decide nothing: one that fails is recorded as
failed (`"failed"` in `generation.json`, a row of the README) and the generation goes on to the next stage.

A rerun skips the stages `genN/generation.json` records as done, so an interrupted generation resumes. Self-play keeps
the shards it closed and plays only the games they don't hold. Training continues from its last checkpoint (every 200
steps), with the optimizer, step and epoch. A gate that finished but wasn't recorded is read back from its log, when
the log is this run's gate: the same champion and seat, the same acceptance rule (its stop line, with the early stop's
z and minimums) and the same pairing mode.

To run generation after generation:

```bash
while python -m cantrip_ai.generation --root ../../ai-data --device auto; do :; done
```

What a generation leaves in `ai-data/`:

- `genN/README.md`: the stage table and its numbers (the first thing to read);
- `genN/logs/{selfplay,train,export,parity,gate}.log`: the full output (`tail -f` them to watch), and with the GPU
  server `server-{selfplay,parity,gate}.log`, its throughput; `netduel.log`, `evaluate.log` and `ladder-*.log` for the
  measurements that ran, and `genN/*-pairings.txt` the matchups each duel played;
- `genN/generation.json`: the same numbers, machine-readable;
- `models/genN/`: the exported ONNX model and its `manifest.json`;
- `champions/current`, a symlink to the champion in force, and `champions/history.json`, every gate (paired fields
  included);
- `heldout/` (once chosen) and `ladder.json` / `ladder.md` (when the ladder runs).

### Distributed self-play

`--workers FILE` plays a generation's self-play on this machine and on other Windows machines over SSH, with nothing
run by hand on them (`ai/cantrip_ai/distributed.py`). Off by default; with it off, the driver runs self-play as above.

**The workers file** (e.g. `ai-data/compute/workers.json`):

```json
{
  "key": "C:/Users/me/.ssh/cantrip_workers_ed25519",
  "workers": [
    {"name": "worker1", "host": "192.0.2.10", "user": "worker", "root": "D:/compute"}
  ]
}
```

`key` (top level, or a worker's own) is the SSH key; a worker has `name` (in the logs, the shards' names and
`HOSTS.json`), `host`, `user`, `root` (where everything of the driver's goes on it), and optionally `port`, `lanes`
(its lane ceiling, `--worker-lanes` 256 otherwise: its governor scales below it to its first limiter), `device` (its
CUDA device, 0), `server_args` (its `cantrip-infer`'s; otherwise `--worker-server-args`, which defaults to
`--server-args` with `--server-impl native` and else to `--precision fp16 --pipelines 1`, the native server's settings
for self-play; `--shapes` is refused, since smaller batch shapes make a game depend on its batch companions), `backend` (`gpu`, the default:
`cantrip-infer` on its GPU; or `tract`/`ort`: no server, the CPU), `enabled: false` to leave it out, and anything else
as notes.

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
   Check from the driver's machine: `ssh -i KEY worker@HOST hostname`.
4. An NVIDIA driver recent enough for CUDA 12 (the bundle carries CUDA's and cuDNN's libraries; nothing else is
   installed), and the Visual C++ runtime (`vcruntime140.dll`, `msvcp140.dll`; present on most machines).

**What the driver pushes**, to `ROOT_REMOTE` (the worker's `root`), with `scp`, each file once: a bundle of what a worker
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
closed shards, runaways and exit; it pulls each new shard with `scp`, reads it whole (every game, its seed in the
generation's range, its header this self-play's), checks rule 2, and moves it into `genN/selfplay` as
`<worker>-<name>`, where the training window reads it. Every few minutes it logs each host's games, games an hour,
lanes allowed and running, its limiter and its backlog. `genN/selfplay/HOSTS.json` records which host (and GPU) played
each shard's games: a different GPU can round fp16 differently, so a game replays exactly only on the machine that
played it, and its server's arguments and precision with it. `generation.json`'s self-play stage gains `hosts` (each
host's games, games an hour, lanes and limiter) and `attempts`, and the README's row names each host's part. A worker's
shards are deleted there once accepted (at the next poll). ssh and scp run in process groups of their own, so a Ctrl-C
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
`genN/selfplay/selfplay.stop`), stops every host gracefully: the driver writes each worker's stop file, every
self-play finishes its games in flight and closes its shards, the driver pulls them, and exits with the stage not
done; run it again to resume. A second Ctrl-C (or `--stop-now`) stops the games in flight too; a third kills the local
self-play. `--no-selfplay-local` leaves this machine out of the self-play.

```bash
python -m cantrip_ai.generation --root ../../ai-data ... --workers ../../ai-data/compute/workers.json
python -m cantrip_ai.generation --root ../../ai-data --stop     # from another terminal
```

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

- `--net-duel-games N`: the two networks with no search, paired; tens of thousands of games an hour, which resolves
  steps of about 1%.
- The permanent held-out set: choose it once, with `--heldout-games 1000` on the generation whose self-play it should
  come from (whole shards are moved, so the set is at least that many games); from then on every generation's
  `evaluate` stage scores the candidate and the champion on the same positions (`genN/heldout-eval.json`, the README's
  evaluate row). A set made by hand is named with `--heldout-dir DIR`; the window leaves that directory out, but a copy
  of shards still in a generation's `selfplay` would be trained on, so move them, as the driver does. To have
  `train.py` score the set during training as well, pass it through `--train-args '--eval-data ROOT/heldout'` (in
  place of `pi/eval`: `--eval-data` takes one list).
- The anchor ladder: `--ladder-every 2` plays the champion after promotion against `--ladder-anchors` (gen0 and the
  tactical bot by default; any champion or model by name, a model directory, or a BotLab seat) for `--ladder-games`
  (1,000) at `--ladder-budget` (the self-play budget), and every `--ladder-play-every`-th time also at
  `--ladder-play-budget` (250ms, the play budget). The rows go to `ROOT/ladder.json` and `ladder.md`: a scale that
  does not move with the champion.

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
--net-duel-games 20000             # the networks alone, by pair, every generation
--heldout-games 1000               # choose the permanent held-out set from this generation's self-play (once)
--ladder-every 2 --ladder-play-every 3   # the champion against gen0 and the tactical bot; at 250ms every 6th generation
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
