# The AI host

`cantrip-ai-host` (`crates/cantrip-ai-host`) is the AI player, run as a process of its own beside a table. It keeps
a **mirror** of each game the table hosts. When asked, it answers a seat's request with a network, alone or by a
search over it (`cantrip_ai::search`). A search never holds the rules host's lock. The plan it serves is `docs/ai-player-plan.md` ("Serving and measuring").

```
cantrip-ai-host --model DIR [--model DIR ...] [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--workers N]
    [--prior DIR]
```

- **`--model DIR`** is a model directory: `manifest.json` and `model-t*.onnx`, as `ai/cantrip_ai/export` writes it.
  Export writes a graph for each of `GRAPH_BUCKETS` (32, 64, 96, 128). A directory exported before T96 has none for 96
  and serves those rows at 128 as before; `python -m cantrip_ai.export --add-graphs DIR` adds the graph from its
  `model.pt`, checked against PyTorch on its references. The manifest changes, and with it the model's identity: the
  remote protocol's (a hash of `manifest.json`) and self-play's (`model_id`, over the manifest and the `*.onnx` files,
  which `--resume` holds a run to). Run it between generations, with no server or self-play using the model and no
  self-play run of it left to resume.
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
    [--batch N | 32:N,64:N,96:N,128:N] [--shapes N,N,... | T:N/N,... | none] [--flush-ms 2] [--pipelines 2]
    [--device 0] [--no-check] [--ready-file PATH]
cantrip-infer bench --model DIR [--clients 64] [--rows 8|A-B] [--mix T:share,...] [--think-ms MS] [--seconds 20]
    # the engine alone, on the references; --mix and --think-ms make self-play's traffic
cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] # two servers on the same positions
cantrip-infer check --model DIR [engine options]                             # start, hold to the references, stop
    # engine options also: [--fp16-graph products|attention|ffn|residual] [--gelu gelu|biasgelu] [--no-cuda-graphs]
cantrip-infer time --model DIR [--bucket 128] [--batch N] [--seconds 20]     # one session on one stream: rows a second
cantrip-infer graph --model DIR [--bucket 128] --out FILE [engine options]   # the graph as the engine runs it
```

- **Libraries.** onnxruntime is loaded at run time: `ORT_DYLIB_PATH` (or `--ort-dylib`) names an `onnxruntime.dll` built
  with the CUDA provider, `CANTRIP_CUDA_LIBS` (or `--cuda-libs`) the directory of CUDA 12's and cuDNN 9's libraries.
  onnxruntime-gpu 1.24.4 from PyPI is built for CUDA 12 (1.25 on are built for CUDA 13); unpack its wheel and take
  `onnxruntime/capi/onnxruntime.dll`. PyTorch's `torch/lib` has CUDA 12's and cuDNN 9's libraries. The ROCm path stays
  serve.py's: the engine is the CUDA provider only.
- **Batching** is serve.py's: a queue per model and bucket, a GPU call padded to the bucket's fixed batch size, run
  when full, when every client is waiting and nothing runs, or after `--flush-ms`; `--pipelines` workers, each with its
  own sessions and CUDA stream. A row's outputs never depend on its companions (one shape per bucket, deterministic
  compute); the tests check it bitwise, on the tiny model and on trained ones.
- **Batch sizes.** T96 and T128 batch 48 rows by default (T32 256, T64 128; before 2026-11-15 T96 was 96 and T128
  64, serve.py's). Self-play's requests are small (a world, a search batch of 16, an evaluation cache), so a batch is
  usually run partial, and a partial batch costs the GPU as much as a full one: at 96 and 64, 47% of the padded rows
  were real at 42 lanes (gen5's self-play here, and `bench` made to match it: `--clients 42 --rows 1-16 --mix
  96:57,128:43 --think-ms 30`), with the GPU 92-97% busy. A row costs about the same at 48 as at 96 (gen4 at fp16 on an
  RTX 4080, `cantrip-infer time`: T96 21.2k rows a second at 48, 20.9k at 96, 17.6k at 32; T128 14.9k at 48, 16.5k at
  64), so 48 halves a partial batch's cost and gives a full one up only at T128. With two pipelines (the default; one
  assembles and copies while the other's batch runs) batch 48 was as fast as or faster than every other setting
  measured at every load: at 42 clients 9.4k rows a second against 8.7k (latency 7.8 ms against 10.7), at 64 14.1k
  against 13.1k, at 96 18.8k against 17.9k, and saturated (no think time) 19.0-19.6k against 16.5-18.1k for 96/64
  on one pipeline (`--pipelines 1`, which the templates line's gen.sh passes) and 19.1k on two. It is one shape a
  bucket, so as deterministic as before. With the GPU shared (another generation's gate serving beside it) two
  pipelines lost 10-20% at every size, and one pipeline at 96/64 and at 48 measured within each other's spread:
  `bench` at 42 clients 8.4-8.5k against 7.9-8.1k, a real self-play of gen4 at 32 games 6.6-6.8k rows a second (56-61%
  real) against 6.6k (70% real). So: batch 48 and two pipelines where the server has the GPU to itself (self-play),
  one pipeline where it shares it. `--flush-ms` 4 measured the same as 2, and 8 worse (fewer rows a second, more
  latency): the flush is not what leaves batches partial. Raising the larger buckets' batch does not help either: a
  row costs more at 192 or 256 than at 96.
- **Smaller shapes (opt-in, not guaranteed deterministic).** `--shapes 48,64,80` (every bucket) or `--shapes
  96:48/64/80,128:48` (by bucket) also captures those smaller shapes, each a CUDA graph of the bucket's one session
  (onnxruntime's `gpu_graph_id`: a session a shape had cost 0.8 GB a session), and runs a partial batch at the
  smallest that holds it. A shape is kept only if, at start, every output of up to 512 probe rows (the references that
  run at the bucket, then longer ones cut to it) is bitwise the full shape's; the start says which it dropped, and
  warns while any is in use. That is evidence, not proof: a shape's matrix products run other kernels than the full
  shape's, and kernels that add in another order can agree on almost every row (on gen4 at fp16, T128 at 16 rows
  differed from 64 on 3 references of 209, on gen2 on 1 of 210). Every engine runs cuBLAS without a workspace
  (`CUBLAS_WORKSPACE_CONFIG=:0:0`, so no split-K; a value already set is kept; at one shape it costs nothing
  measurable), with which nearly every shape of 48 rows or more agrees with the full one on the probe, where without it
  nearly none did. It measured no faster than batch 48 with two pipelines (shared or not); it is there to be measured
  again where full batches matter more.
- **One launch a batch.** Each session's inputs and outputs are bound to device buffers, so it replays a captured CUDA
  graph instead of launching ~190 kernels; the batch is written into pinned host buffers and copied on a stream of the
  worker's own. The graph takes kind, categoricals and pointers as bytes and the card row as i32 (widened on the
  device, the same values).
- **Precision.** `fp16` casts every MatMul and Gemm to half precision (inputs to half, output back to float), what
  PyTorch's autocast does to them, and `--fp16-graph` takes more into half, each level with the ones before it:
  - `products`: the matrix products alone (`--unfused-attention` says the same);
  - `attention`: each attention fused into onnxruntime's `MultiHeadAttention` (cutlass's memory-efficient kernel where
    the head size is a multiple of 8, else onnxruntime's unfused one; both accumulate the scores and softmax in fp32):
    what autocast and `scaled_dot_product_attention` do;
  - `ffn` (the default): also each GELU after a MatMul fused with its bias, so the feed-forward networks' widest
    tensors stay half. `--gelu gelu` (the default) adds the bias in half and runs onnxruntime's `Gelu`, erf's GELU
    computed in fp32 and rounded once, as autocast does (the session turns off onnxruntime's `BiasGeluFusion`, which
    would make the pair `BiasGelu` again); `--gelu biasgelu` (opt-in) is one kernel, `BiasGelu`, whose half kernel
    computes erf's GELU in half arithmetic (a few roundings where autocast makes one). On an RTX 4080 at T128, batch
    64, gen1 runs 17.3k rows a second at `ffn` against 14.5k at `attention` and 11.7k at `products`; `biasgelu` is 7%
    faster (18.7k) but its largest probability drift is 4.5 times `gelu`'s (0.0082 against 0.0018), a
    margin the larger networks to come would eat;
  - `residual` (opt-in only): also each residual Add fused with the norm that reads it (`SkipLayerNormalization`, in
    onnxruntime's strict mode: the sum and the statistics in fp32), so the residual stream is half, rounded once a
    sublayer, where autocast keeps it in fp32. It is faster again (21.0k on gen1) but fails the reduced bar on gen2 of the templates line (3 references non-finite; under the policy bar before 2026-11-13 it failed gen0 too, probability 0.015),
    and on gen1 doubles the logits' and the life swing's drift, so it is not the default.

  Half nodes read each other's outputs directly, and Reshapes and Transposes move half tensors as they are; the
  masking and the value and aux heads' non-linearities stay fp32, and so does the input stage at every level: every
  node the first norm's input is computed from (the card table, the embeddings, the numbers' and the pointers'
  projections). Those read the raw features, which are unbounded (a count is scaled linearly), and gen2's pointers
  projection overflowed fp16 on a card with 131,700 counters; everything after the first norm reads normalised
  activations. `--inputs-tf32` lets those fp32 products use TF32. serve.py keeps the same stage fp32 under autocast.
  The startup check fails a model with any output not finite or any reference whose best legal option is lost, which
  the drifts alone (each a max, which skips NaN) let through. A graph a rewrite does not recognise keeps that part as
  exported and says why at start. The graph is rewritten in memory (the files stay plain ONNX for tract and the CPU); the model
  directory is never written. At start the references are held to export's 1e-3 at fp32 and to serve.py's reduced bar
  below it.
- **Where a batch's time goes.** The periodic summary line ends with a batch's average time assembled (written into
  the pinned buffers), copied in, in the network, copied out and answered, each bucket's rows' mean real tokens, and
  the share of T128's at most 96 (rows a model without a T96 graph runs at 128); `cantrip-infer time` gives the same
  breakdown for one session.
- **Self-play in one process.** `cantrip-selfplay --backend gpu [--precision fp16] [--gpu-batch ...] [--gpu-shapes ...]
  [--pipelines N]`
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
    - Any other search setting by the name `search::Config::set` reads (a number, a flag or text; BotLab's per-seat
      `+Key=Value` sends them): `SharedRoot`, `RootBandit`, `Gumbel`, `GumbelConsidered`, `GumbelNoise`,
      `GumbelCVisit`, `GumbelCScale`, `GumbelBudget`, `EvalCache`, `CountDuplicates`, `LazyInner`, `Snapshots`
      (`"256,1024"`), `EarlyStop`, `EarlyStopMargin`, `RootFloor`, `RootNoise`, `RootNoiseAlpha`, `ForcedPlayouts`,
      `RootShortcut`, `CapValue`, `SkipForcedObserve`, `TimeUse`, `TimeMinShare`, `TimeEntropy`, `TimeSwing`,
      `Shortcut`, `TurnCap` (`docs/ai-player-plan.md`, "The root, throughput and exploration"). A name the host does
      not know is refused (`InvalidSetup`), so a misspelt setting is never searched without.
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
    `Pruned`, `PrunedImmediate` and `PrunedShallow` (edges branch trimming cut), `SearchUs`, `Rows` (rows sent to the
    network), `CacheHits` and `DuplicateRows` (`EvalCache`, `CountDuplicates`), `EarlyStopped`, `TimeStopped`,
    `ShortcutWould` (the shortcut's rule held at the root), `CappedValued` (lines valued at the cap),
    `ForcedUnobserved` (passes played unobserved) and `RootOptions` (each root option's `Path`, `Prior`, `Visits`, `Q`,
    `CompletedQ` and `Target`). A request with one legal answer, a payment, and an answer the shortcut gives before any simulation
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
    [--resume] [--shard-games N] [--precision fp32|tf32|fp16] [--gpu-batch N|32:N,64:N,96:N,128:N] [--pipelines N]
    [--flush-ms MS] [--device N] [--opponents DIR[:SHARE] ...] [--starts DIR[:SHARE]] [--belief-deal]
    [--belief-model DIR] [--memory-budget GB] [--memory-reserve GB] [--commit-reserve GB] [--memory-floor GB]
    [--memory-per-game GB] [--autoscale | --no-autoscale] [--scale-start N] [--scale-step N] [--scale-window S]
    [--scale-min-gain F] [--cpu-ceiling F]
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
  seats and first player come from its index, so the rest of the schedule is unchanged. What is left is decided by the
  games present, not by counting shards: a shard closed partly full (at the end of a run, or by a stop) counts for the
  games it holds. A `.part` shard is recovered only once its header is the run's. One that reads whole is renamed; one that
  ends inside a game has the games before the cut written into a closed shard named after it
  (`<name>.recovered.shard.zst`, written as `<name>.recovered.tmp` and renamed over any earlier one) and is then
  removed, so only the cut game is played again (one with no header is removed). A recovery interrupted at any step is
  done again with the same result, so no game is ever on disk twice; a leftover `.recovered.tmp` is removed.
  `--also-record`'s directories are read the same way: a game the run's own shards hold that one of them lacks (a kill
  between the writes) is played again and written there alone, and a game is never written where it already is. A
  lock file, `OUT/selfplay.lock` (the process id), keeps a second process out of a live run's directory, so it never
  recovers that run's `.part`s; a lock whose process is gone is taken over. The report's `Resumed` and `GamesThisRun`
  say what was which.
- `--shard-games` (100) bounds the games in an open shard, and so the work lost to an interruption. Every lane's open
  shard is closed when the lane ends, however full.
- `--opponents DIR[:SHARE]` (none; repeatable, SHARE 0.2 if not given): a share of the games is played against the
  earlier champion in `DIR` (a model directory), the model being trained (the learner) in one seat. The champion
  searches at the fast budget (`--simulations`, never the full one); its decisions are recorded one-hot, unsearched and
  with no root value, and the game's meta lists the learner's seats (`learners`, from 1) and names each seat by its
  model. So only the learner's decisions are search targets; the champion's positions stay value and aux targets
  (`dataset.Data.learner` is 0 for them and their `policy` 0). Which game, and which seat the learner takes, is a draw
  from the game's index, so a resumed run deals them alike. `--backend remote` needs a server serving every
  opponent's model too; `--backend gpu` refuses opponents (its engine serves one model). The report's `Opponents` has
  each one's games this run and the learner's wins.
- `--starts DIR[:SHARE]` (none; SHARE 0.2 if not given): a share of the games begins from the positions of the
  `.cantrip` scenarios in `DIR` instead of a deal - each scenario played by its runner (`cantrip-script`) to the first
  priority with an empty stack after its last line. Each game is dealt from the position by its seed
  (`Start::game_for`): every library's cards that no seat knows (`Knowledge`) are put in another order among their own
  places and the game's generator is reseeded, so games from one scenario draw differently; a card a seat knows (one it
  saw put on top, say) keeps its place. A scenario keeps its libraries' top N cards in its own order with a comment line
  `# selfplay: keep-top N`, or its libraries whole with `# selfplay: keep-library`. The script's answers make no
  samples: the decisions are counted from the position, and the meta names the scenario (`start`), as does the trace
  (`Trace::start`, replayed with `Trace::replay_on` from the game `Start::game_for` deals for the trace's seed, as
  reanalyse and the audit stand it in). Such a game's decks are recorded as
  `start:<scenario>`, so a training set filtered by deck names leaves it out. The report's `Starts` counts this run's
  games by scenario.
- `--belief-deal` (off) deals the worlds of the learner's Decklist searches by the model's belief head
  (`cantrip_ai::belief`; `--belief-model DIR` for another model's, which implies it; refused outside `--mode
  decklist`): at each search the head is read once, on the CPU with tract, at the request's first step, and each card
  of an opponent's unseen pool gets the odds the head gives that their hand holds one over the odds a uniform deal
  gives it, clamped to [0.001, 1000]. The opponent's hidden hand is then drawn by weighted sampling without replacement
  on those weights, and the rest of their unseen cards uniformly from what is left (`Prior::with_hand`). Every world is
  still one the seat's knowledge allows; a run is still the same games for a seed. An earlier champion's searches deal
  uniformly. A head that fails is counted (`Belief.ErrorsThisRun`) and that search deals uniformly.
- `Snapshots` in the report: with `--search Snapshots=256,1024` (or `--full-search`, for the full searches alone), for
  each count asked at which the search kept a snapshot of the root's visits (the review's item 5; the search's
  `Searched::snapshots`, read by `snapshot_agreement` in `cantrip_ai_host::selfplay`), how many of the learner's
  searches kept one and how many of those already named the finished search's most visited root option (a whole answer,
  or the step's option). Null when no search kept snapshots.
- A sub-step the search reached and chose (`Searched::steps`) is labelled with its visit distribution, which sums to 1
  over the legal options (under `Gumbel=true` the root's first sub-step holds Gumbel's target, made shares the same
  way), and beside each visited option its root value for the seat (the mean of what the search backed up through it
  over every world, from the search's `root_options`: `q`, for a Q head; NaN past what the root's options spell, a
  later sub-step searched in steps of its own), and is marked `searched`. Any other sub-step is labelled with the
  option chosen, not searched, no q: among them a sub-step of a searched decision the search never reached. Each
  searched decision's root value goes in the game's meta (`root_values`). That is shard format 2
  (`crates/cantrip-ai/src/shard.rs`); format 1 shards still read, their samples not searched and without q, and
  `ai/cantrip_ai/dataset.py` gives the per-sample columns `searched`, `q_offset`/`q_token`/`q_value` and `learner` (its
  module says how a format 1 shard fills them).
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
  cantrip-infer's (`--precision`, `--gpu-batch`, `--gpu-shapes`, `--pipelines`, `--flush-ms`, `--device`); its
  references are checked at start as the server checks them, and its throughput is printed at the end.
- `--lanes` is a ceiling; the governor below decides how many games run at once. A run it stopped exits with code 3.
- `--grant FILE` plays only the games a grant file grants (`selfplay::parse_grant`): a line `A..B` (games A to B - 1),
  `A`, or `end`; only lines ended by a newline count. The run reads it again (every half second) while its lanes have
  nothing granted to start, and ends once it says `end` and every game granted is played. Game i is still dealt from
  `--seed` + i with its own pairing, seats and first player, and the shards' header names the whole run's seeds, so
  games split among processes by grants are the games one run plays (`tests/governor.rs` compares them byte for byte).
  This is the distributed driver's work split (docs/ai-training.md, "Distributed self-play"). `--host NAME` names
  the machine in the report and the status file.
- `--lease FILE` (`--lease-seconds S`, 900): once FILE has not been touched for that long (or is gone), the run stops
  gracefully, as at its stop file: the distributed driver touches it at each poll, so a run whose driver went away
  does not play on. `--print-header` prints, as JSON, the header the run's shards will carry (producer, mode, text and
  encoder hashes, schema, format, seeds) and plays nothing: the driver holds every shard to it.
- `--recover` recovers the `.part`s a run killed outright left in `--out` (as `--resume` does first) and plays
  nothing: the games they hold are then in closed shards before another process is given the rest.
- On Windows it allocates with mimalloc: the system heap serialised the lanes' allocations (32 lanes made half the
  evaluations a second with it). The Linux build keeps the system allocator.
- Build it with `cargo build --release -p cantrip-ai-host --features ort`.

### The governor

Self-play budgets its own memory and scales its own lanes (`cantrip_ai_host::governor`), instead of being watched from
outside and killed with its open shards lost. It decides only how many games run at once and when the next one starts,
never how a game is searched: no budget, cache or tree is cut under pressure, and a game is dealt and searched by its
index alone, so a game's record is the same bytes whatever the lanes, the scheduling or the memory
(`tests/governor.rs` compares them game by game).

| Flag | Default | Meaning |
|---|---|---|
| `--lanes N` | the CPU's threads | the ceiling: at most this many games at once |
| `--memory-budget GB` | none | a game starts only while this process's private bytes, plus what the games in flight may still grow by, plus one more game, stay under this |
| `--memory-reserve GB` | 6 | ... and while the system's available physical memory less that much stays above this |
| `--commit-reserve GB` | 6 | ... and while the commit headroom (commit limit less committed) less that much stays above this |
| `--memory-floor GB` | 2 | available memory or commit headroom under this for four readings in a row (a second) stops the run (below) |
| `--memory-per-game GB` | 1 | the guess at a game's memory until games teach it |
| `--autoscale`, `--no-autoscale` | on | scale the lanes to the first limiter (below); off, every lane up to `--lanes` plays |
| `--scale-start N`, `--scale-step N` | 4, 2 | lanes allowed at first, and added at a time |
| `--scale-window S` | 45 | seconds of throughput each scaling decision measures |
| `--scale-min-gain F` | 0.2 | the least marginal gain worth another lane (below) |
| `--cpu-ceiling F` | 0.95 | the system's busy CPU share past which no lane is added |

GB is 2^30 bytes. The readings are Windows' `GlobalMemoryStatusEx` (`ullAvailPhys`, and `ullAvailPageFile` for the
commit headroom), `GetProcessMemoryInfo` (`PrivateUsage`) and `GetSystemTimes`; on Linux `/proc` (anonymous resident
and swapped bytes for the process, `MemAvailable`, the commit headroom only with `vm.overcommit_memory` 2, `/proc/stat`
for the CPU); elsewhere none, and no memory limit applies.

- **Admission.** A lane asks before it starts each game, and waits (woken when a game ends, or every quarter second)
  while there is no room. What a game takes is learnt as the run goes: at each tick no game started or ended since
  (memory a game just let go of may not be returned yet), the process's growth over its least private bytes with no
  game running, less what the lanes loaded since took for their models (measured around each lane's loading), is shared
  among the games running; a game takes the mean of its readings, and the estimate moves half the way up to a game
  that took more and a tenth of the way down to one that took less, so it falls as well as rises. With no game running
  a lane is always let in. A lane's models are loaded at its first game, so a lane never let in holds none. A lane
  that waits for admission, or is switched off, is no client of a batching evaluator meanwhile (`Evaluator::idle`):
  `--backend gpu`'s engine stops counting it among the clients it waits on before running a partial batch, and
  `--backend remote` closes its connections (the servers count connections), connecting again for its next game.
  Batching is row-invariant, so no result changes. A lane that idles keeps its models' memory.
- **Scaling** (`--autoscale`). A feedback controller on the lanes allowed to play: it starts at `--scale-start` and
  adds `--scale-step` each window while the evaluations a second keep rising enough. The gain is measured from a level
  at most 0.8 of this one, as the throughput each lane added brought over what an average lane made there (1 is linear
  scaling, 0 none), so it does not shrink just because the lanes are many (a threshold on the throughput's own rise a
  lane - "3% more for each lane added" - would stop at about 33 lanes even when they scale perfectly); a gain under
  `--scale-min-gain` is measured for a second window before it counts, and then the lanes go back to the last level
  that paid (the lanes it allowed, not the lanes that happened to be busy). That plateau is the GPU
  (its batches full) or the CPU (the lanes' engine work) saturating. Growth also stops while the CPU is busier than
  `--cpu-ceiling` or memory admission deferred a game in the window, and resumes after two clear windows; a plateau is
  probed again after ten windows held. A level held is watched: two windows in a row more than 15% under it back the
  lanes off a step. A lane switched off finishes its game and then idles. Each decision is logged with its reason.
- **Graceful stop.** Available memory or commit headroom under `--memory-floor` for four readings in a row, or Ctrl-C
  (Ctrl-Break, a console's close, SIGINT, SIGTERM), stops admission. After an interrupt the games in flight finish; a
  second interrupt stops them at their current decision, and a third exits at once (code 130; the shards it leaves open
  are recovered on `--resume`). A console's close or SIGTERM counts as two. Under the floor the youngest game is
  stopped at its current decision, and the next only once that one has let go, while memory stays under; a game that
  finished as it was being stopped counts as finished. A stopped game is not recorded (the report's
  `Stopped.GamesStopped`), nor is one that failed once a stop was asked for (its evaluator gone with the console, say),
  and a lane that cannot load its models after a stop ends as part of the stop. Every lane then closes its
  open shard as a valid `.shard.zst` holding the games it finished, the report is written with `Stopped.Reason`, and
  the process **exits with code 3**: run it again with `--resume` and it plays the rest. A run that is killed outright
  loses its games in flight and whatever its open shards had not yet written to disk (what they had is recovered on
  resume).
- **Stop file.** `OUT/selfplay.stop` asks the same as Ctrl-C of a run nothing can interrupt: one started over SSH (no
  signal crosses it), or by a shell that disabled Ctrl-C (Windows passes that flag to its children). It is read on the
  governor's tick: there, admission stops and the games in flight finish; saying `now`, they are stopped too (as a
  second interrupt). One there when the run starts stops it before any game, so whoever starts a run removes a stale
  one first (the generation driver does; `generation.py --root ROOT --stop` writes it, `--stop-now` with `now`).
- **Status.** `OUT/selfplay-status.json` is rewritten every five seconds (whole, then renamed) and once at the end:
  the process id, the games finished (all and this run's), runaways and drops, the games an hour, the grant's games
  granted and handed out, the lanes allowed, running and their ceiling, the limiter, why the run stopped, and whether it
  is over (`Finished`). It is what the distributed driver reads of a worker.
- **Report.** `selfplay-report.json` gains `Stopped` (null, or the reason, the games stopped and the exit code) and
  `Governor`: `Memory` (peak private bytes, the least available memory and commit headroom seen, the per-game estimate
  and the games it was learnt from, the lanes with models loaded and what a lane's models take, the limits), `LanesRunning` (the ceiling, and min/mean/max from the first game
  until no game was left to start), `Deferred` (admissions deferred for memory and the seconds they waited), and
  `Autoscale` (its settings, the lanes allowed at the end, the limiter that last stopped growth, and each window's lanes
  running, lanes before and after, evaluations and searched decisions a second, CPU, whether memory deferred, and the
  reason). A line is logged when admission starts deferring (at most once a minute) and when a stop is triggered.

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
  a report;
- that a searched sub-step carries a q beside each visit and every other sub-step is one-hot and unsearched, and that
  the per-sample flag is the search's (`Searched::steps`): a sample searched for each sub-step it reached, and the
  sub-steps of a searched decision it never reached not searched;
- that Gumbel's targets are recorded as shares summing to 1, as visits are;
- that a game against an earlier champion records the learner's decisions alone as search targets, that a game from a
  scenario start records from the position and its trace replays on the game dealt from it, that games from one start
  draw differently by their seeds and keep what a seat knows, that a run deals opponents and starts by the game's
  index, and that Decklist self-play deals by the belief head.

`crates/cantrip-ai-host/tests/governor.rs` plays self-play under its governor, with injected memory readings. It
checks that every game's record is the same bytes by seed whether played on one ungoverned lane or on three with
scaling and memory admission deferring games; that a stop for memory closes every shard with the games finished, a
resumed run plays exactly the rest and the games come out as the ungoverned run's; that an interrupt lets the games in
flight finish; that `.part`s cut inside a game keep the games before the cut, each in a shard of its own, and that a
recovery interrupted between its steps and done again leaves each game once; that a resume writes what a hard kill
left out of an `--also-record` directory there and nothing twice; that a live run's directory is refused and a dead
one's lock taken over; and that the binary exits 3 when stopped. `cantrip-ai`'s `tests/remote.rs` checks that an idle
remote evaluator closes its connections and answers alike when it connects again.
The governor's own tests (`governor.rs`) check admission against each limit with a fake reading, a waiting lane idled
once, the learnt estimate (falling as well as rising, not inflated by a dip in the games running, the lanes' models
apart), the floor (four readings, one game stopped at a time, a game finished as it was stopped not counted) and
interrupts, and the scaler against a made-up throughput: it grows while gains continue, holds at the
plateau, stops at each limiter, grows again when one clears, probes again, and backs off on a drop but not on one
noisy window.

`crates/cantrip-ai-host/tests/reanalyse.rs` reanalyses traced self-play games, whole and with their traces cut short;
`crates/cantrip-ai-host/tests/explore.rs` checks the search's floor on a nominated option, a verification and the
shortcut audit;
`crates/cantrip-ai/tests/shard_format.rs` the shard formats (2 written, 1 still read); `crates/cantrip-ai/tests/belief.rs`
the deal by hand weights and the tiny model's belief weights.

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
