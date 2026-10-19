# The AI player: implementation plan

The plan for building the [AI player design](ai-player-design.md) as a robust prototype, network included. Status
2026-10-18: a plan, nothing built; revised once after an adversarial review (what changed is at the end). It takes
the design's four parts (search, evaluator, belief, training) and fixes what the design left open, against the engine
as it is now ([engine-design.md](engine-design.md)): Rust, 6-45 us per decision depending on the decks (6 us is random
play), a game-log tree that branches anywhere (`Game::goto`), decisions as data.

Two requirements beyond the design:

1. **Hardware.** Training runs on NVIDIA and AMD GPUs. Evaluation (inference) runs on CPU, in WebAssembly in the
   browser, and on the same GPUs.
2. **Information modes.** The network, and the search around it, play under any of three modes, a toggle per seat:
   **full** (every card everywhere, the opponent's hand and both libraries in order), **decklist** (the opponent's
   decklist is known, not its order or which cards are in hand) and **hidden** (normal Magic: the opponent's deck is
   unknown beyond what has been seen).

## Decisions

| Question | Decision | Why |
| --- | --- | --- |
| Training framework | **PyTorch**, run from containers: the official CUDA images and AMD's `rocm/pytorch` | The one framework with maintained builds for both CUDA and ROCm. The containers make NVIDIA and AMD the same command and keep the GPU stack off the NixOS host, which has no Python. |
| Model exchange format | **ONNX**, the exporter and the opset both pinned | Every inference target below reads it. |
| GPU inference for self-play and server seats | **A batched inference server in the same container as training** (PyTorch, CUDA or ROCm), fed by the Rust actors over a Unix socket with shared-memory batches | The one GPU path that is equally real on both vendors today: onnxruntime's ROCm provider is gone (removed around 1.23) and MIGraphX has no prebuilt `ort` binaries. |
| Native in-process inference | **ONNX Runtime through `ort`**: CPU always; CUDA as an option; MIGraphX on AMD as an option, **unverified** until built and run on AMD hardware | In-process CPU is the default for a server seat; the GPU providers are a speed-up, not a requirement. |
| Browser inference | **onnxruntime-web**: WebAssembly on the CPU (the tested path), WebGPU where the browser has it (NVIDIA and AMD alike) | The same model file and operators as native. |
| Pure-Rust fallback | **tract**, CPU, for `cargo test` with no native library and as a second parity reference | Checked in phase 0 to run the exported graph (LayerNormalization, masks); if it cannot, the model is changed to what it can run, or tract is dropped from the matrix. |
| The hidden-information boundary | **One function**, `observe(game, knowledge, seat, mode) -> Observation`, plus a per-seat **knowledge** record; the observation never carries an engine `ObjectId` | The design asks for a boundary as hard as the client's. One producer, tested by permutation, is that. |
| Network shape | **A small transformer over object tokens** (1-2 M parameters, at most 128 tokens), value, pointer-policy and belief heads; the mode an input | One model for the three modes; sized for the throughput the machine has (see Throughput). |
| Card representation | **A frozen text embedding of each card's Oracle text**, computed offline, plus printed numbers and type bits, through a learned projection; a learned "unknown card" row for a card with no embedding yet | A card the network never trained with still means something, and a new card never breaks play. |
| Search | **PUCT over the engine's tree in determinized worlds** (PIMC) first, its bias stated; **information-set MCTS** second, kept if it wins | PIMC is simple and parallel; ISMCTS is the answer to its bias. Which is worth its cost is a duel result. |
| Where search runs | **Off the host's request lock**: in a worker pool in its own process, called asynchronously with a deadline | The host is one process serving every table, and every call holds a lock (`NativeHostProcess.cs`); a five-second search there would stall the server. |
| First slice | **The fixed decks the tactical bot is measured on** (`rw-aggro`, `ub-reanimator`, the `demo` decks), then the 67 `decks/standard` lists | A tactical-bot baseline exists for exactly these; the standard lists are the hidden mode's metagame. |

## Architecture

```
          container (Linux, CUDA or ROCm)                              play
  ┌────────────────────────────────────────────┐
  │ ai/ : model, trainer, ONNX export, parity, │
  │       card-text embedder, inference server │◀───────── batches (Unix socket + shm)
  └──────────▲─────────────────────┬───────────┘                  │
     shards  │                     │ model.onnx, vocab, manifest  │
  ┌──────────┴─────────────────────▼───────────────────────────── ┴─┐
  │ cantrip-ai (Rust, native)                                       │
  │  Knowledge (per seat), observe() - the one redaction point      │
  │  worlds: fork + determinize      search: PUCT, PIMC / ISMCTS    │
  │  selfplay actors, shard writer   Evaluator: remote GPU | ort | tract
  └──────────┬──────────────────────────────────────┬───────────────┘
             │ depends on                           │ used by
  ┌──────────▼──────────────────────┐   ┌───────────▼──────────────────┐
  │ cantrip-obs (no engine, wasm)   │   │ cantrip-ai-host (a process): │
  │  Observation, InfoMode, encode, │   │  search workers for server   │
  │  vocab, answer factoring        │   │  seats and BotLab, async,    │
  └──────────┬──────────────────────┘   │  deadline, fallback          │
             │                          └──────────────────────────────┘
  ┌──────────▼──────────────────────┐
  │ cantrip-ai-wasm → browser seat  │  hidden mode only, network only
  │  (NeuralPlay, onnxruntime-web)  │
  └─────────────────────────────────┘
```

| Where | Holds | Depends on |
| --- | --- | --- |
| `crates/cantrip-obs` | `Observation`, `InfoMode`, their postcard form with a schema version; `encode` into fixed-shape buckets; the vocabulary; the **answer factoring** (sub-steps and their constraint data, so the browser runs the same code) | serde only |
| `crates/cantrip-ai` | `Knowledge`, `observe`, worlds, the belief sampler, the search, the `Evaluator` trait and backends, the self-play actor, the shard writer, the Rust arena | `cantrip-engine`, `cantrip-obs` |
| `crates/cantrip-ai-host` | A process (JSON lines, like the native host) that holds games forwarded to it and answers AI seats from a worker pool | `cantrip-ai` |
| `crates/cantrip-ai-wasm` | `wasm-bindgen` exports of `cantrip-obs` | `cantrip-obs` |
| `ai/` | Python: model, dataset, trainer, export, parity, embedder, inference server; `Dockerfile.cuda`, `Dockerfile.rocm`; `uv`-managed `pyproject.toml` | PyTorch, onnx, onnxruntime |
| `tools/ai-*` | `ai-selfplay`, `ai-train`, `ai-export`, `ai-duel`, `ai-parity`, `ai-embed` | |

The AI is a consumer of the engine, like the host. Its changes to the core are the fork, determinization and the id
order (below). Each is a core change: measured with `cantrip-bench --games 128` before and after, as CLAUDE.md asks.

## The information boundary

### Knowledge: who knows what

The engine records no one's knowledge (the header of `cantrip-host/src/view.rs`: "the engine keeps no record of who
has been shown which hidden card"; `seen_by` covers face-down exile only). Observations and worlds both need it, so
`cantrip_ai::Knowledge` is a per-seat record **folded from the event history** (`q.history()`):

- each hidden object the seat knows the identity of: its own hand; cards revealed to it, or to everyone; cards it
  looked at (scry, surveil, "look at the top N"); cards it put into a hidden zone itself;
- known library positions ("I put these two on top in this order"), reset by a shuffle of that library;
- for the opponent: every card it has been seen to have (cast, revealed, discarded, milled), the multiset the
  belief and the decklist mode subtract.

It is computed incrementally per event, and checked against a refold from scratch under `CANTRIP_VERIFY=1`. The host's
`known_by` and `ZoneView::known`, left at the default today, come from the same code once it exists. Where the history
does not record enough to know (a look whose cards are not in an event), the event is extended: a core change, since
the core records events.

### `observe`: the only door

`observe(game, knowledge, seat, mode) -> Observation` is the one function that reads the real game for the AI. It uses
the same visibility rules as the host's views, moved out of `view.rs` into code both call, so the two cannot disagree.
It is taken only **at a step boundary, under the host's own deferral rule**: while another seat is in the middle of
announcing or paying, a seat sees its last observation, as it sees its last view
([host-protocol.md](host-protocol.md)).

It carries: every visible object (card name, zone, controller, owner, computed characteristics, counters, damage,
tapped, combat role, sickness, attachments, on the stack with its targets); every hidden object as a hidden token
(zone, owner, a known position, the identity if `Knowledge` has it); the players; the turn, step and priority; a
window of recent public events; the pending request and its candidates.

**No engine ids.** Objects are numbered per observation in a canonical order (zone, then the zone's visible order,
then name), as the host's `HandleSpace` numbers handles. Ids are not neutral. `Game::new` used to create library
objects in decklist order before the shuffle, so an id said where a card sat in the list. It now shuffles the
decklist first and creates the objects in dealt order, with the same draws, so a seed deals the same library. An id
still says where a card was dealt, and that goes stale at the next shuffle; `determinize` has to account for it.

By mode:

- **full**: hidden tokens filled in, both hands, both libraries in order. The RNG seed is never in it;
- **decklist**: the opponent's decklist less what the seat has seen, as pool tokens (a card and a count);
- **hidden**: no pool tokens. The seat's own decklist is in every mode.

### Who may use which mode

The mode is the **table's** setting, enforced by whoever produces the observation:

- **full and decklist**: server-side seats only (`cantrip-ai-host`), and full only at a table with no human player;
- **hidden**: everywhere, the browser included.

A browser bot runs in a human player's own tab ([tactical-bot.md](tactical-bot.md)). Anything sent to it is sent to
the human, so a browser seat is sent hidden-mode observations only, which hold nothing that seat's own view does not
already hold. A modified tab can therefore only play its own seat badly; the observation bytes need no integrity
beyond the transport's.

### The redaction tests

The property, tested directly: what the AI produces may not depend on what the seat cannot know.

- **Observation.** Over scenario games and bot games, take two games that agree on everything the seat knows and
  differ in the hidden zones (built by permuting the physical hidden objects and their ids, not only swapping card
  names). Their observations are byte-equal, the event window included. In decklist mode the permutation stays
  within the opponent's decklist.
- **Worlds.** For the same two games, `determinize` with the same seed gives full engine states that are equal
  after canonical renumbering. That is a scenario-style full-state dump comparison, not only an observation
  comparison, and it covers every cache, `last_known` entry and history record.
- **Search.** The same search seed in the same two games gives the same answer.

## Worlds: what the search plays in

The search needs a complete game. It makes one without reading what is hidden.

1. **Fork.** `Game::fork_for_search(&self) -> Game`: the working state, plus a deep copy of the journal back to the
   lowest pending frame mark (the mark `forget()` already computes). Without it, `answer` and cancel inside a
   resolution prompt revert to a mark the fork no longer holds, and `revert` asserts
   (`cantrip-state/src/state.rs:1114`). The fork has a fresh `Tree` rooted at that mark, so a search cannot go back
   up the real path. A plain `GameState::clone()` already drops the journal, so `clone_with_journal` (state.rs:982)
   is the starting point. A scenario test answers and cancels inside a resolution prompt after a fork.
2. **Reseed.** The fork's RNG comes from the search's own generator; the real seed would tell it every shuffle.
3. **Determinize.** `cantrip_ai::determinize(&mut Game, &Knowledge, seat, mode, &Prior, rng)`
   (`cantrip-ai/src/worlds.rs`) gives every hidden object the seat does not know another card, and resamples where
   the library cards it knows but cannot place lie. Only a fork's root is touched (`Game::rebind_hidden` asserts the
   tree has not left it). The engine operations are **not journalled**: a re-run inside a resolution prompt reverts
   to its frame's mark, and a journalled rebind would be undone by it, bringing the real cards back. Like
   `GameState::reseed`, they rewrite the stored copies in the journal's tail instead.
   - `GameState::rebind_hidden(chain, card, abilities, statics)`, wrapped by `Game::rebind_hidden(chain, card)`, rebinds
     one object together with the rows it became from while it stayed hidden;
   - `GameState::reorder_library`, used only when no `ZoneInserted`/`ZoneRemoved` entry in the tail touches that
     library (`zone_order_touched`);
   - `GameState::restamp_hidden(id, 0)`;
   - and `Game::ask_again`: the waiting question was put to the real cards (a Priority request's castable options),
     so after a change it is asked again of the world's. It reverts to the top frame's mark, truncates the log and
     re-runs, replaying the frame's answers.

   **Where an identity lives** (read from the code, and kept honest by the worlds test above):
   - `Obj::card` of the live object. For a face-down object, `card` is the real card and `copy` holds the face-down
     values, so only `card` is rebound.
   - Its `became` predecessors that were themselves hidden from the seat, which are dead rows of `objects`. The
     history and `roots` (the deck as dealt) read identity through these rows. The chain stops at the first row the
     seat saw: it was shown, or it stood where the seat looks.
   - The journal tail: `Undo::Object(id, _)`, and `Undo::ObjectPopped(_)`, whose index is found by walking back from
     `objects.len()`.
   - `AbilityRef { card, .. }` (the card and its back face) in the `stack`, `left_stack`, `triggers` and `delayed`
     entries of a rebound object, and in the tail's `StackPopped`, `StackSet`, `LeftStackPopped`, `TriggerRemoved`
     and `DelayedRemoved`.
   - The history: no event carries a `CardId` directly, but `AbilityActivated`, `Resolved`, `Triggered`,
     `ModesChosen`, `Done` and `Cause::Ability` carry an `AbilityRef`. `Game::rebind_hidden` patches those in the log
     where the source is rebound.
   - `LastKnown` is kept only for objects leaving the battlefield or the stack, which are public; a face-down
     permanent is revealed as it leaves (CR 708.9). Debug builds assert it.
   - Caches: an `epoch` bump makes `static_sources`, `lost_to_statics` and `land_text_lost` recompute. When the new
     card has static abilities, `statics_seen` is set, and so is every `Undo::StaticsSeen` in the tail.
   - Timestamps. Every move takes a new timestamp, and a dealt card has 0. An object whose arrival no event records
     (no `ZoneChange` or `Drew` makes it: a card of the opening hand) takes 0, so that the world does not keep its
     place in the deal. That applies even when the seat sees the card now (a hand Duress shows).
   - The pending request, above.
   - Left as they are: `loop_watch` (hashes of the real positions, so a world catches only loops that begin after the
     fork); `Marks::chosen_card`, `Duration::AsLongAs`, effects, delayed triggers and permissions (public cards, or
     objects named by id); `rng` (`reseed`).

   **The deal.**
   - Hidden objects are classified: those the seat sees or knows keep their card. A known card it cannot place in a
     library is K; the rest are U.
   - A pool is made per owner and per origin (library roots, or sideboard roots). It holds the deck as dealt (the
     seat's own; also the opponent's in decklist mode; a list drawn from `Prior` in hidden mode), less the cards of
     that owner's objects of that origin that are seen or known. Under `CANTRIP_VERIFY` it must come out at exactly
     |U|.
   - Each library's unplaced places are dealt again, with the rng, among its K and U objects. Objects are ordered by
     what the seat can tell apart: the card of a K object, the timestamp, the first event naming the object, and then
     the real place.
   - The pools are then shuffled onto the U objects, by zone and place.

   **Approximations, accepted** (docs/known-issues.md):
   - A library the tail rearranges keeps its order.
   - In hidden mode, the cards that began outside the game (sideboards) stay what they are.
   - A face-down object may be rebound to a card that could not have been face down.
   - A staged question (`ask_then`, Duress's choice) is not asked again.
   - The lower frames' runs, and the answers `ask_again` replays, were made against the real cards.
   - Which hidden object an event named stays as it was, for example whether the opponent discarded the card it just
     drew or one it had kept.

   **The fallback**, had the list not closed, was to build each world by replaying the real game's public answers
   in a fresh game dealt from the sampled decks.

What is sampled, by mode:

- **full**: nothing; the fork is the real game;
- **decklist**: the opponent's unseen decklist cards dealt over its hidden slots (hand, library, face-down),
  weighted by the belief head once it is trained;
- **hidden**: a whole decklist first, then the deal. The prior is a mixture over the metagame lists (the 67
  standard decks, the cube lists for drafts), conditioned on the cards seen. The belief head reweights the
  archetypes. Every sampled list respects the format's legality and the four-copy limit. When no list fits the cards
  seen (a deck that is not in the metagame), it falls back to the seen cards plus the format's most-played cards in
  those colors.

The seat's own library is reshuffled within its unknown positions: a seat does not know its own draw order.

## Network

### Inputs

At most 128 tokens per position, in fixed shape buckets (32, 64, 128) and batch buckets, so MIGraphX, WebGPU and
TensorRT-style compilers see static shapes:

| Token | Built from |
| --- | --- |
| card object | the frozen text embedding → projection; printed numbers and type and keyword bits; dynamic state as learned embeddings and scaled numbers |
| hidden object | the "unknown card" embedding plus zone, owner, known position |
| pool card (decklist) | the card's embedding plus a count |
| player, game | life, poison, sizes, mana, lands played, speed; turn, step, priority, **mode** |
| event | recent public events, typed, pointing at canonical object indices |
| option | each candidate of the current (sub-)step, with pointers to the objects it names |

Objects past the cap are pooled per zone. The **cap-hit rate is logged** in self-play: if it is common, the cap moves.

The card-text table is built by `ai-embed` in the container from `content/*/cards/*.json`. It ships with a hash of
the card set. A card without a row plays with the unknown-card row, and a fast test (`test.cmd`) reports how many
cards lack one, so the table is rebuilt when cards are added but play never breaks before then.

### Body and heads

- **Body**: a pre-norm transformer encoder, 4 layers, width 128-192, 4 heads, masked attention, no positions.
- **Value**: the expected outcome for the seat (tanh), plus auxiliary targets (life swing over the turn, dead next
  turn).
- **Policy**: a pointer over the option tokens of the current sub-step, masked to the legal ones.
- **Belief**: the probability of each **archetype** in the metagame, and for each candidate card in the sampled
  pool (not the whole vocabulary) a probability of being in hand. Also the design's threat classes (counterspell
  up, removal up, trick up, lethal next turn). The labels are the true hidden state, which training has; the input is
  only the observation.

### Information modes in training

Following Suphx properly:

- **Oracle dropout.** Full-mode features (the hidden contents) are fed during early training and annealed away by
  dropout, so the network learns to play without them.
- **The mode as an input.** At the end of training the mode token says what the observation holds.
- **Opponents' modes.** Each sample records both seats' modes. Hidden-mode targets come from games where the
  opponent was also hidden or decklist mode, because a full-information opponent makes a different game. Each
  mode's share of the data is reported, and reweighted if it drifts.

## Answers: from the engine's request to policy steps

Requests give **candidates and constraints, not every legal answer**. Examples:

- `target_candidates` merges every slot;
- block requirements (CR 509.1c) and menace minimums constrain the whole block;
- `TotalAtMost` caps a total;
- legality is finally `validate::check`'s.

Each request kind therefore gets its own factoring, in `cantrip-obs`, with the constraints carried as data:

| Request | Steps |
| --- | --- |
| priority, modes, a yes/no, a color, which replacement | one choice among the candidates |
| attackers | per creature: attack (at what) or stay; then validated |
| blockers | per potential blocker: which attacker or none; constraints applied as masks where they are local, the whole block validated at the end |
| N cards or targets | one at a time, a slot at a time, "done" when "up to" allows |
| X | a choice among buckets up to what the seat can pay |
| damage assignment, divided amounts, counter removal | one unit or chunk at a time, in the rules' order where they impose one |
| trigger order, card order | one at a time, "next first" |
| a card name | among the format's names the belief and the seat's own deck make plausible, not all names |
| mana payment | the existing payment heuristic (`auto_pay`, moved from `cantrip-script/src/run.rs` to `cantrip-ai`); a policy step later only if duels show it loses games |

An assembled answer is checked with the engine's validator before it is sent. In the search, a rejected answer is
masked and the step backtracks. In the browser (no validator), a rejected answer makes the seat fall back to
the network's next choice, then to the tactical bot. A fast test sends every factoring's answers through the
validator, for every request kind the scenario corpus raises.

## Search

### Moves not branched on

- A request with one legal answer is answered without a node.
- A priority where the policy gives everything but pass under 1% is passed after the first visit says so.
- Mana abilities are paid, never moved.

### PUCT in worlds (PIMC)

1. Draw `W` worlds: 1 in full mode, 4-16 otherwise.
2. In each, PUCT: select by `Q + c · P · √N / (1 + n)`; expand with one batched call (priors and value from the
   acting seat's observation of that world); back up. Moving is `Game::goto` in the fork. Sub-steps of a factored
   answer are search nodes the engine never sees.
3. Sum the root visits across worlds; play the most visited (sampled proportionally for self-play's opening moves).

**The bias, stated.** In each world the tree's Q values come from a fully determined game, so the simulated opponent
plays as if it knew the AI's hand. Priors from the acting seat's own observation soften this, but values are not
fixed by it. PIMC overvalues bluffs and plays into tricks it "knows" are absent. The mode gap (below) measures it.

**ISMCTS, phase 5.** Multi-observer information-set MCTS: one tree per seat, each keyed by what an action looks like
to that seat (card name and targets, not ids). A fresh world each iteration, with both seats' hidden cards resampled
from the other's knowledge. It is kept only if it wins the duel.

Also noted:

- The tree never prunes (`tree.rs`); a fork is thrown away after each decision, so its memory is bounded per
  decision. That memory is measured.
- Request ids are part of a child's key, so transpositions are not merged. That is acceptable for now.
- Leaves are batched across worlds and across games with virtual loss.

### Budgets and fallbacks

- **Self-play**: a fixed simulation count per decision, so the data does not depend on the machine's load.
- **Play**: a deadline per decision. When it expires, the answer is the best found so far. With no search result,
  it is the network alone; with no network, the tactical bot. A seat never times out the table.
- **Game length**: self-play games are capped (turns and decisions). A capped game is a draw (value 0), and the cap
  rate is logged. A loop the engine itself draws for (CR 104.4b) is a draw as well.
- **Mulligans and play first** are policy steps like any other, with their own sub-step factoring (keep, which cards
  to bottom). They are in the data from the first generation.

## Evaluation targets

```rust
pub trait Evaluator: Send + Sync {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError>;
}
```

| Backend | Where | Status |
| --- | --- | --- |
| `RemoteEvaluator` → the container's PyTorch server | NVIDIA (CUDA), AMD (ROCm) | the GPU path for self-play and server seats on both vendors |
| `OrtEvaluator` CPU | any machine | the default in-process path |
| `OrtEvaluator` CUDA | NVIDIA | optional speed-up |
| `OrtEvaluator` MIGraphX | AMD, Linux | optional, unverified until it is built and run on AMD |
| `TractEvaluator` | pure Rust, CPU | tests; parity reference |
| onnxruntime-web WASM / WebGPU | the browser | WASM tested; WebGPU where the browser has it |

**Parity is a test.** `ai-parity` runs a fixed set of encoded positions (a few thousand, from scenario replays)
through every available backend and PyTorch. It checks that values agree within 1e-3 and that the top choice is the
same on 99.5%+. The CPU backends run in every check. The GPU half runs on whatever vendor the machine has. The AMD
half (ROCm training and the inference server, and MIGraphX if built) runs on AMD hardware, borrowed or a rented
ROCm instance, at the phase gates. **Until that run passes, AMD support is untested, and this file says so.**

AMD consumer cards (RDNA) are checked against the ROCm support matrix; some need `HSA_OVERRIDE_GFX_VERSION`. The
export script pins the exporter (TorchScript-based, not dynamo, unless phase 0 says otherwise) and the opset, and
checks the graph against an operator allow-list.

## Training

### Data

A **shard** (zstd) has a header with the observation schema version, the encoder hash, the card-set hash, the engine
commit, the model and backend that produced it, and the seeds. Each sample holds:

- the encoded observation and its legal mask;
- the search's visit distribution;
- the outcome;
- the auxiliary targets and the true hidden state;
- both seats' modes.

Size: about 2-4 KB a sample compressed was estimated; the spike's tensors are 8.5-34 KB raw (Phase 0 results).
Shards are kept for the last eight generations, and older ones are deleted unless pinned.

**Reproducibility.** The engine is deterministic and self-play's seeds are recorded. Batched asynchronous inference
is not deterministic across runs, so a self-play run is reproducible in its seeds and its model, not bit for bit.
That is stated, not hidden.

### Compatibility

A model's manifest names:

- the observation schema version;
- the encoder hash;
- the card-text table hash;
- the engine commit it was trained against.

The loader refuses a mismatch of schema or encoder. A newer card set is accepted, with unknown-card rows for the new
cards.

### Bootstrap, then expert iteration

1. **Imitation of the tactical bot.** BotLab's games already run on the native host
   (`dotnet/tools/Cantrip.BotLab/Match.cs:105`). A host flag (`--ai-record DIR`) writes each decision's observation
   and the tactical bot's answer, decomposed into the plan's sub-steps (`cantrip-obs` turns an assembled answer into
   its steps). At about thirty games a second, data is plentiful; it is shaped like the tactical bot's play, which is
   what generation 0 is for.
2. **Short problems from scenarios.** Forced positions ("win this turn", "this combat", "this stack fight"),
   reached by a scenario's own script and never by a seed, labelled by a large-budget search and trained on as a
   curriculum. This is the most data per GPU-second available.
3. **Expert iteration.** Each generation:
   - self-play (Rust actors with GPU batches), then train, then export, then the parity check, then the gate;
   - each seat's mode is drawn per game;
   - candidates are gated by **SPRT** against the current best (H0: a 50% win rate, H1: 55%, α = β = 0.05), with the
     confidence interval reported, rather than a fixed 400 games.

### Throughput, against the machine (CPU measured: Phase 0 results, under Phases; GPU to be measured)

- A 1.5 M parameter model over 128 tokens is about 0.4 GFLOP a position. On an RTX 4060 that is perhaps 5-15 k
  positions a second at batch 512, through the inference server.
- A decision with `W` = 8 worlds and 32 simulations each is about 256 evaluations, so about 20-60 decisions a second.
- A game is 300-1000 decisions, sub-steps included, so self-play makes about 2-15 k games a day.

That is enough for expert iteration on the fixed-deck slice, but not from scratch. The bootstrap and the short
problems carry the early generations. The levers, each measured:

- fewer worlds;
- Gumbel root selection at low simulation counts;
- full mode (W = 1) for value-heavy data;
- a smaller model.

The browser runs the same model on WASM, an estimated 20-100 ms an evaluation, network only.

## Serving and measuring

### The AI host

`cantrip-ai-host` is a process of its own, so a search never holds the rules host's lock.

- **Getting the game there.** The native host forwards a game to it: the decklists, the seed and the answers so far.
  That replays deterministically, and is then kept in step by forwarding each answer.
- **Asking for an answer.** `AiStart{Seat, Model, Mode, Deadline}` returns at once. `AiResult` arrives when the
  answer is ready or the deadline passes. The C# side calls it asynchronously.
- **BotLab** runs its own pair of hosts per worker, as it runs its own native hosts now.

### Measuring

- **Against the tactical bot.** A `neural` seat in `cantrip-botlab` and `bot-duel.cmd`. It plays at a fixed budget,
  with seats swapped. Results are reported per mode with confidence intervals, and on `--decks` pools as the
  tactical bot is measured.
- **AI against AI.** In the Rust arena (`ai-duel`):
  - model against model;
  - search against network only;
  - PIMC against ISMCTS;
  - full against decklist against hidden.

  The **mode gap** (what full information is worth to the same network and search) is reported every generation.
- **Champions.** A frozen model and its manifest are kept under `.cantrip-lab`.

## Phases

Each phase ends at a gate. The times are estimates for one person and will be revised from phase 0's numbers.

**Phase 0: environment and measurement (2 weeks).**
- **Environment on NixOS.** nvidia-container-toolkit or CDI for the CUDA container. `ort` with CUDA either in the
  container or with CUDA 12 and cuDNN 9 from Nix. The ROCm container on rented AMD hardware.
- **Bench.** Games a second on 1 and 16 cores on the slice's decks. The cost of a fork with its journal tail at
  turn 1, 5 and 15. A search's tree memory.
- **Spike model.** A throwaway model of the planned size, exported (pinned exporter and opset) and run through:
  - PyTorch on CUDA and ROCm;
  - `ort` CPU and CUDA;
  - tract;
  - onnxruntime-web WASM in a page (single-threaded unless COOP/COEP headers sit with the Uno host).

  Positions a second at each bucket.
- **Gate:** every target in requirement 1 runs the same graph, AMD on real hardware, and the numbers are written
  here. Anything that does not run is listed as unsupported with the reason.

**Phase 0 results (the CPU half, October 2026).**

Measured on a Ryzen 7 5800X3D (8 cores, 16 threads), NixOS, no GPU. The code is `ai/`: `shell.nix` (the CPU
environment), `pyproject.toml`, `Dockerfile.cuda` and `Dockerfile.rocm` (written, not yet built), and `ai/spike/`:
`model.py`, `export.py`, `bench_torch.py`, `rs/` (ort and tract, a crate outside the workspace) and `web/`
(onnxruntime-web).

**The spike graph.** 1.28 M parameters: 4 layers, width 160, 4 heads, FFN 640, 64 features a token. Exported by the
TorchScript exporter (`dynamo=False`), opset 17, torch 2.11. **One file per token bucket with a free batch
dimension** (`spike-t{32,64,128}.onnx`, 5 MB each): the token count is a constant in the graph, so it has no Shape
ops, and a backend that wants static shapes pins the batch when it loads (tract builds one plan per (T, B)). The
graph uses only Add, Cast, Clip, Constant, Div, Erf, Gather, Gemm, Identity, LayerNormalization, MatMul, Mul,
ReduceSum, Reshape, Softmax, Split, Squeeze, Sub, Tanh, Transpose, Unsqueeze and Where; `export.py` refuses anything
else. Parity is against PyTorch's outputs on 512 reference positions per bucket: value within 1e-3 and the same
argmax on 99.5%.

| Target | Status | Parity | Positions/s, T32 / T64 / T128 (best batch) |
| --- | --- | --- | --- |
| PyTorch CPU, 1 thread | runs | reference | 965 / 551 / 252 |
| PyTorch CPU, 16 threads | runs | reference | 3.9 k / 2.0 k / 909 |
| `ort` 2.0.0-rc.14 CPU, 1 thread | runs | 1e-7, 100% | 1.2 k / 586 / 276 |
| `ort` CPU, 16 threads | runs | 1e-7, 100% | 5.3 k / 3.0 k / 1.5 k (B64) |
| tract 0.23, 1 thread | runs, graph unchanged | 1e-7, 100% | 934 / 449 / 201 |
| onnxruntime-web 1.23 WASM, 1 thread (Node) | runs | 1e-7, 100% | 337 / 156 / 73 |
| PyTorch on CUDA; `ort` CUDA (`--features cuda`) | **blocked**: no NVIDIA device | - | - |
| PyTorch on ROCm; `ort` MIGraphX | **blocked**: no AMD GPU | - | - |

- **What unblocks NVIDIA:** the user's NixOS config with `hardware.nvidia` (the driver) and
  `hardware.nvidia-container-toolkit.enable = true` plus docker or podman, then `docker build -f ai/Dockerfile.cuda`
  and `docker run --device nvidia.com/gpu=all`. For native `ort` CUDA, CUDA 12 and cuDNN 9 from Nix on the library
  path and `cargo run --release --features cuda` in `ai/spike/rs`.
- **What unblocks AMD:** a ROCm GPU (owned or rented) with the amdgpu driver, `ai/Dockerfile.rocm`
  (`HSA_OVERRIDE_GFX_VERSION` for consumer cards). MIGraphX has no prebuilt `ort` binary or wheel; it is built from
  source in that container.
- Batch barely matters on one thread (the work is the matrix products); threads pay from B16 up, and B512 is slower
  than B64 on CPU at every T (cache). Sixteen concurrent single-thread processes at T128 make 1.1-1.3 k/s together,
  no more than one 16-thread session: **the CPU ceiling is about 1.5 k positions a second at T128, 5 k at T32.**
- The WASM run is Node over the same `.wasm` a page loads, single-threaded: 14 ms an evaluation at T128, 3 ms at T32,
  inside the 20-100 ms estimated. The page itself (and threads behind COOP/COEP) is phase 5's.

**The engine** (`cantrip-bench --games 128 --scaling --forks`, `--forks` added for this):

| Decks | Games/s, 1 / 2 / 4 / 8 / 16 threads | µs a decision | Decisions a game | Turns |
| --- | --- | --- | --- | --- |
| rw-aggro vs ub-reanimator | 33 / 64 / 124 / 226 / 231 | 42.7 (engine), 44.2 (with the seat) | 662 | 21 |
| the bench's default decks | 91 / 177 / 348 / 607 / 665 | 9.8 | 1108 | 47 |

Threads scale to the 8 cores and SMT adds little. A game allocates about 12 MB. Forks, rw-aggro vs ub-reanimator:

| Turn | Journal entries | `GameState::clone` | `clone_with_journal` | Whole `Game` (journal, log, tree) |
| --- | --- | --- | --- | --- |
| 1 | 382 | 0.9 µs | 10 µs, 162 KB | 10 µs, 164 KB |
| 5 | 1 506 | 1.2 µs | 46 µs, 437 KB | 51 µs, 459 KB |
| 15 | 5 884 | 1.7 µs | 198 µs, 1.6 MB | 220 µs, 1.7 MB |

The state alone is 52-78 KB and under 2 µs. The whole journal grows with the game, so `fork_for_search` must copy only
the tail back to the lowest pending mark, as planned; the full copy is the upper bound. A search's tree memory is not
measured: there is no search yet (phase 3).

`fork_for_search` (phase 1), the bench's default decks (`--forks`): 1.0 µs and 40 KB at turn 1, 1.3 µs and 50 KB at turn 5,
1.9 µs and 87 KB at turn 15, against 10, 44 and 166 µs for the whole game. Between answers the tail is 2 entries; the
size is mostly the event history, copied whole.

**Bytes a sample**, from the spike's tensors, uncompressed: T128 is 32 KB of f32 features, 1 KB of i64 kinds (128 B as
u8), 256 B of masks and up to 512 B of visit distribution, about **34 KB**; T32 about 8.5 KB. That is 10× the 2-4 KB
estimated. The real encoding stores card ids into the text table rather than dense features and can store f16 and
u8, which is where the estimate has to come from; random spike features say nothing about zstd, so the compressed size
is measured on phase 2's shards.

**Throughput, revised.** On this CPU at T128, 256 evaluations a decision is about 6 decisions a second (20 at T32);
at 662 decisions a game that is about 800 games a day (2.6 k at T32), below the 2-15 k a day planned from a GPU. The
engine is not the cost: a whole game's rules are 28 ms against about 110 s of inference, and eight forks a decision at
turn 15 with the whole journal are 1.8 ms. So expert iteration needs the GPU numbers (unmeasured); without them, the
levers are the T32 and T64 buckets, fewer worlds and a smaller model, and CPU is enough for the bootstrap and for
parity and gate runs.

**Phase 1: the boundary (3-4 weeks).**
- `Knowledge` folded from history, with the event extensions it needs.
- Shared visibility code; `observe` with canonical numbering; `Game::new` assigns library ids after the shuffle.
- `fork_for_search` with its journal tail; `rebind_hidden` after a written inventory of every place an identity lives
  (or the replay fallback); `determinize` with metagame priors and legality.
- Done: `fork_for_search` with its journal tail and `Game::reseed` (tests in `cantrip-rules` and
  `cantrip-script/tests/fork.rs`), and library ids in dealt order. Engine 10.0 → 9.8 µs a decision.
- Done: the identity inventory (under Worlds); `rebind_hidden`, `reorder_library`, `restamp_hidden` and
  `Game::ask_again`, none of them journalled; `determinize` with `Prior` (`cantrip-ai/src/worlds.rs`). The redaction
  test deals worlds in A and B and checks they are the same game. It plays some of them on for 40 decisions, and it
  deals a world inside a resolution prompt and inside a cast, then answers and cancels; its ignored run (300 bot games
  and every scenario that looks into a hidden zone) passes in 31 minutes. Engine 10.1 → 9.8 µs a decision (noise: a real
  game takes none of these paths).
- **Gate:**
  - the three redaction tests over the scenario corpus and a few hundred bot games, with `CANTRIP_VERIFY=1`;
  - the fork and resolution-prompt scenario;
  - bench numbers before and after;
  - `test.cmd` green.

**Phase 2: encoding and a network that plays (3 weeks).**
- `encode` with buckets and the vocabulary; the card-text table; the factoring for every request kind with its
  validator test; shards with headers.
- The tactical-bot recorder; generation 0 by imitation.
- `cantrip-ai-host` with network-only answers; the `neural` BotLab seat.
- **Gate:**
  - generation 0, network only, scores 40%+ against the tactical bot on its decks (95% CI reported);
  - parity on CPU and CUDA.

**Phase 3: search (3 weeks).**
- PUCT, PIMC worlds, batched evaluation through the inference server, deadlines and fallbacks, game-length caps.
- **Gate:** under SPRT, search over generation 0 beats network-only generation 0, and beats the tactical bot.

**Phase 4: expert iteration (3-6 weeks, mostly machine time).**
- The self-play loop with mixed modes and oracle dropout, the scenario curriculum, SPRT gating, champions, shard
  retention.
- **Gate:** five accepted generations, and the hidden-mode seat scores 60%+ against the tactical bot on its decks,
  significant under SPRT.

**Phase 5: hidden information and the browser (3 weeks).**
- The belief head reweighting worlds; ISMCTS, kept if it wins.
- `cantrip-ai-wasm`, hidden-mode observations on a seat's steps (a protocol bump), `NeuralPlay` with its fallback.
- A Playwright test of a whole game with a browser neural seat on the WASM backend.
- **Gate:**
  - the mode gap measured and shrinking across generations;
  - the browser seat plays a game end to end;
  - the AMD parity run on real hardware.

## Testing

- **Fast** (`tools\test.cmd`):
  - encoding round trips and golden tensors;
  - the three redaction tests on a small set;
  - `Knowledge` against a refold;
  - `determinize` keeps known cards, zone counts and legality;
  - the fork-and-prompt scenario;
  - the factoring through the validator for every request kind;
  - the card-text table's missing count;
  - a tiny checked-in ONNX model run through tract, so search and evaluation are tested without a GPU or a native
    library.
- **Scenarios** for AI decisions use a **pinned tiny model** checked in for that purpose, never the current
  champion, so a retrain cannot break them. Positions are forced by the script, never by seeds.
- **Wall-clock** (marked; run when the work touches them): self-play smoke runs, the AI host, duels, the browser
  seat.
- **Parity**: at every export, on every backend the machine has.

## Risks

| Risk | What is done about it |
| --- | --- |
| A card identity survives `rebind_hidden` somewhere | Full-state world comparison; the written inventory; the replay fallback |
| `Knowledge` misses a way a seat learns a card | Folded from events, refold-checked; each missing case is an event extension and a scenario |
| AMD native inference does not build or run | The inference server covers AMD GPUs; CPU and WebGPU cover the rest; AMD stays "untested" until it runs |
| NixOS GPU environment eats time | Phase 0 budgets it; containers for everything Python and GPU |
| Throughput too low | Bootstrap, short problems, fewer worlds, Gumbel, smaller model; phase 0 numbers decide |
| PIMC's bias makes hidden play exploitable | Stated; mode gap measured; ISMCTS in phase 5 |
| A model trained on fixed decks does not generalise | Card-text embeddings; the metagame in phases 4-5; `--decks` pools in duels |

## Out of scope for the prototype

- Multiplayer (CR 800).
- Drafting: the draft bot stays.
- 17lands imitation.
- The language-model critic.
- Search in the browser.
- Training on Windows with AMD.
- Full and decklist modes for browser seats, which would hand the information to the human at the keyboard.

## What the review changed

An adversarial review read this plan against the code. It found these problems, and this revision answers them:

- **The fork as first written would assert.** Answers inside a resolution prompt revert to marks before the fork.
  The fork now keeps the journal's tail.
- **Overwriting one object's card would leave its identity in several other places:** predecessor objects,
  `last_known`, links, `statics_seen`, epoch caches and history. This is now one journalled `rebind_hidden`, built
  from a written inventory, with a full-state test and a replay fallback.
- **Object ids encoded decklist order.** Observations now number objects canonically, the engine assigns library ids
  after the shuffle, and the tests permute ids.
- **There was no model of what a seat knows.** `Knowledge` is added.
- **Observations could have shown another seat's announcement while it was in progress.** They now follow the host's
  deferral rule.
- **The plan's claim that acting-seat priors fix strategy fusion was wrong.** The bias is now stated, and
  multi-observer ISMCTS answers it.
- **Requests list candidates, not every legal answer.** Each request kind now has its own factoring, checked by the
  validator and backed out when rejected; payment uses the existing heuristic.
- **A search inside the shared rules host would stall every table.** Search now runs in its own process, called
  asynchronously with a deadline and fallbacks.
- **onnxruntime's ROCm provider is gone.** AMD GPU evaluation now goes through a PyTorch inference server; MIGraphX
  is optional and marked unverified; shapes are bucketed.
- **Full or decklist observations sent to a browser seat would reach the human player.** Browser seats are hidden
  mode only.
- **The throughput estimates were 3-10x high and left out the number of worlds.** The model is smaller, the
  arithmetic is redone, and imitation and short problems carry more of the load.
- **The belief covered only the hand, over the whole vocabulary.** It is now archetype-level and covers whole
  decklists under legality limits.
- **Suphx's oracle guiding was misread.** It is now oracle dropout, with modes kept apart in the data.
- **Several pieces were missing and are added:**
  - schema versions;
  - a compatibility matrix;
  - SPRT gating;
  - a pinned model for scenarios;
  - game-length caps;
  - mulligans;
  - shard retention;
  - unknown-card rows;
  - bench measurements for the core changes;
  - realistic times.
