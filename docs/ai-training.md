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
  32:256,64:128,128:64` and `--pipelines 2`.
- **Reproducible.** Every batch is padded to its bucket's fixed size and the kernels are deterministic, so a position's
  outputs do not depend on what it was batched with, and self-play at a seed plays the same games on every run through
  the same server settings (batch sizes, precision, GPU and PyTorch). `--precision` is `fp32` by default, held to
  export's 1e-3 on every output. Below it the references are held to what the search uses (`serve.REDUCED`): value,
  dead-next-turn and each option's probability within 1e-2, the life swing (which no search reads) within half a point,
  the same best option on 99.5%. Measured on gen0's 512 references, the largest drift:

  | Precision | Value | Life swing | Legal logits | Option probability | Same best |
  | --- | --- | --- | --- | --- | --- |
  | fp32 | 3e-6 | 1e-4 | 4e-5 | 2e-6 | 512/512 |
  | tf32 | 2e-3 | 0.08 | 0.04 | 4e-3 | 512/512 |
  | fp16 | 4e-3 | 0.12 | 0.08 | 4e-3 | 512/512 |
  | bf16 | 3e-2 | 0.63 | 0.98 | 3e-2 | 511/512 (refused) |

  fp16 and tf32 pass on gen0, gen1 and the bench model; bf16 (an 8-bit mantissa) is refused. Each precision is
  reproducible bit for bit in itself, not across precisions.
- **Lanes**: a lane spends most of its time waiting on the server, so give self-play more lanes than threads. On 32
  threads and an RTX 4080, 16 lanes made 11.8 k evaluations a second, 32 made 17.8 k, 64 made 20.3 k and 128 made
  21.1 k, against 3.3-3.7 k on the CPU; the driver's default with the server is two lanes a thread. The server prints
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
next one. Its eight stages (with the GPU server, self-play, parity and the gate each run beside a server of their own,
started and stopped by the driver):

| Stage | What it does |
|---|---|
| build | `cantrip-ai-host` and `cantrip-selfplay` (release, `--features ort`), the native host, and BotLab; on every run, so a resumed run never uses a stale binary |
| selfplay | the champion plays itself with search (`--games`, `--simulations`, `--lanes`), shards to `genN/selfplay` |
| train | fine-tunes the champion on the window of recent generations (`--window`, `--older-weight`, `--imitation-weight`) |
| export | ONNX models and the manifest, to `models/genN` |
| parity | the Rust evaluators against PyTorch on held-out positions, and with the GPU server, the server too (it checks the references itself as it starts) |
| gate | the candidate against the champion in BotLab, an SPRT (H0 50%, H1 `--gate-h1`, 52%: Magic's variance hides a 55% step) over at most `--gate-games` (6,000) |
| promote | an accepted candidate goes to `champions/genN` and becomes `champions/current` |
| retire | deletes the shards and models of all but the last `--keep` generations (gen0 and pinned ones are kept) |

A rerun skips the stages `genN/generation.json` records as done, so an interrupted generation resumes. Self-play keeps
the shards it closed and plays only the games they don't hold. Training continues from its last checkpoint (every 200
steps), with the optimizer, step and epoch. A gate that finished but wasn't recorded is read back from its log.

To run generation after generation:

```bash
while python -m cantrip_ai.generation --root ../../ai-data --device auto; do :; done
```

What a generation leaves in `ai-data/`:

- `genN/README.md`: the stage table and its numbers (the first thing to read);
- `genN/logs/{selfplay,train,export,parity,gate}.log`: the full output (`tail -f` them to watch), and with the GPU
  server `server-{selfplay,parity,gate}.log`, its throughput;
- `genN/generation.json`: the same numbers, machine-readable;
- `models/genN/`: the exported ONNX model and its `manifest.json`;
- `champions/current`, a symlink to the champion in force, and `champions/history.json`, every gate.

On a terminal the driver streams the running stage's progress; `--progress off` keeps it quiet and `--progress on`
forces it when piped.

### Common variations

```bash
--dry-run                          # print every stage's command and the window; run nothing
--games 2400                       # a smaller generation (default 5,000)
--lanes 8 --gate-parallel 8        # games at once (default: one a CPU thread; with the GPU server, two lanes a thread)
--gpu-server on|off                # the inference server for self-play, parity and the gate (default: on with an NVIDIA GPU)
--server-args '--flush-ms 1'       # anything else for serve.py
--train-steps 8000                 # more training (default from --train-hours at --train-rate)
--train-args '--batch 128'         # anything else for train.py
--decks-dir ''                     # the fixed rw-aggro/ub-reanimator slice instead of decks/standard
--older-weight 0 --imitation-weight 0   # train on this generation's self-play only
--mode hidden                      # an information mode other than full (hidden needs --decks-dir's prior)
--retrain 'value overfit'          # run a rejected generation again from training, keeping its self-play
```

`--tiny` is the end-to-end smoke test, at toy scale (a handful of games at 8 simulations, a few training steps, a
4-game gate). Run it against a copy of the data, so it can't touch the real champions:

```bash
cp -a ../../ai-data /tmp/ai-tiny-data
python -m cantrip_ai.generation --root /tmp/ai-tiny-data --tiny --no-build
```

`--tiny` still loads the whole training window, so it takes minutes, not seconds.

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
| gate | neural against neural at 64 simulations | about 1.3 h at the SPRT's expected 3,700 games at H1 52%; at most 2.1 h for 6,000 (47 games a minute with the GPU server) |

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
