# tools

Everything here runs from a double-click or a prompt at the top of the checkout. Each `.cmd` says what it takes in its
header; most pass every argument on to the script behind them.

## Test

| | |
|---|---|
| `test.cmd` | builds everything and runs the fast tests: the C# solution (`dotnet\Cantrip.sln`), then the engine's (`cargo test --profile checked` over the workspace at the root - the cards profile with debug assertions on - which plays `scenarios\`); `/all`, `/noweb`, `/filter`, `/detailed` |
| `test.sh` | `test.cmd` for Linux, with the same options as `--all`, `--noweb`, `--nobuild`, `--engine`, `--filter`, `--detailed` and the same verdict line; takes the `dotnet` on PATH (`nix shell nixpkgs#dotnet-sdk_10 -c tools/test.sh`), and builds `dotnet/Cantrip.NoWeb.slnf` when the `wasm-tools` workload the web client needs is not installed |
| `scenario-in-browser.cmd name` | plays one `.cantrip` scenario with Bot1 a browser page, in a window to watch (`/headless` without one); `test.cmd /filter "FullyQualifiedName~ScenarioInBrowser"` plays every scenario marked `# Browser` (`docs/scenarios-in-the-browser.md`) |
| `card-render.cmd` | draws a sheet of sample card faces with the real web client in a real browser (`?cardsheet`) and saves it as `TestResults\web\CardRenderTests\card-sheet.png`, to look at after a change to the card template |

## Play a game in the browser

| | |
|---|---|
| `play-game.cmd` | a local table: each seat's deck, and whether you, a bot or a language model plays it; `-Format` the format the decks are checked against (freeform, no restriction, by default) |
| `play-vs-bot.cmd` | you against the tactical bot, which your own browser tab plays (`docs/tactical-bot.md`) |
| `play-vs-llm.cmd` | you against a language model (OpenRouter) |
| `play-release-build.cmd` | you against a bot in the shipping Release build of the web client |
| `in-person.cmd` | two players at one table: a phone each and one or two tablets for the board, over the local network; opens the setup page, which starts a grid draft and shows a QR code per device (`docs/in-person.md`) |

## Draft against bots

Each drafts, builds a deck from what was taken, and plays the games. Events are kept in `.cantrip-data`.

| | |
|---|---|
| `draft-winston-vs-bot.cmd` | Winston: 90 cards of the cube in three piles, you and one bot |
| `draft-grid-vs-bot.cmd` | grid: eighteen 3x3 grids, a row or a column each, you and one bot |
| `draft-booster-vs-bots.cmd` | booster: a pod of eight, you and seven bots, three packs then three Swiss rounds |
| `draft.cmd` | any of the three with every option (`-Cube`, `-Player2 Human`, `-Humans N`, ...), and `-Event ID` to host one again |

## Bots only, no browser

| | |
|---|---|
| `bots-random-games.cmd` | thousands of random-bot games at the session layer, reporting any that got stuck |
| `bots-deck-duel.cmd` | two decks against each other many times, and which won more |
| `bots-llm-games.cmd` | a language model against the random bot, with every decision it made |
| `bot-duel.cmd` | one way of playing a seat against another, many times; `watch` tells one game turn by turn; `--ai-record DIR` has the native host record every game that ends as AI training shards (`docs/ai-player-plan.md`, "Data"), which `python ai/shards.py DIR --check` reads back |
| `bot-duel.sh` | `bot-duel.cmd` for Linux, the same arguments: builds the native host, the AI host (with ONNX Runtime) when a seat is `neural`, and BotLab, then plays; `--a neural --b tactical` is the AI player's Phase 2 gate; `neural@N` searches N simulations an answer (`neural@Nms`, N ms), so `--a neural@64 --b neural --sprt` is search against the network alone, stopped by the SPRT (`docs/ai-host.md`); `--deck A --deck B --mirrors` plays half the games the matchup and a quarter each mirror (the Phase 3 gate: `--a neural@256` against `neural` and `tactical`) |
| `bots-soak.cmd` | thousands of tactical-bot games over the every-card deck and any pool, reporting only those that did not finish, each with the command that replays it |
| `bot-train.cmd` | makes the tactical bot's weights: fits them to its own games, then tunes them by play |
| `bot-draft.cmd` | drafts pods of eight with nobody at the table, two ways of drafting in alternate seats: what their decks are and, with `--games`, which wins; `watch` tells one seat's draft pick by pick |

## Discord

| | |
|---|---|
| `discord-bot.cmd` | runs the Discord bot (`docs/discord-bot.md`) |
| `discord-bot-release.cmd` | stops the bot, builds the Release web client, and starts it again |
| `publish-server.cmd` | publishes to the server: builds the bot and the web client here, builds the engine host there, publishes the art, and restarts the bot once (`-Fast`, `-SkipClient`, `-SkipArt`); the server is in `tools\deploy.local.json` |
| `publish-art.cmd` | publishes card art alone, with the server up: brings the pre-sized art cache up to date here and sends what differs (`-DryRun`, `-SkipBuild`) |

## `maintenance/` - generated files and audits

| | |
|---|---|
| `bump-protocol.cmd` | moves `PlayerView.ProtocolVersion` on and records the wire digest |
| `push-public.cmd` | replays the commits on `origin/main` since the last publish onto the public `cantrip-rs/cantrip-rs` (`-DryRun` lists them) |
| `card-coverage.py "Card"` (or `tools\card-coverage.cmd`) | the lines of a card's Rust file its scenarios never run; exits 1 if any (`cargo-llvm-cov`) |
| `card-mutate.py "Card"` | breaks a card's file one mechanical change at a time (a number one off, a comparison, `&&`/`||`, a `!`, a call left out), runs its scenarios against each and lists the changes no scenario caught; `--list` shows the changes, `--lines`, `--max`, `--file` narrow them; restores the file |
| `card-lint.py "Card"` | the add-cards "never" list checked over a card's Rust file: NEVER findings (exit 1) are forbidden outright, LOOK findings are read and answered; `--all` counts each check over every card |
| `new-card.py "Card"` | everything to read before adding a card, and its file to start from (`docs/adding-cards.md`); for a card new to content it writes the content JSON from Scryfall first (one face, a double-faced card, an adventure); `--scenario <what-it-shows>` starts a scenario file instead (the decks, the card cast off free mana, a TODO expectation) |
| `find-cards.py --type creature --toughness 4` | implemented cards to build a scenario's board from, by type, power, toughness, mana value, color, keyword or text, least rules text first |
| `scryfall.py "Card"` | a card's current mana cost, type line, stats and Oracle text, face by face, and its official rulings from Scryfall's bulk data, cached in the main checkout's `target/scryfall` for every worktree; `new-card.py` prints them and flags content whose Oracle text is out of date. Only `scryfall.py --refresh` downloads and rebuilds the cache: the coordinator's, from the main checkout (refused in a pool slot), so agents never write it at once |
| `cr.py 702.21 601.2c "scry"` | a Comprehensive Rules rule and its subrules by number, or a keyword or keyword action by name, from `docs/rules`; `new-card.py` prints the card's keywords and keyword actions with it |
| `ability-index.py "Card"` (or `--text "..."`) | each ability of a card and the implemented cards that already do it or its parts (cost, trigger, effect), with what they use and how their text differs; `rank` sorts the constructed pool's missing cards by how much of them is implemented and lists the missing ability templates the most of them print (Scryfall bulk data cached in `target/scryfall`). `new-card.py` prints its report |
| `rebuild-card-test-deck.ps1` | regenerates the card-test deck from the card files |
| `card-list.py` | any list of cards (a decklist, a set, a Scryfall search, a text file, a pasted list) as the names the card files use, checked against Scryfall (the `import-cards` skill) |
| `missing-cards.py` | cards on the cube lists, or on one list or file (`--list`, `--file`), that have no card file yet |
| `format-scrape.py <format>` | the cards a format's winning decks play, from MTGGoldfish and MTGTop8, ranked, into `docs/formats/<format>.json` (pages cached in `target/format-cache`) |
| `worktrees.py` (or `tools\worktree.cmd`) | the agents' worktree pool, `.claude/worktrees/pool-<n>`: `acquire` a clean warm slot for new work, `token` lets one subagent `join` it (single-use, two minutes), one member at a time, `leave`, `release` once its work has landed (refused while it holds unlanded work), `status`, `reconcile`, `clean` (empties an unleased slot set aside as needing attention), `create`; its `hook` subcommand is the Claude Code hooks that hold agents to it, opted into per machine with `hooks install` (the `worktree-pool-setup` skill; `coordinator.md` step 0). Tests: `python -m unittest tools/maintenance/test_worktrees.py` |
| `pr-watch.py` (or `tools\pr-watch.cmd`) | waits for pull requests on cantrip-rs/cantrip-rs - the REST API every minute with the last ETag, so an unchanged list costs no rate limit - and for each new PR or new commits starts a fresh `claude -p` session that clears the queue through the `review-card-pr` skill, two review agents at a time, then waits again; logs and state in `.claude/pr-watch/`, `--mark-seen`, `--once`, `--dry-run` |
| `card-issues.py` | GitHub issues for the cards a format plays that have no file: `sync <format>` opens them, `next <format>` lists the free ones, `claim`/`release "Name"` take and give back a card (a claim is a comment, good for 24 hours from the last PR commit or `card/<slug>` push), `status` lists live claims, `log <N> <event>` posts a fixed progress comment on a card's issue (committed, review-started/-passed/-blocked, fixes-committed, blocked/unblocked, landed) and `times` reads them back as minutes per card and stage (the `add-cards` skill, `claims.md`) |
| `cubecobra-sync.py` | syncs one cube list from its CubeCobra cube, or fetches a new cube whole by its CubeCobra name (the `import-cards` skill) |
| `missing-art.py` | the card faces and tokens of a cube list or file (`--list`, `--file`) that have no art yet, one a line |
| `pick-order.py` | ranks the cubes' cards into their `*.picks.json` pick orders |
| `fetch-legality.py` | each card's format legalities and color identity from Scryfall's bulk data into `content/mtg/legality.json`, what the deck editor's catalog is built from; run again as formats move |
| `art-credits.py` | fills `content/<dataset>/artists.json` with the original printing's artist for each picture not yet credited |
| `mtgpics-art/fetch_art.py` | downloads the original, non-promo art-only picture of each card name read from stdin from mtgpics.com, under its slug file name (the `fetch-card-art` skill) |
| `scryfall-art/fetch_art.py` | the fallback for what mtgpics lacks: downloads the earliest regular printing's Scryfall `art_crop` of each card name read from stdin, under its slug file name; never replaces a file (the `fetch-card-art` skill) |

## The rest

- `scripts/` - the PowerShell behind the launchers above; run the `.cmd`, not these.
- `deploy/` - the server's systemd units, kept for reference; the Discord token and OAuth secret live only on the server, in `/etc/cantrip/bot.env`. `dotnet\tools\Cantrip.ArtWarm` builds the pre-sized art cache `publish-art.cmd` sends.
- `dotnet\tools\Cantrip.Bot` - plays one seat of a real server without a window; every bot seat above is one of these.
- `dotnet\tools\Cantrip.Playtest`, `Cantrip.LlmGames` - the runners behind the `bots-*.cmd` launchers. `Cantrip.BotLab` - behind
  `bot-duel.cmd`, `bot-train.cmd` and `bot-draft.cmd`.
- `ai/` - the AI player's Python side (setting it up on CUDA, ROCm or the CPU, and running training: `docs/ai-training.md`): `shards.py` reads the training shards `cantrip-native-host --ai-record DIR` writes, and with `--check` checks every sample's chosen option is legal. `python -m cantrip_ai.train` trains a network on them and `python -m cantrip_ai.export` writes it as the ONNX model directory `cantrip_ai::eval` loads (`ai/cantrip_ai/__init__.py`). `--data DIR:WEIGHT` mixes several directories (self-play's visit targets with soft cross-entropy), and `--init CHECKPOINT` fine-tunes. `python -m cantrip_ai.generation --root ../ai-data --generation N` runs one generation of expert iteration end to end. Its stages are build, `cantrip-selfplay`, train, export, parity, the SPRT gate against `champions/current`, promote and retire. Each stage is logged to `genN/logs/`, and the generation's times and numbers to `genN/README.md`. `--dry-run` prints the commands, `--tiny` runs a generation at toy scale, a rerun resumes after the last stage done, and `--retrain REASON` runs a rejected generation again from its training (its self-play kept, the earlier attempt recorded under `attempts` and `genN/logs/attemptK/`). `python -m cantrip_ai.retention --root ../ai-data --keep 8` deletes the shards and models of all but the last eight generations, except gen0 and those with a `PINNED` file or `--pin` (docs/ai-player-plan.md, Phase 4). `cantrip-selfplay` (`crates/cantrip-ai-host`) is the self-play binary (`docs/ai-host.md`).
- `dotnet\tools\Cantrip.WebHost` - serves the Release client for `play-release-build.cmd`.
