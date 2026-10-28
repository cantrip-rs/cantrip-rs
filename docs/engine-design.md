# The rules engine: design

Status 2026-10-15. The engine is Rust, the Cargo workspace at the repository root (`Cargo.toml`, `crates/`). Every card
in content is ported (841 cards); 1253 of the 1257 scenarios in `scenarios/` pass and none fails (the other four name
cards that are not in content). It answers in about 5 us per decision, against the old C# engine's 261 us. The server
plays every game on it, through `IRulesHost`, and the C# engine is gone. Next is step 6 of the plan: Python bindings
and the AI work.

Card behavior is per-card code. Everything above the rules is C# under `dotnet/`: the server, events, the Discord
bot, the web client and the protocol. They reach the engine through `IRulesHost`, implemented by the
`cantrip-native-host` sidecar.

The reason for the rewrite is total compute. The AI work in [ai-player-design.md](ai-player-design.md) will run
millions of simulated games, most of them inside search. A rewrite is paid once; a slower engine is paid on every run.

## Goals

- **Throughput.** Steps per second, legal-move enumeration, and above all the cost of reaching another position, so
  that search can branch anywhere.
- **Branch anywhere.** Search can move to any decision, including a prompt in the middle of a resolution.
- **Per-card code.** A card's own conditions, values and sequencing stay in that card's code (CLAUDE.md, "bricks,
  and what is built with them"). Nothing about one card goes into a central switch.
- **Two ways in.** One is the server, through `IRulesHost`. The other is Python, in-process, for training: batches of
  games, step, branch, encode.

Not goals: compiling cards at runtime (cards are compiled with the engine); running the rules on a GPU (see the AI
design: rules diverge per card and the state has no fixed size); parsing Oracle text.

## What the C# engine cost

A read of the C# engine (2026-10-14) found five costs that came from its design, not its language, so a port that
kept the design would have kept them:

| Cost | Where it was |
| --- | --- |
| A full deep copy of the game on every batch of zone changes | `ZoneChanger.cs` (`StateGraph.Clone`, reflection-driven) |
| Every card's characteristics recomputed after almost any change, each recompute applying the layers twice to verify | `LayerEngine.cs` (`CharacteristicsCache`), invalidated from ~157 places |
| A paused game could not be copied: a prompt was a suspended `await` held in a delegate, so branching meant replaying the game from the start | `GameTask`, `GameEngine.Ask` |
| Strings at runtime: effect verbs and keywords were strings, `CardFilter.Parse` ran when an effect resolved | Engine, CardLibrary |
| State was a tree of reference objects in `Dictionary` and `List` | `GameState.cs` |

What it did well, and this engine keeps: full determinism (a seeded RNG in the state, shuffles and rolls recorded),
and self-play on every core in one process.

## Architecture

### Crates

| Crate | Holds |
| --- | --- |
| `cantrip-state` | the game state: the arena, its journalled write API, IDs, zones, the decision record, the generated subtypes. Plain data, no behavior. |
| `cantrip-rules` | the bricks: turn structure, priority, stack, targeting, layers, replacement ordering, triggers, state-based actions, combat, mana, costs, the game-log tree. |
| `cantrip-api` | the public surface card code is written against: `Game` and `Query`, card definitions, the event history, the request kinds. The only engine crate card code may depend on. |
| `cantrip-mechanics` | shared mechanics: parameterised recipes printed cards share, and the shared history questions. Depends on `cantrip-api` only. |
| `cantrip-cards` | the cards, one file each in `src/cards/`; its build script registers every file. Depends on `cantrip-api` and `cantrip-mechanics` only. |
| `cantrip-oracle` | characteristics, generated from `content/*/cards/*.json` by its build script (below) |
| `cantrip-engine` | the card table and the engine's entry points, tying the rules to the cards |
| `cantrip-host` | the `cantrip-native-host` sidecar behind `IRulesHost`: JSON lines over stdio ([host-protocol.md](host-protocol.md)) |
| `cantrip-script` | the `.cantrip` scenario runner, and `corpus`, which runs the corpus and says how each went |
| `cantrip-bench` | the benchmark |

The `cards` profile (release code, incremental) is the card loop's: an edit to one card rebuilds in about a second
([adding-cards.md](adding-cards.md)). Python bindings (`cantrip-py`, PyO3) are step 6.

### Layers: core, shared mechanics, cards

1. **Core (`cantrip-rules`): rules authority.** The Comprehensive Rules' procedures and the state they act on:
   turn structure and priority; the stack and the casting, activating and resolving procedures (601, 602, 608); the
   cost-payment framework, targeting and legality; zones, zone changes and new-object identity (400.7) with
   last-known information; the layer system with timestamps and dependencies (613); application of replacement and
   prevention effects (614-616); trigger collection and ordering (603); state-based actions (704); combat; mana;
   game end; hidden information; the game log.
2. **Shared mechanics (`cantrip-mechanics`): parameterised recipes.** Code that is the same across cards and differs
   only in their data: keyword actions (scry, surveil, mill, investigate, explore), keyword abilities that are
   ordinary abilities with parameters (cycling N, ward N), common effect shapes, and history questions more than one
   card asks. Admission: name a second printed card that would use it exactly as written.
3. **Cards (`cantrip-cards`): everything one card does alone.** Grist's repeating +1, Gollum's parity choice. Built
   from the public API and the shared mechanics, nothing else.

**The test for the core is "does it need rules authority", not "is it in the Comprehensive Rules".** Keyword
actions (701) and most keyword abilities (702) are in the rules, but most need nothing beyond the public API, so they
are shared mechanics. A keyword is core only when it changes a core procedure: trample changes damage assignment,
first strike adds a combat damage step, split second changes when players may act. Shared use is not a reason for
core ownership; needing rules authority is.

**The boundary is enforced by the compiler.** `cantrip-mechanics` and `cantrip-cards` depend on `cantrip-api` alone,
and Rust cannot reach another crate's private items, so card code cannot name the rules crate's internals.

### State: one arena, every write journalled

Search needs two things from the state: reaching another position must be cheap, and a change must be small enough to
store as a delta. The layout is chosen for those two and nothing else. It is not an entity-component system: a game
has a few hundred objects at most, and rules code touches a handful of them at a time in specific ways, so ECS's
strength, sweeping many similar entities every frame, buys nothing here.

- The state lives in a few contiguous buffers of plain data: object rows, zone lists, continuous effects, triggers,
  the agenda, the RNG. Everything refers to everything else by ID or offset.
- **Objects are IDs.** An ID indexes a row holding the object's card (an index into the immutable card table),
  counters, damage, status, controller, owner, zone, attachments and timestamps. An object that changes zones becomes
  a new object (CR 400.7) with a new ID; its last-known information is a frozen row.
- **Zones are ordered lists of IDs.** Continuous effects, pending and delayed triggers and replacement effects are
  compact records that name objects by ID and card behavior by function index.
- **Derived data is not state.** A cache of something computed from the state (the list of static-ability sources)
  is keyed on `GameState::epoch`, which moves on every write and every undo and never goes back. It needs no
  journalling and is ignored by equality and hashing.

The card table is shared and immutable, so it is never copied. A whole position copies in about 1 us
(`clone_with_journal` is the deep copy, the journal included).

**Every write goes through one journalled API.** Nothing mutates the state directly. Each write (move this object,
add this counter, tap this, push this frame) records an undo entry. The journal serves three purposes at once:

- **Moving through the game tree.** The journal's entries are what the game log's nodes hold (next section), so
  any position is reached from any other by undoing and redoing them.
- **Checkpoint and rollback** for anything that tries a move and takes it back.
- **Rollback for card code** (below): undoing an effect's mutations before it re-runs uses the same journal.

The RNG lives in the state. Every random event goes through it, so a game replays exactly.

### The game log is the search tree

The game is a tree of nodes, one per answer (`cantrip-rules/src/tree.rs`). The real game is one path from the root;
a search branch is a sibling path that shares everything above it.

- **A node holds the state writes its answer made**, each with redo and undo data (the journal above).
  `Game::goto` moves the working state from one node to another as a checkout does: undo up to the common ancestor,
  redo down to the target. An answer already given at a node is reused rather than played again. A move costs
  4-9 us. **Search moves through the tree, never by copying the game.**
- **A working state is the quick reference.** The live game, and each search thread, holds one fully built state and
  moves it through the tree. A value lookup is a read of the working state, never a rebuild.
- **A search thread starts from `Game::fork_for_search`**: the working state, the journal back to the lowest mark
  a waiting frame needs (so a question asked mid-resolution or mid-payment can still be answered and taken back), and
  a tree of its own rooted at the current position, whose ids follow the real tree's. Going above its root panics.
  `Game::reseed` gives it its own generator.
- **A search's world is dealt on a fork's root** (`cantrip_ai::determinize`). The operations it uses are not
  journalled, since a re-run reverting to its frame's mark would undo them. They rewrite the journal tail's stored
  copies instead, as `reseed` does:
  - `GameState::rebind_hidden` and `Game::rebind_hidden` give a hidden object, and the hidden rows it became from,
    another card, and patch the `AbilityRef`s naming it in the stack, triggers and log;
  - `GameState::reorder_library` reorders a library, renaming the objects the tail's removals put back and keeping in
    place what the tail made (`zone_order_fixed`);
  - `GameState::restamp_hidden` resets a timestamp no event recorded;
  - `Game::ask_again` asks the waiting question again of the new cards.
- **An answer without stages redoes its run from the run's start**, so its node begins before the events the run
  recorded until it asked; the node keeps those events (`displaced`, a range of one arena the tree holds, so an answer
  allocates nothing for them) and gives them back when the game goes up to its parent.
- `CANTRIP_VERIFY=1` turns on the engine's own checks while it plays: every answer is round-tripped through the tree
  and the position it lands on is compared.

**The event history is the record the rules ask about the past** (`cantrip_api::history`, read with `q.history()`).
Every rules event since the game began is a `Record` (serial, turn, step, the active player, its cause, and the
`Event`), recorded after its triggers are collected and its replacements applied. Records name objects and copy
nothing out of them: what an object was is read from its row, frozen as it left, or its last-known snapshot. It is
also the last-known-information record for one-off mechanics. A question about the past ("life gained this turn",
"the first card drawn in its controller's draw step") is a query over it: written in the card's file when one card
asks it, moved to `cantrip_mechanics::history` when a second printed card asks it in the same words. The core only
records events; no per-turn counter answers a history question, the layers' included: a static ability's condition reads
the history through `LayerQuery::history` (below). A state that changes in place is recorded as it changes, so the
history can answer for it afterwards: a permanent entering face down is `EnteredFaceDown` (with its controller then) and
turning one face up is `TurnedFaceUp` (with the player who did it, CR 708.8), as the face-down status itself is gone
once it is turned up; a `Damage` record carries its source's controller and colors as it dealt the damage, which
neither a later change of control or color nor the source leaving can alter.

- **Causes are recorded.** Each event names what produced it, which serves explanations, debugging and the AI's
  features.
- **Visibility is derived, not stored.** Who knows what follows from the state and the log, provided every action
  that gives a player knowledge is an event: a reveal, a look at cards (`seen_by` on a face-down card), a search.
- **Narration, replay and training data are views of it.** "State before a decision, the options, the choice, the
  outcome" comes out of the tree with no extra instrumentation.

**Hidden information is the bot's problem, not the engine's.** The real game's path holds the true hidden state,
and a search must not read what its seat cannot know. Search expansion ignores what is hidden from the AI and
branches on categories with probabilities, carrying each probability into the node's cost; what that needs
(visibility, and the events that give a player knowledge) comes out of the log. See
[ai-player-design.md](ai-player-design.md).

### Decisions are data

The engine's interface is a step function, not a coroutine:

```text
start(setup)               -> Step
step(state, seat, answer)  -> Step
Step = Pending { requests } | Over { outcome }
```

Everything the engine is in the middle of lives in the state, as data: the agenda is a stack of frames (resolving
this stack entry, applying this zone-change batch, putting these triggers on the stack), and a pending prompt is a
frame waiting for an answer. There is no suspended Rust future, so a game waiting on a prompt is an ordinary value.

#### Card code that asks questions

Card code is ordinary straight-line code calling `g.ask(...)`. When it reaches a question with no answer yet, it
stops and the frame records the question. When the answer arrives, the effect runs again from its start, with the
answers so far replayed, and continues past the question. The mutations it made before the question are journalled
and rolled back before the re-run, and card code is deterministic, so this is exact. It costs about 0.2 us per
question over writing each card as explicit stages, which the spike measured, and keeps every card readable.

Every answer is checked against its question before the card sees it (`cantrip-rules/src/validate.rs`): a card that was
not offered, the wrong number, an over-budget total is refused and asked again, so a card's file reads the choice and
nothing else.

#### Selection rules

A pile separation (CR 700.3a, 700.3c-d; Fact or Fiction, Liliana of the Veil) uses `choose::select` with `ANY_NUMBER`: the chosen
objects form the first pile and the complement the second, either possibly empty. The later pile choice carries each
pile's members in `Label::Pile`, so `RequestKind::objects` shows the chooser those objects and the host names the contents
in the options. Separating a pile neither targets (CR 115.10a) nor moves its members (CR 700.3c).

A question that picks a set of cards - a choice of cards (`RequestKind::ChooseCards`) or a library search
(`RequestKind::SearchLibrary`) - carries its **selection rule** (`Selection`, `cantrip-state/src/decision.rs`): which sets
of its cards are an answer, as data. A rule is a `Count` (`Exactly(n)`, `UpTo(n)`, `AnyNumber`, `AsManyAsCan(n)` - fewer only
when nothing more may be added, a search with no quality to fail at, CR 701.23b), the noun its caption uses (`kind`), any
number of **parts** (`Parts`: borrowed for the whole program for a rule built in a constant, `Selection::of`, shared when put
together as the question is asked, `.with`), and, for a question asked by a list (the older `choose` functions, `search_library`, the core's
costs), the list of cards it may take at all (`among`). A part is a validator over the cards chosen so far - a plain
function, `cantrip_api::select::SelectFn`, `fn(&dyn Query, &Choice, u32) -> Verdict`, with the one number it takes as data
beside it - never a closure, so the rule moves through the game-log tree (`Game::goto`) and replays like the rest of the
pending question. It reads the game through the `Query` and nothing else: "a card with the same name as a card in your
graveyard" reads the graveyard. (The state holds the function as `&'static dyn Any`, since `cantrip-state` cannot name the
`Query` it takes; `cantrip_api::select::part` / `tallied` make one - the validator is private to `Part`, so a part holds a
`SelectFn` or does not exist - and `judge` runs it. Parts compare by the function and their number;
they hash by their number and words.)

`cantrip_api::select::judge(q, &rule, &Choice { chooser, source, candidates, selected })` says what the rule says of the cards
`selected` so far: a `Verdict { complete, addable }` - whether they are an answer, and which of the candidates not yet chosen
may be added (a bit set over the candidates, in their order). Every part must allow a card for it to be addable and call the
cards complete for them to be an answer; then the count: a card is addable only while the count has room, and
`AsManyAsCan` is complete short of its number only when nothing more is addable. Removing a card is always allowed. A selection holding a card that is not a listed candidate is refused before
any part is run (`complete: false`, nothing addable), since parts read the characteristics of the cards they are given. The
cards a question may take at all are the addable set of none chosen.

**"Addable" means that adding the card does not break the rule - not that a complete answer is still reachable.** A client
that picks only addable cards reaches a complete answer one card at a time only if the rule is *downward-closed*: every
subset of a valid selection is itself addable-from (it breaks nothing), so the cards of any valid answer can be added one
at a time in any order. Every shipped part is: a quality of each card, `DIFFERENT_NAMES` (a pair is allowed only if each card
is allowed beside the others), and `total_mana_value_at_most` (a sum only grows as cards are added). A new part that
combined `exactly(n)` with a set rule that is not downward-closed ("exactly two, total mana value exactly 5", an
all-or-nothing group) could leave the player with cards that are all addable and no way to a complete answer: the client
would dead-end, so such a part must keep the property or answer the dead end itself. `GameClient`'s settling of a bot's answer
mends by backing out of a dead end, but a player picking by hand cannot. The core checks every answer with the same `judge`
(`validate.rs`: the cards are distinct candidates the list allows, and complete by the rule), the scenario harness answers
`Choose(..)` and `Search(..)` through it (each name the first card of that name the rule lets be added, `run.rs`), and a client
asks it as its player picks: the prompt carries the verdict for none chosen, and the host's `Check` request
(docs/host-protocol.md, "Checking a selection") answers any other selection. `cantrip_rules::selection_verdict` gives the same
`Verdict` for a cost's tapping for power, collecting evidence and crafting, so a client checks every card choice one way.

The ready-made rules are `cantrip_mechanics::select`, where a new card looks first: whole rules for the common wordings
(`select::CARD`, `BASIC_LAND_CARD`, `basic_land_cards(up_to(2))`, ...), the counts, qualities of each card (`BASIC_LAND`,
`CREATURE`, `mana_value_at_most(n)`, `named(card)`, ...), rules of the cards together (`DIFFERENT_NAMES`, CR 201.2b;
`total_mana_value_at_most(n)`, CR 202.3) and how to combine them (`Selection::new(count).with(part).called("noun")`). They
are asked with `choose::select` and `library::search` (or the `library::find_to_*` wrappers). The older card-facing
functions - `choose::{card, cards, card_from, cards_from, cards_from_kind, any_number_of, cards_within_total}`,
`search_library`, `library::Find` and the `search_to_*` wrappers - keep their signatures, re-implemented on the rules, and
are kept for existing cards; new cards use the rules.

"Up to two creature cards with total mana value 6 or less" (CR 202.3) is a rule with `select::total_mana_value_at_most(6)`,
each card's mana value read where it is (an {X} in a library or graveyard is 0, CR 202.3e); the part's `Tally` has the client
keep the running total for the player, which is display alone. Michelangelo's Technique asks it,
through `library::look_at_top_onto_battlefield` (which Aang, at the Crossroads and United Battlefront use without a total),
and Lively Dirge asks it from the graveyard; Ao, the Dawn Sky prints it too. "With different names" (CR 201.2b: each has a
name and no two share one, a split card sharing each half's name, CR 201.2a) is `select::DIFFERENT_NAMES`.

A target slot's targets together are judged by a set rule (CR 601.2c, 603.3d, 115.1): `TargetSpec::together` is one part
made with `select::set_part`, which besides its validator has a `LargestFn` finding one of the largest sets it allows, exactly
and in polynomial time (the first card of each mana value; the smallest values while they fit a total; for names, a search
within each group of cards whose names are one printed card's). `TargetSpec::together` refuses at compile time a part that
is not a set rule. A quality of each target alone ("with mana value X or less") is never the set rule's but the slot's
`filter`'s, which reads the X announced (`Query::announced_x`), so that on resolution a target that lost it is illegal alone
(CR 608.2b). A set rule whose number is `TargetSpec::COUNTED` reads the slot's count - the X announced, or the count a
reflexive ability was given (`Game::reflexive_trigger_counted`), kept on the item as `StackItem::counted`. "Any number of
target creature cards with total mana value X or less" (Fire Lord Sozin) is
`.together(select::total_mana_value_at_most(TargetSpec::COUNTED))`; "any number of target creature cards that each have a
different mana value X or less" (Agadeem's Awakening) is a filter of "creature card in your graveyard with mana value
`announced_x` or less" and `.together(select::DIFFERENT_MANA_VALUES)`; "X target cards with different mana values" (Long
Rest) is `.counted().together(select::DIFFERENT_MANA_VALUES)`; "up to six target creature cards with different names"
(Behold the Sinister Six!) is `.together(select::DIFFERENT_NAMES)`. `cantrip_api::select::judge_targets` runs the rule over
the objects among the targets (a player is no card to it, always addable and judged by nothing). Wherever targets are
chosen - a cast or activation (`actions::choose_slots`), a triggered ability going on the stack
(`settle::choose_trigger_targets`), the announcement search that offers a spell (`AnnouncementSearch`), new targets for a
copy (CR 115.7d) and a changed target (CR 115.7a, `ops::fits_together`) - `actions::within_together` takes out every
candidate the rule refuses by itself, `actions::enough_together` asks whether the slot's fewest targets can be chosen
together at all (`select::first_targets`, by the rule's `LargestFn`: a count no set reaches is no legal announcement, CR
601.2c), and the question carries the rule (`ChooseTarget::together`), by which the validator checks the answer and the
host's `Check` judges each pick, as for a card choice. The targets are judged together again as the spell or ability
resolves (CR 608.2b, `resolve::target_legal`): a set the rule no longer allows - a target gone read as it last was - makes
every target of the slot illegal (Nethroi, Apex of Death's and Run Away Together's rulings). Every part is downward-closed
(any of the cards of a set it allows are allowed too: `Part`, `SelectFn`), so the candidates offered are exactly those that
keep the choice legal. The client keeps a total's running total in the caption (`Tally`), shows the rule's words, and offers
only the candidates the host's verdict on those picked says may be added.

### Characteristics

- **Computed on query** (`chars()`), through the layers (613), from the object's card, its counters and the
  continuous effects that apply.
- **Control-changing effects (613.1b, layer 2)** set the permanent's stored controller (`Obj::controller`, with
  `marks.control_base` the one it had before any applied). They are resolved `EffectKind::Control(player)` effects
  (`Game::gain_control`, `exchange_control`) and static abilities of `Layer::Control` made with `control_changing(affects,
  text)` ("You control enchanted creature": Mind Control, Treachery), whose source's controller gains what they affect,
  judged on what layer 1 made it, at the source's timestamp (an Aura's is when it became attached, 613.7e). Layer 2 is
  ordered as every layer is (`layers::control_order`): one control-changing effect depends on another if applying the other
  changes what it applies to (a reach that reads control through `LayerQuery::controller`, which in layer 2 is the
  controller the effects applied so far leave) or whom it gives control to (its source's controller), a dependency loop is
  broken by timestamp order (RX-3), and each permanent starts from whoever controlled it before any effect did. Its result
  is written to the stored controller (`ops::write_controller`); new control is new for summoning sickness and takes the
  permanent out of combat. `ops::settle_control` writes it for every permanent at every event and added effect and at the
  end of each instruction, at cleanup, in the untap step and as permanents phase in, and as the state-based actions are
  checked - with the "for as long as" effects ended in turn until neither changes anything
  (`ops::settle_lasting_and_control`: a change of control can end "for as long as you control [it]"); free while nothing
  has ever changed control. A phased-out permanent is reached by no control-changing static ability (702.26b). With
  `CANTRIP_VERIFY`, every trigger collection checks that each stored controller is what layer 2 says, and the contract
  check asks each control-changing reach with the stored controllers moved and the view's held. Every reader of a controller reads the stored one, which layer 2 settled, so layers 3 to 7 see it. A layer-2 ability applies though its source
  loses its abilities in layer 6 (CR 613.1). Durations end it as they end any effect: end of turn (`turn.rs`), a source
  leaving, a permanent untapping (`ops::end_lapsed_durations`), or a card's own "for as long as" condition
  (`Duration::AsLongAs(card, index)` naming one of `CardDef::lasting`, asked by `ops::end_lapsed_conditions` at each check
  of the state-based actions, and before the effect begins: one false then does nothing, CR 611.2b - Old Man of the Sea's
  "for as long as this creature remains tapped and that creature's power remains less than or equal to this creature's
  power"). "You may choose not to untap this ... during your untap step" is `CardDef::may_not_untap`: the untap step asks
  the active player of each such tapped permanent before anything untaps (CR 502.3). "Doesn't untap during its controller's
  untap step" is the permanent's own `Keyword::DoesNotUntap` (Mana Vault, Bombur), or another permanent's
  `CardDef::untap_bans` (Intruder Alarm, Infinite Coursework): a rule of that source, no ability of the permanent (CR 113.1a),
  so losing its abilities leaves it and no keyword of the permanent says it ("creatures with no abilities", CR 613.1f).
  "Can't become untapped" is `CardDef::cant_become_untapped` (Frozen in Ice): the same kind of rule, but it holds in every untap, an
  effect's and a stun counter's too (`ops::untap`, CR 101.2, 122.1d).
- **Text-changing effects (612, layer 3)** are `EffectKind::ChangeText { from, to }`, a `Word` (a color word or a
  subtype) replaced by another. The layers change the type line; card code reads every word of its rules text through
  `Query::words` (`Words`, the changes in timestamp order unless one depends on another), so the card does what its changed text says. A token's
  colors and creature types are those of the text that made it (`Copiable::words`, 612.4), a permanent spell's effects
  go on to the permanent (400.7a), and an object that has left keeps the words it last had (`LastKnown::words`). Layer 3
  also changes the words of printed keyword abilities that name a color or a land type: "hexproof from [color]",
  "protection from [color]" (`CardDef::protection`, White Knight) and a basic landwalk (`Keyword::Islandwalk` ...,
  702.14: Bog Wraith's swampwalk to Magical Hack's plainswalk); an ability another permanent grants reads the word through
  its source's text instead (Lord of Atlantis's islandwalk). A text change is no copiable value (707.2), so a copy of a
  changed permanent has the printed text, and a permanent that leaves and returns is a new object with it too. An
  effect that sets a name (612.8, "named Everflame, Heroes' Legacy") is `EffectKind::SetName`, in layer 3: the object
  has that name alone (`Chars::name_set`), for the legend rule and every "same name" or chosen-name question
  (`library::object_names`).
- **Dependency (613.8): the layer pass.** Each layer's application order is fixed once per position over the whole game
  (`layers.rs`, "The layer pass" below for the design): every entry of the layer - each static ability of each source, each
  ability an object was given, each resolved effect and keyword counter - in timestamp order (613.7), then ordered by the
  dependency system, judged on the game as the entries applied so far leave it (613.8c). An entry depends on another if
  applying the other would change whether it exists (its source losing it: `Chars::lost_all`, `Chars::text_lost`), what it
  applies to, or what it does to an object it applies to - judged on what it does to the object as it stands, so that two
  effects that set the same thing do not depend on each other through it (613.9: "has flying" and "loses flying") - over
  the objects the other applies to and any object the question read (`Judge`). A dependency loop is broken by timestamp
  order among its members, every other dependency kept (`docs/rules-exceptions.md`, RX-3). An object's characteristics are
  then its entries applied in that order (`at`, `through`), each read by the static abilities applied at the same step:
  a static ability applied at step `k` reads every other object at step `k` (`View::stage`). What is worked out is kept for
  the position (`GameState::layer_pass`, keyed on `GameState::chars_epoch`, which only writes that could change a
  characteristic move, and on the history's length once a static ability asked about the past); a layer with nothing that
  could depend on anything is in timestamp order and judges nothing. Characteristic-defining abilities apply first and apart
  (613.3, 613.8a(c)). The words of rules text follow the same order (`Words::word`). With `CANTRIP_VERIFY` one answer in
  sixteen is worked out again in a pass of its own and must agree.
- **Type-changing effects (613.1d, layer 4)**: `AddTypes`, `SetTypes`, `AddSubtype`,
  `SetCreatureType`, `AddAllCreatureTypes` ("with all creature types", only to a creature or Kindred, 205.3d) and
  `LoseAllCreatureTypes`. Changeling's every creature type is a characteristic-defining ability applied first (613.3).
- **Static abilities given to objects (613.1f, 613.7a)** are `granted_static` entries (`AbilityDef::Static`) of a card's
  `delayed` list, given with `EffectKind::AddAbility` or by a static ability that gives abilities (Rune of Flight's shape).
  One of layer 6 comes into being in layer 6 itself, once the entry that gives it has applied, and joins the entries still
  to apply; from 7a on the ones an object still has once layer 6 is done apply (so "loses all abilities" later in timestamp
  order takes them) - to it alone, with it as their source, or to what they affect (`granted_static_ability`, The Irencrag's
  "Equipped creature gets +3/+3") - each at the later of its object's timestamp and its giver's. One of an earlier layer is
  refused at table construction: it would come into being after its layer was done.
- **"Loses all abilities" (613.1f)** comes from a resolved effect (`EffectKind::LoseAllAbilities`: Oko's Elk, The Wondrous
  Wasp) or a static ability (`loses_all_abilities(affects, text)`, a `StaticDef` with `loses_all`: Eaten by Piranhas; Darksteel
  Mutation, Kenrith's Transformation, Frogify and Humility print the same; `loses_all_abilities_except_mana_abilities` keeps
  the mana abilities the object has at its timestamp, 605.1: Blood Sun). Either is a layer-6 effect at its timestamp, so an
  ability gained after it stays (613.7). A permanent about to enter that such a static ability would strip is judged as it
  would be on the battlefield (614.12, below): its own `enters` replacements do not apply. Every rule read off a card - its static abilities from layer 6 on, cost changes,
  replacement effects, play rules, trigger, block and activation rules, granted ways, miracles - asks the one answer,
  `Query::lost_all_abilities` (`layers::lost_all_abilities`): the effects, then what the layer pass left the object
  (`Chars::lost_all`, set where the "loses all abilities" applied in layer 6's order), free while no such ability is on the
  battlefield. A source's static abilities of layers 1 to 5 have applied by the time
  layer 6 takes them (613.6: Painter's Servant enchanted by Eaten by Piranhas, or turned into an Elk, still paints everything),
  so the static-source list keeps such a source and leaves it out from layer 6 on - all but the effects that had begun by
  then. **An effect in parts (613.6)** is one `StaticDef` per layer, each later part marked `.continues()` (Opalescence:
  layers 4 and 7b; Puppet Crafting, Eaten by Piranhas, Ygra, Kaito, Wurmwall Sweeper). As an object's characteristics are
  worked out, the layers note the static abilities that applied to it (`Begun`, only while a source has an effect in parts); a
  later part applies to it exactly when its first part did - "the same set of objects", whatever its words say by then - and
  goes on applying though its source loses its abilities in a layer between, if the first part was of layers 1 to 5 or is the
  "loses all abilities" ability that took its own source's (Humility made a creature). A layer-6 part of a source that
  loses its abilities in layer 6 never begins (613.8a: its existence depends on the effect that removes it). One "loses all
  abilities" ability that would take another's source's applies first (613.8a dependency), each taking the other's in
  timestamp order (613.8b). The host lists every
  rules line of an object that lost all abilities as lost, not only its keywords and activated and triggered abilities.
- **Setting a land's subtype (305.7, layer 4)** is a static ability made with `sets_land_subtypes(affects, apply, text)`
  (`StaticDef::sets_land_subtypes`: Magus of the Moon, Multiversal Passage; Blood Moon, Spreading Seas and Evil Presence print
  the same), whose `apply` calls `Chars::set_land_subtypes`: the land's land types are replaced (its other subtypes, an animated
  land's creature types, stay) and every ability it has in layer 4 - its rules text's and its copy effects' - is taken; the
  basic land type's mana ability follows in layer 6 (305.6), and what other effects grant it after. Where in layer 4's order
  it applies, the land is marked (`Chars::text_lost`): its static abilities that have not begun to apply exist no more, and
  each depends on the setting effect (613.8a: existence), which applies first unless they are in a loop with it (Urborg
  under Magus makes nothing a Swamp, either order; a land whose own ability makes lands what another setting ability sets
  applies in timestamp order with it, 613.8b) - and `Query::lost_all_abilities` answers for it, so every rule its card
  prints is gone. The
  setting ability itself applies whatever its source loses in layer 6 (613.6: Magus under Eaten by Piranhas), and a land's own
  ability that sets its own subtype (Multiversal Passage) goes on applying and does not count it as having lost its text. A land
  about to enter is judged as it would be on the battlefield (614.12, below): under such an effect its own `enters`
  replacements - "enters tapped", "as this enters" - do not apply.
- **A permanent about to enter is judged as it would exist on the battlefield (614.12)**: `ops::as_it_would_exist` puts it
  there for the moment of the question - under the player it enters under, tapped or not, with the face, copy effect,
  choices, counters, cast door and protector the replacements applied so far gave it, at a new (the latest) timestamp, and
  with the permanents leaving the battlefield in the same move gone (`GameState::leaving_together`, Goblin Welder) - reads
  its characteristics and
  `lost_all_abilities` through every layer, other objects' static abilities and its own alike, and reverts. One answer serves
  every entering check: `prepare_entry` leaves its own `enters` replacements to apply, and daybound's "enters transformed", only if it
  would keep its abilities (asked only while a "loses all abilities" or land-subtype-setting static ability is on the
  battlefield or its own, `layers::abilities_may_be_lost`), and asks again as the copy once a copy replacement chose one; and
  `ops::judged_entering` sets the answer on `Cx::entering` while the replacement effects of its zone change and the counters
  it enters with are judged - again after each replacement that changes whom it enters under or whether tapped, and not at
  all once one sends it elsewhere (`ops::rejudge_entering`), its own replacements read off what it enters as - so `Query::chars` and `Query::lost_all_abilities` of the entering object answer as it would be
  there ("creatures your opponents control enter tapped" taps a Forest Life and Limb would make a creature; Containment
  Priest exiles a card entering transformed into a creature) - asked only where a replacement effect could apply
  (`replace::possible`) or a counter ban could limit its counters. A resolved effect that sets a land's
  subtype ("target land becomes an Island until end of turn") has no `EffectKind` yet; the copiable values a copy effect gave
  such a land other than abilities stay: CR 305.7 lists copiable effects among the sources of the abilities it loses,
  against the abilities other effects granted, which it keeps.
- **The order of a permanent's entering replacements (CR 616.1c-e)**: a replacement that has it enter as a copy applies
  first, then one that has it enter back face up (daybound at night, judged on the copy, `ops::enters_by_daybound`), then
  the rest. A card's own copy replacement is its typed `CardDef::enters_as_copy` (built with
  `cantrip_mechanics::copy::enter_as_copy*`), which `ops::prepare_entry` runs first; a plain `CardDef::enters` that makes it a
  copy stops a verifying run (`ops::verify_no_copy_in_enters`), so the engine, not a card, decides what is a copy
  replacement. Copy replacements from other sources - Essence of the Wild's "creatures you control enter as a copy",
  Mystic Reflection's - are not built yet; when one is, it must take the same place, before the back-face-up one.
  Daybound only ever turns an entry over: a face an instruction or CR 707.8a gave it stays. The rest (CR 616.1e) - its own
  `CardDef::enters` and those of what it enters as a copy of (CR 707.9a), which `prepare_entry` only notes as still to apply
  (`Entering::own`), and every other replacement of its zone change - are applied by `ops::entry_replacements` in the order
  its controller chooses (`replace::entry`, `Origin::Enters`, offered in the card's own words), each judged again as it would
  exist once the ones before it applied (CR 616.1f): a land's own "enters tapped" and Spelunking's "lands you control enter
  untapped" in either order, so it enters tapped or untapped (Spelunking's ruling). Where no other replacement could apply
  (`replace::zone_change_possible`) its own simply apply, and nothing is judged or asked.

- **An effect that says what a card is as it enters is that from the start (614.12)**: `Game::put_onto_battlefield_as`
  takes an `EntersAs` ("It's an enchantment", the Enduring cycle; "It's an Aura enchantment with enchant Forest you control
  and '...'", Old-Growth Troll), carried on `Entering::becomes`. `as_it_would_exist` judges it with those effects, and
  `move_together` gives them to the new object as it arrives, before its arrival is announced, so nothing that watches
  creatures enter sees a creature: ordinary continuous effects of `Duration::Indefinite` (types in layer 4, then abilities
  lost and gained in layer 6, at timestamps in that order). As an Aura it goes through `attach::choose_aura_hosts` as a
  printed one does (303.4f-g); its enchant ability is `EffectKind::Enchant(card)`, the card's `becomes_aura_enchant` (Necromancy's too).
- **Enchant abilities are abilities (702.5a)**: a printed one and a given one are each an `AbilityRef::enchant(card)` among
  the object's abilities (layer 6), so "loses all abilities" takes them (613.1f). `attach::host_allows`, `player_allows` and
  `choose_aura_hosts` read them off the characteristics: each must allow the host (702.5c), one that has none is restricted
  by none, and an Aura that is a creature enchants nothing (303.4d). "It loses '[enchant ...]'" is
  `EffectKind::LoseEnchant(card)` (113.10b, 613.1f), every instance of that card's printed one: Animate Dead loses it and
  gains its `becomes_aura_enchant` as its enters ability resolves, so a card that then can't return leaves it attached to
  what it can no longer enchant (704.5m).
- **A face-down permanent is what the rule that put it there lists (708.2)**: `Game::put_onto_battlefield_face_down` takes
  `FaceDown::Manifested` (701.40a, a 2/2) or `FaceDown::Cloaked` (701.58a, a 2/2 with ward {2}), and gives the card those
  copiable values (`intrinsic::FACE_DOWN`, `intrinsic::CLOAKED`) before it enters (708.3). On the battlefield they are its
  face-down status, layer 1b over its copy effects (613.2b): `Marks::face_down_as` names them and `Obj::copiable` lays them
  over `Obj::copy`, which holds only the copy effects, so a face-down permanent that becomes a copy is still a nameless
  2/2 and the copy once turned face up (708.10); every reader of what an object is goes through `Obj::copiable` or
  `CardTable::current`. The core recognises them as data,
  `CardDef::face_down`, never by comparing card ids, and the special action that turns either face up is each one's ability
  `intrinsic::TURN_FACE_UP` (`CardTable::turns_face_up`, `CardTable::turn_up`), paid with its card's mana cost (701.40b, 701.58b). Ward {N}
  (`intrinsic::ward`) is defined with them, since a cloaked permanent has it by the rules; cards reach it as
  `cantrip_mechanics::triggers::ward`, and cloak itself as `cantrip_mechanics::cloak`.
- **Morph, megamorph and disguise cast a card face down (702.37c, 702.168b, 708.4)**: a card's way of `WayKind::Morph`,
  `Megamorph` or `Disguise` (`cantrip_api::face_down_way`, made by the keyword modules `cantrip_mechanics::morph`,
  `megamorph`, `disguise`) costs {3} and casts the face `intrinsic::MORPHED` or `intrinsic::DISGUISED` (`actions::cast_face`),
  so everything that judges a spell before it is cast - permissions (601.3e), its speed, bans, cost changes, its additional
  costs (none) - judges that 2/2 with no text. `cast` gives the card those values as its copy before the move, and the move to
  the stack makes the spell face down with them as `Marks::face_down_as`, as one onto the battlefield does (708.3); resolving,
  it enters face down the same way (`resolve`). Its controller alone may look at it (708.5: `hidden_from`,
  `visibility::knows_face`); a face-down spell or permanent that goes to any other zone arrives face up and is `Revealed`
  (708.9). Turning a face-down permanent up by its morph or disguise cost is a special action every set of face-down values
  carries (`intrinsic::TURN_UP_MORPH_TEXT`, `TURN_UP_DISGUISE_TEXT`; `MORPHED` and `DISGUISED` have no mana-cost one), offered
  where the permanent would have that keyword face up (`Query::turn_up_way`) - cast face down, manifested or cloaked
  (701.40c-d, 701.58c-d). Morph and disguise are keywords a card has by its way (`CardDef::printed_keywords`,
  `Keyword::Morph`, `Keyword::Disguise`), so they go through layer 6 as any ability does; "would have it face up" is the
  layers' answer for the permanent judged with its normal copiable values (its card's, or its copy effects', 708.10) under
  every effect that applies to it now (708.8): `Query::chars_if_face_up` names it on `GameState::judged_face_up`, which keeps
  that pass apart. "Loses all abilities" (Humility, Turn to Frog) thus leaves no morph action, while a manifested or cloaked
  creature card is still turned up for its mana cost - a rules action, no ability of its card (Whisperwood Elemental's
  ruling). Its cost is the way's `turn_up`, a whole `Cost` paid as an
  activation's is (`special_action_whole_cost`: "Morph—Pay 5 life", a discard, a return), X announced and kept on the
  permanent (702.37f); megamorph's +1/+1 counter is put by `ops::turn_face_up_as` before `TurnedFaceUp` is recorded
  (702.37b, 708.11), only for that action.
- **A prototype card cast prototyped (702.160a, 718)** is cast as its prototyped face (`WAY_PROTOTYPE`): the card with only
  the inset frame's mana cost, power and toughness, and the color that mana cost gives (718.3b). The values are the card's
  content (`"prototype": {"manaCost", "power", "toughness"}`, `CardDef::prototype`); the face is made from the card by
  `cantrip_api::prototyped_face` (one leaked definition per card for the process, `CardDef::prototyped` set), never written
  beside it, so it has every rule the card prints (718.5), and every card table registers it after its card. It is one of
  `faces_to_cast`, so it is offered wherever the card could be cast and judged as that face (718.3a), for its own mana cost or
  any alternative cost (the ruling: casting it prototyped is no alternative cost). The spell keeps the face as its copiable
  values (`Obj::copy`) and so does the permanent it becomes (`resolve`, as a bestowed spell does), so a copy of either is
  prototyped (718.2a, 718.3c-d); anywhere else, and once it leaves the stack or the battlefield, it is a new object with its
  normal characteristics (718.4, 400.7). `cantrip_mechanics::prototype::prototyped` asks whether an object is.
- **A damage event names its source as it dealt the damage**: `Event::Damage::known`, a last-known row pushed as it is
  dealt (`ops::dealer_known`, from the characteristics the damage was judged by), read through `History::damage_dealt` and
  `DamageDealt::chars`, so "a creature that, at the time it dealt that damage, was an Assassin creature ... under your
  control" (freerunning, 702.173a) asks what it was then. It made `Event` 28 bytes rather than 24.
- **A card exiled face down has no characteristics (406.3a)**: `layers::chars` gives a face-down card outside the
  battlefield and the stack (`Obj::face_down_card`) those of `intrinsic::NO_CHARACTERISTICS` - no name, no mana value, no
  types, no abilities - for every reader, so nothing that counts or matches cards in exile sees what it is, and its abilities
  function nowhere. Who may look at it (`seen_by`, 406.3) is knowledge, not characteristics: the host view shows them the
  card, and a script names it by its card (`Game::known_name`). Playing it is asked of the resulting spell or land (406.3a-b,
  601.3): while the offers, the costs a menu row shows and a cast during resolution are worked out, `layers::judged_face_up`
  names that one card on `GameState::judged_face_up` and the layers give it the card's characteristics; as it is played,
  `ops::turn_up_to_play` turns it face up just before the announcement (601.2) or the land play.
- **A spell not yet cast is judged as the spell it would be (601.2a-c)**: a spell is on the stack, as the face it is cast as
  (709.3, 715.3, 712.8, 720.3), under its caster, before its modes and targets are announced. `layers::can_target` judges a
  spell whose card is elsewhere on everything that does not read it; where something on the board reads the spell that would
  target (`layers::targeting_readers`: a hexproof from a quality, a protection, a `TargetingRuleKind::Forbid`),
  `actions::priority_options` and the casts an effect makes (`cast_ways_during_resolution`,
  `could_cast_transformed_during_resolution`) judge each cast with modes or targets again with the card put on the stack for
  the moment as that spell (`ops::as_cast`, taken back after, with what the layers had worked out for the position set
  aside and put back: `GameState::set_derived_aside`), so every effect that applies to it there applies. A permission to
  cast as though it had flash (601.3b, 601.3e) is judged the same way (`actions::SpellsAsCast`, open while
  `priority_options` answers): where one exists (an effect's `CastAsThoughFlash`, a permanent's `flash_rules`) and the spell
  could differ from the card where it is - a static ability that functions outside the battlefield exists
  (`layers::statics_everywhere`), or the card has a characteristic-defining ability - the offer takes a cast it has not
  judged as having no permission, asks for it where a cast at instant speed could be paid, and is made again once a
  judgment grants one (Rattlechains flashes an opponent's creature card that their own Arcane Adaptation makes a Spirit as
  their spell); suspend's timing (702.62a) reads the same. With neither, the card where it is has what its spell would have,
  and is read there. An alternative cost another permanent gives (118.9: Dracogenesis's "Dragon spells") reads the spell's
  characteristics so judged (`actions::spell_chars`, through the view `AsSpell`), and the card where it is for anything
  else (Omniscience's "from your hand"); a cast an effect makes judges them at once (`chars_as_cast_now`). A ban
  on spells with some qualities (`spell_rules`) is judged on the spell there too, for each cast offered and each suspend
  (`casts_forbidden_on_the_stack`) and each cast an effect makes. The cost changes read a face through
  `layers::chars_as_face`: the card where it is, with the face's values (`GameState::judged_as`), through the layers - and,
  in the offer, the spell's characteristics judged on the stack where they differ (`spell_chars` in `deltas_watched`: Herald's
  Horn with Arcane Adaptation makes an opponent's creature card the caster casts cheaper).
- **A card's printed characteristics are made once** (`CardTable::printed`), as a pure function of the card, both
  before and after the ability layer, so an object nothing acts on costs a lookup.
- **The static-ability sources are found once per position** (the epoch cache above), not on every query.
- **A static ability whose condition is about the past** ("as long as you've lost life this turn", "as long as this Aura
  entered this turn") asks the event history (`LayerQuery::history`, the same records `q.history()` reads). The effect of
  a static ability is not locked in (CR 611.3a): the layers apply it whenever its condition holds as characteristics are
  computed. The history is a pure function of the position - kept beside the state, restored with it as the game tree
  moves, never cached - so every layers entry point (`layers::chars`, `can_target`, `targeting_rule`, `summoning_sick`,
  `describe`) is handed the log with the state. No cache holds characteristics: only the static-ability source list is
  cached on the epoch, and which permanents have static abilities does not depend on the history, so recording an event
  (which does not move the epoch) needs no invalidation; the condition's answer changes the moment an event is recorded
  and the moment a new turn begins. Essence Channeler and Shardmage's Rescue read it; the "this turn" conditions printed on
  Tenured Concocter, Thornfist Striker, Brightspear Zealot, Eyekite and the celebration cards are the same shape.
- **No strings at runtime:** keywords, types, subtypes and colors are enums or bitsets, card behavior is reached by
  function index, and nothing about a card is read from JSON while a game runs.

### The layer pass (built; the design as reviewed)

**Why.** CR 613.8 orders *effects*, not objects. An effect depends on another if applying the other "would change the text
or the existence of the first effect, what it applies to, or what it does to any of the things it applies to"
(613.8a(b)). Today `layers::chars_before` works out each object alone, and orders a layer's effects by what that one
object shows (`in_order`, `by_dependency`, `ready`). A dependency visible only through another object is missed (KI-41).
The pass fixes one application order per layer per position, over every object the layer's effects could reach. That
order is the same whichever object is asked about.

**Entries and what each declares.** An *entry* is one part of one effect, in one layer or sublayer:
- a static ability's part (its own, or one granted to an object; 613.7a);
- a resolved effect's part;
- the continued part of an effect begun in an earlier layer (613.6);
- a keyword counter (613.7c).

Each entry declares, at table construction (for statics and granted statics) or when the effect is made (for resolved
effects):
- its *write set*: which characteristics its `apply` changes. Types, subtypes, colour, abilities, P/T, control, text and
  name are separate bits.
- its *read set*: which characteristics its reach (`affects`), its condition and its `eval` read; whether it reads the
  history; and which zones it reaches (battlefield; stack; hand, graveyard, library and exile for `everywhere` statics and
  `affects` over those zones).
- an `eval` beside `apply`, described next.

**`eval` and `apply`.** `eval(view, object)` works out the effect's instruction for one object it applies to:
- the N of "+N/+N" with N counted;
- the colour set;
- the ability list granted or removed;
- the P/T set;
- the types set or added;
- the controller.

`apply(instruction, chars)` carries the instruction out. The "what it does" criterion compares instructions, never
resulting characteristics. Take "has flying" and "loses flying" on a creature with printed flying (613.9). Applying
either one changes neither the other's reach nor its instruction ("add flying" and "remove flying" stay what they are),
so they are independent and go in timestamp order. Comparing results would make each "depend" on the other, through
whether flying is already there. Akroma's Memorial and Turn to Frog behave the same way: the Frog's "loses all abilities"
in layer 6 does not change what the Memorial's grant does.

**When a layer builds a pass.** A layer builds a pass only when some entry B in it could change another entry A of the
same layer. That means B's write set meets A's read set, or B strips A's source (existence; see below). Otherwise the
layer applies in timestamp order per object, as today, and nothing is built.

So an anthem plus Giant Growth never builds a pass: both write P/T in 7c and neither reads P/T, colour or anything the
other writes. A plain board builds none. The gate is on the entries present in the position, never on the card table.

**The set of objects.**
- Every permanent except phased-out ones (702.26b: a phased-out permanent is treated as though it doesn't exist, so it is
  outside every set, and its own static abilities do nothing).
- Every object on the stack.
- Every object a resolved effect names.
- Every object in a zone that an entry's declared zones reach.

Cards outside the game (sideboards) take characteristic-defining abilities only (604.3). Effects don't reach them, so
they are worked out alone. Abilities on the stack are outside the set, as now: their text is their source's.

**Existence.** An entry exists while:
- its source has the ability that generates it;
- its condition holds (an "as long as" that turns false makes the entry not exist);
- its source is where the ability functions (the battlefield, or any zone for `everywhere`).

Applying B changes A's existence when B removes A's ability from A's source, makes A's condition false or true, or (in
layers 1-4) changes whether A's source is a permanent whose ability functions. Examples of B: "loses all abilities"
(613.1f), a land-subtype setter under 305.7, a type change that turns a condition.

For 305.7, the abilities lost are those from the land's text, including text a copy effect gave it (KI-51: a land whose
subtype is set loses the abilities a copy effect gave it and keeps the other copiable values).

Entries also appear part-way through a layer. A static ability granted in layer 6 whose own part is in layer 6 (Rune of
Flight grants Colossus Hammer "Equipped creature has flying"; the Hammer's own "loses flying") enters the layer-6 order
as soon as its grant has applied. Its timestamp is the later of its object's and the granting effect's (613.7a). It is
re-stamped with the object, keeping the relative order (613.7e). Here the Rune's grant stamps it after the Hammer's
"loses flying", so the creature has flying.

An entry is dropped when its source loses the ability part-way through the layer, unless the entry began in an earlier
layer (613.6: it goes on). A granted static with parts in layers 4-6 joins those layers once it is granted. A layer-6
grant of a static that has a layer-4 or layer-5 part cannot apply that part, because those layers are done. Its parts in
layer 6 and later apply. Today's code ignores granted statics whose parts are earlier than 7a, and refuses them at table
construction. The pass keeps that refusal for parts in layers already done, and applies a granted static's parts in
every layer from the grant's own onward.

**613.6: the set is fixed when an effect first applies.** The first time an effect applies, in any layer, the set of
objects it applies to is recorded. Every later part of that effect applies to the same set, even if the ability is gone.
A begun effect therefore cannot depend on anything through "applies to" or through "existence"; it can still depend
through "what it does" (its later parts' `eval`).

This settles Humility and Opalescence in both orders:
- Opalescence's type change begins in layer 4, so its layer-7b part keeps its set (Humility among them, as a creature).
- Humility's 6 and 7b parts begin in layer 6, on the creatures of then.
- Their order in 7b is then by timestamp, as the rulings give.

**Building one layer (613.7-613.8).** The working copy holds each object of the set with what the earlier layers left it.
While a layer is being built, every characteristic a reach, condition or `eval` reads, of any object, is read from the
working copy as it stands at that point in the layer.

Dependencies are judged on this working copy at every step - the whole board with the effects applied so far - and judged
again after each application (613.8c); never on a pair of effects against all the rest. A depends on B when all of these
hold (613.8a):
- both are in the same layer and sublayer;
- both or neither are characteristic-defining abilities (CDAs go first, among themselves; 613.3);
- applying B to the working copy would change A's text (B is a text change to A's source), A's existence, A's reach, or
  A's instruction (`eval`) on an object in both reaches.

These are judged only over reach(B), restricted to where A's declared zones meet B's. An object B doesn't reach is
unchanged by B, so A's reach and instruction there are unchanged.

Of the ready entries (below), the earliest-timestamped applies, and then dependencies are judged again (613.8c). The judging is
incremental. After B applies, only the pairs whose read set meets B's write set are judged again, over the objects B
changed. Trial applications go into a copy-on-write overlay of the working copy (only the objects touched are copied),
which is then dropped.

**"Ready" (613.8b), decided: reading (C), `docs/rules-exceptions.md` RX-3.** 613.8b says a dependent effect waits for
all of the effects it depends on, and that "if several dependent effects form a dependency loop, then this rule is
ignored and the effects in the dependency loop are applied in timestamp order". Where a member of a loop also depends on an
effect outside it, only the dependencies inside the loop are ignored: the loop is broken by timestamp order among its
members, and every other dependency is kept, so the graph is a DAG. An entry is ready when every unapplied entry it depends
on is in a loop with it. Of the ready entries the earliest-timestamped applies. The two readings not chosen were:
- (A) the loop waits as a unit for every outside dependency of its members;
- (B) the rule is ignored for the loop's members entirely.

Every dependency is judged on the working copy as it stands at that step: the whole board with the effects applied so
far, re-judged after each application (613.8c). It is never judged on one pair of effects against all the rest; the
held KI-41 branch did that, and it contradicts 613.8c.

The Arcane Adaptation board (`blood-moon-life-and-limb-and-arcane-adaptation-making-a-saproling-depend-through-the-layer`,
held with KI-41) has Life and Limb (turn 1), Arcane Adaptation naming Saproling (turn 3) and Blood Moon (turn 5), all in
layer 4. Its starting graph, judged on the working copy:
- Arcane Adaptation and Life and Limb depend on each other, so they are a loop.
- Life and Limb also depends on Blood Moon.
- Blood Moon depends on nothing.

Results under each reading (C, decided, is the engine's):
- **(A):** Blood Moon, then Arcane Adaptation, then Life and Limb.
  - Taiga is a plain Mountain, tapping for {R}.
  - Llanowar Elves are a Creature Land, Elf Druid Saproling Forest, keeping {G}.
- **(B):** Life and Limb, then Arcane Adaptation, then Blood Moon.
  - Taiga is a Land Creature, Mountain Saproling, 1/1.
  - The Elves are a Creature, Elf Druid Saproling, keeping {G}, and not a land.
- **(C):** Arcane Adaptation, then Life and Limb, then Blood Moon.
  - Taiga is a Land Creature, Mountain Saproling, 1/1.
  - The Elves are a Creature Land, Elf Druid Saproling Mountain, tapping for {R} (305.7).

The held scenario expects (C). Tests pin the other two timestamp orderings of the same board, among them Blood Moon on turn 3
and Arcane Adaptation on turn 5.

**Layers 1 and 2.**
- **Layer 1:** copy effects and face-down values stay the copiable values the per-object pipeline starts from. A copy
  effect's values are fixed when it begins (707.2b, 707.2c), so layer 1 has no cross-object dependency left to judge.
  305.7's removal of what a copy effect gave a land is recorded by the layer-4 pass: per land, the point in the order at
  which its text abilities, including copied ones, went. Layer 6 reads that record.
- **Layer 2 (control):** ordered by the same rules (`layers::control_order`): its effects - control-changing static
  abilities and resolved control effects - in timestamp order unless one depends on another, a reach judged on what layer 1
  made each object and the controllers the effects applied so far leave (`LayerQuery::controller`), a loop broken by
  timestamp order (RX-3). Its result is written to the stored controller where control is settled (at every event, added
  effect and instruction's end), and every reader keeps reading the stored controller.

**Fixed eagerly, applied lazily.** Building a layer fixes its order and each entry's recorded set, nothing more.
- An object's characteristics through a layer are applied on demand: the entries of the order that reach it, in order,
  with their instructions.
- They are memoised per (object, layer, point in the order).
- An object asked about outside every set is worked out alone, and nothing it computes mid-layer is ever cached.

**Re-entrancy.**
- A pass is built into a local value and installed only when it is complete.
- While layer L is being built, a read of a later layer's characteristics answers from the working copy as it stands in
  L, plus the earlier layers' passes. Layers after L are not yet applied to anyone, which is what the rules see at that
  point.
- A request to build or read a pass while one is being built for the same position is a bug. Under CANTRIP_VERIFY it
  panics; otherwise it is answered from the working copy.
- The pass is never built or read while any of these is set:
  - `judged_face_up`;
  - `judged_as`;
  - the `GIVERS`, `WATCHED` or `WATCHING` ability-giver tracking;
  - the 305.7 and "loses all" `finding` flags.

  Those paths work their object out alone, uncached. The same holds for `as_it_would_exist` (a permanent about to enter,
  614.12), and for last-known information and look-back snapshots: each builds its own pass on its own state, in a local
  value, and never writes the live cache.

**The cache and its key.**
- A `RefCell<Option<Rc<LayerPass>>>` on `GameState`, no part of the position or the journal. A clone of the state shares
  the `Rc` or drops it; it never copies a large pass.
- The key is a *characteristics epoch*. It moves only on writes to layer inputs: objects' zones, timestamps, status,
  counters, attachments and face, effects, permissions that grant abilities, and control.
- `log.len()` is in the key only for a layer with an entry that reads the history.
- Re-stamps (613.7e-g: attaching, turning face up or down, transforming) move the epoch through the timestamp write.

**CANTRIP_VERIFY cross-check.** For each object asked about, the characteristics from the pass are compared with the
object worked out alone wherever the two must agree: a layer with no pass, or an object no entry with a dependency
reaches. A mismatch panics.

**What goes.** `in_order`'s snapshots and its waiting, `by_dependency`, `ready`, and `lost_to_statics` and
`land_text_lost` as separate fixpoints (their answers become the pass's records). KI-41's held per-pair judgement
(`through_others`, `layer_dependencies`) is not landed.

**Performance acceptance.**
- The normal bench must be within noise of main.
- The layers deck (Life and Limb, Blood Moon, Arcane Adaptation x4) must be at or below main's ~395 us per decision.
- Measured against main and against 8b841db84 (the per-pair judgement, ~5,100 us on that deck).

**Tests.** Kept: the existing layer-dependency scenarios and Rust tests, and KI-41's held tests (the Arcane Adaptation
scenario, `layer_dependency.rs`, the reviewer's order and off-field probes). New:
- the 613.9 pair: "has flying" and "loses flying" on a creature with printed flying, in both timestamp orders;
- Akroma's Memorial with Turn to Frog;
- Rune of Flight on Colossus Hammer, the creature equipped;
- Humility and Opalescence, in both orders;
- Blood Moon and Urborg, Tomb of Yawgmoth, in both orders;
- the 613.6 examples (Act of Treason; the "become 2/2 artifact creatures" set kept into 7b; Svogthos);
- Conspiracy with Arcane Adaptation, reaching cards in hand and library;
- a phased-out Changeling;
- a face-down card judged face up under Painter's Servant;
- Vesuva copying Urborg under Blood Moon;
- KI-61's loop (a land's own layer-4 ability beside Blood Moon);
- the same characteristics for two objects asked in either order.

### Card data: three sources

1. **Characteristics, generated.** Name, mana cost, types, colors, power and toughness, loyalty, core keywords and
   rules text are `content/<dataset>/cards/<card>.json`, which the client also draws from, so the engine and the
   client cannot disagree. `crates/cantrip-oracle/build.rs` turns them into Rust constants at build time (into `OUT_DIR`; nothing
   generated is checked in, so adding a card adds files and edits none). The subtype enum in `cantrip-state` is
   rules data, every subtype of CR 205.3, edited only when the rules add one; the build fails naming the card
   and the subtype if a card prints one that is not there. A card with nothing but characteristics (a vanilla creature, a basic
   land) needs no file. An MTGJSON import may replace the hand-kept JSON later.
2. **Behavior, in Rust.** Every ability: a card's file adds behavior with struct update syntax over its generated
   constant (`CardDef { spell: ..., ..oracle::LIGHTNING_BOLT }`). The compiler checks all of it, so there is no
   load-time validator, nothing is parsed at runtime, and a typo is a build error rather than a surprise in a game.
3. **Presentation, in JSON, editable without a rebuild.** Art and frames, never read by the rules. Planned: a
   variant list per card (`content/<dataset>/presentation/<card>.json`, keyed by the card's name) with an art asset,
   artist, frame template and a default; decklists may name a variant; `Cantrip.Art` serves art by card and variant.
   The tag rides outside the rules state and is shown under the same visibility as the card's identity.

### Events and triggers

Triggers are indexed by the kinds of event they listen for (`EventMask`), so an event checks only the triggers that
could match it. Where a triggered ability functions (CR 113.6) is `TriggeredDef::functions`, and the engine asks it in
those zones only, so no matcher checks its own zone: the battlefield, unless its trigger condition can't trigger from
there (CR 113.6k) - "when you cast this spell" is made by `when_this_spell_is_cast` and functions from the stack, "when
this card is put into a graveyard from anywhere" is a shared trigger that functions from the graveyard, "when you discard
this card" functions wherever a discarded card can end up (`WHERE_A_DISCARD_ENDS`, a library included: CR 113.6k,
702.29c), a discard being the event rather than a move to a graveyard (Sand Golem's "return this card from your
graveyard" included). A card a discard put into a hand or a library unrevealed has undefined characteristics (CR
701.9c): `Event::Discarded::undefined` says so in the history, its own abilities trigger on nothing, and what asks what
was discarded reads it through `cantrip_mechanics::hand::discarded_card`, never `chars` (Aclazotz's land card, Seasoned
Pyromancer's nonland cards). A cycle (CR 702.29c) is the discard paying a cycling or typecycling ability's cost
(`ActivatedDef::cycling`, set only by the keyword's shared mechanics and held to the Oracle text by
`cantrip-cards/tests/cycling_matches_oracle.rs`), never another "Discard this card" cost (channel): `Event::Cycled`
follows that `Discarded` in the discard's batch, naming the same object, the card where it ended - or its own text
names the zone (CR 113.6b, 113.6m: "return this card from your graveyard"), which the card declares with `functions_in`.
`CardTable::functions_elsewhere` says per card which other zones to look in, and
`cantrip-cards/tests/functioning_zones_match_text.rs` holds every ability's zones, declared or derived, to its words and
its card's Oracle text. "Leaves the battlefield" look-back (CR 603.10) reads the frozen last-known rows. Whether a triggering
looks back is the rules' to say from the event it matched (`CardTable::looks_back`), never a card's: a zone change
from the battlefield (a sacrifice's included) or from a graveyard, an object all players can see put into a hand or a
library, a permanent phasing out, an unattachment, a countered spell, a player losing the game. The two things a
zone-change event can't show are read from the ability's own trigger-condition words as the table is made
(`read_zone_words`): "from anywhere" (CR 603.6c: never a leaves-the-battlefield ability) and a card leaving a graveyard
(which looks back on a card put onto the battlefield from a graveyard, an enters-the-battlefield event for every other
ability, CR 603.6a). Whether all players could see the object is decided by the move as it is made
(`ops::seen_by_all`): a card in a hand or a library is public only while a reveal of it is in effect (CR 701.20a) - its
instruction moving it is the one that reveals it ("reveal it and put it into your hand": the reveal-and-move verbs,
`Game::return_to_hand_revealed` and its kin, mark the move; an effect's reveal lasts only through that instruction, so a
later draw in the same effect moves a hidden card), a replacement reveals it as it moves, a cost or announcement of a
spell or ability still on the stack revealed it (`StackItem::members`) or it is an ability's hidden source (CR 602.2a),
or its reveal triggered an ability not yet off the stack (miracle). No card the engine has keeps a card revealed by a
continuous effect; the first that does builds that. The history's `Event::Revealed` is what was shown, never whether a card is revealed
now. No card declares anything. An ability that matches events of both kinds ("enters or dies") is
judged on each as that event says. An unattachment and a countered spell are announced just before they happen, so the
board they are judged on is the one before them. Phasing out and losing the game are recorded in the history but are no
trigger events yet (`EventMask::of` gives them no mask), so nothing triggers on them; the rule already says they look
back. No event records a change of control (CR 603.10d).

A static ability that stops abilities from triggering (CR 603.2: Torpor Orb's "Creatures entering don't cause abilities
to trigger", Doorkeeper Thrull, Hushbringer's "entering or dying") is a `CardDef::trigger_rules` entry, asked by
`collect_batch` of each event and each triggered ability the event matched (`Triggering`: its source, controller and
definition); true drops that triggering before it is put on the stack or counted against "triggers only N times each turn". Which permanents' rules
are asked follows CR 603.10: those on the battlefield after the event for an ability that triggers then (so a rule
entering with the permanent applies), those before it for a look-back ability (so one dying with the creature applies).
The rule judges the event through `Query` - an entering permanent as it is on the battlefield, copy and other effects
applied; a dead one by its last-known characteristics. Replacement effects and "as [this] enters" are not triggers and
are never asked about, and neither is the end of an "until" exile (CR 610.3, a one-shot effect). A trigger whose event is a sacrifice rather than the zone change it makes is
`TriggeredDef::on_sacrifice` (Hushbringer's ruling). The sources are found from the board, once per batch and only once
something has matched.

Objects that move together (`ops::move_together`, and the tokens of one `create_tokens_worded`) are planned before any of
them moves (CR 614.12, 101.4): each one's copy and face (`prepare_entry`, done by the callers first), its own "as enters" choices and its zone-change
and entering-counter replacements and the last-known information of a permanent leaving are all worked out against the
game as it was before the event, then every object moves. So permanents entering together never see one another (a
creature returned with Gev or Containment Priest is untouched by them; Giada counts only the Angels already there), and
permanents leaving together all still see one another (Dauthi Voidwalker dying with an opponent's creature exiles it).
A permanent's own "enters with" and "as this enters" abilities (`CardDef::enters`, its own and those of what it enters
as a copy of) apply only if, as it would exist on the battlefield, it would still have its abilities (CR 614.12, Yixlid
Jailer's example): under Humility or Dress Down a creature enters with none of them (a Walking Ballista with no counters,
a Clone copying nothing). `prepare_entry` asks it by putting the object on the battlefield for the moment of the question,
under the player it enters under and as the copy it enters as, and reverting that; only while a "loses all abilities"
static ability is on the battlefield (`layers::loses_all_static_applies`).

A static ability that makes triggered abilities trigger an additional time (CR 603.2d: Elesh Norn, Mother of Machines,
Panharmonicon, Teysa Karlov, Harmonic Prodigy) is a `CardDef::additional_triggers` entry, the same shape asked of the same
permanents (CR 603.10) after the suppressing rules: a triggering they stop is never doubled. Each entry that says yes adds
one triggering (two Elesh Norns make three), so a "one or more" ability triggered once for a batch triggers once more for
each such ability that applies to any of the batch's events. The triggerings are separate (`collect_batch` pushes the
pending trigger that many times): each is ordered, gets its modes and targets as it goes on the stack, checks its
intervening "if" again as it resolves (CR 603.4), and counts against "triggers only N times each turn" (no numbered rule covers that wording), so a
once-each-turn ability under a doubler still triggers once. `Triggering::permanent` says the ability is a permanent's: a
delayed triggered ability is no ability of the object (CR 603.2d, 603.7) and is never asked about, and an ability of a
card in another zone or of an emblem (CR 114.4) is not "of a permanent". Triggered mana abilities (CR 605.1b) are asked too, each
triggering resolving at once.

A trigger event is asked of the ability's source and controller (`TriggeredDef::fires`, an `Asker`): a matcher that says
"you" is written `when_asked` and reads the controller it is given. For an ability of a permanent that is the permanent's
controller; for a delayed triggered ability it is the delayed ability's own controller, the player who controlled the spell
or ability that created it (CR 603.7d-e), whoever controls its source afterwards and wherever the source has gone. Its
intervening "if" is asked with the same controller as it triggers.

A state trigger (CR 603.8: Mazemind Tome's "when there are four or more page counters on this artifact", Dandân's "when
you control no Islands") listens for no event: `state_triggered` gives it a `TriggeredDef::state` condition, asked of each
permanent and emblem after every event (in `collect_batch`'s `consider`, on the characteristics already found there) and
again each time a player would receive priority (`settle::state_triggers`, for a state that changed with no event), before
each check of the state-based actions, so a state a resolution left and a state-based action ends has still triggered (CR
704.3: Thespian's Stage that became a Dark Depths with no ice counters and lost the legend rule). A change with no event in
the middle of a resolution - an effect beginning (`Game::add_effect`) or ending, an object becoming a copy - asks them too
(`Cx::state_changed`, `settle::state_triggers_now`), so a state true only between two instructions of one resolution
triggers. A resolving spell or ability is a run of instructions (CR 608.2c), each judged once it is done and none half-done (`StateCheck` in `cantrip-rules/src/lib.rs`): an "each" or "all", an animation or "gets +2/+2 and gains trample" is one (CR 611.2c). Where the rules make the step themselves the engine judges between: each draw of several (CR 121.2), each mode of a modal spell (CR 700.2d), each half of a fused split spell; an event judges the board as it happens. Elsewhere only the card's text knows, so a card that changes the game with no event more than once in a resolution says, between each two such changes, either that they are one instruction (`cantrip_api::instruction`, `Game::one_instruction`; engine calls such as `add_effect` are each one already) or that one ends (`Game::end_sentence`). Under `CANTRIP_VERIFY` a resolution that says neither is refused, naming the card (`Cx::unsaid`), so the corpus - which runs every line of every card - finds a card that has not said it; `cantrip-cards/tests/one_instruction_loops.rs` finds a loop of such changes over a set of objects, which a scenario with one object in the set would not. Unsaid, a resolution is judged as one instruction. A staged run (`Game::ask_then`) that stops at a question keeps an unjudged change in its frame (`Frame::unjudged`) and goes on with it when answered; a run redone from its start is judged when it is redone. Only the objects that could have one are asked: a card as it is now
(face or copy) that prints or gives one (`CardTable::state_triggers`, its `delayed` list included), a copy's exception
abilities, an effect's granted ability, and every permanent while a static ability that gives others abilities is on the
board (`CardTable::ability_grants`). It
triggers while the condition holds unless it has already triggered and not left the stack - found this batch, pending, or
on the stack (`state_trigger_waiting`) - so it triggers again once it has resolved or been countered, if the state still
holds; an object that changed zones is a new one with a new ability (CR 400.7).

A delayed triggered ability waits as a `Delayed` record in the state, on no object (CR 603.7): once at the next such step
(`DelayedWhen::Step`), at each such step for the rest of the game (`EachStep`: paradigm, CR 702.192a, which goes on whatever
happens to the card it came from), or on events: each one this turn (`ThisTurn`), until an event (`UntilEvent`), until a
player's next turn (`UntilNextTurnOf`), or the next one this turn (`NextThisTurn`: "when you next cast ... this turn", CR
603.7b, used up by the event it triggers on and ended with the turn if none comes; two made in one turn both trigger on
that event, Doublecast's ruling).

An instruction to transform (CR 701.27) is `ops::transform`, which turns a double-faced permanent over and records
`Event::Transformed`. An activated or triggered ability of a permanent that tries to transform that permanent does so only
if it hasn't transformed since the ability was put onto the stack, and a delayed triggered ability of it only if it
hasn't since that delayed ability was created (CR 701.27f); otherwise the instruction is ignored. The verb asks the
history: each `StackItem` keeps `since`, the history's head when it was put on the stack (as it is cast, as an ability
is activated, before its costs, CR 602.2a, as a triggered ability or a copy goes on the stack), and a `Delayed` keeps its
own, which its pending trigger and stack item carry; the resolving item's source transforming at or after that serial
stops it. An ability of another object and a spell are not stopped, and a permanent that left and came back is a new
object the instruction never finds (CR 400.7). Aang, at the Crossroads's delayed ability (its 2025-10-02 ruling), the
activated transforms of the Incubator token, Ulvenwald Oddity, Aang, Swift Savior, Norman Osborn and Temple of the
Dead, and Dowsing Device's and Cecil, Dark Knight's triggered ones are what it governs. Nothing in content converts (CR 701.28), so no convert event is recorded.

"As this [permanent] transforms (into [this face]), ..." is `CardDef::as_transforms` (`AsTransforms`, CR 712.20: an ability
the permanent would have after it transforms "is applied while that permanent is transforming ..., not afterward"). It is
a static ability of the face turned over to, so `ops::turn_over` - where every transformation happens, an effect's
(`Game::transform`), the permanent's own ability's (`transform_by`) and day and night's - turns the permanent over, then
applies the ability of the face now up unless the permanent has lost all its abilities there (CR 613.1f), with its
controller as "you", and only then records `Event::Transformed`. The ability may ask a question, so the transform verbs,
`Game::become_day`/`become_night` and the day/night procedures return the suspension and the run replays as every
question does. A permanent put onto the battlefield transformed did not transform (CR 712.14a) and never reaches it.
Sephiroth, One-Winged Angel ("you get an emblem"), Shinryu, Transcendent Rival ("choose an opponent"), Curse of Leeches
("attach it to a player") and Olag, Ludevic's Hubris print it; `cantrip-rules/src/as_transforms_tests.rs` proves it over
test cards. The benchmark measured 11.57 and 11.55 us per decision before and 11.15 and 11.26 us after (with two noisy
runs of 11.93 and 13.57 between, on a shared machine, 2026-10-18): the check is one field read per transformation.

Becoming a target is `Event::BecameTarget { target, by, controller }`, emitted by `ops::became_targets` as a spell is cast,
an ability activated or put on the stack, a copy put on the stack (CR 707.10) and targets changed or chosen anew (CR 115.7):
once per distinct object or player, since one chosen for several instances of "target" (CR 115.3) becomes the target of that
spell or ability once (CR 601.2c) - ward, Bonecrusher Giant and Pawpatch Recruit trigger once for it.

A card put onto the battlefield transformed (CR 712.14a) enters with its back face up as one entry, `ops::put_onto_battlefield_as`: the counters the instruction gives it ("with a finality counter on it", CR 122.6, 614.1c) are on its `Entering` record with those the face it enters with gives it (a Saga back face's lore counter, CR 714.3a; a planeswalker's loyalty), so the replacements of counters put on a permanent apply to all of them at once and they are announced as it enters - chapter I triggers, and two lore counters from Innkeeper's Talent trigger I and II. `Game::put_onto_battlefield_transformed_with_counters` is Esper Origins's (and would be Ojer Pakpatiq, Deepest Epoch's, tapped with three time counters). A finality counter is the rules' replacement (CR 122.1h) in `ops::move_together`. The benchmark measured 6.75 and 6.88 us per decision before and 6.91 and 7.64 us after on a busy machine (2026-10-16): the path is only reached by the cards that put a card onto the battlefield transformed.

A crime (CR 700.13) is `Event::CrimeCommitted { player, by }`, emitted by `ops::commit_crime` from the three procedures that
choose targets for a new object - casting a spell (CR 601.2i), activating an ability (CR 602.2) and putting a triggered
ability on the stack (CR 603.3d), the trigger's controller the criminal - after their `BecameTarget` events, once for the
object however many of its targets are an opponent, a permanent, spell or ability an opponent controls, or a card in an
opponent's graveyard. A copy (CR 707.10) and a changed target (CR 115.7) choose no targets that way and commit none. It costs
nothing where nothing targets an opponent's things. The scenario dump lists a player's crimes as `BotN.Crimes`.

### Life: gaining it, and damage that can't be prevented

Damage originally proposed to a player who has left the game does not happen (CR 102.1, 120.1, 800.4a).
`ops::deal_damage_with` rejects that recipient before replacement/prevention; shield ordering leaves them out.
An invalid redirection is different: the redirection effect does nothing, leaving the immediately prior proposal
for other effects (CR 614.9). `replace::valid_redirection` checks changed recipients before offering/applying an effect;
absent/non-damageable permanents or departed players cannot be its origin/destination. No side effect or once-only
resource is consumed by an invalid redirection. Earlier replacement effects retain their one opportunity (CR 614.5).
A live player at zero life still receives simultaneous damage before state-based actions. Life changes,
including lifelink from a departed source-controller's independent ability, likewise affect only players still in the
game. The original simultaneous batch and each source's live damage totals are preserved.

Every gain and loss of life goes through `ops::change_life` (CR 119.3): an effect's, a cost's (CR 119.4), damage (CR
120.3a), lifelink (CR 702.15b), and a life total set to a number (`Game::set_life_total`, CR 119.5: the player gains or loses
the difference). A gain does not happen while the player can't gain life (CR 119.7, `static_rules::cant_gain_life`): a
resolved effect about the player says so (`EffectKind::CantGainLife`, Screaming Nemesis's "for the rest of the game"), or a
static ability of a permanent on the battlefield does (`CardDef::cant_gain_life`, a `LifeGainRule` asked of the player:
`cantrip_mechanics::statics::players_cant_gain_life` for Sunspine Lynx, Giant Cindermaw, Rampaging Ferocidon and Leyline of
Punishment, `opponents_cant_gain_life` for Knight of Dusk's Shadow). The battlefield is searched only when a player would gain
life, and a permanent that has lost all its abilities has none of these. No life gain event is recorded for a gain that did
not happen, so "whenever you gain life" does not trigger (CR 119.9); paying life is a loss and is still allowed. The host shows
"You can't gain life." / "They can't gain life." beside a player's total while it applies (`Game::cant_gain_life`).

A loss does not happen while the player can't lose life (CR 119.8, `static_rules::cant_lose_life`: a static ability's
`CardDef::cant_lose_life`, a `LifeLossRule`; Platinum Emperion's "your life total can't change" is both rules). Damage is still
dealt - lifelink still gains, "whenever damage is dealt to you" still triggers - and only the life it would take is not lost (the
Emperion rulings). `change_life` asks both rules before the replacement effects and again of what they leave, so an effect that
would turn the event into a gain or a loss the player can't have leaves nothing. Such a player can't pay life (CR 119.8, 119.4b:
`actions::payable_life` is 0 for them): a cost with life in it is not offered, Phyrexian symbols are paid with mana, and X paid
in life is announced at 0. The host shows "You can't lose life." or, with both, "Your life total can't change." beside the total.

Draws (CR 121) are judged one card at a time (`ops::draw`): first whether the player can't draw it (`cantrip_api::cant_draw`,
a permanent's `CardDef::cant_draw` rule: "can't draw more than one card each turn", CR 121.2b) - then nothing happens and no
replacement effect replaces it (CR 614.17c) - then the `Proposal::Draw` replacements (CR 121.6), ordered by the drawing player
(`ChooseReplacement`'s `draw`, worded "You would draw a card"). A replaced draw is gone and nothing else replaces it (CR 614.6).
What an effect does instead is its `then`, done as it applies (CR 121.6b); the draws and the rest it makes are the modified
event, and meet only the effects that have not yet had their one opportunity at the event (CR 614.5): `Cx::lineage` carries the
effects applied so far into them, so two "draw two cards instead" make four, and a Notion Thief on each side returns the card
to its drawer. A "you may ... instead" (`ReplacementDef::optional`, dredge, Abundance) asks its question as it would apply, and
declined leaves the event to the others. Dredge functions in its owner's graveyard (`ReplacementDef::in_graveyard`), the one
kind of replacement a card there has. A resolved "the next time you would draw a card" is a granted replacement that is used
up as it applies (`GrantedReplacementDef::once`). A question a `then` asks (which card to discard) suspends a run that cannot
stop there; the run is wound back to where the question was asked (`Cx::asked_at`), so the question names objects that exist.

Replacement effects of a gain or loss of life (CR 614.1a: "you gain that much life plus 1 instead", "they lose twice that much
life instead", "that player loses that much life instead") are `ReplacementKind::LifeChange` effects over a
`Proposal::LifeChange { player, amount }`, `amount` signed (a gain above zero, a loss below), applied by `change_life` through
`replace::life_change` after the can't-gain check (a replacement of a gain of a player who can't gain life does nothing, CR
119.7) and ordered by the player whose life it is (CR 616.1, `RequestKind::ChooseReplacement`'s `life`, which the host words as
"You would gain 3 life"). Each is judged again on the event as the one before left it (CR 616.1f), so a gain turned into a loss
meets the effects on losing life; an amount left at zero is no event (CR 119.10). The `LifeChange` event records the life that
actually changed, so the history's `life_gained`/`life_lost` count it. Damage is replaced as damage before it is dealt; the life
lost to it is a separate event (Bloodletter of Aclazotz doubles the loss, not the damage nor its lifelink). Lifelink is one gain
per source of a simultaneous damage event, however many recipients (`ops::DamageBatch`, CR 120.3f and Angel of Vitality's
ruling). The shared effects are in `cantrip_mechanics::replacements`. A cost's payability (CR 119.4) is judged on the payment,
not on what a replacement makes of the loss. A cost paid as an effect resolves - "you may pay {W}{B} and 2 life. When you
do", "counter it unless its controller pays {3}", a shockland's "you may pay 2 life" (CR 118.12, 608.2d) - is one payment,
`Game::may_pay` (`actions::may_pay`, asked as `RequestKind::UnlessPays` with its `life` and the card's `Question`): the
payer is not asked without the life (`actions::can_pay_life`), may activate mana abilities while there is mana to pay (CR
605.3a), and may pay once the pool covers the mana and the life total still covers the life, so a mana ability that costs
life (Mana Confluence) can leave only declining. The life a cost pays is a `u32` in `Cost` and in the request, so a
variable amount ("pay that much life", Niv-Mizzet, Ghost Counsel) is kept whole however large, and it is compared with the
life total at full width: more life than the payer has is never payable, and nothing narrows it on the way. A cost with other parts - "sacrifice a creature", "discard a card",
"return a tapped creature you control", energy, counters - is paid the same way: paying is offered only while every part
can be paid now (`resolution_rest_payable`, judged as an activated ability's is), the permanents and cards are chosen once
the payer chooses to pay, after the mana abilities (CR 601.2g-h), with the questions a spell's or an ability's cost asks,
and a choice given up is undone and the payer asked again. The parts only a spell or an activation has (loyalty, emerge,
craft's materials, a granted ability's "sacrifice the permanent that gives it") are never paid. The whole is paid together,
and "if they do" asks whether they paid (CR 118.12). "You may pay {X}" where the text leaves X to the payer (CR 107.3f) is `Game::may_pay_x`
(`actions::may_pay_x`): the same question with its `x`, raised and lowered (`IncreaseX`, `DecreaseX`) while mana is made, any
X from 0 up, and the X paid is what it returns, for "when you do" to carry (Fire Lord Sozin). "You may pay {G} up to three
times" and "any number of times" are `Game::may_pay_times` (`actions::may_pay_times`): the same question with its `times`, the
number raised and lowered from 1 to the most (`ANY_NUMBER_OF_TIMES`: what the mana cost's shape holds, `ManaCost::times`) and
the mana that many times over paid at once, its number returned. "When you pay this cost one or more times" triggers once
for any number above 0 (CR 603.12a), given that number (`reflexive_trigger_counted`), and a modal reflexive ability's "choose up
to that many" is `ModalDef::up_to_that_many` (`max` `ModalDef::COUNTED`), its most modes read from that count as it is put on
the stack (`settle::put_trigger`, CR 603.3c, 700.2b) - Tranquil Frillback, Hawkeye, Master Marksman. Not built, because no card in content has one: exchanges of life totals, costs
that gain life, and an effect a spell or ability makes that a player can't lose life (CR 119.8: "can't lose life this turn").

"Damage can't be prevented" (CR 615.12) is asked by `ops::damage_can_be_prevented` before any prevention: a turn an effect said
so (Bonecrusher Giant's Stomp) or a `CardDef::unpreventable_damage` static (Questing Beast's combat damage,
`statics::damage_cant_be_prevented` for Sunspine Lynx and Leyline of Punishment). Every prevention effect asks it.

### Excess damage

`Game::damage_result` returns damage's actual recipient, amount and excess after replacement and prevention
(CR 120.10). `damage` retains its amount-only result. `ops::DamageBatch` keeps each recipient's pre-damage
creature toughness and marked damage, loyalty and defense alongside its existing marked-damage accounting.
Excess uses the greatest amount among its creature, planeswalker and battle types; damage beyond 1 dealt to a creature by a
source with deathtouch, combat or noncombat, is excess (CR 120.4a). The batch finishes this calculation over all its sources before damage triggers are collected,
including combat damage. All simultaneous recipients are read before any damage is dealt; when replacement
effects are active on the current battlefield, all battlefield objects are read to cover redirected recipients. Each `Event::Damage::recipient_excess` repeats the recipient's total for that simultaneous
event; it is never summed across source records or attributed to one source. Violent Echoes uses the direct result;
Toralf and Aegar can ask the event history without reconstructing pre-damage characteristics from later state.

### Preventing damage

Prevention effects (CR 615) are applied by `replace::damage` with the replacement effects of the same damage, and the affected
player orders them together (CR 616.1, `RequestKind::ChooseReplacement`), each judged again on the damage as the one before
left it (CR 616.1f). There are three kinds besides protection's (CR 702.16e, `replace::Origin::Protection`):

- a static ability's, `CardDef::preventions` (`PreventionDef`: a hint and an `applies` over the `DamageEvent`), functioning
  while its permanent is on the battlefield and gone with its abilities (CR 613.1f): Black Panther, Hope Enduring's "Prevent
  all damage that would be dealt to Black Panther". It prevents all of what it applies to, or as much as its `amount` says
  of the damage as it now is (Gisela, Blade of Goldnight's "prevent half that damage, rounded up"), which is then ordered with
  the others like a shield;
- one a resolving spell or ability made (CR 615.3, 611.2), `EffectKind::Prevention { card, index, to, shield }`: of damage to an
  object (which it ends with, CR 400.7), to a player, or to anyone (`Target::None`), and of the damage the making card's
  `granted_preventions[index]` describes (any damage, combat damage: `cantrip_mechanics::prevention::ANY_DAMAGE`,
  `COMBAT_DAMAGE`). `Shield::All` prevents all of it while it lasts (Fog's "all combat damage that would be dealt this
  turn"); `Shield::Next(n)` is a shield (CR 615.7), reduced by what it prevents (`GameState::effect_set`) and gone at 0
  (Healing Salve's "the next 3 damage that would be dealt to any target this turn"). The definition's `applies` is handed the
  object the effect is about, so one about no recipient can name the damage by its object: Maze of Ith's "all combat damage
  that would be dealt to and dealt by that creature" (`to: Target::None`, the creature the affected object, `of_it` how the
  creature's face words it);
- where two or more sources would deal damage to a shielded permanent or player at the same time (combat damage,
  `damage_together`), its player or controller chooses whose damage the shield prevents first (CR 615.7,
  `ops::order_for_shields`, a `ChooseCards` of the sources with `Purpose::PreventDamageFrom(left)`), asked whenever the
  shield would not cover all of it as assigned or a damage replacement on the board might make it more (Valley Flamecaller);
  the assignments then meet the shield in that order.

None applies to damage that can't be prevented (CR 615.12): it would prevent nothing, and a shield is not reduced by it. A
player is not asked to order prevention effects that each prevent all the damage and use nothing up, since the order
changes nothing. Each prevention that prevents something records `Event::DamagePrevented { source, target, amount, combat,
by }` (CR 615.6, 615.13), protection's too, before what is left is dealt: what "if damage is prevented this way" (CR 615.5) would ask,
and the scenario dump's `Prevented`. It is one record for each damage event an effect prevents some of, emitted as it is
applied rather than with the damage batch, so a "whenever damage is prevented" trigger (CR 615.13: once each time an effect
is applied to one or more simultaneous damage events) would need them batched first, and an additional effect "if damage is
prevented this way" would make the order of prevention effects that each prevent it all matter, so the player would have to
be asked again. Not built, because no card in content has one:
"the next time a source would deal damage" (CR 615.8), a source of a player's choice (CR 615.9, 609.7), "prevent N of that
damage" from a static ability (CR 615.10) and prevention with an additional effect (CR 615.5). The host shows a resolved
prevention effect on the object it shields ("Prevent the next 3 damage that would be dealt to it this turn (Healing Salve)."),
or beside the life total of the player it shields, or of its controller for damage to anyone.

### Damage redirected, and damage that stays

A replacement effect may change whom damage is dealt to as well as how much (CR 614.1a: "all damage that would be dealt to
you and other permanents you control is dealt to this creature instead"): `DamageEvent::target` is the recipient, and
`ops::deal_damage_with` deals what the effects leave to whomever they leave it with, by the same source and still combat
damage if it was (the rulings). Everything about the recipient is judged there: protection, infect's counters or poison, toxic's poison (combat damage to a player alone, CR 702.164c),
lifelink, deathtouch, a planeswalker's loyalty, a battle's defense, the monarch's combat-damage steal and the `Damage`
event "deals combat damage to a player" triggers read. Once one effect has applied, the others are judged again against the
changed event and its recipient's controller orders them (CR 614.6, 616.1f), each applying once (CR 614.5), so with two such
creatures the damage passes from the one whose effect applied first to the other: the player chooses which takes it, as the
rulings say. Protection's prevention is ordered with them (CR 616.1, `replace::Origin::Protection`): Pyroclasm's damage to a
protected creature is either prevented or redirected, as its controller chooses, and protection of the new recipient
prevents what reaches it. The question carries the damage as the effects have left it (`RequestKind::ChooseReplacement::
damage`), which the host words into the prompt's `Event` ("3 damage from Lightning Bolt to you"). Ancient Adamantoise has
the redirection (`cantrip_mechanics::replacements::DAMAGE_TO_YOU_AND_YOUR_OTHER_PERMANENTS_IS_DEALT_TO_THIS`, which Palisade
Giant prints too).

A permission to play a card may say what becomes of a land played by it (CR 305.1, 614.1c: Lightstall Inquisitor's "each land
played this way enters tapped", `PlayGrant::enters_tapped`). That is a replacement effect of the permission, the core's own
(`replace::Origin::PlayedTapped`), ordered with the others that would change the same entry by the land's controller (CR
616.1): Archon of Emeria's "nonbasic lands your opponents control enter tapped" is a second, and once either has applied the
other has nothing left to change (CR 616.1f). Which permission a land play is made by is the one its action names
(`ActionOption::granted_by`, "Actions" below): the play keeps that permission alone as the land moves, so the replacement
finds it, and none for a play from the hand or by a rule for a graveyard or a library's top.
A permission may stand under a condition (`PlayCondition`, CR 611.2): "during any turn you attacked with a Rogue" (Robber of the
Rich), "if you control a Wizard" (`ControlsSubtype`, Flameshape), asked whenever the card would be played - a land play by the
permission as much as a cast - and so are its other terms (`actions::permission_stands`: a warp card's later turn).
A permission may also be for the card's Adventure alone (`PlayGrant::adventure_only`, CR 715.3: Mosswood Dreadknight's "cast it
from your graveyard as an Adventure"): neither the card itself nor another face of it is cast by it.
A spell cast by a permission may be paid for with life equal to its mana value rather than its mana cost
(`PlayGrant::life_for_mana_value`, CR 118.9, 119.4: Inside Information), as Bolas's Citadel's rule pays for the top of a library
(`actions::life_for_mana_value`, the face cast, X as 0); no other alternative cost goes with it (CR 118.9a).

The cleanup step removes all marked damage (CR 514.2) except from a permanent a static ability keeps it on:
`CardDef::damage_stays` (`DamageStaysRule`, asked of each damaged permanent by the permanents on the battlefield with their
abilities, only in a cleanup step with damage marked). Ancient Adamantoise's "damage isn't removed from this creature during
cleanup steps"; Patient Zero's "from creatures your opponents control" and Case of the Market Melee's "from creatures" are the
same rule. Effects that remove all damage still do (regeneration, CR 701.19a), and a permanent that leaves and comes back is a
new object with none (CR 400.7). Having been dealt deathtouch damage is cleared either way: it matters only to the next check
of state-based actions (CR 704.5h).

### Miracle, printed and given

Miracle (CR 702.94a) is the core's: as a card is drawn, if it is its player's first draw of the turn, `ops::draw` asks
`cantrip_api::miracles` what miracle abilities it has - its own `CardDef::miracle`, then each one a permanent on the
battlefield gives it (`CardDef::granted_miracles`, asked of the permanent with its controller as "you", and not while the
permanent has lost its abilities). With one, the player is asked whether to reveal it; with several, whether and then which
(`Label::Miracle`), since each miracle ability is linked only to its own triggered ability (CR 607.5). The linked ability
(`MIRACLE_ABILITY`) carries which one it was in its data, so the cost it offers is that one's even once the permanent that gave
it has left (CR 113.7a), and a granted cost is read for the face being cast (a Room's door, CR 709.3a). A grant's
`less` ("its mana cost reduced by {4}") is part of what the cost is (`reduced_by`): it takes the cost's generic mana, never
colored, before anything increases it (CR 601.2f), and what is left of it comes off what an {X} adds once X is announced
(`CastTerms::less`, CR 118.7a); the spell's mana value is unchanged. The battlefield is
searched on every draw a player makes after the first turn begins, before the history is asked whether it is the first.

### Ending the turn and the cleanup step

"End the turn" (CR 724.1) is `Game::end_the_turn`, called by a resolving spell or ability (`turn::end_the_turn`): the
waiting triggered abilities cease to exist (724.1a), every object on the stack is exiled at once, the resolving one
included, whose later instructions still run (724.1b), and state-based actions are checked alone (724.1c,
`settle::state_based_actions_alone`). It records `Event::TurnEnded` and sets `Turn::ending`; the priority round below the
resolution then ends without asking anyone, and `next_step` removes every creature from combat if the turn was in
combat and goes straight to a cleanup step of the same turn - a new one if it was already in one (724.1d). The end step
is never begun, so nothing "at the beginning of the end step" triggers (724.1e). Abilities that trigger during the
process wait for that cleanup step (724.1f).

The cleanup step (CR 514) gives priority only when state-based actions were performed or a triggered ability is waiting
once its turn-based actions are done (514.3a), and then marks `Turn::cleanup_again`, so another cleanup step follows when
everyone has passed. Ultima, Time Stop, Sundial of the Infinite, Glorious End, Day's Undoing, Hurkyl's Final
Meditation, Discontinuity and Obeka, Brute Chronologist end the turn.

"At the beginning of the next cleanup step" is a delayed triggered ability like any other (`DelayedWhen::Step` with
`Step::Cleanup`): it triggers as the step begins, waits through the discard and the damage wearing off (514.1-514.2), and
is one of the triggered abilities 514.3a puts on the stack. Necromancy's cycle ("if you cast it any time a sorcery couldn't
have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step") is
`CardDef::untimely_cleanup_rider`: a static ability that functions on the stack, so `resolve` creates the delayed ability
(the rules' own, `intrinsic::UNTIMELY_SACRIFICE`, so no card names it by an index of its own) as the spell becomes a
permanent (CR 608.3g), about that permanent, when the spell was cast at such a time by its own "you may cast this spell as
though it had flash" (`Marks::untimely_by_own_flash`; Necromancy's 2022-12-08 ruling). Where another permission would let
it be cast then too - an effect's or a permanent's (Valley Floodcaller), an alternative cost's (Primal Prayers) - its
caster chooses which one they cast it by (CR 601.3): `actions::cast` judges the other permissions on the spell as it is on
the stack under its caster (CR 601.2a, 601.3b: an opponent's Necromancy cast from exile under Encroaching Mycosynth is an
artifact spell there, so Raff Capashen's permission applies) and asks it then, a
`ChooseModes` of `Label::CastPermission`s, its own first, since the sacrifice may be what they want. A spell an effect casts
as it resolves is cast by that effect's permission and is never marked.

### Extra turns and "that turn"

An extra turn (CR 500.7) is a `cantrip_state::ExtraTurn` in `GameState::extra_turns`, the most recently created last and taken
first: whose it is and a number that names it (`Game::add_extra_turn`). The turn being taken carries that number in
`Turn::extra` (0 for a turn of the ordinary rotation), which is what "that turn" means to an ability that made it:
`DelayedWhen::ExtraTurnStep { step, turn }` triggers at that step of that turn alone, and is dropped as the next turn begins
once that turn has ended without the step (an effect ended it, CR 724.1e) or will never be taken (skipped, CR 500.11). Final
Fortune and Last Chance's "at the beginning of that turn's end step, you lose the game" is
`cantrip_mechanics::delayed::extra_turn_then_lose`.

### Controlling another player

"You control target player during that player's next turn" (CR 723.1: Mindslaver, Sorin Markov, Worst Fears) is
`Game::control_next_turn`, a player effect `EffectKind::ControlNextTurn` that waits on the player for the next turn they
actually take: a skipped turn leaves it waiting (723.1b). As that turn begins (`turn::take_controlled_turn`, after the
skip is judged) every such effect on the player is used up, each whose controller is still in the game becoming an
`EffectKind::ControlsPlayer` with the timestamp it was created with; `GameState::player_controller` takes the newest
whose controller is in the game (723.1a; one on a player themselves gives no one else control, 723.9), so when that
controller leaves during the turn (CR 800.4a ends only theirs) the one it overwrote works on. The control lasts through
the turn's cleanup and any turn skipped after it - a skip is decided as a turn would begin, which a skipped turn never
does (CR 614.10, 500.11), so the controller answers it - and ends as the next turn actually begins (723.1). Emrakul, the Promised End's "after that turn, that player takes an extra turn" is
`Game::control_next_turn_then_extra_turn`: each such effect gives its extra turn as the controlled turn begins, the newest
extra turn being taken first (CR 500.7), so it comes right after that turn however long the turn waited, and an
overwritten effect gives its turn too (its rulings).

The engine changes nothing else: every request is still asked of the controlled player, who stays the active player and
controls their objects (723.3), and pays with their own resources (723.5a). Who answers is
`GameState::player_controller(player)` (723.5, 723.8), which the host routes by (`visibility::answering`: the request goes
to that seat with `ForSeat`, in that seat's handles, docs/host-protocol.md) and the scenario harness drives by (a script's
lines still answer as the player asked). What a seat sees follows `GameState::sees_as` (723.4): the controller looks into
the controlled player's hand, knows the faces they know, sees the top of their library when they may, and is shown what
their questions name - never the cards they own outside the game, which they therefore can't be made to choose from while
controlled (`Query::outside_the_game` is empty then). Conceding stays the seat's own (723.6). Word of Command and
Opposition Agent (723.2) control a player for other durations and are not built; they would be a `ControlsPlayer` of their
own duration.

### Phasing

A permanent's phased-in or phased-out status (CR 110.5, 702.26) is `Marks::phased`, a `Phased` saying how it phased out: under
whose control, directly or indirectly with what it is attached to (702.26g), and "until [a permanent] leaves the
battlefield" (610.4), tapped as it returns or not. A phased-out permanent is treated as though it does not exist (702.26b), so
it is kept off the battlefield's list - which every rule that looks at the battlefield reads - in `Zone::PHASED_OUT`
(`ZoneKind::PhasedOut`, "not a zone", as `Gone` is), as the same object (702.26d): no zone change, no new object, no
"leaves" or "enters" trigger, counters, damage, status, attachments and the effects on it kept, a token still in existence
(704.5d leaves it alone), and its characteristics worked out as on the battlefield for what it shows. `phasing.rs` is the
whole of it: `phase_out` (attachments phase out indirectly, recursively, and everything is removed from combat, 506.4;
"until" a permanent that has already left phases nothing out, 610.4b), the untap step's first turn-based action (502.1:
what phased out directly under the active player phases in, with what phased out indirectly with it, before anything
untaps; not one phased out "until", 610.4a), and `phase_in_returning`, the one-shot effect that phases a permanent in once
the permanent it waited for has left (610.4), carried out with the returns of exiled cards the moment it leaves (below). An
attachment that phased out directly phases in unattached if what it was attached to is gone (702.26i), for the state-based
actions. "Until [this] leaves the battlefield" goes on while its source is phased out (`phasing::still_on_battlefield`:
Banishing Light's ruling), "for as long as" durations end (702.26f: `WhileSourceOnBattlefield` asks the battlefield), and a
resolved effect never includes a phased-out permanent (702.26e, `add_effect`). Cleanup removes damage from phased-out
permanents too (514.2). Events: `PhasedOut`, `PhasedIn` (recorded; nothing triggers on them). The phasing keyword itself
(702.26a's phasing out in the untap step) has no card here. Teferi's Protection (`Game::phase_out`) and Oubliette
(`Game::phase_out_until_leaves`) use it; the host shows a phased-out permanent with the battlefield, marked
`ObjectView::phased_out`, and the script dump lists them as `BotN.PhasedOut`.

Counters and phasing (CR 702.26b-d): `counters::put_together` excludes phased-out recipients before replacements and rechecks
presence after their continuations; removal rejects phased-out recipients too. Existing counters remain through phasing;
normal placements on cards in other zones still work. Identity liveness is unchanged, so phasing does not become a zone
change or erase a source's history. A failed counter placement never stops the rest of a card's effect.

"Your life total can't change" (CR 119.7, 119.8) is `EffectKind::LifeTotalCantChange`, a player effect `ops::change_life`
asks before any replacement (nothing replaces what can't happen) and `actions::can_pay_life` asks of a cost of more than 0
life. Damage is still dealt; only its loss of life doesn't happen.

### Exiled or phased out until a permanent leaves

"Exile [it] until [this] leaves the battlefield" (CR 610.3) and "phases out until [this] leaves the battlefield" (CR 610.4)
are one-shot effects that each create a second one: it "is created immediately after the specified event" and gives the
card back - to the zone it came from, a permanent under its owner's control (610.3c) - or phases the permanent in. It is
neither a triggered ability nor a state-based action, so it waits for no player to receive priority and for no check: it
happens in the middle of whatever made the permanent leave - a cost being paid (Yasharn back on the battlefield as the White
Auracite holding it is sacrificed to Krark-Clan Ironworks, so a sacrifice later in the same payment can't be made, 601.2h),
a resolution (the card back before the spell's next instruction), the state-based actions (back before the next check,
704.3: a check is one event, so its returns wait until all of it - the losers' leaving the game included - is done, and
are then made together, 610.3d). The link is the exiled card's `ExileLink` (`until_leaves`) or the phased-out permanent's `Phased::until_leaves`;
`phasing::still_on_battlefield` says the permanent has not left (phasing out is no leaving, 702.26d).

"Exiled with [this]" is per ability, not per object (CR 607.1, 607.2a, 607.5): an `ExileLink` names the object and the card
whose text the exiling ability is printed in (`Linked`: `Query::linked` for a resolving ability, `Query::linked_to` for one of
the text the object has now; either face of a transforming card and either door of a Room are one text, 607.1b,
`CardTable::linked_card`), and how the object acquired the ability (`Acquired`: by its own text, by a copy effect applied
to it later - the timestamp of its `EffectKind::RestoreCopy`, `GameState::copy_acquired` - or by the effect that granted it,
which `layers::ability_givers` names), and `Query::exiled_with` answers only an ability of that same text acquired the same
way. A stack item, a pending trigger and a delayed trigger carry how their ability was acquired (worked out as it is
activated or triggers, `layers::acquired`, with nothing to work out unless an effect could have given it; a look-back
trigger as it was then, `Then`). So a creature that copies another (Mirrorweave) does not see, through the copied text's
linked ability, the cards its own text exiled, nor a creature the cards an ability lent to it exiled (Agatha's Soul
Cauldron). A replacement
effect that exiles links what it exiles the same way when its card has an ability that refers to them (607.2b:
`ZoneChange::exiled_with`, set by `replacements::exile_instead_linked_apply`); only its direct result is linked, so an effect
that applies afterwards and sends the card elsewhere leaves it unlinked (`replace::unlink_if_sent_elsewhere`).

`ops::return_until_left` carries both out (`ops::returns_due`: `return_exiled`, `phasing::phase_in_returning`), and is
called by whatever can make a permanent leave: `move_together`, after the move's events and their triggers (so "leaves"
abilities and the return's "enters" ones are collected in that order, and the history has the return after the leaving),
`leave_game`, whose leaving player's permanents cease to exist (800.4a), and `settle::state_based_actions` once its check
is done, which holds the others' calls for the length of the check (`GameState::returning`). The returns of one event are
one move (610.3d, 610.4d), and it goes round until nothing is due, since a return can make another such permanent leave;
the moves it makes don't call it again. A return can ask a question - what a returning Aura enchants (303.4f), an "as enters" choice - and
the move it follows returns no suspension. The question waits on the frame and the run goes on, as for a replacement
effect's question asked mid-move (`Cx::ask_core`); the run is then wound back to the position the question was asked in
(`Cx::asked_at`) whether it ends or stops at a later question, so the question is judged and answered there, and redone
from the frame's start with the answer. Outside a run (a rules test's bare `Cx`, a concession between runs - a player
conceding while their own announcement waits) nothing could be asked, so it does nothing; settling (`settle::settle`,
`state_based_actions_alone`) carries out any return still due first, before the state triggers and the state-based actions
look at the board, and finds nothing due when the moves have done it. Nothing runs between: a change sets
`GameState::unsettled`, and priority settles before it is given.

### Winning, losing and drawing

A player loses to the state-based actions of CR 704.5a-c through `ops::loses_to_state_based_actions`, which leaves out each
way a static ability says the player doesn't lose (`CardDef::loss_rules`: Platinum Angel's "you can't lose the game",
Phyrexian Unlife's "you don't lose the game for having 0 or less life"); a draw from an empty library is forgotten with the
check that saw it (704.5b: "since the last time state-based actions were checked"). An effect that says a player loses
(`lose_game`, 104.3e) is stopped the same way, and one that says a player wins (`win_game`, 104.2b) does nothing for a player
who can't win (`CardDef::win_rules`) or who has an opponent who can't lose. A concession is the host's and is never stopped
(104.3a). Every loss records `Event::GameLost` with its `LossReason`, and a draw `Event::GameDrawn` with its `DrawReason`,
which the host tells each seat (`PlayerLostEvent`, `GameEndedEvent.DrawReason`).

Loops are `loops.rs`'s (CR 104.4b, 732; `docs/rules-exceptions.md`, RX-4), watched in `GameState::loop_watch` and
`GameState::loop_keys`, journalled like any other write. The engine finds one kind of loop only: a run of passes that
comes back to the same exact position. Any choice ends a run - an action at priority, or an answer to any other question
with more than one legal answer (a "may" declined is one, RX-2, and so is every question a player who left was asked and
the choice of who makes it, CR 800.4g); a question with one legal answer is none. Every priority given with something on
the stack in a run but its first two is fingerprinted up to the identity of objects (objects numbered by where they are,
references to objects that left followed one move to what they became - no rule follows `became` further - and named
past that by what they still say, timestamps by their order), with every amount in it (life, poison, counters, damage,
mana, libraries). A fingerprint seen again is the loop come round once more, but no round rests on a hash: the first
time a position comes round, it is written out in full (`loops::words`, every word the fingerprint is made of) and kept;
each time after, a round is counted only if its words are the same. When one has come round `LOOP_ROUNDS` (three) times
so confirmed, the game is a draw (`DrawReason::MandatoryLoop`). The rounds are let go by because a player holding an
answer outside the loop (a Disenchant in hand) may still choose to end it, and no player can be made to (CR 732.5). The
event history is not in the fingerprint; every read of it is counted instead (`cantrip_api::history::note_read`, made by
`History::new`), and a read clears the positions seen, so a loop that "the third time this turn" or "only once each
turn" could end is never found by its positions (`loops::settled_read` asks a question whose answer the rest of the turn
can't change without counting it; what working out a priority's options reads is not counted, as it changes nothing
while every player passes; a layer pass kept on the state that was worked out asking the past notes a read each time it
is used again, as its characteristics depend on the history). Every other loop - a fragmented one (CR 732.3 is not
applied), one player's, one whose players decline what its objects offer - is played by hand and never cut off, capped
or made to change: the engine supports no loop shortcuts (CR 732.2), players perform every loop action by hand, and a
combination may go round as many times as it needs; a person is held to the match clock (RX-4's draw offers come with
the server's support for them), and a seat nobody drives stops itself (`LoopGuard`, C#: past 4096 questions in one step
it passes, declines and picks each answer in turn). A game whose history reaches `MAX_GAME_EVENTS` (2,097,152 events) is
a draw, checked between two frames (`Game::run`; RX-1): a run of play that never repeats exactly is neither a loop the
rules can find nor ever ended by them, a loop of mandatory actions that never gives anyone priority is drawn there, and
an automated seat whose passive answers cannot end a run reaches that draw rather than conceding. The cap bounds every
count of things that happen - objects, effects, grants, rounds, questions, the history, arena entries - at 2^21 between
frames, where it is checked (within one frame, only memory bounds them), so such a count is a `u32` (a length made one
by `count_u32`, which saturates rather than wrap, and sent as the host's `int` by `wire`), and one kept narrower is a
bug; a game deals at most `MAX_SEATS` (eight) seats, which a set of seats holds a bit each. It bounds no amount:
counters, damage, mana, life, power and toughness can double in one event. The watch keeps each position of a run once
(found through an index keyed by the fingerprint itself, journalled an entry at a time), and the words of those that
have come round, so a run of any length costs a fingerprint and a lookup per priority, and a position seen again its
words besides; nothing bounds what it keeps but `MAX_GAME_EVENTS`. The fingerprint is a folded-multiply hash, and an
object's row hash is kept from one priority to the next while the row is the same (`loops::row`). A normal game pays a
fingerprint per priority with something on the stack but the first two of each run, and a count per read of the history.

### Casting: when a cost's choices are made

A spell's costs are chosen and paid in the order of CR 601.2b-h, as the C# engine asked them: the permanents a cost
sacrifices or returns, its discards and exiles are chosen before the mana is paid, an additional cost's choices after
it. A cost of tapped permanents (`PermanentCost::tapped`, web-slinging's "return a tapped creature you control", CR
702.188a) is chosen once the mana is paid, since mana abilities come first (CR 601.2g) and one of them may have tapped
the creature (Spider-Sense's ruling): it is offered while a creature is tapped or a mana ability its controller could
activate would tap one, its own {T} or Springleaf Drum's, and a payment that leaves none tapped can't be completed, so
the cast is taken back (CR 601.2h). An activated ability's is the same (CR 602.2b: Urban Retreat's "{2}, Return a
tapped creature you control to its owner's hand", activated from the hand), offered by the same test and chosen once
`activate` has paid the mana; only a mana ability, whose permanents come with its activation, can't have one (none
printed does), refused by a card-table test (`every_activated_ability_has_a_cost_activation_can_pay`). The host marks
such a cast or activation `TappedAfterMana`, and the web client's payment caption for it says to tap a creature for
mana while none is tapped; one that leaves none is still taken back after it is paid.

### Costs that depend on the targets

The total cost of a spell or ability is determined once its modes, X and targets are announced (CR 601.2b-c, 601.2f,
602.2b), so every cost change (`CostChange`) is handed the targets announced, in slot order, and the total is locked in
there: what happens to a target later changes nothing paid. Casting and activating apply the changes after the targets
are chosen (`actions::cast`, `activate`), and the payment asks for that total, which is what the client's payment shows.
Whether an action is worth offering is judged before anything is announced: where its one target is announced with the
action (equip, most single-target spells) and something could change the cost - a cost changer on the battlefield, or
the spell's or ability's own - it is judged for each candidate, and only the candidates it could be paid for are
offered, with what is announced after it that a change may read or that brings targets of its own (a kicker, an optional
cost, cards spliced onto it). Any other - several slots or a slot of several targets, a modal spell, targets after X or a
kicker, a gift, a division, a splice - is offered if some legal announcement could be paid for (`actions::some_cast`,
`some_activation` for abilities): what comes before the targets is walked (each choice of modes, as many as the board allows (`modes_up_to`), X up to what changes the
targets or what an additional cost that pays X life or sacrifices X permanents allows, kicked or not, the optional cost
paid or not, the gift promised to each opponent or not, each order of the
cards with targets that could be spliced on), the card judged as it would then be (`Announcing`: a filter or a change
that reads its X or kicker sees them), then the targets each brings, as `choose_slots` would take them
(`some_announcement`), each slot's targets different; the payment is judged once for each different thing the cost
changes make of them (`deltas`), at the total that announcement comes to (`Earlier`: the modes' costs, X, the kicker,
the optional cost, the splices). So This Town Ain't Big Enough ("{3} less if it targets a permanent you control", up to
two targets) is offered for {1}{U} with a permanent of its caster's to aim at. The last slot's candidates that no change
is given (its source, what is costed) are taken as alike, and judged by how many of them are chosen, for as long as the
changes judged read nothing that could tell them apart (`Watch`: a candidate's characteristics, an object's record, the
state, the history); once one does, the slot is walked whole. So Fireball ("{1} more for each target beyond the first")
on a board of twenty creatures is judged at twenty-one announcements, not a million. Past `ANNOUNCEMENT_BUDGET` steps (a
change that tells apart the candidates of a slot of many targets among more than a dozen) it is offered, and choices that
then leave the cost unpayable take it back at the payment (CR 601.2h, 733.1; KI-108). Where nothing could change the cost it is judged once, before any target
is looked for. An activated ability's menu row says what it costs with each candidate where that is not the printed cost
(`activation_costs`, the host's `option_message`: "Equip {4}. ... (costs {1} targeting Dwarven Mauler)"). Ajani's Response
("{3} less if it targets a tapped creature") is a spell that reads its target this way. Dwarven Mauler,
Helitrooper and Fervent Champion ("equip abilities you activate that target this creature cost {N} less"), Dragonfire
Blade ("{1} less for each color of the creature it targets") and Ghostfire Blade ("{2} less if it targets a colorless
creature") read them.

A cost change may add life as well as mana (`CostDelta::life`, CR 118.8: Terror of the Peaks' "spells your opponents cast
that target this creature cost an additional 3 life to cast"). It is part of the total cost locked in once the targets are
announced (`apply_cost_changes_total`), paid with the rest of it (CR 601.2h), and a player can pay it only with at least the
whole of the life the total cost asks - an additional "pay X life", Phyrexian symbols and X paid in life counted with it
(CR 119.4, 118.3): a cast or activation that cannot is taken back (CR 733). The advice counts it too: a single-target cast
announced with its action is not offered at a target whose tax its caster could not pay (`added_life_payable`).

A cost change may be made by a resolving spell or ability for a duration rather than be a static ability (CR 611.2a/c:
Armor Wars's "artifact spells you cast this turn cost {1} less to cast", Urza, Planeswalker's +2, Academy Loremaster's
"spells they cast this turn cost {2} more"): an `EffectKind::SpellCostChange { card, index, player }` about the player,
naming the `index`th of `card`'s `lasting_cost_changes` (`Game::spells_cost_this_turn`). `change_parts` applies it to the
spells `player` casts with the static changes (increases first, then reductions), whatever has become of its source, and
it ends with the turn (CR 514.2). The guards that skip cost-change work when no changer is on the battlefield also look
for one (`lasting_changes`, `spell_cost_changers`). "The next spell you cast this turn costs {1} less" (Hardened Berserker,
Kaza, Invasion of the Giants), which is used up by the spell it applies to, is not built.

What besides mana may pay a spell's total cost (CR 601.2f-h: convoke 702.51, delve 702.66, improvise 702.126) is the card's
own `pay_with` together with what the permanents on the battlefield give it (`CardDef.grants_pay_with`: Ironheart, Clever
Champion's "noncreature spells you cast have improvise", Chief Engineer's convoke, Teval's delve): `actions::pay_with`,
read by the cast's payment (`Helpers::with`) and by the advice (`covers_total`, with the grantors found once per decision in
`Available::grantors`). Several instances are redundant (CR 702.51d, 702.66c, 702.126c). "The next spell you cast this turn
has convoke" (Wand of the Worldsoul, Archway of Innovation's improvise) is not built.

A cost change may set the least the total comes to (`CostDelta::at_least`: Trinisphere's "each spell that would cost less
than three mana to cast costs three mana to cast"). It is one of the "effects that directly affect the total cost" CR 601.2f
applies once the total is otherwise determined: after the mana cost or alternative cost, every increase and every reduction
(the cast's own too - harmonize's tapped power, a sacrifice for each, emerge's sacrificed mana value), generic mana making up
the difference, before the total is locked in and paid. A spell cast without paying its mana cost (Omniscience) or for an
alternative cost of no mana (Force of Will's) comes to three like any other. A cast takes it apart from the rest
(`apply_cost_changes_before_least`) and applies it last, after its own reductions; the advice does the same
(`least_cost_parts`, `finish_parts`): a permission's "costs {N} more" and additional mana are in the base the changes apply
to, and the least comes after a sacrifice for each and `way_covered`'s emerge and harmonize reductions. The host's row for a cast says what the total comes to whenever the changes in play make it other than the
cost it is cast for (`cast_total`: "(costs {3})", for a card cast from a hand), as it does for a cost that depends on
the target.

A static ability that forbids casting spells with certain qualities ("noncreature spells with mana value 4 or greater can't
be cast", Gaddock Teeg) is `CardDef::spell_rules` (`SpellRule`, CR 101.2, 601.3). Each cast the rules would offer is judged on
the face it casts (CR 601.3e, 709.3a: one half of a split card, an Adventure, a back face), with X as 0, and left out if a
permanent with such a rule forbids it (`castable`, the casts an effect makes as it resolves too); and once a cast's proposal is
complete (CR 601.2a-d) the spell on the stack is judged again, its X announced, and a cast no longer allowed is taken back
(CR 601.6). The battlefield is searched for these only when something could be cast.

A static ability that forbids some land plays ("your opponents can't play land cards from graveyards", Tomik, Distinguished
Advokist; "you can't play lands", Aggressive Mining; "players can't play nonbasic lands with the same name as a nontoken
permanent", Cornered Market) is `CardDef::land_play_rules` (`LandPlayRule`, CR 101.2, 305.1): asked of the player, the card
where it is and the face it would be played as (the card, or a modal double-faced card's land back face, CR 712.12). It is
asked of every land play whatever allows it - the rules from a hand, a rule for the graveyard or the library's top, a way a
permanent gives, a permission an effect gave, an Adventure's exile - and of a land an effect plays as it resolves
(`could_play_land_during_resolution`, CR 608.2g), and it overrules each (CR 101.2). A land put onto the battlefield by an effect
is not played, so it is not asked (that is `enter_rules`). One look at the battlefield finds these and the enter rules together
(`static_rules::land_play_bans`), made only once the player has a land play left; a permanent that lost its abilities has none
(CR 613.1f). An effect that says a player can't play lands (Solfatara) is not built: no card in content has one.

A special action with a cost (CR 116.2: plotting, suspending, foretelling, turning a face-down permanent up, unlocking a
door) is costed as `Costed::SpecialAction`: it is no activated ability (CR 116.1), so what changes the cost of activating
abilities (Forensic Gadgeteer, Anointed Peacekeeper) leaves it alone, and only a change that names it applies - Doc Aurlock,
Grizzled Genius's "plotting cards from your hand costs {2} less" (CR 116.2k, 118.7), which knows plot's action by
`ActivatedDef::plot`. `special_action_cost` applies the changes to the printed cost, for the advice and the payment alike;
the host's menu row says what it costs when that differs ("(costs {B}{G})"). For the same reason nothing that forbids
activating abilities (CR 602.5) forbids a special action: `static_rules::activation_barred`, the one place an effect's
"can't be activated" and a static ability's `activation_rules` are asked, passes every special action before asking them,
so Clarion Conqueror leaves a face-down creature to be turned up and Sorcerous Spyglass leaves its named card to be
foretold or suspended. What would stop suspend's cast stops suspending, judged as a cast (`cast_barred`, CR 116.2f).

"As an additional cost to cast this spell, you may sacrifice any number of nonland permanents. This spell costs {1} less
to cast for each permanent sacrificed this way" (Rottenmouth Viper) is `VariableCost::SacrificeAnyNumber`: how many is
announced as the spell is cast (CR 601.2b), kept as its `x` though its mana cost has no {X}, so the total cost is determined
knowing it (CR 601.2f) - `less_each` generic mana off each, after any increase (CR 118.7a) - and which ones are chosen once
the mana is paid (CR 601.2g-h). The advice counts every permanent that could be sacrificed. The host's announcement says
what the number is (`PayCostsPromptMessage::SacrificeReduces`). A spell's mana value in its `Cast` record counts X once
for each {X} in its mana cost (CR 202.3e), and not at all for an X a cost defines without one.

"As an additional cost to cast this spell, you may exile any number of white cards from your hand. This spell costs {2} less
to cast for each card exiled this way" (the five Marches) is `Cost::exile_from_hand_for_less`. The cards themselves are
chosen first, with the generic choice of cards from the hand (CR 601.2b: the intention to pay, which fixes how many), nothing
moved; X is then announced with the existing announcement, which starts at the X their reduction pays for; the targets
follow, judged by that X; the reduction comes off the total once it is determined, after any increase (CR 601.2f, 118.7a);
and the cards are exiled as the cost is paid (CR 601.2h). Which cards pay is the payer's choice as it is paid: if a mana
ability activated while paying (CR 601.2g) changed which cards could pay, as many as were chosen are asked for again, and
with too few the cast is taken back. The advice counts every card that could be exiled. X stays the spell's X, so its mana
value on the stack is X plus 1 whatever was paid (the ruling).

An additional cost the targets define ("collect evidence X, where X is the total mana value of the permanents this spell
targets", Urgent Necropsy) is `CardDef::cost_of_targets`, asked once the targets are chosen and added to the total then
(CR 601.2c, 601.2f); one its caster can't pay takes the cast back (CR 601.2h). A batch of zone changes moves an object once
however many times it is named (`move_together`), by the first of its moves (a later one to another zone is dropped): a
permanent a spell targets in two slots is destroyed once.

### Actions: the source's own option, and what made it possible

An action is what a player does with priority or while paying - cast a spell, activate an ability, take a special action,
play a land (CR 116.2a, 305.1) - and an offered action (`ActionOption`) names four things:

- **kind**: `ActionKind` (cast, activate, play a land, and the payment steps).
- **source**: the card cast or played, or the object whose ability is activated.
- **entry** (`index`): which of the source's *own* options it is. For a cast or a land play, the way number: 0 the card for
  its mana cost, n its nth printed alternative way (`CardDef::ways`: evoke, flashback, sneak), `WAY_ADVENTURE`, `WAY_OMEN`,
  `WAY_SPLIT_LEFT`/`RIGHT`, `WAY_FUSED` (a split card with fuse cast as both halves from its owner's hand, CR 702.102a:
  the core makes the fused split spell from the card's halves - the left half's targets then the right half's
  (`CardTable::spell_targets_of`), and as it resolves the left half's instructions then the right half's, each on a view of
  the item that is that half cast alone (`resolve::run_fused`, CR 702.102d), so a fuse card is `fuse: true` and its halves;
  with the card's combined cost and characteristics on the stack, CR 702.102b-c, drawn as both halves with each target
  naming its half; offered by the rules from a hand, for its mana cost or an alternative cost, and to a cast an effect makes
  from a hand (`Label::Fused`), never by a permission from elsewhere), `WAY_BACK_FACE` (a modal double-faced card's spell or land back face), `WAY_BESTOW`,
  `WAY_OVERLOAD`; for an activation, the ability's place in the computed ability list. It names the face the action casts or
  plays and the cost of the card's own it is made for, and never a position in a list another object makes.
- **granted_by** (`Option<Grant>`): what made the action possible where the rules every player has do not (CR 601.3,
  305.1) - the object and which of its abilities. `Grant { object, ability }` is an object, not a card: two Serra Paragons
  are two permissions, and a permanent that changes zones is a new object (CR 400.7). Its `AbilityRef` names an entry of the
  card's `abilities` as anywhere else, or one of the static abilities a card lists apart, by band: `AbilityRef::granted_way`,
  `exile_cast`, `spell_alternative`, `play_rule` (the kth play rule, those under a `While` counted), and `retrace` (the
  card's own keyword, printed or given). A play permission an effect gave keeps the spell or ability that gave it
  (`PlayPermission::by`, from the cause as `allow_play` runs); Yawgmoth's Will's is the spell whose effect it is. None for a
  card cast or played from its owner's hand, a card on an Adventure (CR 715.3d), a prepare spell's copy (CR 722.3) and a
  card's own "you may cast this card from your graveyard".
- **paid_by** (`Option<Grant>`): the alternative cost another object gives that a cast pays rather than its mana cost (CR
  118.9: Omniscience, Jodah, Primal Prayers' energy), or an activation rather than its ability's (CR 602.2b: Forge Anew's and
  Kili's {0} equip, `AbilityRef::ability_alternative`). A permission and a cost are two things (CR 601.2b, 601.3), so a cast
  by Lurrus's permission for Primal Prayers' {E} names both.
- An activation's **granted_by** is the other object whose effect gave the source that ability (CR 113.2c, 613.1f:
  Chromatic Lantern's mana ability on a land, an Equipment's on the creature), with the ability.

**Every open combination is its own option.** `castable_ways` (`Offer`) offers each card by each `Route` that lets it be
cast from where it is - the rules for the hand; each play permission; each permanent's permission to cast from exile; the
card's own say-so or Yawgmoth's Will for the graveyard, retrace beside them; each way a permanent gives that is a permission
(`GrantedWay::permission`: Lurrus, Serra Paragon); each rule for the top of the library (Bolas's Citadel's for life, Mystic
Forge's, Case of the Locked Hothouse's) - and by each route, each face it allows (judged as that face, CR 601.3e), for the
mana cost or what the route names instead, for each alternative cost another object gives where the route names none (CR
118.9a), and for the card's own ways that name no zone (CR 118.9: evoke from exile by a permission; not warp, CR 702.185a).
A way a permanent gives that is its cost too (escape, mayhem, Iroh's flashback) is offered beside them. Nothing is
first-found: Citadel and Mystic Forge both out offer both casts of the top card, two Yawgmoth's Wills two casts of a card in
the graveyard, and the player chooses (CR 601.2b). Land plays are offered the same way (`land_plays`): by the rules from a
hand, and by each rule for the graveyard or the library's top (Crucible-like rules, Yawgmoth's Will, Bolas's Citadel, Case of
the Locked Hothouse), each way a permanent gives (Serra Paragon) and each permission, every one its own option. Additional
land plays (CR 305.2) are counted together, as no card cares which allowed a play: a permanent's static rule
(`PlayRule::ExtraLandPlays`) and the "you may play an additional land this turn" effect a resolved spell or ability makes
for its controller (`Game::additional_land_plays`, `EffectKind::ExtraLandPlays`, until end of turn). The
common case pays as before: a card in hand has the one route, and the rest is looked for only where a card is in another
zone, each list built lazily once per pass, and a face other than the card itself judged through a view of it only where
there is a face to judge. The records keep a grant in eight bytes (`Grant::NONE` for none) rather than an `Option`'s twelve.
The benchmark measured 8.15 us per decision against 8.21 us on main, six runs each alternating the two builds (2026-10-20);
before the faces were judged lazily it was 9.2 us, nearly all of it `CardTable::id_of`'s search for each face's id.

**The executor makes exactly the action offered.** The answer is checked against the options (`validate::fits`: the
grants are part of the option), and `act` reads back what the `Grant` names - the route's terms (`through`) - and hands them
to `cast` (`CastBy`: the grant, a way's cost and riders, what the permission adds, life for mana value, the alternative's
cost). Nothing is re-derived and no permission is searched for again, so what the menu said is what is paid.

**The history records provenance.** `Cast::by` (read with `Cast::granted_by()`) and `Event::LandPlayed { land, player, way, granted_by }` keep what
each cast and land play was made by, so "once during each of your turns" is a history question
(`cantrip_mechanics::history::permission_used_this_turn`, which Lurrus and Serra Paragon share) and "if you cast it this
way" can be one. Riders that outlive the cast are attached at cast from the action's grant (Serra Paragon's "it gains", a
way's flashback). An activation is the same: `AbilityActivated { by }` records the object that gave the source the ability,
where another did. The layers know the giving card of an ability (its `AbilityRef`), not the giving object, so
`layers::ability_givers` works the object's characteristics out again watching each entry of layer 6 that puts an ability on
it (not the dependency system's trial copies) - asked only for an ability whose card is not the object's own, so nothing is
paid for it the rest of the time; two Chromatic Lanterns give a land two abilities, each its own option made by its
Lantern. Each cost a static ability lets a player pay rather than an ability's mana cost (`CardDef::ability_alternatives`)
is an activation of its own (`paid_by`), offered beside the ability for its own cost where that can be paid, described with
the cost and the permanent ("Equip {2} - paying {0} rather than its mana cost (Forge Anew)"), and paid as offered: nothing
is asked as it is announced.

**Cost changes are one list** matched against what is costed (`Costed`: a spell, an ability, a special action, CR 116.1),
asked once the targets and X are announced (CR 601.2f): "can't reduce the mana in that cost to less than one mana"
(`CostDelta::less_to_one`, Forensic Gadgeteer, Zirda) is a change like any other. A land play has no cost and no change
applies to it.

**On the wire** the host numbers each option of a question (`ActionOptionMessage::OptionId`, from 1) and the client echoes
it (`TakeActionAnswer::OptionId`): the grants stay engine-side, so no hidden object leaks, and an answer naming an option by
kind, source and index alone is refused where two options share them. A cast another object makes possible says which on
its menu row where its cost does not (`describe`), and so does a land play: "Mystic Forge", "Retrace", "Yawgmoth's Will", "Pay {E} rather than its
mana cost (Lurrus of the Dream-Den)"; an ordinary cast reads as it always has.

### Ways a permanent gives: from a graveyard or from a hand

A `GrantedWay` names the zone of its controller's own it casts from (`GrantedWay::from`): a graveyard (Lurrus, Serra
Paragon, Underworld Breach's escape, mayhem) or the hand. Each is judged on the face being cast and offered as that face's
cast with the way's `Grant`, which the spell's `Cast` record keeps (`Cast::by`): what the spell was cast with is read from
it and the way it names (`GrantedWay::kind`, as `WayDef::kind` for the card's own ways), so a card escaped by Underworld
Breach's escape escaped as one cast by its own (`escape::escaped`, CR 702.138b). One that is a permission (`GrantedWay::permission`: Lurrus, Serra Paragon) is a route: the card
is cast by it for its mana cost or any alternative cost, as by any permission. Any other is the one alternative cost of that
cast (CR 118.9a); additional costs and cost increases still apply to it (CR 118.9d). Whether X is announced follows from the
cost the way names (CR 107.3a-b): a cost that is or includes the mana cost has its {X}; `Cost::NONE`, "without paying its
mana cost", leaves X 0. A card with no mana cost may be cast that way (CR 118.6a). The host marks a cast for nothing
`WithoutPayingManaCost`, with the cost it comes to as its `CostText`, and the tactical bot takes it over the card's ordinary
cast. An emblem gives ways and alternative costs as a permanent does (CR 114.4: its abilities function in the command
zone), so Tamiyo, Field Researcher's "You may cast spells from your hand without paying their mana costs" is a
`SpellAlternative` on its emblem; nothing else in the command zone gives any (CR 113.6).

### Alternative costs a permanent gives the spells a player casts

"You may cast Dragon spells without paying their mana costs" (Dracogenesis), "you may pay {W}{U}{B}{R}{G} rather than pay the
mana cost for spells that you cast" (Jodah, Archmage Eternal; Fist of Suns), "you may pay {0} rather than pay the mana cost for
Zombie creature spells you cast" (Rooftop Storm), "you may cast spells from your hand without paying their mana costs"
(Omniscience) and "you may cast creature spells with mana value 3 or less by paying {E} rather than paying their mana costs. If
you cast a spell this way, you may cast it as though it had flash" (Primal Prayers) are `CardDef::spell_alternatives`
(`SpellAlternative`: `applies(q, asker, spell, player)`, `cost`, an `InsteadOfManaCost` of nothing, mana or energy (CR
107.14), and `flash`, CR 118.9, 601.2b). Such an ability is a way of paying, not a permission (CR 601.3): it stands beside every
cast the player may already make of a face for that face's mana cost - from a hand, a graveyard (Yawgmoth's Will, the card's
own say-so, aftermath, retrace, Lurrus), exile (an Adventure, `ExileCast`, a `PlayPermission`), a library's top - as an
option of its own (`paid_by`), and the cast goes on as that face's own with that cost as `instead`. A `flash` one's casts are
offered whenever an instant could be cast, by a permission that does not say when (CR 601.3: not plot's), and what forbids
casting at such a time (Teferi, Time Raveler) still does (CR 101.2). Each face is judged alone (CR 709.3a, 715.3a, 712.8a), in
the zone it is cast from; one that two permanents of a player give is one choice. It is offered whether or not the mana cost
could be paid, and for a card with no mana cost (CR 118.6a). It is the one alternative cost of the cast (CR 118.9a), so it is
not offered with a route that names one: a permission's "without paying" or "rather than" (`PlayGrant::instead`), flashback a
permission gives, life for its mana value (Inside Information, Bolas's Citadel), a card's own ways, or a way a permanent gives
that is its cost. Additional costs (kicker, a permission's `more`, an `ExileCast`'s, retrace's discard) are still paid (CR
118.9d), what changes the total applies to it (CR 601.2f: Thalia, Trinisphere, Terror of the Peaks), and X is 0 (CR 107.3b).
A cast an effect makes as it resolves paying its costs (`cast_during_resolution` with no `instead`, CR 608.2g: Chandra, Torch
of Defiance's "you may cast that card") announces the cost as it is cast (CR 601.2b): its caster chooses between
`Label::ManaCost`, each of the card's own ways open from where it is (`Label::Way`: evoke from exile; flashback from its
owner's graveyard only, CR 702.34a) and each `Label::AlternativeCost` that applies. The battlefield is searched for these once
per offer, and only once something could be cast.
The caster is the resolving effect's controller unless the effect names another player (`CastTerms::caster`, CR 608.2g:
Spell Queller's "the exiled card's owner may cast that card", Knowledge Pool's "that player", Allure of the Unknown's "that
opponent"). Whether that player could cast it is asked for them (`could_cast_during_resolution_by`,
`cast_ways_during_resolution_by`: their targets, their "you", what keeps them from casting), the "may" is asked of them, and
they announce the spell, choose its face, modes, targets and costs, pay them and control it (CR 601.2, 405.4).

### Hexproof from a quality

"Hexproof from [quality]" (CR 702.11d) is `Chars::hexproof_from`, a `HexproofFrom` bitset of qualities: the five colors,
monocolored, multicolored and card types. Printed, it is `CardDef::hexproof_from` (part of the copiable values; a color
word in it changes with the text, CR 612.2); a static ability gives it in layer 6 (`hexproof_from::gives`).
`layers::can_target` refuses a source with one of the qualities - a spell, or an
ability's source, as it is now - controlled by an opponent of the permanent's controller, unless a targeting rule lets
hexproof be ignored; losing hexproof (Shadowspear) or all abilities loses it (CR 702.11e). Knight of Grace, Knight of
Malice, Sphinx of the Guildpact, General Ferrous Rokiric and Dragonfire Blade have it. Not built, as no card in content
needs it: a resolved effect that gives it ("gains hexproof from that color until end of turn", Mondo Gecko, Skrelv),
"hexproof from activated and triggered abilities" (Volatile Stormdrake), "hexproof from each of its colors" (Tam), and a
player's hexproof from a quality (Veil of Summer).

### "Can't be the target of" restrictions

A static ability that says what can't target an object - "Thrun can't be the target of nongreen spells your opponents
control or abilities from nongreen sources your opponents control", Fiendslayer Paladin's "black or red spells your
opponents control", Shanna's "abilities your opponents control", Spellbane Centaur's "Creatures you control can't be
the targets of blue spells or abilities from blue sources", Ground Seal's cards in graveyards - is a
`TargetingRuleKind::Forbid` in its card's `targeting_rules`: `applies` names the objects, and the `TargetingForbid`
function reads the spell or ability that would target (`Asker`: its controller, and `Asker::spell`, whether it is a spell
or an ability, CR 115.1a) and the characteristics of its source - the spell itself, or the ability's source as it is or
last was (CR 608.2b). `layers::can_target` asks it of every object target, in any zone, after shroud, hexproof, hexproof
from a quality and protection, so it holds as targets are chosen (CR 601.2c, 602.2b, 603.3d) and as the spell or ability
resolves (CR 608.2b). It is no hexproof ability: what ignores hexproof (CR 702.11e, Nowhere to Run) leaves it. The
permanents with targeting rules are found once per state (`StaticSources::targeting`), so a board without one pays
nothing; one that lost all its abilities applies none (CR 613.1f). Not built, as no card in content needs it: a resolved
effect that says it (Vines of Vastwood, Autumn's Veil, Veilstone Amulet) and Wall of Shadows' "spells that can target
only Walls", which reads the targeting spell's requirement rather than its characteristics.

### Craft

Craft with [materials] [cost] (CR 702.167a) is an activated ability whose cost exiles its source and its materials
(`Cost::materials`, with `exile_self`): `count` objects, or `count` or more, from among the permanents the activator
controls and the cards in their graveyard, the source apart, each passing the materials' filter, which reads where the
object is (a type alone is a permanent or a card of that type, CR 702.167b). They are chosen with the rest of the cost,
before the mana, as a question of their own (`RequestKind::Craft`, minimum and maximum), and exiled together as the last
of the cost, the source first, so the history holds the source's exile and then the materials' just before the
activation. The ability is `cantrip_mechanics::craft`'s: it returns the card the cost exiled transformed under its owner's
control (CR 712.14a; a token, or a card that was only a copy of a craft card, stays where it is). Before it moves, the
materials, read from that stretch of the history, are linked to the craft card in exile as exiled with it
(`Game::link_exiled`, CR 607.2a), so "the exiled cards used to craft it" (CR 702.167c, `craft::used_to_craft`) are the
exiled cards linked to the object that became the permanent: there from the moment it enters, readable by a
characteristic-defining ability through the state (Mastercraft Raptor), and none once it leaves or they do. Such an
ability reads the cards' power with their own defining abilities applied (`LayerQuery::chars_with_defined_pt`, CR 613.8a:
a Barrowgoyf used to craft Wretched Bonemass counts as it is in exile). Materials named one by one ("a Dinosaur, a Merfolk,
a Pirate, and a Vampire", `Materials::parts`) must be separate objects: payable only when each part can be matched to a
distinct candidate, and asked for in turn. Only an activated ability pays materials
(`only_an_activated_ability_that_exiles_its_source_exiles_materials`). No card in content copies an activated ability, so
a copy of a craft ability (which would find the card already returned) has no scenario.

Craft cards not built, each for want of a rule elsewhere: Eye of Ojer Taq ("two that share a card type", and Apex
Observatory's "the next spell you cast this turn of the chosen type can be cast without paying its mana cost"),
Paleontologist's Pick-Axe (Dinosaur Headdress makes the equipped creature a copy of a chosen card, a static copy effect),
The Enigma Jewel (Locus of Enlightenment gains the exiled cards' activated abilities and copies activated abilities),
Unstable Glyphbridge (Sandswirl Wanderglyph's "they can't attack you or planeswalkers you control this turn"), Braided Net
("its activated abilities can't be activated for as long as it remains tapped", and net counters) and Master's
Guide-Mural (Master's Manufactory asks whether an artifact entered under your control this turn, whatever happened to it
since: the history does not keep the controller or the types an object entered with).

### Counters put by an effect, by a cost and by a turn-based action

Every counter comes to be on an object or a player through one path, `cantrip_rules::counters` (`put`, `put_all`,
`entering` and `place_arrived` for what a permanent enters or a card arrives with, `put_on_player`), and every counter is
put by a player, named there as a `PlayerId`, never left out: the controller of the spell or ability whose effect puts it
(CR 608.2; card code's `Game::add_counters` asks `Cx::doer`, which is also the controller of a replacement effect or mana
ability being carried out), the player paying a cost (CR 118.3) or doing a turn-based action ("that player puts a lore
counter", CR 714.3c), the controller of a source whose damage puts -1/-1 or gives poison counters (CR 120.3b, 120.3d), or
for a permanent entering, its controller (CR 122.6a) - its intrinsic loyalty, defense and lore counters (CR 306.5b, 310.4b,
714.3a) included, a token copy's too. The storage is private to `cantrip-state` (`Obj::counters`, `Player::poison`,
`energy`, `experience`); reading and removing are open, and the one adder, `put_path_add`, is called from the put path
alone, which a test (`only_the_put_path_puts_counters`) holds to. No scenario writes counters either: they are played on.

Counters one instruction puts on several objects, or of several kinds on one, are one simultaneous event (CR 608.2, 101.4,
122.6): `counters::put_together` (card code's `Game::add_counters_together`) takes every placement - what one
instruction puts on an object is one (`add_counters_together` makes it so), and each source's damage dealt as counters is
its own (CR 120.3d) - applies the replacement effects of counters being put to each (CR 614.1a) before any counter goes on, then puts them all on and announces every `CountersPut` in one `emit_batch`, so "whenever one or more
counters are put on one or more ..." triggers once (CR 603.2c). `put` and `put_all` are one placement of it.
Proliferate's permanents, the turn's lore counters on each Saga (CR 714.3c) and the -1/-1 counters infect's damage is dealt
as (`DamageBatch::finish`, announced with the damage, CR 120.3d, 510.2) go through it; so do the shared effects that put
counters on each of several permanents (`effects::counters_on_creatures_you_control`,
`double::counters`, `counters_divided`). A card never loops
`add_counters` over objects or kinds its one instruction names together.

Counters that would be put on a permanent are one `Proposal::Counters` (CR 122.6, 614.1a), whose `CounterPlacement` says who puts
them (`putter`, CR 122.6a: "if you would put", Innkeeper's Talent) and whether an effect does (`effect`: "if an effect would put",
Doubling Season); counters on a player are one `Proposal::PlayerCounters`, poison, energy and experience, with its `putter`. An effect is a resolving spell's or ability's, or the replacement effect a permanent enters with counters by
(CR 614.1c: a planeswalker's loyalty, CR 306.5b; a battle's defense, CR 310.4b; a Saga's first lore counter, CR 714.3a). A cost's
counters are none (CR 118.3): a loyalty ability's (CR 606.4), "put a page counter on this artifact" (Mazemind Tome) and blight's
(CR 701.68a) go through the replacements with `effect` false (`counters::How::Cost`), as does the lore counter a player puts on each
Saga as their precombat main phase begins (CR 714.3c, a turn-based action). So Doubling Season doubles a planeswalker's entering
loyalty and not its +1, and Innkeeper's Talent doubles both. A cost that would put counters a static ability forbids (CR 101.2,
`CardDef::counter_bans`: Solemnity, Melira) can't be paid (CR 118.3, Solemnity's ruling): such a loyalty ability, counter cost or
blight is not offered. Whether damage's -1/-1 counters (infect, CR 120.3d) are an effect's has no ruling; they are put as one.
A static ability may forbid some payments of the costs to cast spells and activate abilities (CR 101.2, 118.3, 601.2h, 602.2b:
`CardDef::cost_bans`, a `CostBan` asked of the payer, what the cost is for - `CostFor::Spell`, `Ability` or `ManaAbility` - and the
payment, `CostPayment::Life` or `Sacrifice(permanent)`, through `static_rules::cost_banned`). Yasharn, Implacable Earth forbids life
and nonland sacrifices, Angel of Jubilation life and creatures, Karn's Sylex life outside mana abilities. Every cost path of a cast or
an activation reads it: the life a cost may pay (`payable_life_for`, which also limits Phyrexian symbols and "pay X life"), the
permanents a sacrifice may take (`pays_permanents`, `variable_candidates`), "sacrifice this" and "sacrifice [the granter]". A cost that
can be paid only that way can't be paid, so the spell or ability isn't offered; one with another way (a land for "sacrifice a
permanent", a Phyrexian symbol's color) is paid that way. 0 life can always be paid (CR 119.4b). What a resolving spell or ability
has a player pay (CR 118.12: `may_pay`), and a special action's cost (CR 116.2), are paid for none of these, and pass `None`.
A permanent that "can't be sacrificed" (CR 101.2, 701.21a) can't be sacrificed by anything: it has the ability
(`Keyword::CantBeSacrificed`, printed or granted, so "loses all abilities" ends it), a resolved effect says so
(`EffectKind::CantBeSacrificed`, which outlasts its abilities), or another permanent's static ability does
(`CardDef::sacrifice_bans`, Assault Suit's equipped creature) - all read by `static_rules::cant_be_sacrificed`. It pays no
cost that sacrifices, whatever the cost is for (`may_sacrifice_for`); `ops::sacrifice`, `sacrifice_and_return` and a Saga's
state-based sacrifice (CR 714.4) leave it; and a choice of what to sacrifice asked by card code (`Purpose::Sacrifice`, through
`Cx::as_allowed`, for `ask` and `ask_then` alike) offers only what `ops::sacrificeable` allows - which also leaves out a
permanent whose controller the resolving spell or ability can't make sacrifice (Sigarda) - taking as many as it can when fewer
remain than it asks, and answered with nothing unasked when none does. Card code that takes what it sacrifices without asking,
or offers a sacrifice as one option among others, asks `Game::can_sacrifice`.
A sacrifice always names who sacrifices (CR 701.21a: a player can't sacrifice a permanent they don't control):
`Game::sacrifice(player, ..)` and `can_sacrifice(player, ..)`, `sacrifice_each` for several players at once ("each player
sacrifices"), and `ops::sacrificeable` checks the permanent is that player's, so a card can't forget it. Nor can it sacrifice by moving: `Game::move_together` takes a `MoveReason`, which has only the reasons no verb of its own carries out (no sacrifice, destruction, discard, draw or mill), so the type refuses it. The player is the one
the text names: "you" is the spell's or ability's controller, a delayed ability's included (CR 603.7d-e: Kiki-Jiki's token,
decayed, Underworld Breach), whoever controls the permanent by then; "its controller sacrifices it" (evoke, CR 702.74a; Animate
Dead) is the permanent's controller as it resolves; a cost's sacrifices are its payer's (`may_sacrifice_for`). "You may
sacrifice it" is not offered when `can_sacrifice` says no (CR 608.2d: Cacophony Scamp taken by Old Man of the Sea).
The permanent's controller orders the replacements (CR 616.1), told the counters as the effects so far have left them
(`RequestKind::ChooseReplacement::counters`, which the host words "1 +1/+1 counter would be put on Walking Ballista"); the
tactical bot has one that adds a counter apply before one that doubles them.

### Blight, counters from among permanents, and casting from exile by a permanent's leave

Blight N (CR 701.68a) is `Cost::blight`, paid by a spell's cost (Requiting Hex) and by an activated ability's (Dawnhand
Dissident; Gristle Glutton and Sting-Slinger print it too). An activation chooses the creature after its mana is paid, as a
spell's additional cost does (CR 601.2g-h by way of 602.2b): from what is there then, and with no creature left the activation is
taken back (CR 701.68b). The creature need not survive the counters (the rulings). 701.68d's "whenever a player blights" has no
event yet: no card in content triggers on it.

"Remove N counters from among [permanents you control]" (CR 118.3, 122.1) is `Cost::remove_counters_among`, a `CountersAmong`: a
count and a filter over the payer's permanents, and the kind of counter it names - none for "counters" of any kinds (Dawnhand
Dissident, Tayam), one for "remove two +1/+1 counters from among creatures you control" (`remove_kind_counters_among`: Hopeful
Initiate, Novijen Sages; The Filigree Sylex's oil counters), which no counter of another kind pays. It is payable while there
are that many such counters among them, and is asked as `RequestKind::RemoveCounters` - each kind on each permanent a
`CounterPool` with how many there are (only the cost's kind, when it names one), answered with
`Answer::Counters` (how many of which kind from which permanent, adding up to the total, which `validate` checks) - chosen
after the mana like blight and removed as the cost is paid (`counters::remove`, so a Siege's last defense counter still
triggers). The host sends it as `RemoveCountersPromptMessage`; the web client offers a button for each kind on each permanent
and takes one of a permanent's counters on a click on it, naming the kind in its subtitle when every pool is of one. Dawnhand Dissident's casts pay it; Tayam, Luminous Enigma's ability is
the same cost on an activated ability, which `activate` pays too, though no card in content has it there yet.

"Remove up to N counters from target creature" as an effect resolves (CR 608.2d, 122.1: Heartless Act, Hex Parasite, Price of
Betrayal's permanents) is `Game::remove_counters_up_to(player, source, object, n)` (`counters::remove_up_to`): the same
question with `up_to`, its pools every kind of counter on the one object, answered with none to N of them in all, no more of a
kind than there are (`validate`); they come off through `counters::remove`, and the verb says how many did ("for each counter
removed this way"). Nothing is asked of an object with no counters or one that has changed zones. The prompt carries `UpTo`, and
the web client captions it "remove up to 3 counters", its button removing whatever is chosen, none included. A player's counters
(Price of Betrayal's "or opponent") have no removal path yet.

A permanent's static ability that lets a player cast cards in exile (CR 601.3, 604.1) is `CardDef::exile_casts`, an `ExileCast`:
whether a card may be cast by it now (asked of the permanent, its controller "you", with the card as the face that would be
cast, so "creature spells" refuses a creature card's instant Adventure; a condition on the game state, Null Summoner's threshold,
is asked there too, since a static ability is never locked in, CR 611.3a), an additional cost owed on each cast by it (CR 118.8,
601.2f), and whether mana of any type can be spent on that spell (`any_type`, CR 609.4b, read with the permission as the cast
begins). It is a permission, not a way of paying: `castable` treats the card as permitted, so it is cast for its mana cost or any
alternative cost it has (Solitude's evoke), at the speed its type gives (a flash creature at instant speed, the rulings), and the
additional cost joins `extra` in `actions::cast`, read from the permission the action names as the cast begins (`CastBy`), so the
permanent need not stay while the spell is paid for (Dawnhand Dissident's ruling). A permanent that has lost its abilities gives
none, and "cards exiled with this" are those an ability of its own text linked (`Query::exiled_with`, CR 607.2a), so a
flickered one finds none (CR 400.7). A card that a play permission of its own lets be cast, or a card on an Adventure, is cast by that instead; two
such permissions on one card are not offered as a choice, since no card in content can have both. The host shows the
permission's text on the card while it would allow the cast (`exile_cast_texts`). Rona, Disciple of Gix and Kheru Mind-Eater
print the same kind of ability with no additional cost.

### What a cost chose, for the effect to read

An activated ability's cost may choose permanents - tap, sacrifice or return them (CR 601.2h by way of 602.2b), crew's and
saddle's creatures too - and its effect may name what it chose: "Put a number of charge counters on this permanent equal to
the tapped creature's power" (station, CR 702.184a), "deals damage equal to the tapped creature's power" (Unerring Sling),
"where X is the power of the creature tapped this way" (Keldon Battlewagon, Impelled Giant). `actions::activate` keeps them
on the stack item, in `GameState::members` through `StackItem::members` (the span a "one or more" trigger keeps its batch in,
which an activated ability had no use for), as they are chosen, so a copy of the ability has them too (CR 707.10); the effect
reads them with `Query::cost_objects(item)`, an object id each, whose characteristics `chars` gives as they are now or, once
the object has left, as it last existed (CR 608.2h). Nothing is pushed for an ability whose cost chose nothing, so the
benchmark is unchanged. A spell's additional cost is still read from the history (Kazuul's Fury, Broadside Bombardiers'
boast ability): the cast path does not keep its chosen permanents yet.

### Crewing and saddling as though power were greater

Crew and saddle (CR 702.122a, 702.171a) are the core's `Cost::tap_creatures`, a `TapCreatures` with the total `power` and
`greater_with`, the keyword with which a creature tapped for it counts as though its power were 2 greater:
`Keyword::CrewsAsThoughPowerTwoGreater` for crew (`mechanics::crew`), `SaddlesAsThoughPowerTwoGreater` for saddle
(`mechanics::saddle`), none for teamwork. "This creature saddles Mounts and crews Vehicles as though its power were 2
greater" is both keywords (`tokens::PILOT_SADDLES_AND_CREWS`); a printed one is a JSON `keywords` entry, a granted one a
layer-6 grant. Every printed amount is 2. The power counted is the creature's plus 2, a negative power too, and the
prompt's powers are signed: a creature whose power is still negative takes from the total (CR 107.1b: "If a calculation
or comparison needs to use a negative value, it does so"), so the validation sums them as they are and the cost is
payable while the creatures with positive power are enough.

A creature tapped to pay a spell's teamwork cost (CR 702.194a) becomes tapped to pay it: `actions::cast` emits
`Event::TappedForTeamwork { creature, spell }` for each, right after its `Tapped`, so "whenever [this] becomes tapped to pay a
teamwork cost" (Agent Maria Hill) triggers once the cast is complete. A creature tapped for mana or convoke is none, and a crew or
saddle cost is no teamwork cost.

### What the board was as a spell was cast

"If you controlled a Faerie as you cast this spell" (Faerie Fencing) asks the board as the spell became cast (CR 601.2i), once
its costs are paid: a Faerie sacrificed to a mana ability while paying does not count, and one lost or gained afterwards
changes nothing (the rulings). A card says what it asks with `CardDef::cast_fact` (`CastFact`, a function of the game asked
of the spell and its caster), which `actions::cast` asks just before it records the spell's `Event::Cast`, and the answer is
kept in that record, `history::Cast::fact`: the past is asked of the history, so nothing is added to the stack item. The
effect reads it with `History::fact_as_cast(spell)`. A copy of the spell was not cast (CR 707.10), so it has no `Cast` and
its fact is 0, which is the rulings' "the copy wasn't cast, so whether you controlled a Dragon won't matter". Nor has it
what its object carries of a cast: `ops::copy_spell` gives it no `cast_from` (Sevinne's Reclamation's copy was not cast from a
graveyard and does not copy itself; `statics::spell_you_cast` is `cast_from` alone, so a cast copy of a card, a prepare
spell's, is a spell you cast), no `untimely_by_own_flash` and no granted way's `gains`. What was chosen and paid as the original
was cast it does copy: X, kicks, other additional costs, gift and the alternative cost (`cast_way`, the item's `way`: a copy
of a sneak-cast spell had its sneak cost paid). The `Cast`
record grew from 12 to 16 bytes and the `Event` stayed 20; a card with no fact costs one `Option` test per cast. Steer
Clear ("if you controlled a Mount"), Flame Discharge ("a modified creature"), Lethal Exploit ("for each modified creature you
controlled") and Lifestream's Blessing ("the greatest power among creatures you controlled") print the same thing; a
number goes in the fact as it is. The modes a modal spell may choose by the board as it is cast (Flame of Anor) are
`CardDef::modes_up_to`, asked as the modes are announced (CR 700.2), which is earlier. Not built: the Dragon cycle's
"if you revealed a Dragon card or controlled a Dragon", whose reveal is an optional additional cost a copy does copy
(Draconic Roar, Silumgar's Scorn; none in content). The benchmark measured 6.55, 7.01, 6.75 us against 6.60, 6.68, 7.00 us
before, alternating the two builds (2026-10-16).

### Copies of abilities

`Game::copy_ability(Target::Ability(serial), copier, new_targets)` (`ops::copy_ability`) puts another instance of an activated
or triggered ability onto the stack (CR 707.10): the original's item with the copier as its controller and a new serial, so
the same source (the Lithoform Engine ruling), targets, modes, X, subject and the objects its costs were paid with, and new
targets chosen as for a spell (`retarget`, CR 707.10c). It was not activated: no `AbilityActivated`, so a Locus of
Enlightenment does not copy its own copy. "Whenever you activate an ability ... copy it" names the ability by
`triggers::activated_ability_on_stack`, the topmost activated item of that source and ability as the activation is
emitted (triggers are collected then, CR 603.2), carried as the trigger's subject. An activated ability's item joins
`left_stack` as it leaves the stack (`GameState::left_ability`), so one countered before the trigger resolves is still
copied as it last was (the Rings of Brighthearth ruling); a triggered ability is copied only by what targets it. Rings of
Brighthearth, Kurkesh, Illusionist's and Battlemage's Bracers and Locus copy activations; Lithoform Engine, Strionic
Resonator and Return the Favor target. An item that finishes resolving now leaves the stack by its serial (CR 608.2n),
not its old position, since its effect may have taken an item under it (Ertai Resurrected countering an ability).

"You may activate each of those abilities only once each turn" (Locus, Mairsil, the Pretender) is
`CardDef::granted_once_each_turn`: the card says how many of its permanent's instances of an ability its static gave with
the limit, and `actions::granted_limit_reached` counts that ability's activations by the object this turn in the history
(CR 602.5b); an unlimited instance of the same ability is used first.

An object holds up to `MAX_ABILITIES` (32) abilities, printed and granted (`Chars::abilities`, `LastKnown::abilities`), and
`Obj::activated_mask` is as wide: Locus has every activated ability of the cards used to craft it (CR 702.167c), nine or
more from five ordinary artifacts, which the old eight dropped without a word. The ability copying, the activation limit
and the wider list measured, base (`cf999fa8`) then this branch, three back-to-back pairs on a loaded machine (other agents
building): 25.38 / 16.20, 9.83 / 10.98, 12.79 / 12.22 us per decision - no difference past the noise (2026-10-17).

### Station

Station (CR 702.184a) is `cantrip_mechanics::station::ability(text)`: "tap another untapped creature you control" as a cost
(`Cost::tap_untapped`, not the {T} symbol, so a summoning-sick creature pays it and the station permanent needs no untapping),
sorcery speed, and an effect putting charge counters equal to the tapped creature's power (`cost_objects`), none for a power
of 0 or less. A station symbol (CR 721.2) is the card's own static ability gated on its charge counters,
`station::charged::<N>` as the static's `affects`: "{N+}[abilities]" grants the abilities, which the card lists in its
`delayed` list (`station::has::<INDEX>`, layer 6), and a "[P/T]" box also makes it a creature in layer 4
(`station::is_a_creature`) with that base power and toughness in layer 7b (`station::base_pt::<P, T>`). A station card prints
no power or toughness in its content file (CR 721.2c). Counters from anywhere count and counters removed take the abilities
away (Vampire Hexmage), and a copy has the striations with its own counters (Phyrexian Metamorph). Adagia, Windswept Bastion;
Uthros, Titanic Godcore; Evendo, Waking Haven; Wurmwall Sweeper. The web client greys a striation whose number the
permanent's charge counters have not reached (`CardReading.StationUnreached`); the ability it gives is listed with its
abilities once it has it. "Except it's legendary" (CR 707.9b) is `Copiable::legendary`. "Except it's a 5/5 black Demon"
(CR 707.9b, 205.1a), with no "in addition to its other types", is `Copiable::typed(types, subtypes)`: those are its card
types and subtypes instead of the copied card's (supertypes kept, an instant or sorcery keeping that type), a Changeling it
copies is left behind (CR 707.9d), and an exception added after it adds to it (Ardyn, the Usurper; The Scarab God).

### Effects that end when a card is cast from exile

"Until this card is cast from exile" (CR 611.2a, 400.7) is `Duration::UntilCastFromExile(card)`, the object in exile.
`actions::cast` drops those effects as that object becomes cast (CR 601.2i), after its costs are paid, so a mana ability
the effect grants may pay for the cast (Emrakul, the Exigent Doom's ruling). The card leaving exile any other way is a
new object that is never cast as this one, and the effect lasts as long as its affected object.

### Effects that last while a counter is on the object

"For as long as it has a [kind] counter on it" (CR 611.2b) is `Duration::WhileHasCounter(kind)`. `add_effect` refuses to
start one on an object with no counter of that kind (a replacement kept it off), and the effect ends the moment the last
one comes off - `ops::end_counter_durations`, called wherever counters are removed (`counters::remove`,
a cost that removes counters, a stun counter replacing an untap, damage to a planeswalker), with the state-based check as
the backstop - and is deleted, so a counter put on again later does not bring it back. Its source leaving changes nothing.
A control effect that ends this way gives the permanent back (CR 613.1b). Eluge, the Shoreless Sea; Aquitect's Will,
Xolatoyac, the Smiling Flood and Quicksilver Fountain print the same words. The paths no printed card can show (a counter
put back without a new effect; an effect begun without a counter) are `crates/cantrip-script/tests/counter_duration.rs`.

### Permissions to play under a condition

A standing permission to play a card (`PlayPermission`, CR 601.3, 305.1) may hold only while a condition does
(`PlayGrant::when`, CR 611.2), worked out each time the permission is asked (`actions::permissions_for`), so it is
offered, or not, as the game is then: `PlayCondition::AttackedWithThisTurn(subtype)` ("during any turn you attacked with
a Rogue", Robber of the Rich) asks this turn's history; `PlayCondition::ControlsSubtype(subtype)` ("you may play them if
you control a Wizard", Gandalf, Goblins' Bane's Flameshape; Possibility Technician's Kavu) asks whether the holder controls
a permanent with that subtype now, its characteristics through the layers (a changeling counts). Losing the last one stops
the permission until another arrives; a spell already cast stays cast. The card in exile keeps the permission all along -
only a zone change ends it (CR 400.7). The host says the condition beside the permission ("Playable while it stays here,
if you control a Wizard").

### Restricted mana

An ability's explicit "Activate only as an instant" is `ActivatedDef::priority_only`, written with
`.only_as_an_instant()` (CR 304.5, 602.5e). It does not change mana-ability classification (CR 605.1a): the ability
still resolves immediately at priority. `actions::ManaWindow` separates priority inventory from every payment
inventory, including nested mana-ability costs, casting during resolution, attack taxes, special actions and unless/ward
payments. Cost estimates count only sources available during payment, so a priority-only source must be activated
before announcing the cost; mana already in the pool still pays normally. Restrictions are read from actual current abilities, so copied and granted abilities
keep their restriction, while an ordinary ability of the same permanent is unaffected. Lion's Eye Diamond prints this
restriction on a mana ability. The 128-game benchmark measured 13.80 us per decision before and 13.74 after in
back-to-back runs of the two builds (2026-10-24; shared-machine load, no speed claim).

Mana a restriction limits (CR 106.6) is kept in the pool apart from ordinary mana, one `ManaPool` per restriction present
(`GameState::restricted_mana`, a list of player, restriction and pool beside the players, so that a player stays plain
`Copy` data; written through its own undo entry, only when it changes, and empty in nearly every position). A
restriction is a limit on what may be cast or activated, and an effect the mana has on what it pays for
(`ManaRestriction::effects`: haste, can't be countered): `Spells(types)` pays only for casting a spell with one of the
card types (Mishra's Workshop, Great Hall of the Biblioplex, Gideon's Memorial), `Abilities(types)` only for activating
an ability whose source has one of them, or any source (Sunken Citadel, Steelswarm Operator, Omen Hawker),
`SpellsFrom(zone)` only for casting a spell cast from that kind of zone and `NotSpellsFrom(zone)` for anything but that
(CR 601.2a: `ManaPayment::Spell::from` is the zone the card was in, `Obj::cast_from` once on the stack; Interdimensional Web
Watch's "spells from exile", Heartwood Crafter's "can't be spent to cast spells from your hand"),
`Subtypes(SubtypeSpells)` only for casting a spell with one of up to two subtypes, a creature spell where it says so,
uncounterable where it says so, and where it says so also for activating an ability of a source with one of them
(`SubtypeSpells::abilities`: Flamebraider's Elementals, Eclipsed Realms' chosen type; `ManaPayment::Ability` carries the
source's subtypes) (Desolation of Smaug and Sarkhan, Fireblood's Dragon spells, Maelstrom of the Spirit
Dragon's Dragon or Omen spells, Unclaimed Territory, Pillar of Origins and Cavern of Souls' creature spell of the chosen
type), `NonSpells(types)` only for casting a spell with none of the card types (Chandra, Chill of Compliance and Narset
of the Ancient Way's "a noncreature spell": an artifact creature spell is a creature spell), `ManaValue(ManaValueSpells)`
only for casting a spell whose mana value on the stack is N or greater, X counted as announced and a split card's cast
half alone, or, where it says so, whose mana cost has {X} (Ashling, Rimebound's 4; Troyan, Gutsy Explorer's 5 or {X}),
and `NotNonartifactSpells`, `GainsHaste` and `LegendaryUncounterable` are the three single cards' own. A subtype
is any of hundreds, so the pool's list holds whatever restrictions the game made; those with no subtype or number are a named list
(`ManaRestriction::NAMED`, which the build checks a card's `.only_for` against). A mana ability's restriction is made
concrete as it is activated (`actions::restriction_from`): its subtype words read through its source's words (CR 612),
"the chosen type" the source's `chosen_subtype` (CR 607.2a), or none - no spell - for a source that never entered. A spell
is judged by its characteristics on the stack (`cx.chars`: changeling, text and type changes count); the advice whether
a card in hand is castable (`spell_kind`) takes the face's subtypes, its changeling and the whole card's own as it is in
the hand. A spell's subtypes are only worked out where some mana at hand is restricted by subtype. `ManaPayment::Spell`
carries the spell's mana value (`Query::mana_value` on the stack; for the advice, the face's with X at 0, a nonmodal back
face's its front face's, CR 712.8c) and whether its mana cost has {X}: a card with {X} still to announce is offered when
some X would bring it to a mana value the restricted mana pays for (`Available::covers_for`). A copy of a spell had
no mana spent on it, so it gets nothing the mana did to the original (CR 707.10). What a payment is for
is `ManaPayment`, judged as the cost is paid (`actions::payment_for`): casting a spell on the stack (CR 601.2g), activating
an ability of a source in any zone - a mana ability's own mana cost too (CR 605.3a) - or anything else, which no
"abilities" mana pays: a special action (CR 116.2, turning a face-down permanent up, unlocking a door), "unless ... pays"
(CR 118.12). `ManaRestriction::may_pay` is the one rule, read by the payment (`restricted_for`), by the advice whether a
cost can be paid (`Available::covers_for`), and by the options offered while paying: a mana ability whose mana could pay
nothing of this cost is still offered (CR 601.2g) and is marked `pays_nothing`, which the bots read. The restriction's
mana goes where the player sends it: two mana of one restriction may pay one ability or two. The pool empties at the end
of each step and phase as any mana does (CR 106.4).

A mana ability without {T} may be activated again for as long as its cost can be paid (CR 602.2, 605): where that cost is
one permanent sacrificed or counters removed from its source (`actions::Repeat`), the advice counts it once for each thing
it can spend. Each permanent it could sacrifice is a source of its own for `Available`, whose alternatives are the
abilities that could sacrifice it (Ashnod's Altar, Phyrexian Altar: one or the other, never both), and each counter one
activation (Cryptic Trilobite); the permanent's own mana abilities are counted apart, since it may be tapped for mana and
then sacrificed. An ability that taps an untapped permanent instead (Urza, Lord High Artificer's "Tap an untapped artifact
you control: Add {U}") joins the alternatives of each permanent it could tap: that permanent's own {T} mana abilities or
it, never both (where it falls short, KI-108).

### Mana spent as though it were of any color

"Spend mana as though it were mana of any color" (CR 609.4b) is a `CardDef::spend_any_color` rule, asked of a payment
(`Spending`: the player, what it is for, and whether it is casting that spell, activating an ability of it or another
payment, `SpendingFor`): Agatha's Soul Cauldron's for its controller's creatures' abilities, Oath of Nissa's for its
controller's planeswalker spells, Mycosynth Lattice's for every player and payment. Where one says yes, the cost is paid
as `ManaCost::paid_with_any_color` has it - each colored and hybrid symbol by any mana, colorless included, a {C} symbol by
colorless only - by the payment itself (`pay_mana_used`) and by the advice (`Available::covers_for`, which asks once per
player whether the rule holds for every payment; whether a card can be cast, which asks for that spell, read as the face
it would be cast as, CR 712.11c, 709.3a, 715.4 - `spell_covers`, `covers_total`; and the activation offers, which
ask for their source); the cost, its mana value and the mana spent are unchanged, and no restriction on the mana is lifted. "Mana of
any type can be spent to cast that spell" (`PlayGrant::any_type`, `ExileCast::any_type`) is the same with {C} too (`ManaCost::paid_with_any_type`,
`Helpers::any_type`). The payment keeps the cost's symbols and asks for the mana by that view of what is left, so what
pays instead of mana still pays only its own share: delve and improvise the generic mana, convoke a symbol of its
creature's color.

### Monocolored hybrid

`ManaCost::twobrid` counts {2/W}-style symbols by color (mana value 2 each, CR 202.3f; the card is that color); {C/W} is
an ordinary `hybrid` pair with `Mana::C`. As a spell is cast or an ability activated, `announce_twobrids` asks which are
paid with {2} rather than their color (CR 601.2b, 118.13a; `Label::PayGeneric`, advised with the fewest {2} that leave a
cost the player could pay), before the total cost, so a generic reduction reduces what was announced as {2}; a cost paid
on resolution announces just before it is paid (CR 118.13b), and `pay_mana_used` announces whatever is left. The advice
(`Available::covers_for`) tries every announcement.

### Tapped for mana

`Event::TappedForMana { permanent, player, produced }` is recorded when a mana ability with {T} in its cost resolves
(CR 106.12): `produced` is the mana that ability itself added, every restriction counted, told from every player's
pools before and after it (its tapper's, and another's it adds to) - so a replacement of its mana counts, and the mana a triggered mana ability adds for the same tap
does not (`actions::tapped_for_mana`).

Replacement effects on that mana (CR 106.12b: Damping Sphere's "produces {C} instead of any other type and amount", Mana
Reflection's "twice as much of that mana", Contamination's {B}) are `ReplacementKind::ManaProduction` effects over a
`Proposal::ManaProduction { permanent, player, to, mana, restriction, lasting }` (`player` the tapper, `to` whose pool): while one could apply
(`replace::mana_may_be_replaced`, a permanent whose card has one - `CardTable::mana_replaces` - or a lasting granted one), the
mana the ability's effect adds is held (`Cx::producing`) rather than added, proposed whole once the effect is done
(`actions::produce`, `replace::mana_production`), ordered by the player who tapped it (CR 616.1), each effect once (CR 614.5),
and what is left added to the pool with the ability's rider (CR 106.6) - before the tap is recorded, so the triggered mana
abilities it triggers see the mana as replaced (CR 106.12a). Everything that predicts a tap's yield judges it through the
same effects (`replace::mana_outcomes`: every outcome, in every order the player could choose, each "you may" taken or not,
no `then` run): what the player has to pay with (`available_without`, one alternative per outcome, its triggers judged on
it), the types a permanent could produce (CR 106.7, `producible_types`), a mana ability's row (`replaced_mana_yields`, the
host's `produces`) and the scenario harness's "for {B}". The triggered mana abilities it triggers (CR 605.1b) resolve at once, and only when
it produced some mana (CR 106.12a); one that asks (`cantrip_mechanics::mana::add_one_mana_of_any_type_that_land_produced`,
a type of `Label::ManaType`, "Choose which type of mana to add") suspends the activation, which runs again with the
answer. "One mana of any type that land produced" reads `produced`: Lavaleaper, Mana Flare, and Heartbeat of Spring,
Dictate of Karametra, Zendikar Resurgent, Mirari's Wake and Vorinclex, Voice of Hunger as written.

A delayed triggered ability can be a mana ability too (CR 603.7, 605.1b): "until end of turn, whenever you tap a Mountain
for mana, add an additional {R}" (Molten Tide, High Tide, Bubbling Muck) is a `DelayedWhen::ThisTurn` ability whose def
is `.mana_ability(TriggeredMana::One(Mana::R))`. `tapped_for_mana` runs the waiting ones whose matcher accepts the tap right after the permanents'
(`delayed_mana_abilities`, looked for only while some delayed ability waits on an event), each at once with its own
controller as "you" (CR 603.7d-e), stopped as any delayed ability is but never made to trigger again, its mana produced by
its source, the spell that made it, wherever that has gone (CR 106.3); settle's delayed abilities skip a mana ability, so
it never goes on the stack (CR 605.4a). "Whenever [you tap / a player taps] a [land type] for mana, [that player] add[s]
an additional [mana]" is `mana::you_tap_a_land_type_for_mana` or `a_player_taps_a_land_type_for_mana` with
`mana::that_player_adds_an_additional`, the type read through the source's words (Nissa, Who Shakes the World too).

Every triggered mana ability states what it adds (`TriggeredDef::mana_yield`, a `TriggeredMana`: one mana of a type, one
of a type the permanent produced, one of its source's chosen color), since its effect is code. The advice of what a
player can pay (`actions::available_without`) counts it: while some permanent's or delayed triggered mana ability waits
(`waiting_mana_triggers`, empty and unallocated otherwise), each mana source the player could tap is asked whether its
tap would trigger it (`tap_triggers`, the ability's own matcher on a `TappedForMana` of that source), and what it states
it adds is counted with the source's mana - so with High Tide resolved a {2}{U}{U} spell is offered from two untapped
Islands before either is tapped (CR 601.2g-h). Every
object leaves last-known information as it ceases to be that object (CR 400.7, 608.2h, 113.7a) - a move out of any
zone (`move_together`) or ceasing to exist (`ops::cease_to_exist`: a token or a copy of a spell, CR 111.7, 707.10a, a
player's objects as the player leaves, CR 800.4a) - a row of `GameState::last_known` that the old object names
(`Obj::last_known`), so `last_known(id)` is one index and no reader needs to know which zone it left. So the delayed
ability of a High Tide whose "Island" Magical Hack made "Swamp" reads "Swamp" through High Tide's words once the effects
on the spell are gone (CR 603.7d, 611.2c, 612.1). A row keeps the name (a copy's, one an effect set) and supertypes as
well as the types, colors, power, toughness, keywords, abilities and words, and `Chars::as_known` lays it over the
object. A spell is also kept as it became cast (CR 601.2i): a row pushed then, named by its stack item
(`StackItem::cast_known`), which `Cast::chars` reads, so "the first non-Lemur creature spell with flying you cast" judges
an earlier cast as it was cast, not as Artificial Evolution made it on the stack. The rows are compared with the position but not hashed: the
objects naming them are.

"You may activate abilities of creatures you control as though those creatures had haste" (CR 302.6) is
`CardDef::activate_as_though_haste`, asked (`static_rules::activates_as_though_haste`) only where a summoning-sick
creature's {T} cost is paid: it gives no haste, so the creature still can't attack (Shang-Chi, Master of Kung Fu;
Thousand-Year Elixir).

The restrictions by zone, by creature source and by a subtype's spells or sources, `produced` and the haste permission were
benchmarked (`cantrip-bench --games 128`, "engine answering", 2026-10-16, with about twenty agents building on the machine):
main 24.45, 33.98, 35.55, 44.43 us against this work 29.22, 31.98, 26.85, 18.83 us alternately, and again once the
last of it was in, main (2bbe5edc) against the branch, three back-to-back pairs: 10.72/11.44, 10.35/10.30, 11.24/11.22
us. Within the noise.

### Where mana came from

Every unit of mana has a source (CR 106.3): the source of the ability that produced it - a mana ability's own source, a
triggered mana ability's source (Mana Flare's additional mana is Mana Flare's, not the land's), the spell or ability
resolving - or none for a player's own mana ability (CR 605.1a, `ObjectId::NONE`). The pools keep it beside the counts:
`GameState::mana_from` breaks each player's pool and restricted mana down by source (and by restriction, and whether it
lasts until end of combat), written only by the state's three mana methods (`add_mana`, `spend_mana`, `empty_mana`),
which write the counts too, so the two cannot disagree; each entry is undone on its own (`Undo::ManaFromSet` and its
kin), not by copying the list. Card code still says `g.add_mana(player, mana, n)`: `Cx::mana_source` says whose it is.

What a payment spent is in the history, not on the spell: `actions::pay_mana_used` records one `Event::ManaSpent { player,
source, on, cast, mana }` for each source it took mana from, as the payment completes, so a spell's records come before its
`Cast`. "Whenever you cast a spell using mana produced by [this]" asks `History::cast_using_mana_from(spell, source)`
(`mana_spent_to_cast` gives each source with its mana); a copy was not cast and had none spent (CR 707.10). Mana that was
spent on anything else (an ability, "unless pays") is recorded with `cast: false`, for whatever asks next.

The player chooses which mana pays a cost (CR 601.2h, 118.3a), and mana that some rule tells apart by its source is a kind
of its own while it is: a permanent's `CardDef::mana_told_apart` rule (`ManaSourceRule`, asked of the permanents on the
battlefield with their abilities) says which sources' mana it cares about, and `static_rules::mana_told_apart` gives the
sources in a player's pool it tells apart, asked only while that pool's ordinary mana came from more than one source. Such
mana is offered as an `AssignMana` option naming its source, after the ordinary mana of its kind, and a payment that would
spend only some of a kind split among such sources is the player's choice rather than paid at once, as restricted mana is.
The host shows it as a pool chip of its own (`RestrictedManaView::source`, captioned "from Barracks of the Thousand"); a
scenario spends it with `Spend: W from Barracks of the Thousand` and the dump lists the pool by source (`BotN.ManaFrom`).
Mana no rule tells apart is spent in the order it came, ordinary before lasting, as before. Barracks of the Thousand tells
apart its own. The same records answer "mana from a Treasure was spent to cast it" (Kalain, Reclusive Painter), "spend only
mana produced by creatures" (Myr Superion) and {S}, "mana from a snow source" (CR 107.4h; Icehide Troll), none in content
yet; restricted mana is told apart by its restriction alone, since no card yet tells it apart by source too. The benchmark
(`cantrip-bench --games 128`, "engine answering", 2026-10-17, main 13a3944f against the branch, three back-to-back pairs):
9.82/9.93, 9.53/9.92, 9.81/10.06 us, about 2.5% for keeping every unit's source and recording each payment.

### Creatures that convoked a spell

A creature tapped to pay for mana in a spell's total cost convoked it (CR 702.51a, 702.51c). `actions::pay_mana_used` records
`Event::Convoked { creature, spell }` as it taps one for convoke, right after its `Tapped`, so a spell's come before its
`Cast`; a creature tapped for mana or for improvise while paying convoked nothing. It is recorded only, and costs nothing
where nothing convokes. "Creatures that convoked it" asks `History::convoked(object)`: the creatures, in the order they were
tapped, of the spell `object` is, or of the spell a permanent was (its move from the stack, CR 608.3), counted whether or not
they are still on the battlefield (Knight-Errant of Eos's ruling) and read through `chars` as they are now or as they last
existed (CR 608.2h) - the record copies nothing out of them. A copy of a spell was not cast, but an effect of it that refers
to the objects used to pay its costs uses the original's (CR 707.10, as Fling's example has it): a copy, a copy of a copy and
the token a copy of a permanent spell becomes (CR 707.10f, whose move from the stack names the copy) read the original's
creatures. Venerated Loxodon, Zephyr Singer, Knight-Errant of Eos, Ancient Imperiosaur, Lethal Scheme and Everything Comes
to Dust print it; `cantrip-rules/src/convoked_tests.rs` proves it over test cards. The benchmark (`cantrip-bench --games 128`,
"engine answering", 2026-10-24) measured 14.70, 14.82, 14.86 us before and 15.29, 14.32, 14.29 us after: within the noise.

### Assist

A spell with assist (CR 702.132a, `PayWith::assist`, content `"assist": true`) whose total cost, once locked in (CR 601.2f),
has generic mana is paid in the rule's order (`actions::assist`): its caster may choose another player in the game
(`Purpose::Assist`, then `ChoosePlayer` with more than one); that player has a window to activate mana abilities, then the
caster has one (`PayCosts` with `AssistStep::HelperActivates` / `CasterActivates`, `ConfirmCasting` closing it, asked only
while the player has one to activate); then the chosen player announces how much of the generic mana they pay, no more than
it and than their pool could pay (`HelperAnnounces`), and pays it from their pool (`HelperPays`, no cancelling another's
spell); then the caster pays the rest (`CasterPays`) - with no mana ability offered to either once paying has begun
(`Helpers::abilities_closed`, CR 601.2g). The assisting player's mana is mana spent to cast it (`Obj::spent`, its
`ManaSpent` records, what its restrictions do to the spell). The offer counts on the most any other player could pay of
the generic mana (`assist_could_pay`), so a cast its caster could not pay alone is offered.

Prototype, surge, freerunning (with the damage event's source row) and assist together measured 7.40, 7.47, 7.49 us per
decision against 7.42, 7.52, 7.44 us before, three alternating rounds of 128 games (2026-10-27): no change the noise shows.

### Spells in an inset frame: Adventures and Omens

An adventurer card (CR 715) and an omen card (CR 720) print a second set of characteristics in an inset frame:
`CardDef::adventure` and `CardDef::omen`, from the content keys `adventure` and `omen`, and `CardDef::inset_faces` names
either with the way it is cast by (`WAY_ADVENTURE`, `WAY_OMEN`) and the face a granted way names it by. The card is cast as
that face wherever the card itself may be cast - from the hand, or from elsewhere by a permission, by its own say-so or by an
effect's (Yawgmoth's Will), for what the permission says to pay (CR 601.3, 715.3, 720.3) - but not as an Adventure by the
permission an Adventure gives the card it exiled (CR 715.3d). Only the face is judged to see whether it can be cast, at its own
speed (CR 715.3a, 720.3a) - which a permission to cast spells as though they had flash reads too, through a view of
the face (`actions::Speed`, as for a split half, a Room's door, a back face and a bestowed Aura) - and on the stack it is that face alone (`Obj::copy`, CR 715.3b, 720.3b), which a copy of it and a
choice of new targets for it read too (CR 715.3c, 720.3c). As it resolves, an Adventure is exiled on its adventure (CR 715.3d)
- into its owner's exile, by and for its controller (`Obj::on_adventure`), who "may play it" though another player owns it
(an Adventure cast by Null Summoner's permission) - and an Omen is shuffled into its owner's library by its controller (CR 720.3d); a copy of an Omen still shuffles its owner's
library and ceases to exist. Countered, or with every target gone, either goes to the graveyard. The host sends the face as
`ObjectView::inset_spell`, printed beside the card. A land card with an Adventure (Lindblum, Industrial Regency) is cast
only as the Adventure, never as itself (CR 305.9), and exiled on its adventure it is played from exile as a land play (CR
715.3d). A static ability about "each X card in your graveyard" (Iroh, Underworld Breach) judges the card, which has only
its normal characteristics there (CR 715.4), and takes the cost from the face cast. From the top of a library a permission offers each face it allows as it offers the
card (`Offer::by_route`): Bolas's Citadel casts Stomp for Stomp's mana value in life, and a filter
(`PlayRule::LibraryTopSpells`) judges the face, not the card.

### Costs to attack

"Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you" is a
cost to attack (CR 508.1d, 508.1h), a `CardDef::attack_costs` entry (`AttackCostRule`) asked, while its permanent is on
the battlefield with its abilities, of each creature that may attack and each player, planeswalker or battle it may
attack: the cost of that one attack, or nothing. `cantrip_mechanics::statics` has "attacking you" (Ghostly Prison,
Propaganda, Windborn Muse; Dáin, Lord of the Iron Hills while its controller has an enduring story) and "attacking you or
planeswalkers you control" (Baird, Steward of Argive, Archon of Absolution). The declaration (`turn::declare_attackers`)
follows CR 508.1: the question carries each attack's cost (`RequestKind::DeclareAttackers::costs`, every ability's
added up, so two Ghostly Prisons ask {4} a creature, and `can_pay`, advice on the mana the player could pay); the chosen
creatures are tapped (508.1f); the total is locked in (508.1h) and paid with the mana abilities offered while it is owed
(508.1i-j, `PayCosts` with the `attackers` it pays for). A payment given up (or one that cannot be completed) makes the
declaration illegal: everything since it is reverted, mana abilities included (CR 733), and attackers are declared again.
Requirements on a declaration of attackers are counted as CR 508.1d says, as blocks are (below): `turn::attack_rules` puts
one `RequiredAttack` per source in the question's `AttackRules` - each instance of "attacks each combat if able"
(`Chars::must_attack` counts them, printed or gained in layer 6), two for each player who goaded the creature (attack; attack
a player other than that one, CR 701.15b-d - goad is an effect, read from the effects, so losing abilities keeps it), and one
per "attacks [that player or permanent] if able" (`EffectKind::MustAttackAt`: Ursine Monstrosity's player this combat,
`Duration::EndOfCombat`; Gideon, Battle-Forged's planeswalker during the creature's controller's next turn,
`Duration::DuringControllersNextTurn`, in force only during the first turn its controller then begins after both the effect
and their gaining the creature (`controlled_since`) -
asked of the history's `TurnBegan` records, `turn::in_force` - spent with that turn once attackers are declared or its
cleanup comes, whoever controls the creature afterwards, `turn::spend_next_turn_effects`). An aim that is not among what may be attacked (the planeswalker gone) obeys nothing. The declaration must
obey as many as any declaration can attacking for free (a cost is never a requirement): a goaded creature told to attack its
goader may attack the goader or another player, each obeying two of three. `AttackRules::judge` (`cantrip_state::attacks`) is
the one validator - complete, issue, addable, cost - that the core, the scenario harness and the host (`CheckAttacks`) use,
and `GameClient.Answer` settles every declaration of attackers with the host. A restriction on one attacker at a time lets
`AttackRules::solve` add up each attacker's best free attack. A cap on how many creatures may attack one player,
planeswalker or battle (CR 508.1c, `CardDef::attack_limits`: Tomik, Orzhov Lawmage's and The Eternal Wanderer's "No more
than one creature can attack this planeswalker each combat", Judoon Enforcers', Mirri, Weatherlight Duelist's while
tapped) is room the attackers share, so the most is no longer each creature's best added up but a maximum-weight assignment
of creatures to targets with the caps as capacities (`attacks::Assignment`): each creature takes its best attack where
nothing caps it, and the room at the capped targets goes where it obeys the most beyond that, by successive augmenting
paths - exact, and polynomial in the creatures and targets, so sixty creatures that attack if able against seven capped
targets are settled at once - and a creature that must attack a capped target is not displaced by one that need not.
`AttackRules::limits` carries each target's cap (the smallest where two apply), `judge` refuses a declaration over one
(508.1c) and the most it is measured against counts them, and `addable` asks for each attack whether some declaration
extending the declaration with it still reaches that most. One best assignment of the creatures not yet attacking
answers that for every attack: a creature taken out frees its place, which goes where it adds the most (`fill`), and a
creature put at a full capped target makes room there the way that loses the least (`shed`) - each the best change of
an optimal flow for one unit of capacity more or less, so exact - in time quadratic in the creatures (400 creatures
against six capped planeswalkers: well under a millisecond). The requirements are read once per question into a table of
what each attack obeys. With no cap among the targets both are the sums they were. The
cap is on the declaration alone: a creature put onto the battlefield attacking, or stated to be attacking, is not under it
(CR 508.4c). The board is searched for these only as attackers are declared. The
benchmark measured 5.73 us per decision before and 5.76-5.85 us after (2026-10-16, the same noisy machine); the caps
(`attack_limits` and the assignment) 10.29 and 10.29 us against 10.25 and 10.41 us on main, alternating (2026-10-25); with `addable` from one best
assignment and the AI's attack masks, 10.60 and 10.79 us against 10.78 and 10.85 us on main (2026-10-25).

Optional costs to attack (CR 508.1g) are a static ability of the attacker and the "when you do" ability linked to it (CR
607.2h), kept as one triggered ability (`paid_as_it_attacks`, `TriggeredDef::attack_option`: `AttackOption::Enlist`, CR
702.154; `AttackOption::Exert`, CR 701.43d), so each instance has its own cost and its own trigger (CR 702.154d). Once the
chosen creatures are tapped, `turn::choose_attack_options` asks about each instance each attacker has now (lost abilities
offer none), in declaration order: enlist as a choice of up to one creature (`Purpose::Enlist`: untapped, controlled by the
attacking player, not attacking, free of summoning sickness, and not chosen for another enlist), exert as a yes or no. The
choices are locked in with the mandatory costs (508.1h); after the mana is paid (508.1i) each creature to be enlisted must
still be untapped, or the declaration is illegal and taken back as an unpaid one is; then each is tapped and each exerted
attacker exerted (508.1j). `Event::PaidToAttack` (attacker, option, which instance of which ability, the creature tapped) is
emitted in the declaration's batch, so the linked ability - whose trigger the core owns (`settle::paid_to_attack`) - goes on
the stack with the attack triggers, about the creature enlisted; "whenever this creature enlists a creature"
(`cantrip_mechanics::enlist::THIS_ENLISTS`) reads the same event. Enlist's +X/+0 reads X as it resolves (CR 608.2h).

### Blocked by at most one creature

"Can't be blocked by more than one creature" (CR 509.1b) is `Keyword::CantBeBlockedByMoreThanOne`, an evasion ability when
the creature prints it, so "loses all abilities" takes it (a content `keywords` entry: Stalking Tiger, Charging Rhino, Ironhoof
Ox, none in content yet; Sunder Shaman and Bristling Boar print the same). Another permanent's "enchanted creature ... can't be
blocked by more than one creature" (Meltstrider's Resolve; Familiar Ground, Challenger Troll and Vorrac Battlehorns say the
same) gives the creature no ability - there is no "has" or "gains" (CR 113.1a) - so it is never in the creature's keywords,
which are the abilities it has ("creatures with no abilities", Muraganda Petroglyphs, asks them): it is that permanent's
`CardDef::blocker_limits`, read with the block rules' sources (`turn::maximum_blockers`), so the creature's losing its
abilities (CR 613.1f) leaves it and the source's losing its own ends it. "Can't be blocked" follows the same split: printed or
gained, `Keyword::CantBeBlocked`; another permanent's (Cryptic Coat; Wrecking Ball Arm's "by creatures with power 2 or
less"), that permanent's `CardDef::block_rules`; a resolved "it can't be blocked this turn" (Escape Tunnel, Manifold Key,
Creeping Tar Pit), `EffectKind::CantBeBlocked`, an effect on the creature that `turn::can_block` reads (CR 611.2c), as
`EffectKind::CantBlock` is; and a resolved "can't be blocked this turn except by ..." (Gingerbrute's "except by creatures with
haste"), `EffectKind::BlockRestriction { card, index }`, which names the `index`th of the `CardDef::lasting_block_rules` of the
card that made it - a `BlockRule` asked of each possible block, with the effect's `affected` object as its source, while the
effect lasts, as `EffectKind::SpellCostChange` names a `lasting_cost_changes` entry. It is read beside the
static block rules in `turn::can_block`, but from the effects, not from a source permanent's abilities, so neither the
creature's nor the source's losing abilities ends it. `Chars::maximum_blockers` is the counterpart of `minimum_blockers`: a declaration giving an
attacker more blockers is refused (`validate`, CR 509.1b), and an attacker whose minimum is above its maximum (menace as well:
the restrictions are cumulative) is offered no block at all (`turn::declare_blockers`). The question carries both per attacker
(`DeclareBlockersPrompt::maximum_blockers`, 0 for any number); the web client sends at most that many of the creatures picked up
at the attacker clicked and says why the rest are still held; the tactical, random, automatic and language-model seats never
declare more. A multi-pair refusal in a scenario is judged by the engine's check of that declaration.

### Blocks an effect requires

A requirement to block (CR 509.1c) is one `RequiredBlock` of the question's `BlockRequirements`, one for each effect or static
ability and each creature it is about (CR 509.1c counts requirements: two Lures on a creature are two of each able blocker,
two "blocks this turn if able" on a creature two): a blocker and an attacker it must block; a blocker with no attacker, which
must block whichever attacker it blocks; or an attacker with no blocker, which must be blocked by any creature ("must be
blocked if able", obeyed once however many block it). Lure's "all creatures able to block enchanted creature do so" is a
`CardDef::block_requirements` entry (`BlockRequirement`), asked of each block the restrictions allow with the permanent as its
source (`static_rules::block_requirements`, only while a permanent with one is on the battlefield with its abilities); "target
creature blocks this turn if able" is a resolved effect, `EffectKind::MustBlock` (an effect, not an ability: a creature that
loses its abilities still must), one requirement per effect of a creature that may block something. A creature that is
tapped, can't block or may block no attacker is required nothing. Costs to block (CR 509.1d) are data in the question
(`BlockRequirements::costs`, one `BlockCost` per block that costs something; no effect makes one yet) and the verdict carries a
declaration's total; `most` counts free blocks only, since no requirement is obeyed by paying (CR 509.1c). Paying would reuse
attack costs' payment (CR 509.1d-f mirrors 508.1h-j). `turn::block_rules` puts the requirements, each attacker's fewest and most
blockers (`limits`), the costs and the bands in the question, and `BlockRequirements::solve` (`crates/cantrip-state/src/blocks.rs`)
finds the most of them any declaration that breaks no restriction obeys by a cut search over the blocks of the attackers a
requirement can be obeyed through, with one such declaration (`suggested`). A branch is bounded by what the creatures still
to come could obey and by what the attackers' room for blockers lets them obey, whichever is less, and the search ends once a
declaration reaches the bound - so Lured attackers that can't be blocked by more than one creature each stay quick at any
size hand play builds.

One validator judges every declaration: `BlockRequirements::judge` gives a `BlockVerdict` - `complete` (it may be declared as it
is), `issue` (why not: the creature, the words, the other creature, the CR rule, for a caption), `cost` and, when asked,
`addable`, the blocks some legal declaration extending it has, each found by `solve` with the declared blocks fixed. The core
checks every answer with it (without `addable`, a walk over the blocks), the scenario harness picks among like-named blockers
and judges refusals with it, and the host gives it to a client as the player declares (`CheckBlocks`, docs/host-protocol.md).
The web client holds any set of blocks and offers exactly what the verdict adds, and every client's declaration is settled
with the host by `GameClient.Answer`, so a new restriction, requirement or cost changes the engine alone. Not built, as no card
in content needs it: the static "blocks each combat if able" (a source of the same no-attacker requirement), "must be blocked if
able" (whose requirement already fits), "this combat" durations, an effect that makes blocking cost something, and a creature
that can block additional creatures (the validator's one-attacker-per-blocker rule, CR 509.1a).

### Banding

Banding (CR 702.22) is a core keyword, `Keyword::Banding` (Benalish Hero, Mesa Pegasus, plain cards). As attackers are
declared (CR 508.1e, before they tap), `turn::declare_bands` asks the active player, for each attacker with banding not yet in a
band, which other attackers attacking the same thing (CR 702.22d) and in no band join it: a `ChooseCards` of
`Purpose::Band`, up to all of them, which `validate` refuses with more than one creature without banding (CR 702.22c). Each
`Attacker` keeps its `band` number for the rest of combat (CR 702.22e); one that leaves combat is no longer `live` and so out
of it (CR 702.22f). A creature declared blocking one member of a band blocks every member (CR 702.22h, `declare_blockers`), its
restrictions judged on the declared block alone, as the rule's example has it. In the damage step each creature with a choice
has a `DamageDivision`: an attacker blocked by two or more, or a trampler past lethal (CR 510.1c, 702.19b), and a blocker
blocking two or more (CR 510.1d) - which only a band makes. Banding moves a division to the other player (`banding`): an
attacker blocked by a creature with banding is divided by the defending player among its blockers, past lethal and none to
the player (CR 702.22j); a blocker blocking a creature with banding by the attacking player among the creatures it blocks
(CR 702.22k). The active player divides first, then each defending player (CR 510.1). The engine's suggestion for a banding
division spares: all of it on one creature it would not kill, or on the one worth least by power and toughness; the client
captions it as the opponent's creature's damage, and the attacking band carries its number to the view (`BAND 1` on the
card). "Bands with other" (CR 702.22b) is not built: no card in content has it.

### The regular combat damage step

The quantity assigned is read by the core (`static_rules::combat_damage_of`, CR 510.1a), shared by attackers, blockers
and assignment choices. `CardDef::damage_by_absolute_power` selects creatures that assign as though negative power
were positive (CR 609.4, Loot, the Anomaly); only this combat calculation uses the unsigned magnitude, with actual
power unchanged everywhere else (CR 107.1b). A rule assigning by toughness uses toughness instead, without applying
an exception about power to it.

Combat assignments themselves leave out a departed player (CR 800.4e), not merely the later damage operation.
`turn::combat_defender_present` and `combat_assignments` exclude that defender from an unblocked or blocker-free
trample assignment and from a trample division's options; remaining creatures still receive assignments normally.
Both damage steps use the same procedure.

The combat state is the one truth of what is attacked. An `Attacker` names what it attacks (`target`) and its defending
player (`defending`, CR 508.5), fixed as it begins attacking. A planeswalker or battle being attacked is removed from
combat the moment the rules say (CR 506.4), and the creatures attacking it go on attacking nothing - `target` becomes
`Target::None`, `defending` stays - so they may still be blocked (by that player, `declare_blockers`) and, unblocked,
assign no combat damage (CR 506.4c, 510.1b): the damage step asks nothing of the permanent again, and one that is a
planeswalker again, or its first controller's again, by then is still not attacked. `ops::remove_from_combat` is the
one place combat forgets a permanent - leaving the battlefield, phasing out, regenerating while attacking or blocking,
a change of controller (`ops::write_controller`, CR 613.1b) or of a battle's protector (the state-based action of CR
704.5x-y); `ops::settle_combat` removes one that has stopped being what it is attacked as, by its card types as the layers have
them. `Attacker::attacked_as` records that - planeswalker, battle or both, as it was attacked, kept up to date while it
stays attacked (one that becomes both is attacked as both) - because CR 506.4e is about a permanent that is both as it
changes: one that was both stays while it is either (a planeswalker no longer a battle only while its protector controls
it), while one that was only a planeswalker and becomes only a battle, or the reverse, is removed (CR 506.4). It is asked
wherever control is settled - every event, every effect begun, the end of an instruction,
the state-based actions (`settle_lasting_and_control`) - and at every change with no event (`Cx::state_changed`), free
while nothing attacks. The same check removes an attacking or blocking creature that has stopped being a creature (CR
506.4: an artifact Ensoul Artifact or March of the Machines animated, once the enchantment leaves), for good, though it
be a creature again before damage; its blocks end with it, and a creature it blocked stays blocked. The two are apart
(CR 506.4d): a planeswalker creature that blocks and is attacked keeps being attacked once it is no longer a creature,
and keeps blocking once it is no longer a planeswalker. A battle still being attacked
keeps its departed protector through combat (CR 704.5x). A card asks a creature's defending player of the creature, never of what it attacks
(`Query::defending_player(attacker)`, CR 508.5): its `Attacker::defending` while it attacks, else the one recorded on
its last `Event::Attacks` - so a planeswalker that has changed controller since does not make its new controller, or
the attacking player, the defending player (Annihilator, Generous Plunderer, Greatsword of Tyr, Outland Liberator,
Robber of the Rich). `defending_player_of(state, target)` is only the core's, as a creature begins attacking.
The departed-recipient and assignment checks measured 13.86 us per decision before and 14.05 after on 128 games, in
back-to-back runs of the two builds (2026-10-24; shared-machine load, no speed claim).

CR 510.4: the regular combat damage step after a first-strike step is for the attackers and blockers that had neither first
strike nor double strike as the first-strike step began, and those that have double strike now. `Attacker::first_strike` and
`Block::first_strike` record which had one of them as that step began; a creature that loses first strike after dealing
first-strike damage deals no more, and one that gains it only afterwards deals its damage in the regular step.

### Landwalk

Landwalk (CR 702.14) is an evasion ability, a keyword per basic land type: `Keyword::Islandwalk`, `Plainswalk`,
`Swampwalk`, `Mountainwalk`, `Forestwalk`, which `turn::can_block` reads with the land type each names
(`Keyword::LANDWALKS`): no creature of a defending player who controls a land of that type, as the layers have it now, may
block it (CR 702.14c), whatever landwalk the blocker has (702.14d). An ability that grants one names its land type through
its source's words (CR 612.2: Lord of Atlantis with Island changed to Swamp grants swampwalk, `Keyword::landwalk`). A
printed landwalk follows a text change of its land type in layer 3 (Magical Hack's "swampwalk" to "plainswalk" on Bog
Wraith). Another kind of landwalk (nonbasic, snow) is another keyword.

### Can't attack unless

A static ability that keeps a creature from attacking (CR 508.1c), "this creature can't attack unless ...", is a
`CardDef::attack_rules` entry (`AttackRule`, the creature asked with the permanent as its source), the counterpart of
`block_rules` (CR 509.1b), which "can't block unless ..." is. `static_rules::attack_rules_allow` asks the permanents on the
battlefield that have one and have not lost their abilities (CR 613.1f), only when some card in the table has one; a creature
it refuses is not offered as an attacker (`turn::declare_attackers`), so no seat can declare it, and the host lists "Can't
attack." with its granted abilities while that holds (nothing says "can't block": a block rule is asked of a pair, and the
blockers question offers only the pairs it allows). It is judged as attackers are declared: an attacking creature whose
condition fails later stays attacking (CR 506.4, Patchwork Beastie's ruling). Patchwork Beastie's delirium; Sea Serpent's and
Dandân's "unless defending player controls an Island" are the same rule in a duel. Not built, as no card in content needs it: a
rule that refuses some of one creature's attacks and not others (the prompt's `Forbidden` pairs, which the clients already
read, are never filled).

### Attacking as though it didn't have defender

A creature with defender can't attack (CR 702.3b). "Can attack as though it didn't have defender" is a permission about
that one rule (CR 609.4): the creature keeps defender for every other purpose (a "target creature with defender" may still
choose it), and nothing else that stops an attack is lifted - tapped, summoning sick (CR 508.1a), an effect saying it can't
attack (CR 508.1c). `static_rules::may_attack_despite_defender` is the one place it is asked, by the declaration of
attackers (`turn::declare_attackers`, last among the checks, so only of a creature that could otherwise attack) and by the
host (`Game::may_attack_despite_defender`): a resolved effect about the creature (`EffectKind::CanAttackDespiteDefender`,
Assault Formation's {G} ability), or a static ability of a permanent on the battlefield with its abilities
(`CardDef::attacks_despite_defender`, an `AttackDespiteDefenderRule` asked of the creature: Demon Wall's "as long as this
creature has a counter on it"; Rolling Stones' "Wall creatures", High Alert's and Ghalta's "creatures you control", Animate
Wall's "enchanted Wall" and Skyclave Sentinel's "as long as it has a +1/+1 counter" are the same rule, none in content yet).
It is judged as attackers are declared: a creature whose permission ends after it attacks stays attacking (CR 506.4). The
host lists "Can attack as though it didn't have defender" with the creature's granted abilities while it applies (with the
effect's duration), which the tactical bot reads where it guesses who can attack. The benchmark is unchanged within the
noise (2026-10-16, alternating with the build before: 6.61, 7.81, 7.52, 6.38 us before and 7.05, 6.52, 7.79, 6.29 us
after).

### Speed

Speed (CR 702.179) is `Player::speed`, `None` until something sets it (CR 702.179b). Start your engines! is
`CardDef::start_your_engines`, a static ability read by a state-based action (CR 704.5aa, `speed::engines_start`): a player
with no speed who controls such a permanent that has not lost its abilities gets speed 1, so gaining control of one counts
and losing it changes nothing (the rulings). The player's inherent triggered ability (CR 702.179d) has no source: it is the
rules' own, `intrinsic::SPEED_UP`, queued from `ops::change_life` when an opponent of the active player loses life and the
active player's speed is below 4, from an object in no zone as the monarch's are (`designations::queue`). "Only once each
turn" is asked of the history: the turn's `Event::Triggered` records of that ability (only the active player's can trigger in
a turn). "If your speed is less than 4" is its intervening "if" (CR 603.4). `Game::increase_speed` raises speed, never past 4
(CR 702.179c, e; the release notes), and records `Event::SpeedChanged`. Effects read it with
`cantrip_mechanics::start_your_engines::{speed, has_max_speed}` (0 for none, CR 702.179f); "Max speed — [ability]" is
`max_speed::max_speed(text)`, a layer-6 static granting the card's `delayed[0]` while its controller has speed 4
(CR 702.178a), so below that the permanent has no such ability at all. A max speed ability granting one that works from
another zone (CR 702.178b) waits for a card. The host sends `PlayerSummaryView::speed` and the client draws "Speed N" or "Max
speed" beside the life total. The benchmark is unchanged within the noise (2026-10-16, a very busy machine, alternating
with the build before: 13.54, 22.13, 16.20, 18.59 us before and 12.16, 22.53, 14.30, 13.31 us after).

### Cases and the solved designation; suspect

Solved (CR 719.3b) is a designation, `Marks::solved`: no ability and no copiable value, so a permanent that becomes a copy of
a solved Case is not solved, a solved Case that becomes a copy of another Case stays solved, and a Case that leaves and
returns is a new object without it (CR 400.7). `Game::solve` (CR 719.3a) gives it to a permanent on the battlefield that
lacks it and records `Event::Solved`; `Query::is_solved` reads it, and a static ability of the layers reads it off its source's
row. `cantrip_mechanics::solved` is every Case's two keyword abilities (CR 702.169): `to_solve(condition, text)` is "At the
beginning of your end step, if [condition] and this Case is not solved, this Case becomes solved", both halves an intervening
"if" (CR 603.4); a "Solved" triggered ability is `solved::triggered`, which triggers only if the Case is solved
(`TriggeredDef::only_if`, asked with the trigger event and never again, CR 702.169c); an activated one is `solved::ability`
("Activate only if this Case is solved", CR 702.169d); a static one of play rules is `solved::play_rules(&[..])`, a
`PlayRule::While` ("as long as", CR 702.169b) that every reader of play rules walks through (`actions::each_play_rule`). The
host greys a Case's "To solve" line once it is solved and its "Solved" line while it is not, and says SOLVED on the card.

Suspect (CR 701.60) is `Game::suspect`: an `EffectKind::Suspected` on the permanent until it leaves the battlefield, read as
the designation (`Query::is_suspected`), whose layer-6 grant is menace and "This creature can't block" (CR 701.60c) at the
effect's timestamp, so "loses all abilities" later takes them while the creature stays suspected (the rulings). A suspected
permanent is not suspected again (CR 701.60d). `Event::Suspected` is recorded. Nothing yet ends the designation early ("is no
longer suspected"): no card in content says so.

"You may look at the top card of your library any time" (CR 401.2) is `PlayRule::LookAtLibraryTop` (Bolas's Citadel, Mystic
Forge, a solved Case of the Locked Hothouse): the host shows that card to its player (`Game::may_look_at_library_top`).
### Preparation

Prepared (CR 722.1) is a designation, `Obj::prepared`, given and removed by core procedures (`Game::become_prepared` /
`become_unprepared`, CR 722.3a-c). Becoming prepared creates a copy of the permanent's prepare spell in exile, linked to it
by `Obj::prepared_from`: its controller may cast that copy, and it is exempt from ceasing to exist (CR 704.5e), for as long
as the link lasts. Becoming unprepared, or phasing out, severs the link of the old copy for good (`ops::expire_prepared_copies`,
CR 722.3c, 611.2b, 702.26f) without ending it in the middle of an effect: the next state-based check ends it, and becoming
prepared again or phasing in prepared before then creates a fresh copy and never revives the old one.

Casting the copy consumes the preparation (CR 722.3c, 601.2i). The copy's old object keeps its `prepared_from` link
through the shared casting procedure; its permanent remains prepared throughout announcement and payment, and after the
as-cast observations and the completed cast record the live original permanent loses preparation, before cast triggers
are collected. Cancelling reverts the whole announcement, including mana abilities. A copy cast by a stolen permanent's
current controller is controlled by its caster (CR 601.2a), not its exile zone's owner.

### Graveyard order

A graveyard is kept in order (CR 404.2), and "if an effect or rule puts two or more cards into the same graveyard at the
same time, the owner of those cards may arrange them in any order" (CR 404.3). Every simultaneous move goes through
`ops::move_together`, so that is where it is asked (`arrange_graveyards`), whatever the cause - a mill, a discard, a
destruction, a state-based action, "put the rest into your graveyard" - and no card can put cards in together without it:
card code has no verb that orders a graveyard. Once the cards have arrived, in the order the move listed them, each owner
with two or more of them, in APNAP order (CR 101.4), is asked with the ordering question the library's ends use, a
`DistributeCards` with one destination, `DestinationKind::GraveyardOrder`, the cards listed top first (the host shows it as
an ordered graveyard pile); they are rearranged as answered before the move is announced. A token or a copy of a card
ceases to exist there (CR 111.7, 704.5d-e) and is not among them. A card's own question with a graveyard destination
(surveil's, Underrealm Lich's) chooses which cards go, not their order.

It is asked only while the order can be told apart: while some object of the game - in any zone, outside the game
included, or a copy - is a card whose text reads a graveyard's order (`CardDef::reads_graveyard_order`: Corpse Dance,
Shallow Grave; `cantrip-cards/tests/graveyard_order_matches_oracle.rs` holds it to the Oracle text). That is a guard on the
game state, every object row the game has had, not on the card table: without such an object nothing that can happen in
the game distinguishes one order of those cards from another, so the order they arrived in is as good as any the owner
could choose, as several effects that each prevent all of some damage are not ordered (CR 616.1, `replace::prevents_all`).
The question is the rules' own (`Cx::ask_rule`): a run that reaches it unanswered goes on with the order they arrived in and
is replayed with the answer. The bots answer it as they answer any placement, in one pass over the cards.

### Cards outside the game

Outside the game is not a zone (CR 400.11), but the cards a player owns outside it - in a match, their sideboard (CR 100.4,
400.11a), whose owner is the player who started the game with it there (CR 108.3b) - are what "a card you own from outside
the game" chooses among, so the state keeps them as objects in a per-player `ZoneKind::Sideboard` (`Zone::sideboard(p)`),
dealt from `Setup::sideboards`. Nothing walks it but what brings a card in (CR 400.11c): `Query::outside_the_game(p)` lists
it, and `Game::bring_into_hand` moves cards from it to their owners' hands (CR 400.11b), each a new object with a
`ZoneChange` from the sideboard, in the game until it ends. Only its owner may look at it: it is a hidden zone, and the host
shows its contents to that seat alone (a `ZoneKindView::Sideboard` zone per seat, the other seat told only the count). A
Limited sideboard also holds any number of basic lands (Magic Tournament Rules, and North Wind Avatar's ruling):
`Setup::basic_lands_outside` names the event's basics, each sideboard gets one of each, and one brought in is replaced
there by another (`GameState::outside_supply`, fixed at the deal). The server sends each seat's sideboard with its deck, and
the Limited events (`BoosterPod`, `DraftPod`) the five basics; a card the engine does not have refuses the deal, in the
sideboard as in the deck. `cantrip_mechanics::outside::may_put_card_into_hand` is North Wind Avatar's and Death Wish's "you may
put a card you own from outside the game into your hand", asked as `Purpose::PutIntoHand` over the sideboard's cards, which
the client captions as coming from outside the game. Not built, because no card in content needs them: a revealed fetch of
a quality (Burning Wish, Living Wish, Karn, the Great Creator), learn (CR 701.48: a Lesson card from outside the game,
revealed, `Subtype::Lesson`, which would bring it in with this same verb), and casting from outside the game (Spawnsire of
Ulamog). A companion is below. The
benchmark is unchanged within the noise (2026-10-16, alternating: 57.4 (an outlier), 12.06, 14.10 us before and 13.87,
12.45, 11.12 us after).

### Companion

Companion (CR 702.139) is a keyword ability that functions outside the game. A card with it declares its condition,
`CardDef::companion`, a `CompanionCondition` over the `StartingDeck` (its cards and the format's minimum deck size, CR
702.139b), and a test (`companion_matches_oracle`) holds every card whose text has a "Companion -" line to declaring one;
what several conditions say alike is `cantrip_mechanics::companion` (`each_permanent_card`: Lurrus of the Dream-Den, Zirda,
the Dawnwaker). The player's choice to reveal a companion (CR 103.2b) is made at deck building: a deck names its companion
(`Setup::companions`), dealt into the sideboard beside the rest, and the server refuses a deck whose companion has no
companion or whose condition the deck does not fulfil (`DeckValidation`, asking the host's `Companions`, which runs the
card's own condition through `eligible_companions`). `Game::new` judges the condition against the library as dealt and keeps
the companion to reveal (`Player::companion_to_reveal`); once the starting player is known (CR 103.1) the opening reveals each
in turn order (`turn::reveal_companions`): `Player::companion` and an `Event::Revealed`, nothing asked of anyone, so no question
or its absence says anything of a sideboard. One whose condition the deck does not fulfil is a sideboard card, unrevealed. The
revealed card stays outside the game, and every seat sees it there (`revealed_companion`, `visibility::observable`). Once
during the game, with priority in a main phase of their turn and the stack empty, its owner may pay {3} and put it into their
hand (`ActionKind::TakeCompanion`, CR 116.2g): a special action, no ability, so nothing that changes or forbids activations
touches it and mana for abilities only can't pay it; offered only where the {3} can be paid, and only while the revealed card
is still outside the game - once it is in the game it is a new object (CR 400.7), so the action is had once, and a card a wish
fetched instead is no longer there to take. It is brought in by `bring_into_hand` (CR 400.11b) and stays (CR 702.139c). The
format's minimum deck size, which Yorion, Sky Nomad's condition names, is `GameState::minimum_deck_size`, which the server says
outright (`Setup::minimum_deck_size`, the host's `MinimumDeckSize`): sixty in Constructed, forty in Limited or the event's own,
a hundred in Commander (CR 100.2a-b), and none in Freeform - the default format, with no deck size restriction at all (a deck of
no cards is a Freeform deck), so Yorion's "twenty cards more than the minimum" is twenty there. The server checks every deck against its table's format before dealing it, and each
game of a match after sideboarding (`DeckRules`, shared with the deck editor, and `DeckValidation`).
The benchmark is unchanged within the noise (2026-10-23, four alternating rounds, engine answering: 9.50, 9.53, 9.37, 9.20 us per decision before and 9.48, 9.32, 9.23, 9.28 us
after).

### Keyword counters

A keyword counter gives its permanent the keyword it is named for (CR 122.1b): flying, first strike, double strike,
deathtouch, decayed, haste, hexproof, indestructible, lifelink, menace, reach, shadow, trample and vigilance
(`CounterKind::KEYWORDS`). The ability is a layer-6 effect (CR 613.1f) with the timestamp of its counters: each counter
gets one as it is put on, and every counter of its kind already there gets the same (CR 613.7c). `counters::put_together`
stamps the kind (`GameState::counter_stamps`, written only when a keyword counter is put on a permanent, and empty in
nearly every game); a kind the permanent entered with has the permanent's own timestamp. `layers::keyword_counters` puts
each kind the object has among its layer-6 effects as "gains [keyword]" at that timestamp, so "loses all abilities" takes
the keyword of a counter put on before it and not of one put on after (Abigale, Eloquent First-Year's ruling), and a
second counter of the kind put on after the loss gives the keyword back to them all. Counters of one kind give the
keyword once. Decayed (CR 702.147a) is a core keyword for this reason: the blocking rule reads its "can't block", and
the layers give an object that has decayed once layer 6 is done the rules' triggered ability (`intrinsic::DECAYED`:
"when this creature attacks, sacrifice it at end of combat", a delayed trigger at the next end of combat step), whether
the keyword is printed, granted or a counter's. Not built, because no card in content needs them: exalted counters
(exalted is a triggered ability each card prints, `triggers::EXALTED`), counters of a keyword's variants ("hexproof from
white", CR 122.1b), and keyword counters on a card in a zone other than the battlefield. Adding ten counter kinds made
every object 20 bytes larger (the dense counter array); the benchmark measured 6.12-6.56 us per decision before and
6.28-7.12 us after, alternating the two builds on a busy machine (2026-10-16), and as much with the layer-6 work turned
off, so the cost is the larger row.

### Legal moves as a first-class output

Every pending request comes with its legal answers enumerated, cheaply and in a stable order. The client's prompt
and the AI's move list are the same data.

## Interfaces

**The server.** `cantrip-host` implements the `IRulesHost` protocol as a sidecar (JSON lines over stdio,
[host-protocol.md](host-protocol.md)), so `GameSession`, events, the Discord bot and the web client do not depend on
the engine's language. The server finds the binary under `target/` when it runs from the repository, or by
`CANTRIP_NATIVE_HOST`. Calling the engine in-process through FFI is possible later if the sidecar shows up in
profiles; for human play it will not.

**Python (step 6).** `cantrip-py` exposes batches of games to a training loop: create N games, step them with a batch
of answers, branch, and encode each game's state for a seat into tensors. It encodes a redacted view, with hidden
cards masked or filled in from a belief model, never the true hidden state.

**Scenarios.** `cantrip-script` runs the `.cantrip` corpus in `scenarios/`; `cargo test` makes one test per file in
`scenarios/` (its build script lists the folder; nothing is registered). A scenario whose decks name a card the
engine does not have yet is skipped; one that can run must pass.

## Correctness

- **Phased-out source information (CR 113.7a, 608.2h, 702.15c, 702.26b,d,g).** Before any permanent in a simultaneous
  phase-out changes status, its complete characteristics, controller and text words are captured in the existing
  LKI arena. The phasing status links that row, not the object's zone-change identity. Rules queries and damage source
  quality/controller checks use it while the source is absent; projected characteristics and layer queries remain
  current. Phasing in clears the status/link and recomputes current effects; each later phase-out takes a fresh snapshot.
  The game-log tree journals the link/row and loop canonicalization hashes its contents, not its arena index.
- **The scenarios are the reference.** A card is done when it is exact and its scenarios pass; a scenario that is
  wrong by the rules is fixed, with the rule.
- **`CANTRIP_VERIFY=1`** runs the engine's own consistency checks while it plays; `tools\test.cmd` sets it.
- **Seeds are not swept.** Positions come from scenarios (CLAUDE.md); random self-play is fuzzing, kept separate.

## Performance

The C# engine measured 261 us and 380 KB allocated per decision on one thread, scaling to only 4.5x across 16 cores,
0.8 ms for a deep copy and 25-93 ms to branch at a prompt by replay; 58% of its time was the deep copy per
zone-change batch and 28% the layer recompute. The targets set then: 10 us per decision, no allocation per step,
14x on 16 cores, a 10 us copy.

`cargo run --release -p cantrip-bench -- --games 128` measures the engine on the same games every run ("engine
answering ... us per decision", one thread). The spike
(nineteen cards, no features) answered in 1.95 us. With every card and every rules feature it is **5.06 us** per
decision (2026-10-15), 5.90 us after ending the turn (CR 724) and the exact cleanup step (2026-10-16, 5.99 us before); 6.03-6.45 us after mana
restricted to abilities of [type] sources (CR 106.6), against 6.08-6.14 us before on the same, noisy, machine. Costs that depend
on the targets (CR 601.2c, 601.2f) and hexproof from a quality (CR 702.11d), 2026-10-16, three runs alternating with
origin/main: 5.80, 6.03, 5.83 us before and 5.95, 5.89, 5.82 us after (5.89 mean both). Craft's materials as a cost (CR 702.167a): 5.65-5.94 us against 5.52-6.24 us before, interleaved on a busy machine. The enduring story's look at the board as each event happens (CR 702.195c, 2026-10-16) measured 6.15-6.55 us against 6.01-6.37 us before, alternating the two builds. Stack items record each target's slot (`StackItem::spec_of`, 16 bytes more per stack item): 5.92-6.42 us (best 5.92) against 5.87-5.99 us (best 5.87) before (2026-10-16). The solved designation, suspect and the play-rule walk (`each_play_rule`), alternating with the build before them: 6.38, 6.83, 6.33 us against 6.36, 6.50, 6.08 us (2026-10-16, a busy machine). Damage redirection and damage that stays (CR 614.1a, 616.1, 514.2), seven runs alternating with origin/main on a busy machine: best 6.46 us against 6.36 us before, medians lost in the noise (2026-10-16). Static abilities reading the event history (the log handed to every layers entry point, CR 611.3a), six runs alternating with the build before on a busy machine: best 6.43 us against 6.34 us before, medians 7.0 and 8.1 us (2026-10-16). Static "loses all abilities", "blocked by at most one creature" and "can't attack unless" (CR 613.1f, 509.1b, 508.1c), alternating with the build before on a machine shared with other builds (2026-10-16): 14.72, 27.31, 10.57, 28.13 us after against 14.53, 25.60, 11.08, 147.72 us before - lost in the noise, as nothing new runs while no card with such an ability is on the battlefield. Blight in activated abilities, counters removed from among permanents and permanents' permissions to cast from exile (CR 701.68a, 118.3, 601.3), alternating with main on a machine shared with other builds (2026-10-16): 11.20, 10.97, 10.05, 15.58 us after against 10.91, 11.32 us before - lost in the noise. Ways given to cards in a hand (a lookup of granted ways for each card in hand), life and special actions in cost changes, sacrifice-any-number and `move_together`'s check for an object named twice (CR 118.9, 118.8, 116.2k, 601.2f), three back-to-back pairs against main on a machine shared with about twenty agents' builds (2026-10-17): 10.71, 14.05, 13.69 us after against 10.83, 20.86, 14.08 us before - no cost that the noise shows. Counters put by an effect
apart from a cost's or a turn-based action's, loyalty costs through the counter replacements and costs a counter ban makes
unpayable, with "can't lose life" (CR 614.1a, 606.4, 118.3, 119.8: the two changes measured together), against the build before on
a shared machine (2026-10-17): 10.88, 10.60, 12.39 us after against 10.56, 10.91 us before - lost in the noise. The last six feature landings had taken it from 6.93 to 7.5-7.8 us; a sampling profile found
the cost in the layers, not in the new features' own code:

- `chars()` rebuilt a card's printed values for every object on every question, and scanned the battlefield for
  static-ability sources on every call. Printed characteristics are now made once per card, and the source list once
  per position (the epoch cache). 7.76 -> 5.34 us.
- The journal is never dropped (the game-log tree keeps it) and an `Undo` was as large as an `Obj` (296 bytes); the
  big rows are boxed in it (192 bytes), 5.30 -> 5.06 us. The frame entries stay inline: boxing them too measured
  slower.

Left on the profile: `chars()` is still about a fifth of the time, mostly the 170-byte `Chars` copied out per call
(callers that read one field could borrow the printed values instead); `mana_ability_options`, `castable` and
`available` call it per permanent per priority pass; the journal's `Vec<Undo>` regrowth (a chunked journal) and
`Frame` clones (184 bytes with heap parts) are the next costs; `Obj` is 288 bytes, 112 of them a `Copiable` that
nearly every object leaves empty. A game allocates about 13 MB, because the path's journal is kept; per-field undo
would shrink it.

## Plan

1. **Measure the C# engine.** Done (the numbers above).
2. **Spike.** Done: nineteen cards passed all 49 of their scenarios at 1.95 us per decision, 143 times faster than
   the C# engine, and the spike chose re-run-with-recorded-answers for card code.
3. **The rest of the rules bricks** and
4. **the cards,** together. Done (October 2026): feature agents built one rules area each and ported the cards
   waiting on it; card agents ported cards with the core closed to them and reported what it lacked. Every card in
   content is ported.
5. **Switch the server** to the engine behind `IRulesHost`. Done; the C# engine is deleted.
6. **Python bindings and the AI work.** Next.

Cost reductions by colored mana (`CostDelta::less_mana`, CR 118.7) take off the cost's plain symbols of that color and
then generic mana. A reduction by hybrid mana is split by the payer as it is applied (CR 118.7e, `announce_reduction`,
`Label::ReduceAs`, `Label::ReduceGeneric`): one half of each of its hybrid and {2/C} symbols, and the advice tries every
split (`change_any`). A colored reduction that meets a two-color hybrid symbol of the cost (CR 118.7c: more of that color than
the plain symbols take) asks how that symbol is announced (CR 601.2b, `announce_hybrids`, `Label::PayHybridAs`): as the
color the reduction takes, it comes off; as its other half, it is paid and the reduction takes generic mana. Asked once
the targets are, since a reduction may read them; the advice tries every announcement (`announcements`,
`change_any`). {2/C} and Phyrexian symbols are announced before the cost changes (`announce_twobrids`,
`announce_life`).

Known gaps:

- Power-up abilities are not yet a class other cards can name ("power-up abilities of other creatures you control cost
  {3} less", "whenever you activate a power-up ability", "can be activated an additional time"): Hulk, Gamma Goliath,
  Marvel Boy, Wonder Man, Kang, Advancing the Spirit and Quinjet Technician wait on it.
- The web client cannot name colorless mana for a sacrificed land that could also produce a color (KI-114).
- Serving a scenario's handed-over position to the web client, to look at or play on from, went with the C# engine
  and is not rebuilt.
- Web-slinging cards that wait: Spider-UK ("if two or more creatures entered the battlefield under your control this
  turn") needs the controller a creature entered under, which the history cannot answer once control has changed (no
  event records a change of control); Arachne, Psionic Weaver needs a card type chosen as it enters (`Entering` has a
  color, a subtype, a player, a name, odd or even, no card type) and a prompt for it; Peter Parker // Amazing
  Spider-Man's back face grants web-slinging to the legendary spells its controller casts, a way a permanent gives to
  cards in hand (`GrantedWay::from` the hand), which no card in content gives yet.
- `web_slinging::way` is a way from the hand (`ZoneKind::Hand`), like every printed alternative cost. Cast from
  elsewhere at priority by a permission or the card's own say-so, a hand way is offered too (`castable`'s `elsewhere`;
  Spider-Sense from the graveyard under Yawgmoth's Will is a scenario, and sneak's instant-speed way likewise), unless its
  text names the hand (`WayDef::only_from`: warp, CR 702.185a, so a warped card is cast from exile for its mana cost); a cast
  made while an effect resolves paying its costs is offered the card's own ways open from where it is too (`Label::Way`).
- Mandatory costs to attack are mana only: one that asks for life (Sivitri, Dragon Master's +1), a Phyrexian symbol paid
  with life (Norn's Annex), tapping or sacrificing is not built, nor anything that changes a cost to attack. Optional costs
  to attack are enlist and exert (above); one with a condition of its own ("if it hasn't been exerted this turn, you may
  exert it", Combat Celebrant) or a mana part would extend `AttackOption`. No card in content needs them.
- No scenario reaches Questing Beast's unpreventable damage, Emrakul's extra turn on cast, or Tamiyo's -7 emblem.
- Several permanents one effect puts
  onto the battlefield enter together (`ops::put_onto_battlefield`, `Game::put_all_onto_battlefield*`: Show and Tell,
  Redshift's exhaust, Ghost Vacuum), each judged by its entering replacements against the battlefield as it was (CR 614.12)
  and each seeing the others enter (CR 603.6a). Cloaking or manifesting several cards from a single library is one at a time,
  as the rules say (CR 701.58e, 701.40e); cloaking cards from anywhere else (a hand, a face-down exile pile) puts them onto
  the battlefield together (`cloak::cloak`).
- Rooms (CR 709.5) are built: `Marks::doors` holds the unlocked designations, `layers::doors` gives a Room on the
  battlefield the name, mana cost and abilities of its unlocked doors (a copy effect's "except its name is X" keeps its
  name, CR 707.9b: the rule takes away only the halves' names), and the unlock special action (CR 709.5e, 116.2m) is given
  once layer 6 is done, one per locked door, as turning a face-down permanent face up is (`layers::unlock_actions`,
  `intrinsic::UNLOCK_LEFT`/`UNLOCK_RIGHT`, cost read off the door), so an effect that takes all abilities (Humility over
  an Opalescence-animated Room) does not take it. `Event::DoorUnlocked` is what "when you unlock this door" listens for,
  and a door's play rules (`CardDef::play_rules`, Steaming Sauna's "You have no maximum hand size") apply while it is
  unlocked (`actions::each_play_rule`). A copy of a Room spell becomes a token Room with the cast door unlocked, its
  copiable values the whole card's with the exceptions the spell had (`resolve_copy_as_token`, CR 707.10f, 709.5d). Not
  yet: a door with static abilities, replacement effects or an enters rule (the card table refuses one), instructions to
  unlock or lock a door (`Game::unlock_door` exists for the first, nothing locks), and "fully unlocks"; no card in
  content copies a Room spell, so only a Rust test (`room_copy_tests`) reaches that copy.

## Open questions

- **Checkpoints:** how often the live game keeps a full state checkpoint, so that resuming or jumping far through the
  tree does not replay from the root.
- **What a step is for search:** whether the engine exposes plan-level moves (a whole combat) or only primitive
  answers, with plans built above it.
- **Multiplayer:** the rules are written for two players today, and multiplayer (CR 800) must not be designed out.
- **Determinism across machines:** the same results on every platform the server and training run on, which rules
  out depending on hash map iteration order or unordered floating point.
