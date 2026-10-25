# Training the AI player

How to train the AI player's network on any machine: an NVIDIA GPU (CUDA), an AMD GPU (ROCm), or the CPU alone. The
design is [ai-player-plan.md](ai-player-plan.md) ("Training", Phases 2-4); the AI host and self-play binary are
[ai-host.md](ai-host.md). This is the runbook.

Training runs in **generations** of expert iteration. The current champion plays itself with search, a new network is
trained on those games, and the new network replaces the champion only if it beats it in a gate. A GPU speeds up only
the training stage. Self-play, the parity check and the gate run the network with ONNX Runtime on the CPU, so a
machine with many cores matters as much as a GPU.

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
next one. Its eight stages:

| Stage | What it does |
|---|---|
| build | `cantrip-ai-host` and `cantrip-selfplay` (release, `--features ort`), the native host, and BotLab; on every run, so a resumed run never uses a stale binary |
| selfplay | the champion plays itself with search (`--games`, `--simulations`, `--lanes`), shards to `genN/selfplay` |
| train | fine-tunes the champion on the window of recent generations (`--window`, `--older-weight`, `--imitation-weight`) |
| export | ONNX models and the manifest, to `models/genN` |
| parity | the Rust evaluators against PyTorch on held-out positions |
| gate | the candidate against the champion in BotLab, an SPRT (H0 50%, H1 55%) over at most `--gate-games` |
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
- `genN/logs/{selfplay,train,export,parity,gate}.log`: the full output (`tail -f` them to watch);
- `genN/generation.json`: the same numbers, machine-readable;
- `models/genN/`: the exported ONNX model and its `manifest.json`;
- `champions/current`, a symlink to the champion in force, and `champions/history.json`, every gate.

On a terminal the driver streams the running stage's progress; `--progress off` keeps it quiet and `--progress on`
forces it when piped.

### Common variations

```bash
--dry-run                          # print every stage's command and the window; run nothing
--games 2400                       # a smaller generation (default 5,000)
--lanes 8 --gate-parallel 8        # games at once (default: one a CPU thread)
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
| train | 1,700 samples a second on the GPU (445 on 10 CPU threads) | about 15 min on the GPU |
| export, parity | seconds, a couple of minutes | |
| gate | neural against neural at 64 simulations | 0.5-1.5 h for 1,200 games |

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
