# Cantrip

A Magic-like rules engine, server and web client. The rules engine is Rust: the Cargo workspace at the repository root
(`Cargo.toml`, `crates/`). The server, Discord bot and web client are C#, under `dotnet/`, and reach the engine through
`IRulesHost`. `tools\test.cmd` builds everything and runs the fast tests - every test that does not run on the wall
clock, the Rust workspace included (`cargo test --profile checked --workspace` with `CANTRIP_VERIFY=1`: the cards
profile with the debug assertions on); run it before
finishing a change. A change that touches only cards, content, mechanics or scenarios runs `tools\test.cmd
/engine`, the Rust workspace alone: nothing on the .NET side can tell a card apart. A core, host or client change
runs the whole suite. Wall-clock tests - the browser tests, and those marked `[Trait("Category", "WallClock")]` (real
sockets, the engine host's whole games, the Discord bot's tables, whole events played by bots) - run only when the work
touches what they cover: `test.cmd /filter "..."` for those, `/noweb` for all but the browser's, `/all` for everything.
Never run them as a habit or to time the suite; a new test that waits on time takes a fake `TimeProvider` instead,
unless real time is what it is about.

## No rules deviations

**No automated agent ever allows a change that lets, or risks letting, the engine or a card deviate from the game's
rules (the Comprehensive Rules and the official rulings).** Not to save time, not for performance, not "until a card
needs it", and not as a documented known issue. That includes representations that lose information the rules keep: a
count that clamps or floors, a fixed-size list that drops or cuts down, an approximation that only holds for today's
cards. A deviation found in review blocks the change; one found in existing code is fixed, or reported to a human at
once - never filed away as accepted.

Allowing a deviation is a human-only decision, and the answer is almost always no. A human is asked only when the rule
is nonsensical for this domain (paper-only rules such as ante), when it is extremely niche and extremely complicated to
build (sub-games, stickers), or when it affects only cards banned in every format Cantrip supports. Every deviation a
human allows is written in `docs/rules-exceptions.md`, with its rule, its reason and who decided; those decisions may be
applied to later cases of the same kind, and nothing outside that file is an exception.

**Scale is the one standing exception.** The engine does not simulate everything to infinity. Where following a rule
perfectly is only a question of scale - a count, a size, a depth - it has to be exact up to two practical bounds: what
can actually happen in a game where human players perform every action by hand on a time limit, and what a card could
practically print. Combinations of cards can explode exponentially, and that is fine as long as the engine is exact to
a practical scale: a 32-bit count, say, where a combo can push past 255 or 32,767 within a timed game, but no need for
unbounded storage. Beyond those bounds a case may be left unhandled, as an entry in `docs/known-issues.md` marked
"Won't fix: beyond practical scale", naming the limit, what happens past it, and why play cannot reach it; it is fixed
when a practical reason appears. **A count that play can push past a narrow type is made 32-bit, and that is all:**
play cannot reach 2^32, so its arithmetic stays plain - no saturating or checked arithmetic, no "unpayable if it
overflows" logic at the use sites. A bound play *can* reach - a capacity, a fixed-size list - is handled gracefully: it
never crashes (no panic, no `expect`), and it does what makes the most sense for the situation; an object with as many
abilities as it can hold is granted no more, a list that is full takes no more entries. It never turns into a
different, wrong value.

**Bug priority.** Pri-0 is a crash, a hang, a desync or a game that cannot go on. Pri-1 is any rules violation, however
rare the interaction, that some card can reach in a game: any card legal in a format Cantrip supports, implemented
or not, or any card the engine implements, legal or not (Gleemox). A case that no such card can reach - one only an
imagined card, or a card both unimplemented and illegal everywhere Cantrip plays, could produce - is not a rules bug
but outside the game: the Comprehensive Rules are a guide to playing the cards that exist, amended as new ones are
printed, not a framework for every imaginable card. It is not fixed, filed or raised in review, and becomes a bug the
day a card that reaches it is added (that card's work fixes it). Both are blockers: one found is fixed at once,
never filed away, and they are never allowed to accumulate. Other work does not stop for the fix: cards and pull
requests that don't depend on it keep being built, reviewed and landed beside it, and the fix rebases over them before
it lands; only work that needs the fix waits for it. Everything else - the client's display,
tooling, the harness - is pri-2 or lower and is tracked in `docs/known-issues.md`.

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

**A second card is necessary, never sufficient: shared code needs an engineering reason.** A non-keyword function
goes into `cantrip-mechanics` when it holds something that has to be decided in one place, so that a fix or a rule
change lands once: a rules subtlety a card would get wrong on its own, how a fact is represented (`teamwork::used`,
never `paid > 0` in a card), a procedure with real logic. A couple of lines that only combine pieces already shared -
an existing filter and "you control", an existing helper called after the card's own step - stay in every card that
writes them, however many do. Two cards with the same three lines are not a finding and not a cleanup to file: a
shared name for them saves nothing and grows the surface every card has to search. Using what is already shared
is still required: a hand-written copy of an existing shared function is replaced by a call to it.

**The past is asked of the event history, never of a counter.** Every event since the game began is in
`q.history()` (`cantrip_api::history`). A one-off question is written in the card's file (`EventQuery`, or an iterator
over the records); a question a second printed card asks moves to `cantrip_mechanics::history`; the core only records
events. **Search moves through the game-log tree, never copies:** `Game::goto` undoes up to the common ancestor and
redoes down (`cantrip-rules/src/tree.rs`). Both are load-bearing design.

**Use the systems that exist; never build a parallel one.** Before adding a mechanism - a store, a cache, a flag, a
record, a lookup, an arena, a helper - find the one the engine already has for that job and use or extend it: the event
history for anything about the past (especially what is rarely asked: read it when asked, don't keep a copy up to
date), the layers for characteristics, the journal for undoable state, the shared mechanics and `cantrip_state` helpers
for what they already do. A second system for the same kind of thing - a bespoke record of what happened, a private
counter, a one-off cache beside the shared one - is a review blocker, however well it works. If the existing system
truly can't do the job, say why in the change and extend that system rather than routing around it.

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
  nothing); `-- --missing 40` ranks the missing cards that block the most.
- `python tools\maintenance\new-card.py "Name"` prints a card's sources and scenarios and starts its file;
  `tools\card-coverage.cmd "Name"` lists the lines of its file that no scenario runs. Adding a card follows the
  `add-cards` skill: exact to the rules, every line covered, corner cases and interactions scenario'd, playable
  end to end in the client, reviewed.
- **Cards are claimed on GitHub before they are worked on.** Every missing card a format plays is an issue
  ("Card: Name", labelled `format:<format>`), from `docs/formats/<format>.json` (the cards its decks play, scraped from
  MTGGoldfish and MTGTop8 by `tools/maintenance/format-scrape.py`). `python tools/maintenance/card-issues.py next
  <format>` lists what is free, `claim "Name"` takes a card (opening its issue first if it has none), `release`
  gives it back. A claim is a comment and lasts 24 hours from the last pull request commit or push to a
  `card/<slug>` branch; after that the card is free. Never work on a card someone else holds live; commits end with
  `Closes #N`. `sync <format>` adds the issues a new scrape needs.

The client draws on canvases, so there is no DOM to find a card in. Open the page with `&test` and it publishes where
every card and control is, by name, on `globalThis.cantripTest`; drive it with Playwright - screenshots, and real
mouse clicks at those places (`docs/web-client.md`), in `dotnet/tests/Cantrip.Client.Web.Tests`, where `TablePage`
does the reading and clicking. A scenario plays in the browser too (`docs/scenarios-in-the-browser.md`): a real table
plays the script, Bot1 a page that `ScenarioPlayer` clicks through it, every answer the one the line means; `# Browser`
puts a scenario in that suite, `Client.` keys check what the page shows, and `tools\scenario-in-browser.cmd name` plays
one to watch.

If the harness cannot express what a test needs — state detection, a new prompt, a new assertion — extend it. That
is cheaper than a seed hunt and it stays fixed.

## Commits go to GitHub

The repository is `origin`, https://github.com/cantrip-rs/cantrip-rs (public); `cantrip-private`,
https://github.com/jschibel/cantrip, is the owner's older private copy. Push every commit as soon as it is made:
`git push` after each `git commit` on `main`, without being asked. Work in an agent worktree lands on `main` first and
is pushed from there. Card art never goes in: `content/*/images/` is gitignored and was stripped from the history.

## Where things live

- `crates/` — the rules engine (the Rust workspace, `Cargo.toml` at the root): `cantrip-state`, `cantrip-api`,
  `cantrip-rules`, `cantrip-mechanics`, `cantrip-oracle` (generated), `cantrip-cards` (the cards), `cantrip-engine`,
  `cantrip-script` (scenarios), `cantrip-bench`, `cantrip-host` (the `cantrip-native-host` sidecar the server drives
  over JSON lines; protocol in `docs/host-protocol.md`), `cantrip-wasm` (the same requests served in a web page,
  `docs/engine-in-the-browser.md`). Build output is `target/`.
- `.claude/worktrees/pool-<n>` — the agents' worktree pool, reached only through `tools\worktree.cmd`: a lease for
  new work, a single-use token for each agent let into it, one member at a time (`coordinator.md` step 0). Opt-in per
  machine: the `worktree-pool-setup` skill installs its hooks in the gitignored `.claude/settings.local.json`.
- `scenarios/` — the scenario corpus, one `.cantrip` file per case.
- `dotnet/` — everything C#: `Cantrip.sln` (and `Cantrip.NoWeb.slnf`, the solution without the web client, what
  `test.cmd /noweb` builds), `src/`, `tests/` and `tools/` projects.
  - `dotnet/src/Cantrip.Server` — session and table dealing through `IRulesHost` (`Rules/NativeRulesHost.cs`), and
    events (`Events/`): drafts, deck building and the data directory events and players are kept in (`--data` /
    `CANTRIP_DATA`). `docs/events.md`.
  - `dotnet/src/Cantrip.Discord` — the Discord bot: a Discord-free `Matchmaker` under a thin Discord.Net adapter,
    serving the web client its tickets open, and the draft queue and constructed play pages (`/draft`, `/constructed`, Discord sign-in), behind a home page (`/`) and a player's own page (`/home`), and the constructed deck editor's decks (`/decks`). `docs/discord-bot.md`.
  - `dotnet/src/Cantrip.Client.Web` — the client: Uno on WebAssembly, in the browser, drawn with Skia.
    `docs/web-client.md`. `?decks` is the constructed deck editor instead of a table (`MainPage.Decks.cs`; the deck rules are
    `Cantrip.Client/Decks`, the legality data `content/mtg/legality.json` from `tools/maintenance/fetch-legality.py`). What it draws from is in `dotnet/src/Cantrip.Client` and
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
- `leagues/` — the leagues the Discord bot's site runs, one JSON file each (`LeagueDefinition`; `docs/events.md#leagues`).
- `tools/` — `test.cmd`, `play-game.cmd` (you, bots or a language model at a local table), `play-vs-bot.cmd`,
  `draft-winston-vs-bot.cmd`, `draft-grid-vs-bot.cmd` and `draft-booster-vs-bots.cmd` (a draft against bots, then the
  games), `play-release-build.cmd` (the shipping build), and the bot labs. Generators are in `tools/maintenance/`
  (`new-card.py`); `tools/README.md` lists everything.
- `tools/workers/` — the worker manager, `tools\cantrip-workers.cmd` (`docs/workers.md`): the one way code and agents
  get and use a compute machine - this one, another on the network, a rented vast.ai box - through a lease
  (`lease`, `exec`, `push`, `pull`, `release`). Never `ssh` or `scp` to a worker directly; the pool (addresses, keys,
  budgets) is local to each machine and never in the repository.
- `docs/engine-design.md` — the engine's design and plan; keep it current when it moves.
- `docs/known-issues.md` — bugs being tracked: client, tooling and harness problems, and paths no scenario runs yet,
  with severity, notes and the scenarios that show them. A rules deviation is listed there only while its fix is under
  way, never as accepted ("No rules deviations" above).
- `docs/rules-exceptions.md` — the only deviations from the game's rules a human has allowed, each with its rule,
  reason and who decided.
