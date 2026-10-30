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
| GPU inference for self-play and server seats | **A batched inference server in the same container as training** (PyTorch, CUDA or ROCm), fed by the Rust actors (built over TCP on localhost, the same on Linux and Windows; see Phase 4) | The one GPU path that is equally real on both vendors today: onnxruntime's ROCm provider is gone (removed around 1.23) and MIGraphX has no prebuilt `ort` binaries. |
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
  │       card-text embedder, inference server │◀───────── batches (TCP, localhost)
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
   - `GameState::reorder_library`, which renames the objects the tail's `ZoneRemoved` entries put back as their places
     are traded, so a rollback inside the fork lays the library out in the new order, and keeps in place the objects
     the tail made (`zone_order_fixed`), which a rollback takes away by place;
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

This is `cantrip_obs::encode` (its schema version and every width, list and vocabulary are in `ai/encoding.json`,
which a fast test keeps equal to the code; `encoder_hash` is its FNV-1a). Every token has the same shape: a kind
(pad, game, player, card, hidden, pool card, event, option, pooled), a card row (u16; 0 is the unknown card), 9
categoricals (zone 10, owner 9, controller 9, step 13, ask kind 27, event kind 66, from-zone 10, option token kind
29, action 13), 4 pointers to other tokens (255 for none), 96 numbers (powers-of-two scales, rounded through f16)
and a legal bit; dense, a 64-token position is 13.6 kB, and the compact form (the nonzero numbers only) is about
1.4 kB. Tokens come in a fixed order: the game, the players, the objects in canonical order (a library's top 3 kept,
the rest of it and the sideboards pooled into one token per zone and player), the decklist's pool cards, the latest
8 events, the latest 16 tokens already chosen at this request (pointing at what they named, never legal), then the
options of the current step. Full mode makes every object visible and marks those the seat could not have seen.

Past 128 tokens, places are pooled whole (sideboard, library, exile, graveyard, command, phased out, hand,
battlefield, stack), never an object an option or the ask names unless nothing else fits, and pooling a later place
made needless is given back; then pool cards go, then options. Any of those is a **cap hit**, logged in self-play:
if it is common, the cap moves. Events are dropped first and are no cap hit. Over the scenario corpus 9 positions in
32211 hit the cap; 93% are bucket 64.

A card's token carries its row of the **card index** (`cantrip_ai::CardIndex`): the engine's own card names, sorted,
row 0 the unknown card, and a hash of the names. Nothing about it is checked in: the runtime makes it from
`cantrip_engine::cards()`, so every card the engine has has a row, and each shard's header names the index its rows
are of (`cards`). The **card-text table** a row starts from is the network's, never the repository's
(`ai/cantrip_ai/embed.py`): 64 dimensions of TF-IDF over each name's text in `content/*/cards/*.json` (the card's own
name replaced), reduced by SVD. A fresh network fits the basis to its shards' names; the checkpoint keeps the names and
the basis, and a fine-tuned network keeps its rows and projects each new card into the same basis, so a row it trained
on never moves. Export bakes the table into the graphs and writes the names as the model's `cards.json`; the evaluator
maps the runtime's rows onto them by name. A checkpoint or shard from before (whose index was a checked-in table) takes
its names from the nearest `cards.json` above it, and its basis is fitted again from them.

### Body and heads

- **Body**: a pre-norm transformer encoder, 4 layers, width 128-192, 4 heads, masked attention, no positions.
- **Value**: the expected outcome for the seat (tanh), plus auxiliary targets (life swing over the turn, dead next
  turn).
- **Policy**: a pointer over the option tokens of the current sub-step, masked to the legal ones.
- **Belief**: the probability of each **archetype** in the metagame, and for each candidate card in the sampled
  pool (not the whole vocabulary) a probability of being in hand. Also the design's threat classes (counterspell
  up, removal up, trick up, lethal next turn). The labels are the true hidden state, which training has; the input is
  only the observation.

As built (`ai/cantrip_ai/model.py`): every input of a token is summed into the width: the card row through the frozen
64-dimension text table and a learned projection, a learned embedding per categorical and for the kind, and the 96
numbers through a linear layer; then the tokens the 4 pointers name, gathered by a one-hot matmul and projected,
are added. Then 4 pre-norm layers, width 144, 4 heads, FFN 576, attention masked to the real tokens: **1.24 M
parameters**. The value is tanh of a small MLP over the game token and the mean of the real tokens; the auxiliary
head, over the same, is the life swing (scaled by 10) and the probability of being dead next turn; the policy is a
linear score of each token, -1e9 where the step's option is not legal; the belief is a per-token logit of the card
being in an opponent's hand. There is no
archetype head yet (no shard carries the label), and the belief loss is off in Full mode, where the hands are in
the input. An exported model narrower than the table (the tiny test model) has the projection folded into a
per-row table.

### Information modes in training

**Full information first** (decided in Phase 2). Generation 0 and the first trained generations see `InfoMode::Full`
observations: both hands, and the library order as the engine knows it. The recorder records Full observations, and the
trainer and the gate run in Full mode. The Full model then bootstraps the Decklist and Hidden models, by distillation
or as their initialisation. The mode is in every shard's header (`mode`) and every game's meta (`modes`), so Decklist
and Hidden data is recorded later by the same code (`--ai-record-mode`). Encoding and factoring work in every mode
from the start; their tests run all three.

Following Suphx properly, once the other modes are trained:

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
| attackers | per creature: attack (at what) or stay, a defender at its cap (CR 508.1c, `max_attackers`) masked; then validated |
| blockers | per potential blocker: which attacker or none; constraints applied as masks where they are local, the whole block validated at the end |
| N cards or targets | one at a time, a slot at a time, "done" when "up to" allows |
| X | a choice among buckets up to what the seat can pay |
| damage assignment, divided amounts, counter removal | one unit or chunk at a time, in the rules' order where they impose one |
| trigger order, card order | one at a time, "next first" |
| a card name | among the format's names the belief and the seat's own deck make plausible, not all names |
| mana payment | the existing payment heuristic (`auto_pay`, moved from `cantrip-script/src/run.rs` to `cantrip_engine::pay`, which `cantrip_ai::pay` pays with first); a policy step later only if duels show it loses games |

An assembled answer is checked with the engine's validator before it is sent. In the search, a rejected answer is
masked and the step backtracks. In the browser (no validator), a rejected answer makes the seat fall back to
the network's next choice, then to the tactical bot. A fast test sends every factoring's answers through the
validator, for every request kind the scenario corpus raises.

How it is built (`cantrip_obs::factor`, `cantrip_ai::answer`):

- **The ask.** A seat's own request carries an `Ask` in its observation (`RequestObs::ask`): the request's candidates
  and constraints in canonical terms, its lists sorted (options, triggers, replacements, subtypes, counter pools), with
  the permutations back to the engine's kept outside the observation (`cantrip_ai::ask::Perms`).
- **Tokens.** A `Machine` for the ask offers one `Step` at a time: tokens `{kind, a, b, n}` (a `Tok`, two canonical
  references and a small number) and a mask of the legal ones. `Done` is a token wherever stopping is allowed;
  `Cancel` is offered on the first step where the request may be cancelled. Every machine finishes within
  `factor::bound(ask)` steps, asserted on each choice. The finished machine holds a `FactoredAnswer`.
- **X buckets.** X (and a kicker count, or an "unless pays" X) is chosen among `X_BUCKETS` = 0 to 10, 12, 15, 20, 30,
  and X as it is now; a bucket over `x_most` is masked. `x_most` is an estimate: the pool (restricted mana included)
  and, per untapped permanent, the most one of its mana abilities adds, less the cost's colored symbols, divided by the
  number of {X}; life for an X paid in life; loyalty for a loyalty X. It counts no cost reduction, so an X a reduction
  makes affordable may be masked. The engine answer moves X a step at a time (`IncreaseX`, `DecreaseX`) to the bucket
  and then confirms.
- **Payment** is no policy step: `AutoPay`, and paying an "unless pays" cost, are made by `cantrip_ai::pay` (the
  payment heuristic, moved from `cantrip-script`), one engine answer at a time.
- **The decomposition rule** (`decompose`, the inverse): a list the answer orders (cards, targets, modes, colors,
  trigger order, placements) is chosen in the answer's own order, placements destination by destination; a declaration
  per object (attackers, blockers, damage shares, divided amounts, counters removed) is walked in canonical object
  order, an amount a unit at a time; X is the highest legal bucket at or under the X wanted; a payment is `AutoPay`;
  then `Done` wherever the machine still asks.
- **Masks and the validator.** The masks carry every local constraint. What only the whole answer shows is left to
  `check_answer` (`Rejection::global`): menace and "can't be blocked except by N", the block requirements' maximum
  (CR 509.1c), bands, a name that is no card or not of the kind asked for, a total power or mana value short of its
  minimum, a selection rule or a target slot's rule together (CR 601.2c) past a running total of mana value, and more
  wanted than there is to choose. Any other refusal is a fault of the factoring. The attack requirements (CR 508.1d,
  goad among them) are masked by what each attack obeys and the most a declaration must, both from the engine's
  `AttackRules`: an attack (or staying home) is offered where the creatures still to come, each at its best, can make up
  the rest. Without a cap on attackers (CR 508.1c) each restriction is on one attacker at a time and that is exact; a
  cap is room the attackers share, where it is only a bound - no legal declaration is masked, and one the caps leave
  short is refused globally.
- **The test** (`cantrip-ai/tests/factoring.rs`): at every request a scenario answers, the answer is put in canonical
  terms, decomposed, replayed through the machine, mapped back and validated; then three seeded random walks per
  request must be accepted or refused only globally. The fast test takes every 50th scenario; `-- --ignored` takes the
  corpus (7404 scenarios, 6 s): every request kind raised, every round trip good, no walk refused locally, and the only
  global refusals are blocks (2.2% of blocker walks).

## Search

### Moves not branched on

- A request with one legal answer is answered without a node.
- A priority where the policy gives everything but pass under 1% is passed after the first visit says so.
- Mana abilities are paid, never moved.

### Branch trimming

Edges are cut by value, after a batch's backups (`search.rs`). The value is the edge's mean for the seat choosing
at that node, so every node trims moves bad for itself, and an opponent's good reply is never cut.

- **Immediately:** an edge at or below `prune_bad` (default -0.80) when a visited sibling is better.
- **After a look:** an edge still `prune_margin` (0.20) below the best visited sibling, once it has been descended
  `prune_after` (2) times and at least `prune_after_replies` (1) opponent decision below it has been expanded.
- **Never:** the best edge at a node (so every node keeps an option), an unvisited edge, an edge whose child has not
  been materialized, and an edge with pending virtual loss.
- `Child::Dead` still means the engine refused an answer; a pruned edge is not a refusal.
- Pruning is per world and deterministic. Root visits and the root value leave pruned edges out, so the answer and the
  recorded visit target cannot choose one.
- **A cut holds only while the trim does** (`prune_reversible`, on by default). A cut edge keeps the value it had when
  it was cut, a shallow estimate, while its siblings' values keep moving as the search finds the replies to them; each
  later path restores the cut edges at its nodes that the rule would no longer cut. Cut for good (the search before
  2026-10-27), an edge measured against a sibling's early, optimistic value was never looked at again once that
  sibling fell below it: in the advisor, a trained network at 2,608 simulations cut "Cast Carnivorous Cultivator" in 6 of 8 worlds
  after 8-18 visits and tapped a Forest instead, where the untrimmed search, and the reversible one, cast it.

`Prune`, `PruneBad`, `PruneMargin`, `PruneAfter`, `PruneAfterReplies` and `PruneReversible` are `Config` fields and
`AiStart` settings; `Searched` reports `pruned`, `pruned_immediate`, `pruned_shallow` and `restored`.

### PUCT in worlds (PIMC)

1. Draw `W` worlds: 1 in full mode, 4-16 otherwise.
2. In each, PUCT: select by `Q + c · P · √N / (1 + n)`; expand with one batched call (priors and value from the
   acting seat's observation of that world); back up. Moving is `Game::goto` in the fork. Sub-steps of a factored
   answer are search nodes the engine never sees.
3. Sum the root visits across worlds; play the most visited (sampled proportionally for self-play's opening moves).

**A world is a game the seat could be in.** Its root observation for the seat must be the real one
(`Config::reuse_check` compares each world dealt fresh with it, `Searched::fresh_checked` and `fresh_mismatched`;
`every_world_dealt_shows_the_seat_the_game_it_is_in` requires it over whole games, mulligans included). A card the seat saw is that card in every zone, wherever the knowledge has lost track of
which copy is which object (a hand shuffled back by a mulligan, a revealed card put into a library); a library the
seat is searching keeps its order; and a request whose waiting run drew on the generator before it (the opening after
a mulligan: shuffle and redraw) is answered by the network alone, since every world would re-run that run on its own
generator and deal it again (`search::random_before`, `GameState::random_since`). A world's cancel guard is moved to
every board a take-back can return to there (`search::world_boards`, `CancelGuard::rekey`).

**The bias, stated.** In each world the tree's Q values come from a fully determined game, so the simulated opponent
plays as if it knew the AI's hand. Priors from the acting seat's own observation soften this, but values are not
fixed by it. PIMC overvalues bluffs and plays into tricks it "knows" are absent. The mode gap (below) measures it.

**ISMCTS, phase 5.** Multi-observer information-set MCTS: one tree per seat, each keyed by what an action looks like
to that seat (card name and targets, not ids). A fresh world each iteration, with both seats' hidden cards resampled
from the other's knowledge. It is kept only if it wins the duel.

Also noted:

- The game tree (`tree.rs`) never prunes; the search's own edges are cut by the trimming above. A fork is thrown
  away after each decision, so its memory is bounded per decision. That memory is measured.
- Request ids are part of a child's key, so transpositions are not merged. That is acceptable for now.
- Leaves are batched across worlds with virtual loss (`Config::batch`). Across games they are not yet: BotLab runs one
  AI host a game lane, each with its own evaluator, so a batch never holds two games' leaves (Phase 3, under Phases).

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

| Backend | Where | Role | Status (Phase 2) |
| --- | --- | --- | --- |
| `RemoteEvaluator` → the PyTorch inference server (`ai/cantrip_ai/serve.py`) | NVIDIA (CUDA), AMD (ROCm), or the CPU | the GPU path for self-play and server seats on both vendors | **built** (Phase 4, `cantrip_ai::remote`); parity passes on CUDA (an RTX 4080, Windows); ROCm not yet run |
| `OrtEvaluator` CPU | any machine | the default in-process path once a server seat needs speed | **built** (`cantrip_ai::eval`, `--features ort`, off in the workspace's default build because `ort` downloads its binaries); parity passes |
| `RemoteEvaluator` → the native inference server (`cantrip-infer`), or `GpuEvaluator` in process (`cantrip_ai::gpu`) | NVIDIA (onnxruntime's CUDA provider) | the GPU path without Python on the request path: same protocol, batching and reproducibility as serve.py; one CUDA graph launch a batch | **built** (`--features gpu`, onnxruntime-gpu loaded at run time; docs/ai-host.md, "The native inference server"); parity passes at fp32, tf32 and fp16 on an RTX 4080 (Windows); AMD stays on serve.py |
| `OrtEvaluator` MIGraphX | AMD, Linux | optional, unverified until it is built and run on AMD | not built |
| `TractEvaluator` | pure Rust, CPU | tests; parity reference; the workspace's default | **built** (`cantrip_ai::eval`, tract-onnx 0.23); parity passes |
| onnxruntime-web WASM / WebGPU | the browser | WASM tested; WebGPU where the browser has it | spike only (Phase 0) |

A model directory (`ai/cantrip_ai/export.py`) holds `model-t32.onnx`, `model-t64.onnx` and `model-t128.onnx` (batch
free, opset 17), `cards.json` (the card each table row is), `manifest.json` and `reference/`. `Model::load` refuses
a schema or encoder-hash mismatch; a card index hash that differs maps the runtime's rows to the model's by name, a
card the model never saw becoming the unknown card. `FeatureBatch::new` pads a batch of `Encoded` positions to the
largest bucket in it; `Outputs` holds the value, each position's option logits (illegal ones -1e9) and the two
auxiliary targets.

**Parity is a test.** As built, it is `cantrip-ai`'s `tests/eval.rs`: the fast tests run the tiny checked-in model's
48 references (scenario positions) through tract, and `model_parity` (ignored; `CANTRIP_MODEL` names the directory,
`--features ort` adds onnxruntime) runs a real model's 512 held-out references; `export.py` runs them through
onnxruntime in Python when it writes them. As planned, `ai-parity` runs a fixed set of encoded positions (a few thousand, from scenario replays)
through every available backend and PyTorch. It checks that values agree within 1e-3 and that the top choice is the
same on 99.5%+. The CPU backends run in every check. The GPU half runs on whatever vendor the machine has. The AMD
half (ROCm training and the inference server, and MIGraphX if built) runs on AMD hardware, borrowed or a rented
ROCm instance, at the phase gates. **Until that run passes, AMD support is untested, and this file says so.**

AMD consumer cards (RDNA) are checked against the ROCm support matrix; some need `HSA_OVERRIDE_GFX_VERSION`. The
export script pins the exporter (TorchScript-based, not dynamo, unless phase 0 says otherwise) and the opset, and
checks the graph against an operator allow-list.

## Training

The runbook - setting up on CUDA, ROCm or the CPU, generation 0 and the generation loop - is
[ai-training.md](ai-training.md).

### Data

A **shard** (zstd) has a header with the observation schema version, the encoder hash, the card-set hash, the engine
commit, the model and backend that produced it, and the seeds. Each sample holds:

- the encoded observation and its legal mask;
- the search's visit distribution;
- the outcome;
- the auxiliary targets and the true hidden state;
- both seats' modes.

Size, measured on generation 0's shards (`cantrip_ai::shard`, the compact form of `cantrip_obs::encode`): about
**58 bytes a sample** compressed, at 350-475 samples a game, so about 24 KB a game. The 2-4 KB estimated was for dense
features; a sample stores card rows and bucketed numbers, and a game's positions differ little from one to the next.
A shard's layout and the flags that write one are in `docs/host-protocol.md` ("Recording for the AI player");
`ai/shards.py` reads them (`--check` checks each step's chosen option is legal).
Shards are kept for the last eight generations, and older ones are deleted unless pinned.

**Reproducibility.** The engine is deterministic and self-play's seeds are recorded. The inference server keeps it so:
a row's outputs never depend on what it was batched with (one padded shape per bucket, deterministic kernels), so a
self-play run through the same server configuration (batch sizes, precision, GPU and PyTorch) plays the same games,
bit for bit, whatever the lanes and timing. A run through another configuration, or the CPU backends, differs within
parity's tolerance.

### Compatibility

A model's manifest names:

- the observation schema version;
- the encoder hash;
- the card index hash (the model's own names are its `cards.json`);
- the engine commit it was trained against.

The loader refuses a mismatch of schema or encoder. A newer card set is accepted, with unknown-card rows for the cards
the model was not trained with; the next generation gives them rows (above, "Inputs").

### Bootstrap, then expert iteration

1. **Imitation of the tactical bot.** BotLab's games already run on the native host
   (`dotnet/tools/Cantrip.BotLab/Match.cs:105`). A host flag (`--ai-record DIR`) writes each decision's observation
   and the tactical bot's answer, decomposed into the plan's sub-steps (`cantrip-obs` turns an assembled answer into
   its steps). Tactical against tactical through the native host plays about two games a second on sixteen cores
   (the tactical bot's 20 ms or so an answer is the cost, not the engine or the recorder, which adds about 4%), so
   about 7 k games and 2.5 M samples an hour; it is shaped like the tactical bot's play, which is what generation 0
   is for. The tactical bot is deterministic, so a recorded self-duel deals every game its own shuffle
   (`--each-shuffle`): a pair of games from one shuffle, seats swapped, would be one game twice.
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
- **SPRT** (`cantrip-botlab duel --sprt`). The sequential test is H0: a 50% win rate for seat A, H1: 55%, with α = β =
  0.05. The log-likelihood ratio is `W·ln(p1/p0) + L·ln((1-p1)/(1-p0))`, a draw counting half a win and half a loss.
  H1 is accepted at `ln 19 = +2.944` and H0 at `-2.944`. Once the ratio crosses a bound, no new games start, and the
  games already being played finish and are counted. The report gives the game and ratio at the verdict, and the ratio
  over every finished game. `--games` is the cap. A run that reaches it without a verdict reports the ratio it ended
  on.
- **Champions.** A frozen model and its manifest are kept under `ai-data/champions/<generation>/` (beside the
  repository; `CHAMPION.json` says how it won), `champions/current` names the one in force, and
  `champions/history.json` records every gate, accepted or not (Phase 4).

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

- **What unblocks NVIDIA:** the NixOS host's config with `hardware.nvidia` (the driver) and
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
is measured on phase 2's shards: about 58 bytes a sample (under Data).

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
- Done: the factoring for every request kind (`cantrip_obs::factor`; under Answers), the ask in each seat's own
  request, the mapping back to the engine's answer with the validator's check (`cantrip_ai::answer`), and `auto_pay`
  moved to `cantrip_engine::pay` (so the scenario runner does not depend on `cantrip-ai`). Its test runs the whole scenario corpus (ignored; a 1-in-50 sample in the fast tests).
- Done: `encode` (`cantrip_obs::encode`; under Inputs) with buckets, pooling, the compact form and the vocabulary in
  `ai/encoding.json`; the card index (`cantrip_ai::CardIndex`, the engine's own cards). Its tests encode a sample of the
  corpus in every mode (the whole of it ignored), keep goldens (`CANTRIP_GOLDEN=1` rewrites them), and the redaction
  test checks that observations the seat cannot tell apart encode the same.
- Done: the recorder (`cantrip_ai::record`, shards in `cantrip_ai::shard`), the host's `--ai-record` flags
  (`docs/host-protocol.md`), the bot lab's `duel --ai-record DIR`, and `ai/shards.py`, which reads the shards. The
  recorder's test covers the whole scenario corpus (ignored; a sample runs in the fast tests). In the whole corpus, 58
  answers make no sample: a creature type the encoder's cap cut (`docs/known-issues.md`, KI-78).
  Generation 0, in Full mode, has 12000 distinct games of tactical against tactical: rw-aggro against ub-reanimator,
  and each deck's mirror. That is 4.95 M samples and 287 MB, about 58 bytes a sample, played in 2 h on 16 cores with no
  failed answer. It lives outside the repository, in `ai-data/gen0/` beside it, with a README.
- Done: the network (`ai/cantrip_ai/`: `model`, `dataset`, `train`, `export`; under Body and heads), the
  `Evaluator` trait with `TractEvaluator` (the default) and `OrtEvaluator` (`--features ort`, off by default), the
  manifest check, and network-only answers (`cantrip_ai::net::answer`: the factoring walked step by step with the
  policy's best or a sampled legal option, a refused walk's deepest open choice banned and retried 8 times, then
  first-legal walks; payment by `pay`). The fast tests (`crates/cantrip-ai/tests/eval.rs`) run a 10 k-parameter model
  checked in under `tests/data/tiny` (856 kB with its references): tract within 1e-3 of PyTorch, the Rust decoder
  against the Python one, a mismatched schema or encoder hash refused, a newer card set mapped by name, and
  network-only answers to a 1-in-60 sample of the corpus's requests all accepted by the validator. Over the whole
  corpus (ignored), 567,851 requests are answered with none refused and no fallback.
  Generation 0 (`ai-data/models/gen0/`, with a README): width 144, 4 layers, 1.24 M parameters, one epoch of the
  Full-mode gen0 data (3.80 M samples seen, a quarter of the single-option ones kept) in 2 h 22 min on 10 CPU threads,
  about 445 samples a second. Held out by game, it takes the tactical bot's option on **96.3%** of the steps with a
  choice, where the first legal option is the bot's on 73.5%; value MSE 0.486 (0.556 at step 2000), its sign right
  on 82%. Parity over its 512 references: tract and ort CPU within 4e-5 of PyTorch, the same argmax on all of them.
  **CUDA parity has not run**: the development machine has no NVIDIA device.
- Done: `cantrip-ai-host` (`crates/cantrip-ai-host`, `docs/ai-host.md`). It is a mirror of the table that the
  native host's commands are teed to, and it answers a seat by the network alone when asked (`AiStart`, `AiResult`).
  Also done: the `neural` BotLab seat (`dotnet/tools/Cantrip.BotLab/Neural.cs`) and `tools/bot-duel.sh`. Bugs that
  playing found and fixed:
  - tapping chose its sources greedily, and could not read a land whose ability fixes its yield (Plateau);
    `pay::tap` now matches symbols to sources;
  - a cast whose payment or cost the seat cancelled was chosen again forever. `net::Payment` now filters a cancelled
    cast or activation out of the AI's choices while the board before it stands (`cancel::board_hash`), so it is
    not re-offered after the undo; a new board offers it again (KI-79, now fixed generally, not by a per-card cap);
  - an empty card name read back as a nameless token's card (`crates/cantrip-host/src/answer.rs`).
- **Gate result** (2026-10-19). Generation 0, network only, Full mode, ort CPU, `Choice::Best`, against the tactical
  bot. 400 games, seats swapped every other game, `--parallel 14`; every game finished. Generation 0 won **184 of
  400, 46.0% ± 4.9%** (95% CI 41.1-50.9%), so **the 40% bar is met**.

  | Decks | Neural won | Win rate |
  | --- | --- | --- |
  | rw-aggro against ub-reanimator, 160 games | 79 | 49.4% ± 7.7% |
  | (neural playing rw-aggro) | 67 of 80 | 83.8% |
  | (neural playing ub-reanimator) | 12 of 80 | 15.0% |
  | rw-aggro mirror | 35 of 80 | 43.8% ± 10.9% |
  | ub-reanimator mirror | 42 of 80 | 52.5% ± 10.9% |
  | the four demo mirrors, which gen0 never saw | 28 of 80 | 35.0% ± 10.5% |
  | (1-black-red) | 12 of 20 | |
  | (2-white-blue) | 6 of 20 | |
  | (3-green-red) | 3 of 20 | |
  | (4-red-black) | 7 of 20 | |

  - On gen0's own decks it won 156 of 320 (48.8% ± 5.5%).
  - In seat 1, 95 of 200 (47.5%); in seat 2, 89 of 200 (44.5%).
  - On the play, 104 of 200 (52.0%); on the draw, 80 of 200 (40.0%).
  - An answer, p50 / p95 / max, in the rw-aggro against ub-reanimator duel:

    | | Neural (29070 answers) | Tactical (29154 answers) |
    | --- | --- | --- |
    | Whole | 9.8 / 19.9 / 99.5 ms | 4.1 / 11.6 / 74.1 ms |
    | Native host's part | 2.7 / 7.6 / 22.6 ms | 2.6 / 7.5 / 21.2 ms |
    | Seat's own part | 6.7 / 14.6 / 94.2 ms | 1.1 / 5.9 / 73.5 ms |

    The neural seat's own part breaks down as:
    - encoding, 0.08 / 0.10 / 4.1 ms;
    - the network, 3.0 / 6.2 / 87 ms;
    - the rest of the AI host, 0.8 / 2.2 / 7.1 ms;
    - the process round trip, 1.0 / 5.0 / 19 ms.
  - Steps and evaluations an answer: 1 / 1 / 14. That is 294 evaluations a second at batch 1 inside a 14-game run.
  - The other runs were within about 10% of these timings.
  - Fallbacks: 8, 21, 10 and 305 (the demo decks, mostly the cancel guard). Late answers: none. Errors: none.
- **Parity on CPU passes for gen0** (512 references, `model_parity`):
  - tract: within 2.8e-5 of PyTorch, the same argmax on all, 318 positions a second at batch 1;
  - ort: within 3.8e-5, the same argmax on all, 875 positions a second at batch 1 and 1709 at batch 64.
- **CUDA parity is pending.** It needs an NVIDIA device: `hardware.nvidia` with nvidia-container-toolkit and
  `docker build -f ai/Dockerfile.cuda`, or native `ort` CUDA with CUDA 12 and cuDNN 9.
- **Status:** the Phase 2 gate is met on CPU, and CUDA parity is outstanding.
- **Gate:**
  - generation 0, network only, scores 40%+ against the tactical bot on its decks (95% CI reported): met, 46.0% ±
    4.9% (above);
  - parity on CPU and CUDA (CPU passes; CUDA not run).

**Phase 3: search (3 weeks).**
- PUCT, PIMC worlds, batched evaluation through the inference server, deadlines and fallbacks, game-length caps.
- **Gate:** under SPRT, search over generation 0 beats network-only generation 0, and beats the tactical bot.
- Done: the search, `cantrip_ai::search` (under Search, and the module's own text):
  - PUCT over the factoring's sub-steps: a node is one choosing sub-step, and its edges are the legal options;
  - the network's softmaxed legal logits as priors (`prior_temperature`), and its value at the leaves, backed up
    from each node's acting seat's side;
  - leaves batched (`batch`, 8) with a virtual loss on each edge on the way down, across worlds too;
  - `W` worlds through `determinize` in Decklist and Hidden modes, root visits summed by the option's token, and one
    fork in Full mode;
  - forks reseeded, so a line never knows the real game's coming shuffles, and the same position, seed and budget
    give the same answer;
  - requests with no choice, and a payment run after its first answer, are answered in the line without a node;
  - a line ends at a game's end, or is a draw (value 0) `turn_cap` (40) turns past the root or after `MAX_FORCED`
    requests with no choice.
- Done: the priority shortcut. When every option but passing has a prior under `shortcut` (0.01), passing is the
  node's only edge. At the root, the seat passes once the root is expanded.
- Done: branch trimming. An edge at or below `prune_bad` (-0.80) is cut when a visited sibling is better, and one
  still `prune_margin` (0.20) below the best is cut after `prune_after` (2) descents and `prune_after_replies` (1)
  opponent decision. The best edge is never cut.
- Done: budgets and fallbacks. The budget is a simulation count or a deadline, checked before each batch. The answer
  is walked in the real game's factoring, taking the most visited option or a sample. A sub-step the search never
  reached is the network's. A refused answer, a search with no simulation in time, and a forced request go to the
  network alone (`net::answer_by`). A Hidden-mode search with no `Prior` is an error, which the host turns into
  `InvalidSetup`.
- Done: tree memory is measured, not estimated. `cantrip_ai::alloc` is a counting allocator that `cantrip-ai-host`
  installs, and `TreeBytes` is the search thread's peak live bytes above where the search began.
- Done: the AI host. `AiStart` takes `Simulations`/`SearchMs`, `Worlds`, `SearchSeed`, `CPuct`, `PriorTemperature`
  and `Batch`, and `--prior DIR` gives the decks a Hidden-mode world is dealt from. `AiResult.Search` carries the
  stats (`docs/ai-host.md`).
- Done: BotLab. The `neural[:mode][@N|@Nms]` seat searches, and neural against neural is a duel. `--ai-worlds`,
  `--ai-batch`, `--ai-cpuct` and `--ai-prior-temperature` are passed through. `--sprt` is the sequential test (under
  Measuring), and `--mirrors` with two decks plays half the games as the matchup and a quarter as each deck's mirror.
- Done: on Linux, BotLab's three-minute limit a game no longer counts a neural seat's own part of its answers. On
  Windows the limit was already the thread's processor time, which the AI host's process is not part of.
- Tests: `crates/cantrip-ai/tests/search.rs` has 8 tests, each on a forced position:
  - a win in one found with nothing from the network (the tiny model);
  - a lost position valued as lost from the seat's side, and a value backed up from each seat's side;
  - a budget of simulations spent exactly, and a deadline read from the clock it is given;
  - the same position, seed and budget give the same answer;
  - two games a seat cannot tell apart are searched alike (Hidden mode, worlds from a `Prior`);
  - searched answers over a sample of the scenario corpus (one file in 90) are taken by the engine.

  In `crates/cantrip-ai-host/tests/ai_host.rs`, a searched answer carries its stats and the table takes it.
- **Batching across games** was not built in Phase 3. BotLab runs one AI host a lane (`--workers 1 --threads 1`), and
  each has its own evaluator. Inside a 16-lane run, batch 8 gives no more evaluations a second than batch 1 (about
  190-290 a second a lane at full load, against 870 a second for a lone batch-1 run). The CPU is saturated by sixteen
  batch-1 evaluators, so a shared queue would buy nothing here. On a GPU it is the whole point: one evaluator taking
  every lane's leaves in batches. That is the `RemoteEvaluator`, built in Phase 4 (below).
- **Throughput** (2026-10-19). Generation 0, Full mode, ort CPU, 16 games in parallel on 16 cores, against the
  tactical bot, 32 games each, rw-aggro against ub-reanimator (shuffles from 3001). A searched decision is one that ran
  the budget; the rest were forced, a payment, or passed by the shortcut.

  | Budget | Searched decisions a second, a seat | Evaluations a second, a seat | Decisions a second in all, a seat | Answer p95 | Tree, p50 / p95 / max |
  | --- | --- | --- | --- | --- | --- |
  | network only (Phase 2) | | 294 | about 100 | 20 ms | |
  | 16 simulations | about 12 (from 187 evaluations a second) | 187 | 51.2 | 94 ms | 0.25 / 1.9 / 5.2 MB |
  | 64 simulations | 3.3 | 208 | 16.7 | 345 ms | 3.8 / 5.7 / 11.9 MB |
  | 256 simulations | 0.8 | 192 | 4.3 | 1.4 s | 12.4 / 17.7 / 75.9 MB |

  - The 16-simulation run predates splitting out the shortcut, so its count of searched answers includes the shortcut
    passes. Its rate is the evaluations a second divided by 16.
  - About 60% of the answers the search sees are passes the shortcut takes, so a game at 256 simulations is about 3.6 s
    a lane on the clock: a 1,000-game duel is about an hour on 16 lanes. **256 simulations is the gate's budget.**
  - In the gate duels, a tree at 256 simulations was 12.3-12.6 MB at p50 and 17-28 MB at p95. The largest was 200 MB,
    in the demo decks: a long line's game log, which the search's forks grow and throw away each decision.
- **Gate result** (2026-10-19). Generation 0 at 256 simulations, Full mode, one world, `c_puct` 1.5, prior
  temperature 1, batch 8, ort CPU, `--parallel 16`, under SPRT (H0 50%, H1 55%, α = β = 0.05). The decks were
  rw-aggro against ub-reanimator with `--mirrors`: half the games the matchup both ways round, a quarter each deck's
  mirror. Seats were swapped every other game, with fresh shuffles. No tuning round was needed.

  | Duel | Verdict | Result over every finished game |
  | --- | --- | --- |
  | search against network only (shuffles from 562714740) | **H1 at game 65** (LLR 2.984) | 60 of 80, **75.0% ± 9.5%** (LLR 3.611) |
  | (the matchup) | | 27 of 36, 75.0% |
  | (the mirrors) | | 33 of 44, 75.0% |
  | search against the tactical bot (shuffles from 7161008) | **H1 at game 172** (LLR 2.948) | 115 of 187, **61.5% ± 7.0%** (LLR 3.375) |
  | (search playing rw-aggro against ub-reanimator) | | 39 of 46, 84.8% |
  | (search playing ub-reanimator against rw-aggro) | | 15 of 47, 31.9% |
  | (rw-aggro mirror) | | 28 of 52, 53.8% ± 13.5% |
  | (ub-reanimator mirror) | | 33 of 42, 78.6% ± 12.4% |
  | the four demo mirrors against the tactical bot, 160 games, no SPRT (shuffles from 846855268) | | 76 of 160, 47.5% ± 7.7% (Phase 2's network alone: 35.0% ± 10.5%) |
  | (1-black-red / 2-white-blue / 3-green-red / 4-red-black) | | 26 / 13 / 8 / 29 of 40 |

  - Against the tactical bot on gen0's own decks, the network alone scored 48.8% ± 5.5% in Phase 2; search scores
    61.5% ± 7.0%.
  - Against the tactical bot, search won 69.9% on the play and 53.2% on the draw. In seat 1 it won 58.8%, in seat 2
    64.4%.
  - In the duel against the tactical bot, two rw-aggro mirrors (games 1108 and 1404, shuffles 7161562 and 7161710)
    hit the three-minute limit on the clock: about 700 answers of which a third took search time. They were not
    counted. Counted as losses, the result is 115 of 189 (60.8%), with LLR 3.16, still past the bound. The fix to the
    limit (above) came after that run; the demo duel ran with it, and every game finished.
  - Fallbacks (the KI-79 cancel guard): 42 against the network, 90 against the tactical bot and 539 on the demo decks.
    The network-only seat had 6 in the same duel. There were no late answers and no errors.
- **Status:** the Phase 3 gate is met in Full mode on CPU. Decklist and Hidden searches work and are tested; they have
  not been measured in a duel.

**Phase 4: expert iteration (3-6 weeks, mostly machine time).**
- The self-play loop with mixed modes and oracle dropout, the scenario curriculum, SPRT gating, champions, shard
  retention.
- **Gate:** five accepted generations, and the hidden-mode seat scores 60%+ against the tactical bot on its decks,
  significant under SPRT.
- **Step A: the loop, in Full mode** (every seat Full; the mode stays a parameter).
- Done: self-play, `cantrip-selfplay` (`cantrip_ai_host::selfplay`; `docs/ai-host.md`, "Self-play"):
  - both seats search with one model at a fixed simulation count, never a deadline;
  - a seat's first 20 searched decisions are sampled from the root's visits at temperature 1, the rest most visited;
  - games past 60 turns or 6,000 answers are capped (every outcome 0) and counted;
  - the producer `selfplay:<model hash>@<simulations>` goes in the shard header;
  - `--resume` reads the closed shards already in the output and skips the games whose seeds they hold, so an
    interrupted run plays only the rest of its schedule; `--shard-games` (100) bounds what an open shard can lose.
  
  It runs in process (a lane is a thread, an evaluator and a shard writer) rather than as BotLab neural against
  neural with `--ai-record`. That is less code: the search's `RootStep`s go straight to the recorder
  (`record::record_searched`), and are never carried over the AI host's JSON to the native host, which records from
  its own tree. It also has no mirror, no process round trips and no C# per game, so the 16 lanes are all search.
- Done: visit targets. A sub-step the search visited is labelled with its root visits over the step's legal options,
  normalised to sum to 1. Any other sub-step is labelled with the option chosen, as Phase 2's one-hot. Each searched
  decision's root value goes in the game's meta (`root_values`). `cantrip_ai.dataset` reads both. `train.py`'s policy
  loss is cross-entropy against the visit distribution, so a one-hot is the old loss. `--value-mix X` blends the root
  value into the value target (0 by default: the outcome alone). `top1_visits` reports agreement with the most
  visited option.
- Done: training on a window. `--data DIR:WEIGHT` takes several directories. A weight below 1 keeps that fraction of
  the directory's games, chosen by a hash of the seed. A weight above 1 deals each sample that many times an epoch. `--init CHECKPOINT` fine-tunes from the previous model. `--resume` continues from
  `OUT/checkpoint.pt` with its optimizer, step and epoch; a checkpoint is written every 200 steps and at each
  evaluation, atomically. Export, manifest and parity are Phase 2's.
- Done: shard retention, `python -m cantrip_ai.retention --root ai-data --keep 8`. For each generation older than the
  last eight, it deletes everything but the README, `generation.json` and the marker files, and the generation's
  model, then marks it `RETIRED`. gen0 is always kept. A `PINNED` file or `--pin genN` keeps any other. `champions/`
  is never touched.
- Done: the gate. The candidate plays the champion, both searching at the self-play budget (BotLab `--model-b`),
  under SPRT (H0 50%, H1 55%, α = β = 0.05), with seats swapped, `--mirrors` on rw-aggro and ub-reanimator, and at
  most 1,200 games. An accepted generation is copied to `champions/genN` with `CHAMPION.json`, and
  `champions/current` moves to it. Every gate goes into `champions/history.json`.
- Done: the driver, `python -m cantrip_ai.generation --root ai-data --generation N` (run from `ai/`). Its stages are
  build, self-play, train, export, parity, gate, promote and retire. Each logs to `genN/logs/<stage>.log`. After each
  stage, its times and numbers go into `genN/generation.json` and `genN/README.md`. A stage already done is skipped
  (build runs every time, so a resumed run never uses a stale binary), so a rerun resumes. Inside a stage, self-play
  keeps its closed shards (and a `.part` that reads whole) and reads the game seeds back from them, so only the games
  they do not hold are played;
  training continues from `models/genN/train/checkpoint.pt`, with its optimizer, step and epoch.
  - The window is the generation's own self-play at weight 1, the earlier self-play generations still held at 0.5,
    and gen0's imitation data at 0.2.
  - Training is a step count: `--train-hours 0.65` at the measured 445 samples a second, batch 256, so 4,067 steps
    (planned as 1.5 hours, 9,386 steps; generation 1 overfit its value at that, below). lr is 2e-4 with 200 warmup
    steps, seeded by the generation. `--value-mix 0.5`: a searched position's value target is half the outcome, half
    the root value.
  - Shuffles: self-play is dealt from `N·10⁷`, and the gate from `N·10⁷ + 5·10⁶`.
  - `--dry-run` prints every command. `--tiny` runs a whole generation at toy scale.
  - `--retrain REASON` runs a rejected generation again from its training, with the self-play kept. The earlier
    attempt's stages go under `attempts` in `generation.json` and get a row in the README. Its logs and training
    metrics go to `logs/attemptK/`, and its model is deleted.
- Tests:
  - `crates/cantrip-ai-host/tests/selfplay.rs`: the visits sum to 1 over legal options, and the shards and report
    read back;
  - `ai/tests/test_dataset.py`: the dataset reads visit targets and root values;
  - `ai/tests/test_retention.py`: retention on a temporary directory of 12 generations;
  - `ai/tests/test_generation.py`: the window's weights, the gate's parsing, a dry run that writes nothing, and
    `--retrain` setting a rejected attempt aside;
  - a `--tiny` generation run once end to end (self-play, train, export, parity, gate, README).
- **Throughput** (2026-10-19, Full mode). gen0, ort CPU, 16 lanes, 96 games over the three pairings.

  | Budget | Games an hour | Samples an hour | Searched decisions a second (all lanes) | Evaluations a second | Visit-labelled samples | Caps, fallbacks |
  | --- | --- | --- | --- | --- | --- | --- |
  | 64 simulations (shuffles from 5000) | 1,780 | 716 k | 38.6 | 2,510 | 19% | 0, 0 |
  | 128 simulations (shuffles from 6000) | 824 | 340 k | 18.2 | 2,274 | 20% | 0, 0 |

  - Most samples are sub-steps no search reached: forced requests, payments, and the priority shortcut's passes.
  - A game takes 30 s a lane at 64 simulations and 64 s at 128. Runs this short are stretched by their last few
    games.
- **The budget: 64 simulations.** One generation, at 4,800 games:

  | Stage | Planned | Generation 1, measured |
  | --- | --- | --- |
  | self-play: 4,800 games, about 1.9 M samples | about 2.7 h at 1,780 an hour | 3.0 h at 1,590 an hour (36 s a game a lane; the 96-game run's rate did not hold) |
  | training: 1.5 h × 3,600 × 445 samples a second / 256 = 9,386 steps, 2.4 M samples seen | 1.5 h | attempt 1: 1.3 h (540 samples a second); attempt 2, 4,000 steps: 0.6 h |
  | gate: at most 1,200 games | at most 0.8 h; about 0.4 h at the expected SPRT length | attempt 1: 0.7 h for the whole 1,200 (no verdict); attempt 2: 0.25 h, 453 games |
  | build, export, parity | about 0.1 h | 0.1 h |
  | **in all** | **about 4.5-5 h** | **5.1 h for attempt 1; 4.0 h with the 4,000-step training** |

  - The window is 1.9 M samples of self-play plus 0.2 × 4.95 M of gen0, so about 2.9 M an epoch, and (as planned) 0.8 of an epoch
    is trained.
  - At 128 simulations the same time makes less than half the data, and the gate takes twice as long. The visit
    targets would be sharper, but generation 1 has more to learn from volume than from sharpness.
- **Generation 1** (Full mode throughout; `ai-data/gen1/README.md`). Self-play by gen0 at @64: 4,800 games, 1.97 M
  samples, 385 k with visit targets, 0 capped or dropped, first player 52%. The visits mostly agree with the network
  (held-out top1 of the visits 0.972 before training), so the policy has little to learn from them in one step.
  - Attempt 1, the planned settings (9,386 steps, outcome-only value): held out policy CE 0.187 → 0.180 and top1 of the
    visits 0.972 → 0.974, but the value MSE was best at step 4,000 (0.487) and ended at 0.516 while its training loss
    kept falling (0.47 → 0.39). The value overfits the self-play games, whose positions are correlated within a game.
    Gate: 629 to 571, 52.4% ± 2.8% over all 1,200 games, LLR −0.21: no verdict, rejected.
  - Attempt 2 (`--retrain`, the same self-play): 4,000 steps and `--value-mix 0.5` (searched positions' value
    target half the outcome, half the root value). Held out: policy CE 0.181, top1 0.955, top1 of the visits 0.975,
    value MSE 0.490, value sign 0.820. Gate: 252 to 201, **55.6% ± 4.6%**, H1 accepted at game 438 (LLR 3.02).
    gen1 is the champion. These are now the driver's defaults (`--train-hours 0.65`, `--value-mix 0.5`).
- **Generation 2** (Full mode; `ai-data/gen2/README.md`), the driver's defaults with no tuning. Self-play by gen1 at
  @64: 4,800 games in 2.9 h (1,660 an hour, 37.7 searched decisions a second, 2,455 evaluations a second), 1.97 M
  samples, 425 k with visit targets, 0 capped, dropped or fallbacks, first player 52.4%, mean 14.4 turns. Training:
  4,067 steps from gen1 on gen2 ×1, gen1 ×0.5 and gen0 ×0.2 (3.9 M samples in the window), 0.59 h. Held out (a new
  set, with gen2's games in it, so not comparable with generation 1's): policy CE 0.221 → 0.218, top1 of the visits
  0.974 → 0.977, value MSE 0.534 → 0.527, from step 2,000 to the end. Gate against gen1: 280 to 225, **55.4% ±
  4.3%**, H1 accepted at game 490 (LLR 2.96), 0.3 h. gen2 is the champion; the generation took 3.8 h.
- **Short problems from scenarios wait.** The machine time is cheap: about 4 s a decision at 1,024 simulations, so
  the corpus is under an hour on 16 lanes. What they would teach is not.
  - The 7,405 scenarios are rules tests. Their positions are scripted to exercise a ruling, not chosen as decisions
    with a best answer, and almost all of their cards are outside the two decks the slice trains on.
  - Their games end when the script does, so the only value label would be the search's own root value, gen0's
    opinion.
  - The curriculum needs authored problems on the slice's decks ("win this turn", "this combat"), each with its
    answer. A `# Problem` scenario with its goal and a corpus of those is the next step, after the loop shows it
    improves on self-play alone.

- Done (2026-10-26): **the GPU inference server and `RemoteEvaluator`** (the Decisions table's GPU path), on a Windows 11
  machine with 32 threads (a Ryzen 9 7950X) and an RTX 4080. `docs/ai-training.md` ("The GPU inference server") is the
  runbook, `docs/ai-host.md` the backend.
  - **The client**, `cantrip_ai::remote::RemoteEvaluator`, an `Evaluator` like the others. Self-play
    (`cantrip-selfplay --backend remote --server ADDR`, a connection a lane) and the AI host (`--backend remote`, a pool
    of connections its workers share) use it; BotLab's `--ai-server ADDR` gives it to every AI host. The model
    directory is read and checked as `Model::load` checks it (the card rows are mapped to the model's on the client),
    and the hello names the model by its manifest's hash with this build's schema and encoder hash, which the server
    must match.
  - **The transport is TCP on localhost**, one for both OSes, not a Unix socket with shared memory. A batch is about
    2 KB a position on the wire (each row's real tokens only, as bytes and u16 card rows, the numbers sparse as a count,
    feature bytes and f16 values), against 12 KB dense in f16 and 41 KB as the graphs' tensors. At 20 k positions a
    second that is 40 MB a second, which loopback carries without notice. The rows go grouped by bucket, so the server
    takes each bucket's rows as one run. The reply is the value, aux and the legal tokens' logits.
  - **The server**, `python -m cantrip_ai.serve`: the network in PyTorch from the model directory's `model.pt` (export
    writes it; `--weights-only` adds it to an older directory). Every connection's rows are queued per model and
    bucket and run in fixed-size batches (32 tokens: 256 rows, 64: 128, 128: 64) on two pipelines, each with its own
    stream, pinned buffers and captured CUDA graph. It checks the model's references at start, and refuses a model that
    fails them.
  - **Reproducibility.** Every call of a bucket has one shape, the kernels are deterministic
    (`use_deterministic_algorithms`, `CUBLAS_WORKSPACE_CONFIG=:4096:8`), and no operation of the network mixes rows, so
    a row's bits never depend on its companions or its slot (`ai/tests/test_serve.py`). Self-play at seed 1 through the
    server played the same games (every game's bytes hashed, `examples/shard_games`) over 16, 32, 64 and 128 lanes,
    two or three pipelines and separate server runs. Another batch size for a bucket changes the bits (another shape),
    so the batch sizes are part of the configuration.
  - **Precision: fp32.** Against the references at export's tolerance (1e-3), fp32 is within 1e-7. TF32 (value 1e-4,
    aux 4.3e-3) and fp16 (aux 4.4e-3) fail on the life-swing output, which is in life points; bf16 fails everything
    (value 1.2e-3, aux 3.4e-2, 98.4% the same best option). TF32 and fp16 would be 1.3-1.6x faster.
  - **Throughput**, the production-size network (width 144, 4 layers, 1.24 M parameters, untrained, so every game runs
    to the 60-turn cap and the games are long), self-play at 64 simulations from seed 1. Evaluations a second is the
    number that carries over to a trained network.

    | Backend | Lanes, games | Seconds | µs a simulation a lane | Games an hour | Evaluations a second | GPU busy |
    | --- | --- | --- | --- | --- | --- | --- |
    | ort CPU, a thread a lane | 16, 16 | 294 | 4,785 | 196 | 3,343 | |
    | ort CPU, with mimalloc | 16, 16 | 264 | 4,296 | 218 | 3,724 | |
    | remote GPU | 16, 16 | 84 | 1,359 | 690 | 11,773 | 82% |
    | remote GPU | 32, 32 | 111 | 1,794 | 1,035 | 17,836 | 64% |
    | remote GPU, the default (two lanes a thread) | 64, 64 | 194 | 3,146 | 1,187 | 20,343 | 76% |
    | remote GPU | 128, 128 | 374 | 6,056 | 1,232 | 21,136 | 70% |

    - The GPU alone, fp32 with CUDA graphs: about 95 k rows a second at 32 tokens, 54 k at 64 and 22 k at 128 (flat from
      32 rows a batch up). Self-play's leaves were about half 64 and half 128 tokens, so about 29 k a second is the GPU's
      ceiling for them.
    - Where the time goes at 64 lanes: a lane's request cycle is 20 ms, of which 15 ms is waiting on the server and
      0.13 ms encoding the batch for the wire. The server holds a request about 14 ms (6 ms of it queued). The server is
      now the limit: its Python threads (decoding about 60 µs a request, assembling batches, replying) share one
      interpreter lock, and the GPU is 70-76% busy.
    - **The allocator.** On Windows the lanes' allocations contend for the system heap: with it, 32 lanes made 7.9 k
      evaluations a second at 16% of the CPU. `cantrip-selfplay` uses mimalloc on Windows (the Linux build is
      unchanged), which doubled the remote rate and made ort self-play 10% faster; the games are the same.
- Done: `cantrip_ai.train --workers N` on Windows (its batch function is a module-level class, so a worker process can
  be handed it), and the generation driver on Windows: `champions/current` falls back to a directory junction where
  Windows makes no symlink, logs and records are read as UTF-8, `tools\bot-duel.cmd` builds the AI host with ONNX
  Runtime for a neural seat, and `--gpu-server` (on by default when the training device is a GPU) runs self-play,
  parity and the gate through the server.

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
  - every card the engine has has a row of the card index;
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
