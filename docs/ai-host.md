# The AI host

`cantrip-ai-host` (`crates/cantrip-ai-host`) is the AI player, run as a process of its own beside a table. It keeps
a **mirror** of each game the table hosts. When asked, it answers a seat's request with a network, alone or by a
search over it (`cantrip_ai::search`). A search never holds the rules host's lock. The plan it serves is `docs/ai-player-plan.md` ("Serving and measuring").

```
cantrip-ai-host --model DIR [--model DIR ...] [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--workers N]
    [--prior DIR]
```

- **`--model DIR`** is a model directory: `manifest.json` and `model-t*.onnx`, as `ai/cantrip_ai/export` writes it.
  Each model is loaded once, at start. `Model::load` checks each one against this build's encoding, so a mismatched
  schema or encoder hash is refused. A model is named by its directory's name (`gen0`).
- **`--backend`**: `ort` is ONNX Runtime on the CPU, and is the default of a build with `--features ort`. `tract`
  is pure Rust, and is the only backend of a plain build. `remote` evaluates on the inference server (below), in any
  build.
- **`--server HOST:PORT`** is the inference server `remote` evaluates on; giving it implies `--backend remote`.
- **`--threads`** is the evaluator's intra-op threads.
- **`--workers`** is the answering threads. The default for both is 1.
- **`--prior DIR`** is the metagame's decklists (`decks/`, every `.txt` under it, each weighed alike;
  `Prior::from_decks`): what a Decklist or Hidden search deals the opponent's unknown cards from (`determinize`).
  A Hidden search needs it; without it such an `AiStart` is refused, since an empty prior would deal the
  opponent's own library and leak it.

### The inference server

`--backend remote` sends every evaluation to `python -m cantrip_ai.serve` (`ai/cantrip_ai/serve.py`), which holds the
networks on the GPU and runs the rows of every connection together in batches (docs/ai-player-plan.md, "Decisions";
how to run it is docs/ai-training.md, "The GPU inference server"). The client is `cantrip_ai::RemoteEvaluator`
(`crates/cantrip-ai/src/remote.rs`, which documents the wire protocol):

- **The transport** is TCP, on `127.0.0.1` when the server is on the same machine, on Windows and Linux alike. Frames
  are a u32 length and the bytes.
- **The hello** names the model by its `manifest.json` (FNV-1a 64 of the file) and carries this build's encoding schema
  and encoder hash. The server refuses a model it does not serve, and an encoding that is not its own, so a
  mismatched model is refused at start, as `Model::load` refuses it locally. The model directory is still read here:
  its card index maps the runtime's card rows to the model's, as the local backends map them, before they are sent.
- **A batch on the wire** is each row's real tokens only (the padding after its last real token is the server's to
  fill in), in narrow types: kind, categoricals and pointers as bytes, card rows as u16 (u32 past 65,535), the legal
  bits packed, and the numbers sparse: each token's count of non-zero numbers, their features and their values as f16,
  which is exact for an encoded position (a number f16 cannot hold makes the batch go dense in f32 instead). A
  position is about 2 KB, against 12 KB dense in f16 and 41 KB as the graphs' i64 and f32 tensors (measured on 64
  held-out positions). The rows go grouped by bucket, so the server takes each bucket's rows as one run. The reply is
  the value, aux and the logits of the legal tokens only, all f32; every other token's logit is `MASKED` in every
  backend, so the policy is rebuilt exactly.
- **Each row runs at its own bucket**, the smallest that holds its real tokens, padded to a fixed batch size per
  bucket with deterministic kernels, so its outputs never depend on what it was batched with. (The local backends run
  a batch at its largest row's bucket.) The same game played twice through the same server is the same game.
- The evaluator keeps a pool of connections, so each of the host's workers has its own while it evaluates.

### The native inference server

`cantrip-infer` (`crates/cantrip-ai-host/src/bin/infer.rs`, built with `--features gpu`) serves the same protocol as
serve.py, so `--backend remote --server`, BotLab's `--ai-server` and `CANTRIP_AI_SERVER` use either. It is
`cantrip_ai::gpu` (`crates/cantrip-ai/src/gpu.rs`) behind a TCP front: onnxruntime's CUDA provider, no Python on the
request path. A connection is a thread that decodes its frames straight into the engine's rows and waits on them.

```
cantrip-infer serve --model DIR [--model DIR ...] [--port 7710] [--precision fp32|tf32|fp16]
    [--batch N | 32:N,64:N,128:N] [--flush-ms 2] [--pipelines 2] [--device 0] [--no-check] [--ready-file PATH]
cantrip-infer bench --model DIR [--clients 64] [--rows 8] [--seconds 20]     # the engine alone, on the references
cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] # two servers on the same positions
```

- **Libraries.** onnxruntime is loaded at run time: `ORT_DYLIB_PATH` (or `--ort-dylib`) names an `onnxruntime.dll` built
  with the CUDA provider, `CANTRIP_CUDA_LIBS` (or `--cuda-libs`) the directory of CUDA 12's and cuDNN 9's libraries.
  onnxruntime-gpu 1.24.4 from PyPI is built for CUDA 12 (1.25 on are built for CUDA 13); unpack its wheel and take
  `onnxruntime/capi/onnxruntime.dll`. PyTorch's `torch/lib` has CUDA 12's and cuDNN 9's libraries. The ROCm path stays
  serve.py's: the engine is the CUDA provider only.
- **Batching** is serve.py's: a queue per model and bucket, a GPU call padded to the bucket's fixed batch size, run
  when full, when every client is waiting and nothing runs, or after `--flush-ms`; `--pipelines` workers, each with its
  own sessions and CUDA stream. A row's outputs never depend on its companions (one shape per bucket, deterministic
  compute); the tests check it bitwise.
- **One launch a batch.** Each session's inputs and outputs are bound to device buffers, so it replays a captured CUDA
  graph instead of launching ~190 kernels; the batch is written into pinned host buffers and copied on a stream of the
  worker's own. The graph takes kind, categoricals and pointers as bytes and the card row as i32 (widened on the
  device, the same values).
- **Precision.** `fp16` casts every MatMul and Gemm to half precision (inputs to half, output back to float), what
  PyTorch's autocast does to them; everything else stays fp32. The graph is rewritten in memory; the model directory is
  never written. At start the references are held to export's 1e-3 at fp32 and to serve.py's reduced bar below it.
- **Self-play in one process.** `cantrip-selfplay --backend gpu [--precision fp16] [--gpu-batch ...] [--pipelines N]`
  (built with `--features gpu`) runs the same engine inside self-play, every lane an evaluator: no sockets, and each
  position goes to the engine at its own length (`Evaluator::evaluate_positions`) instead of padded to a batch.

The binary installs a counting allocator (`cantrip_ai::alloc`), so a search's `TreeBytes` is measured, not estimated.

stderr is the log. Build it with ONNX Runtime using `cargo build --profile cards -p cantrip-ai-host --features ort`.
A plain `cargo test --workspace` rebuilds the binary tract-only, so rebuild it with `ort` before a duel.
`tools/bot-duel.sh` does that for you.

## The protocol

The protocol is JSON lines, as the native host's is (`docs/host-protocol.md`):

- Each command gets one reply, in order: `{"Ok": ...}` or `{"Rejected": {"Code", "Detail"}}`.
- Separately, each answer asked for gets a line of its own, `{"AiResult": ...}`. A worker thread writes it when the
  answer is ready.

### Mirrored commands

These are the native host's commands, sent to the AI host as well as to the table. The mirror is made and stepped by
the same code (`cantrip_host::session::Session`) on the same commands in the same order. It is the same game, in the
same handle spaces. A view mints handles, so `View` is mirrored too.

- **`NewGame`** takes `GameId`, `Seats` (`Name`, `Deck`, `Sideboard`), `Seed`, `Shuffle`, `FirstPlayer`, `Chooser`,
  `BasicLandsOutside`, or `Scenario` (`Name`, `Text`, `Driven`). The native host's fields mean what they mean there.
  The AI host adds three:
  - **`GameId`** is the table's id for the game. If it is left out, the host uses the next of its own.
  - **`Echo`**: when true, every mirrored command is answered with the table's whole reply (the steps and views),
    which lets a caller check the mirror against the table. When false, the reply is `{"Ok": {}}`.
  - **`Replay`** is a list of mirrored commands without their `GameId` (`Start`, `Answer`, `Concede`, `View`). They are
    run at once, which brings a mirror made late to where the table is. With `Echo`, the reply's `Last` is the last of
    their replies.
- **`Start`**, **`Answer`**, **`Concede`** and **`View`**, each with its `GameId`.
- **`Close`** forgets the game.

### Asking

- **`Models`** replies `{"Backend": "ort", "Models": ["gen0"]}`.
- **`AiStart`** asks for an answer to the outstanding request: `{"GameId", "Seat", "Model"?, "Mode"?, "Deadline"?,
  "Temperature"?, "SampleSeed"?, "Simulations"?, "SearchMs"?, "MemoryMb"?, "Worlds"?, "SearchSeed"?, "CPuct"?,
  "PriorTemperature"?, "Batch"?, "Prune"?, "PruneBad"?, "PruneMargin"?, "PruneAfter"?, "PruneAfterReplies"?}`.
  - The reply comes at once, `{"Ok": {"Started": {"RequestId", "Mode"}}}`, or a rejection:
    - `NotYourTurnToAnswer`: the request is another seat's.
    - `StaleReply`: nothing is outstanding.
    - `InvalidSetup`: no such model or mode, or a Hidden search with no `--prior`.
    - `TableClosed`.
  - `Model` is a model's name; the default is the first one loaded.
  - `Mode` is `Full`, `Decklist` or `Hidden`; the default is `Full`. It is what the network is shown of the game
    (`cantrip_obs::InfoMode`).
  - `Deadline` is in milliseconds from now. After it passes, no step asks the network, and the rest of the answer is
    first-legal (`Late`).
  - `Temperature` samples from the policy instead of taking the best option, seeded by `SampleSeed`. With a search
    it samples from the root's visits (`visits^(1/T)`) instead of taking the most visited.
  - **The search budget.** With `Simulations` (a count) or `SearchMs` (milliseconds), or both (whichever ends
    first), the answer is searched (below). With neither, it is the network's alone, as before.
    - `Deadline` is still the hard limit: the search stops at `SearchMs` or `Deadline`, whichever is sooner, and
      the network's fallback answer is given until `Deadline`. The deadline is read before each simulation and
      between the requests a line plays with no choice, so a search ends soon after it however long its lines are.
    - `MemoryMb` (2048) bounds what the seat's tree may hold, its part kept from the seat's last search included
      (`search::Config::memory`): reached, the search stops as at its deadline, and a kept tree holding more than
      half of it is let go and the search begins afresh. 0 is no bound.
    - `Worlds` (default 8) is the worlds of a Decklist or Hidden search; Full searches one.
    - `CPuct` (1.5), `PriorTemperature` (1.0) and `Batch` (8, leaves an evaluation) tune it.
    - `Prune` (true), `PruneBad` (-0.80), `PruneMargin` (0.20), `PruneAfter` (2) and `PruneAfterReplies` (1) tune
      branch trimming (`docs/ai-player-plan.md`, "Branch trimming").
    - `SearchSeed` reseeds the search's forks. The default is `(GameId << 32) ^ RequestId ^ 0x5ea2c4`, so the same
      game asks the same search of the same request.
- **`AiResult`** fields:
  - `GameId`, `Seat` and `RequestId`.
  - `Answer`: an `AnswerPayload` in the seat's own handles, which the table takes as it would a client's.
  - `Factored`: the factored answer, as debug text.
  - `Fallback`: some step fell back to first-legal, after the validator refused 8 walks.
  - `Late`: the deadline passed.
  - `Minted`: the answer named an object the seat had no handle for. This should never happen; the tests check it.
  - `Value`: the value head on the request's first step, for the seat.
  - `Evaluations`, `Steps` and `Rejected` (the walks refused).
  - `EncodeUs`, `EvaluateUs`, `QueuedUs` and `TotalUs` (from the command's arrival to the answer).
  - `Search`, for an answer that was searched: `Simulations`, `Evaluations`, `Batches`, `Nodes` (the tree's
    sub-step nodes), `Positions` (requests reached), `GameNodes` (what the forks' game trees grew by), `TreeBytes`
    (the peak heap the search held, `Counted` when the counting allocator measured it; `MemoryBound` when `MemoryMb`
    stopped the search, `KeptDropped` when the kept tree was let go for it), `RootValue` (the root's
    visits' mean for the seat), `NetValue` (the network's value of the root), `Worlds`, `Shortcut` (the priority
    shortcut answered pass), `Refused` (the engine refused the searched answer, so the network's was given),
    `Pruned`, `PrunedImmediate` and `PrunedShallow` (edges branch trimming cut) and `SearchUs`. A request with one legal answer, a payment, and an answer the shortcut gives before any simulation
    have no `Search`, or a `Search` with zero `Simulations`.
  - `Error`, when the network could not answer. With an error, `Fallback` is true, and `Answer` is first-legal when
    there is one.

  The table's request may be answered before the AI's answer is ready. The result is then an `Error` and the caller
  drops it.

### What the answer is

- With no budget the answer is network-only: `cantrip_ai::net::answer`. It walks the request's factoring
  (`cantrip_obs::factor`) one step at a time, and at each step takes the policy's best legal option, or a sampled
  one.
- With a budget it is searched: `cantrip_ai::search::search`, PUCT over the factoring's sub-steps in forks of the
  game (docs/ai-player-plan.md, "Search", has the whole of it). Requests with one legal answer, payments and the
  priority shortcut make no nodes; values are backed up from each node's seat's side; leaves are evaluated in
  batches with virtual loss. The answer is the most visited option at each root sub-step, the network's at a
  sub-step the search never reached, and the network's alone when the engine refuses it or no simulation ran in
  time.
- **Paying** is not asked of the network. `cantrip_ai::pay` pays from the pool, and pays X, convoke and delve.
- **Tapping** a mana source chooses its sources by a matching of the cost's colored and hybrid symbols to what each
  source can make (`pay::tap`). A dual land is not spent on a color a basic could pay. A land whose yield is fixed
  (basics, Plateau, `mana: None` on the option) is read from its ability's `adds`.
- **A payment is decided once**, at its first `PayCosts` or `UnlessPays` request, and kept to over the requests that
  follow (`net::Payment`, kept per seat in the mirror).
- **The cancel guard.** A cast or activation cancelled part way is undone (CR 733.1), and asked again the AI may
  choose the same action again, forever. That happens when `pay` cannot finish a payment (Grim Monolith's "{4}:
  Untap"), when the AI answers a cost's own choice by cancelling (a card to discard, a mode), or when a copy is cast
  during resolution and its target is cancelled (Uldaros Theorix and Bitter Triumph).
  - `net::Payment`, per seat, keeps the board before the action (`cancel::board_hash`: the position less the loop
    watch, the RNG, the next timestamp and the append-only history a cancel leaves) and the card whose cast or
    activation began. A staged cast can be cancelled at more than one request, so it keeps a list of boards, each with
    its own cancelled cards.
  - A cancel records that board and card. While the board stands, the card's casts and activations are filtered from
    the AI's requests, and a priority answer that still names it becomes a pass (a fallback). An `UnlessPays` whose
    source is filtered keeps only `DeclineCost`, so an unpayable ward is declined rather than paid and cancelled again.
  - The board after the undo matches the board before the action, so the same action is not offered again; a new
    board offers it again. Nothing is capped per turn and no card is named in the AI.
- **Naming a card.** A `ChooseName` question must be answered with a card name (CR 201.3): the core refuses
  `Answer::Name(None)`, and the native host refuses an empty name (`{"Name": ""}`, `crates/cantrip-host/src/answer.rs`)
  rather than looking up the name "", which a nameless token's card matched. The network's "no name" token
  (`Tok::NoName`) is the question's default name (`cantrip_rules::default_name`): its suggestion, or the first name it
  allows, alphabetically.

## In BotLab

- `cantrip-botlab duel --a neural --b tactical ...`, or `tools/bot-duel.sh` on Linux, plays a `neural` seat.
  - The seat can also be `neural:decklist` or `neural:hidden`; `neural` means Full.
  - `@N` searches with N simulations an answer, `@Nms` for N ms an answer: `--a neural@64 --b neural` is search
    against the same network alone. `--ai-worlds N`, `--ai-batch N`, `--ai-cpuct X`, `--ai-prior-temperature X`,
    `--ai-prune true|false`, `--ai-prune-bad X`, `--ai-prune-margin X`, `--ai-prune-after N` and
    `--ai-prune-after-replies N` go to every searching seat. The AI host is started with `--prior decks`.
  - `--sprt` stops the duel once the sequential test has a verdict (H0 50%, H1 55%, alpha = beta = 0.05, a draw half
    each way; docs/ai-player-plan.md, "Measuring").
  - Each worker runs its own pair of hosts. The table's transport tees every command to the AI host
    (`dotnet/tools/Cantrip.BotLab/Neural.cs`).
  - The AI host is found by `CANTRIP_AI_HOST`, or under `target/<profile>/`.
- `cantrip-botlab watch --a neural ...` plays one game with a neural seat, its own AI host beside it. With `--seed`
  and `--game`, it replays one game of a duel.
- Options:
  - `--model DIR` (the default is `../ai-data/models/gen0`) and `--ai-backend ort|tract`.
  - `--ai-server HOST:PORT` has every AI host evaluate on the inference server there (`--backend remote`), which
    must serve every model the run names (`--model` and `--model-b`). The AI hosts then wait on the GPU, not the CPU,
    so `--parallel` can be more than the cores.
  - `--model-b DIR` plays the b seat's neural seats with a second model, so `--a neural@64 --b neural@64 --model
    models/gen1 --model-b champions/gen0` is a generation's gate. Both models are loaded by one AI host and named by
    their directories, which must differ.
  - `--ai-deadline MS`.
  - `--ai-echo` checks every mirrored reply against the table's.
  - `CANTRIP_AI_TRACE=FILE` writes every question and its answer.
- The report adds a split of each answer's time, for both seats:
  - the native host's part and the seat's own part;
  - for the network: encoding, evaluating, the rest of the AI host, and the process round trip;
  - steps and evaluations per answer, and the fallback, late and error counts;
  - per neural seat: for a searching one, the simulations, nodes and tree memory of its searched answers, and its
    searched decisions and evaluations a second.

## Self-play

`cantrip-selfplay` (`crates/cantrip-ai-host/src/bin/selfplay.rs`, `cantrip_ai_host::selfplay`) is the expert-iteration
loop's game maker (docs/ai-player-plan.md, Phase 4). Both seats search with one model, and every finished game is
written to shards with the search's root visits as the policy target.

```
cantrip-selfplay --model DIR --out DIR [--games N] [--hours H] [--simulations N] [--lanes N] [--seed N]
    [--pairing DECK:DECK ...] [--decks DIR] [--sampled N] [--temperature T] [--mode full|decklist|hidden] [--worlds N]
    [--max-turns N] [--max-answers N] [--backend ort|tract|remote|gpu] [--server HOST:PORT] [--threads N] [--prior DIR]
    [--resume] [--shard-games N] [--precision fp32|tf32|fp16] [--gpu-batch N|32:N,64:N,128:N] [--pipelines N]
    [--flush-ms MS] [--device N]
```

- It runs in process, not as a table with an AI host beside it. Each lane is a thread with its own evaluator and
  shard writer, and plays the engine's `Game` against the search's `Decision` directly. There is no mirror, no JSON
  and no handles, and the visits never leave the process that made them.
- The budget is a simulation count (`--simulations`, 64 by default), never a deadline, so the data does not depend on
  the machine's load.
- A seat's first `--sampled` (20) searched decisions in a game are sampled from the root's visits at `--temperature`
  (1). The rest take the most visited option.
- A game past `--max-turns` (60) or `--max-answers` (6000) is closed as a cap, with every outcome 0, and counted.
- Pairings default to rw-aggro against ub-reanimator and both mirrors, dealt in turn. Game `i` is dealt from shuffle
  `seed + i`, with the seats alternating.
- The shard header's producer is `selfplay:<model hash>@<simulations>`.
- `--resume` reads the closed shards already in `OUT` and skips the games whose seeds they hold. A game's pairing,
  seats and first player come from its index, so the rest of the schedule is unchanged. A `.part` shard that reads
  whole is recovered and renamed; one that ends inside a game is ignored, and its games are played again. The report's
  `Resumed` and `GamesThisRun` say what was which.
- `--shard-games` (100) bounds the games in an open shard, and so the work lost to an interruption.
- A sub-step the search visited is labelled with its visit distribution, which sums to 1 over the legal options. Any
  other sub-step is labelled with the option chosen. Each searched decision's root value goes in the game's meta
  (`root_values`).
- The report, printed and written to `OUT/selfplay-report.json`, has the games, samples, visit samples, caps, drops,
  errors, searched decisions, simulations, evaluations, network fallbacks, games and samples an hour, wins by seat and
  deck, the answers that made no sample, and on a resumed run `Resumed` and `GamesThisRun`.
- `--backend remote --server HOST:PORT` has every lane evaluate on the inference server, each lane a connection; the
  server batches the leaves of all of them. A lane then spends most of its time waiting on the GPU, so more lanes than
  cores keep it busy (docs/ai-training.md has the measured defaults). At a seed, a run through the same server plays
  the same games, whatever the lanes: `cargo run --release -p cantrip-ai --example shard_games -- DIR` prints each
  game's seed and a hash of its bytes, so two runs are compared with `diff`.
- `--backend gpu` (a build with `--features gpu`) runs the native server's engine in the self-play process itself
  (above, "The native inference server"): every lane an evaluator of one engine, no sockets. The engine's options are
  cantrip-infer's (`--precision`, `--gpu-batch`, `--pipelines`, `--flush-ms`, `--device`); its references are checked
  at start as the server checks them, and its throughput is printed at the end.
- On Windows it allocates with mimalloc: the system heap serialised the lanes' allocations (32 lanes made half the
  evaluations a second with it). The Linux build keeps the system allocator.
- Build it with `cargo build --release -p cantrip-ai-host --features ort`.

## Tests

`crates/cantrip-ai-host/tests/ai_host.rs` runs with the tiny model in `crates/cantrip-ai/tests/data/tiny`. It checks:

- over stdio, that a scenario is mirrored, asked and answered;
- in process, that a dealt game played entirely by the network at a `Session` gives the same reply as the mirror to
  every command, views included;
- that a mirror made late by `Replay` matches from then on;
- that a searched `AiStart` (`Simulations`) carries its `Search` and the table takes every answer, and that a Hidden
  search with no prior is refused.

`crates/cantrip-ai-host/tests/selfplay.rs` plays self-play with the tiny model at 8 simulations. It checks:

- that every sample's visits sum to 1, every visited option is legal, and the root values lie in [-1, 1];
- that a run of two capped games writes shards that read back, with `selfplay:tiny@8` and `full` in the header, and
  a report.

`crates/cantrip-ai/tests/remote.rs` checks the remote evaluator against a server written in the test (tract, each row
at its own bucket): a batch arrives as the same tensors (mapped card rows, wide card rows, numbers f16 cannot hold, and
a cut-short frame refused), the references come back within export's tolerances and bitwise what tract makes of each
position alone, threads share an evaluator, and another model or encoder is refused at the hello.
`ai/tests/test_serve.py` checks the server itself; `model_parity` (`crates/cantrip-ai/tests/eval.rs`) runs a real
model's references through a running server when `CANTRIP_AI_SERVER` names it.

`crates/cantrip-ai/tests/search.rs` checks the search itself (docs/ai-player-plan.md, Phase 3).

`crates/cantrip-ai/tests/pay.rs` checks the matching: a Plateau and a Mountain pay {R}{W}.
`crates/cantrip-host/tests/scenarios_at_a_table.rs` (`an_empty_card_name_is_refused`) checks that an empty card name is
refused.
