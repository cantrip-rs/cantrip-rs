# Cantrip

A Magic-like rules engine, server and web client. The rules engine is Rust: the Cargo workspace at the repository root
(`Cargo.toml`, `crates/`). The server, Discord bot and web client are C#, under `dotnet/`, and reach the engine through
`IRulesHost`. `tools\test.cmd` builds everything and runs the fast tests - every test that does not run on the wall
clock, the Rust workspace included (`cargo test --profile checked --workspace` with `CANTRIP_VERIFY=1`: the cards
profile with the debug assertions on); run it before
finishing a change. Wall-clock tests - the browser tests, and those marked `[Trait("Category", "WallClock")]` (real
sockets, the engine host's whole games, the Discord bot's tables, whole events played by bots) - run only when the work
touches what they cover: `test.cmd /filter "..."` for those, `/noweb` for all but the browser's, `/all` for everything.
Never run them as a habit or to time the suite; a new test that waits on time takes a fake `TimeProvider` instead,
unless real time is what it is about.

## Engine and cards: bricks, and what is built with them

The design is `docs/engine-design.md`; the card loop is `docs/adding-cards.md`. Three layers, kept apart
by the compiler through the `cantrip-api` crate:

- **The core** (`cantrip-state`, `cantrip-rules`) owns **rules authority and timing**: the stack, priority,
  targeting, layers, replacement ordering, triggers, state-based actions, simultaneous operations, and the systematic
  keyword mechanics that need them. That is the set of bricks.
- **Shared mechanics** (`cantrip-mechanics`) are recipes several printed cards use exactly as written.
- **Cards** (`crates/cantrip-cards/src/cards/<card>.rs`, one file each) are what is built out of them: a card's own
  conditions, values, branching and sequencing, over `Game` and `Query`. Characteristics are generated from
  `content/<dataset>/cards/*.json` by the oracle crate's build script; a card with nothing but characteristics
  needs no file.

**Shared use is not a reason for core ownership; needing rules authority is.** Before adding anything to the core or
to `cantrip-mechanics`, name a second already-printed card that would use it exactly as written. If the only honest
answer is "another card like this one", it belongs in that card's file. Never encode a card by widening a central
switch. **Keywords are the exception: every keyword action (CR 701) and keyword ability (CR 702) is shared from the
first card that has it** - one function or module in `cantrip-mechanics` named for the keyword and parameterised
(`earthbend(n)`, `evoke(cost)`), never written inside a card's file, because the rules define it and it will be
printed again. It goes in the core instead only when it changes a core procedure (trample, first strike, split
second).

**The past is asked of the event history, never of a counter.** Every event since the game began is in
`q.history()` (`cantrip_api::history`). A one-off question is written in the card's file (`EventQuery`, or an iterator
over the records); a question a second printed card asks moves to `cantrip_mechanics::history`; the core only records
events. **Search moves through the game-log tree, never copies:** `Game::goto` undoes up to the common ancestor and
redoes down (`cantrip-rules/src/tree.rs`). Both are load-bearing design.

**The engine is benchmarked:** `cargo run --release -p cantrip-bench -- --games 128` ("engine answering ... us per
decision"); measure a core change with it. A guard on the card table ("does any card
have X") is no guard: the table holds every card; guard on the game state.

## Testing: scenarios, never seeds

**Do not sweep RNG seeds to reach a game state.** Not to write a test, and not to take a screenshot. A seed that
lands today stops landing the moment a deck or a card changes, and the odds of an automatically played game
stumbling into a particular interaction — a gang block, a trigger on the stack, a specific prompt — are near zero.
Randomness belongs in deliberate fuzzing and nowhere else.

Force the position instead. A `.cantrip` scenario names both decklists in order and every answer:

- Scenarios live in `scenarios/`, one file per case. `README.md` there is the syntax and the dump keys.
- The harness is `crates/cantrip-script` (parser, runner, state dump).
  `cargo run --profile cards -p cantrip-script --bin corpus -- --card "Name"` runs one card's scenarios with full
  failure reports; no arguments runs them all (`cargo test` makes one test per scenario file, so adding one lists
  nothing); `-- --missing 40` ranks the unported cards that block the most.
- `python tools\maintenance\new-card.py "Name"` prints a card's sources and scenarios and starts its file;
  `tools\card-coverage.cmd "Name"` lists the lines of its file that no scenario runs. Adding a card follows the
  `add-cards` skill: exact to the rules, every line covered, corner cases and interactions scenario'd, reviewed.

The client draws on canvases, so there is no DOM to find a card in. Open the page with `&test` and it publishes where
every card and control is, by name, on `globalThis.cantripTest`; drive it with Playwright - screenshots, and real
mouse clicks at those places (`docs/web-client.md`), in `dotnet/tests/Cantrip.Client.Web.Tests`, where `TablePage`
does the reading and clicking. Serving a handed-over scenario position to the web client went with the C# engine and
is not rebuilt yet.

If the harness cannot express what a test needs — state detection, a new prompt, a new assertion — extend it. That
is cheaper than a seed hunt and it stays fixed.

## Commits go to GitHub

The repository is `origin`, https://github.com/jschibel/cantrip (private). Push every commit as soon as it is made:
`git push` after each `git commit` on `main`, without being asked. Work in an agent worktree lands on `main` first and
is pushed from there. Card art never goes in: `content/*/images/` is gitignored and was stripped from the history.

## Where things live

- `crates/` — the rules engine (the Rust workspace, `Cargo.toml` at the root): `cantrip-state`, `cantrip-api`,
  `cantrip-rules`, `cantrip-mechanics`, `cantrip-oracle` (generated), `cantrip-cards` (the cards), `cantrip-engine`,
  `cantrip-script` (scenarios), `cantrip-bench`, `cantrip-host` (the `cantrip-native-host` sidecar the server drives
  over JSON lines; protocol in `docs/host-protocol.md`). Build output is `target/`.
- `scenarios/` — the scenario corpus, one `.cantrip` file per case.
- `dotnet/` — everything C#: `Cantrip.sln` (and `Cantrip.NoWeb.slnf`, the solution without the web client, what
  `test.cmd /noweb` builds), `src/`, `tests/` and `tools/` projects.
  - `dotnet/src/Cantrip.Server` — session and table dealing through `IRulesHost` (`Rules/NativeRulesHost.cs`), and
    events (`Events/`): drafts, deck building and the data directory events and players are kept in (`--data` /
    `CANTRIP_DATA`). `docs/events.md`.
  - `dotnet/src/Cantrip.Discord` — the Discord bot: a Discord-free `Matchmaker` under a thin Discord.Net adapter,
    serving the web client its tickets open. `docs/discord-bot.md`.
  - `dotnet/src/Cantrip.Client.Web` — the client: Uno on WebAssembly, in the browser, drawn with Skia.
    `docs/web-client.md`. What it draws from is in `dotnet/src/Cantrip.Client` and
    `dotnet/src/Cantrip.Client.Presentation`; its tests are Playwright, in `dotnet/tests/Cantrip.Client.Web.Tests`.
  - `dotnet/src/Cantrip.Client/Tactics` — the tactical bot, the seat meant to be played against: a client like any
    other, so it runs in a player's own tab (`?bots=2`). Its weights are fitted, not chosen: `tools\bot-train.cmd` makes
    `tactical-model.txt`, `tools\bot-duel.cmd` says what a change to it is worth. `docs/tactical-bot.md`.
  - `dotnet/src/Cantrip.Client/Drafting` — the draft bot: what a bot's seat drafts a booster pod and builds its deck
    with, a client as the tactical bot is, knowing no card by name. `tools\bot-draft.cmd` drafts pods with nobody at
    the table and says what a change to it is worth. `docs/draft-bot.md`.
  - `dotnet/src/Cantrip.Art` — card art over HTTP at fixed sizes, cached, CDN-ready; hosted beside the table by
    `Cantrip.Server.Host`.
  - `dotnet/tools/Cantrip.Bot` plays a seat of a real server with no window; `Cantrip.BotLab` trains and judges the
    bots.
- `content/<dataset>/cards/` — one file per card, named for the card: its characteristics (name, cost, type line,
  P/T, Oracle text, core keywords, faces), which the engine's oracle is generated from and the client draws; what a
  card does is its file in `crates/cantrip-cards`. A set only lists cards that exist.
- `decks/` — decklists as data files, never in code.
- `tools/` — `test.cmd`, `play-game.cmd` (you, bots or a language model at a local table), `play-vs-bot.cmd`,
  `draft-winston-vs-bot.cmd`, `draft-grid-vs-bot.cmd` and `draft-booster-vs-bots.cmd` (a draft against bots, then the
  games), `play-release-build.cmd` (the shipping build), and the bot labs. Generators are in `tools/maintenance/`
  (`new-card.py`); `tools/README.md` lists everything.
- `tools/workers/` — the worker manager, `tools/cantrip-workers` (`docs/workers.md`): the one way code and agents get
  and use a compute machine - this one, another on the network, a rented vast.ai box - through a lease (`lease`,
  `exec`, `push`, `pull`, `release`). Never `ssh` or `scp` to a worker directly; the pool (addresses, keys, budgets) is
  local to each machine and never in the repository.
- `docs/engine-design.md` — the engine's design and plan; keep it current when it moves.
