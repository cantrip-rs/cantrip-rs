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
- **An answer without stages redoes its run from the run's start**, so its node begins before the events the run
  recorded until it asked; the node keeps those events (`displaced`) and gives them back when the game goes up to its
  parent.
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
nothing else. A limit on what the cards chosen together add up to is part of the question: "up to two creature cards with
total mana value 6 or less" (CR 202.3) is a `ChooseCards` with `total: Some(TotalAtMost { values, most })`, each eligible
card's mana value read as the question is asked (`choose::cards_within_total`), and the client shows the running total and
refuses a card that would go over by the same numbers. Michelangelo's Technique asks it, through
`library::look_at_top_onto_battlefield` (which Aang, at the Crossroads and United Battlefront use without a total), and Lively
Dirge asks it from the graveyard ("up to two creature cards with total mana value 4 or less"); Ao, the Dawn Sky prints it too.

Targets can carry the same limit (CR 601.2c, 603.3d): "any number of target creature cards with total mana value X or less" is
a `TargetSpec` with `total_mana_value` (`TargetSpec::total_mana_value_at_most`, `COUNTED` for X), and wherever targets are
chosen - a cast or activation (`actions::choose_slots`), a triggered ability going on the stack (`settle::choose_trigger_targets`),
new targets for a copy (CR 115.7d) and a changed target (CR 115.7a) - `actions::within_total` takes out every candidate worth
more than the limit by itself and hands the question its `ChooseTarget::total`, which the validator checks as it does a
card choice's. The total is judged as the targets are chosen; as the spell or ability resolves each target is checked on its
own (CR 608.2b), so one gone leaves the rest. Fire Lord Sozin's reflexive ability is the slot that has it, X the count it was
given (`Game::reflexive_trigger_counted`); the client keeps the running total in the caption and leaves a candidate that
would go over unclickable.

### Characteristics

- **Computed on query** (`chars()`), through the layers (613), from the object's card, its counters and the
  continuous effects that apply.
- **Control-changing effects (613.1b, layer 2)** set the permanent's stored controller (`Obj::controller`, with
  `marks.control_base` the one it had before any applied). They are resolved `EffectKind::Control(player)` effects
  (`Game::gain_control`, `exchange_control`) and static abilities of `Layer::Control` made with `control_changing(affects,
  text)` ("You control enchanted creature": Mind Control, Treachery), whose source's controller gains what they affect,
  judged on what layer 1 made it, at the source's timestamp (an Aura's is when it became attached, 613.7e). The latest
  applying effect decides (`ops::refresh_control`, 613.7); new control is new for summoning sickness and takes the
  permanent out of combat. `ops::settle_control` works every permanent out again as the state-based actions are checked
  (and after effects end with their sources), repeating while one controller decides another; it costs a scan of the
  effects and the battlefield, free while nothing has ever changed control. A layer-2 ability applies though its source
  loses its abilities in layer 6 (CR 613.1). Durations end it as they end any effect: end of turn (`turn.rs`), a source
  leaving, a permanent untapping (`ops::end_lapsed_durations`), or a card's own "for as long as" condition
  (`Duration::AsLongAs(card, index)` naming one of `CardDef::lasting`, asked by `ops::end_lapsed_conditions` at each check
  of the state-based actions, and before the effect begins: one false then does nothing, CR 611.2b - Old Man of the Sea's
  "for as long as this creature remains tapped and that creature's power remains less than or equal to this creature's
  power"). "You may choose not to untap this ... during your untap step" is `CardDef::may_not_untap`: the untap step asks
  the active player of each such tapped permanent before anything untaps (CR 502.3).
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
- **Dependency (613.8)**: within each layer and sublayer, the effects that may apply to the object are merged in timestamp
  order (`layers::Merge`: a source's static abilities, those granted to others, resolved effects, the object's own granted
  abilities). An effect whose reach can change - a static ability's "affects", a text change (only where its word is) and
  "all creature types" (only to a creature or Kindred) - applies at once unless a later effect would change its reach or
  another such effect is still to come (`layers::waits`, so an anthem followed by resolved pumps costs a look at each);
  otherwise the rest go through `layers::by_dependency`. An effect depends on another if applying the other changes whether
  it applies to the object or does something to it (613.8a(b): Ygra's "other creatures are Food artifacts" on a Vehicle
  crewed later, Kaito's Ninja form, an impending Overlord's "isn't a creature", Artificial Evolution's Goblin to Dragon
  before a later Elf to Goblin), and waits for it; free effects go in timestamp order, as do the members of a loop among
  themselves (613.8b), and the order is worked out again after each (613.8c). Judged on the object being worked out, an
  effect that would change what another applies to once its own prerequisites have applied counts too (a chain:
  "creatures are artifacts" after "lands are creatures" after the effect that makes it a land), and an earlier effect that
  applied is asked again of the object as it found it with the later one applied, so its own application cannot hide the
  dependency (`in_order`'s snapshots: Life and Limb making a nonbasic Forest a Saproling before Blood Moon takes the Forest
  type - Blood Moon applies first and Life and Limb not at all); a dependency through
  another object alone is not seen (known issue KI-41). Of 613.8a(b)'s four criteria, "what it applies to" and "what it
  does" are judged here; "existence" is a static ability's taken away by "loses all abilities", ordered by
  `lost_to_statics`; no text change the engine has rewrites another effect's text. The words of rules text follow the
  same order (`Words::word`: the earliest change of the word as it is now, again and again). Characteristic-defining
  abilities apply first and apart (613.3, 613.8a(c)).
- **Type-changing effects (613.1d, layer 4)**: `AddTypes`, `SetTypes`, `AddSubtype`,
  `SetCreatureType`, `AddAllCreatureTypes` ("with all creature types", only to a creature or Kindred, 205.3d) and
  `LoseAllCreatureTypes`. Changeling's every creature type is a characteristic-defining ability applied first (613.3).
- **Static abilities granted by effects (613.1f, 613.7a)** are `granted_static` entries (`AbilityDef::Static`) of a card's
  `delayed` list, given with `EffectKind::AddAbility`. Once layer 6 has settled an object's abilities, those it still has
  (so "loses all abilities" later in timestamp order takes them) apply to it alone, with it as their source, in their own
  layer (7a onward, refused at table construction for an earlier one) at the timestamp of the granting effect, merged in
  timestamp order with the effects and other statics of that layer. One that affects other objects
  (`granted_static_ability`, The Irencrag's "Equipped creature gets +3/+3") makes the permanent a static source
  (`StaticSources::granted`) while it still has the ability, applied among the other sources' abilities at its timestamp.
- **"Loses all abilities" (613.1f)** comes from a resolved effect (`EffectKind::LoseAllAbilities`: Oko's Elk, The Wondrous
  Wasp) or a static ability (`loses_all_abilities(affects, text)`, a `StaticDef` with `loses_all`: Eaten by Piranhas; Darksteel
  Mutation, Kenrith's Transformation, Frogify and Humility print the same; `loses_all_abilities_except_mana_abilities` keeps
  the mana abilities the object has at its timestamp, 605.1: Blood Sun). Either is a layer-6 effect at its timestamp, so an
  ability gained after it stays (613.7). A permanent about to enter that such a static ability would strip is judged as it
  would be on the battlefield (614.12, below): its own `enters` replacements do not apply. Every rule read off a card - its static abilities from layer 6 on, cost changes,
  replacement effects, play rules, trigger, block and activation rules, granted ways, miracles - asks the one answer,
  `Query::lost_all_abilities` (`layers::lost_all_abilities`): the effects, then `layers::lost_to_statics`, the permanents such
  static abilities took everything from, worked out once per position and history length (an `affects` may read the past) and
  free while no such ability is on the battlefield. A source's static abilities of layers 1 to 5 have applied by the time
  layer 6 takes them (613.6: Painter's Servant enchanted by Eaten by Piranhas, or turned into an Elk, still paints everything),
  so the static-source list keeps such a source and leaves it out from layer 6 on - all but the effects that had begun by
  then. **An effect in parts (613.6)** is one `StaticDef` per layer, each later part marked `.continues()` (Opalescence:
  layers 4 and 7b; Puppet Crafting, Eaten by Piranhas, Ygra, Kaito, Wurmwall Sweeper). As an object's characteristics are
  worked out, the layers note the static abilities that applied to it (`Begun`, only while a source has an effect in parts); a
  later part applies to it exactly when its first part did - "the same set of objects", whatever its words say by then - and
  goes on applying though its source loses its abilities in a layer between, if the first part was of layers 1 to 5 or is the
  "loses all abilities" ability that took its own source's (Humility made a creature: `Stripped::applying`). A layer-6 part
  of a source that loses its abilities in layer 6 never begins (613.8a: it depends on the effect that removes it). One "loses all abilities" ability that
  would take another's source's applies first (613.8a dependency), each taking the other's in timestamp order (613.8b); no
  card in content can make two of them meet (an Aura is no creature), so that order has no scenario. The host lists every
  rules line of an object that lost all abilities as lost, not only its keywords and activated and triggered abilities.
- **Setting a land's subtype (305.7, layer 4)** is a static ability made with `sets_land_subtypes(affects, apply, text)`
  (`StaticDef::sets_land_subtypes`: Magus of the Moon, Multiversal Passage; Blood Moon, Spreading Seas and Evil Presence print
  the same), whose `apply` calls `Chars::set_land_subtypes`: the land's land types are replaced (its other subtypes, an animated
  land's creature types, stay) and every ability it has in layer 4 - its rules text's and its copy effects' - is taken; the
  basic land type's mana ability follows in layer 6 (305.6), and what other effects grant it after. `layers::land_text_lost`
  finds, once per position and history, the lands such abilities took the rules text of, judged on each as layer 4 leaves it (every
  effect that makes it a land or stops it being one is one the setting ability depends on, 613.8a; the limits are KI-61), and
  orders the abilities whose sources are lands as `lost_to_statics` does. Such a land's static abilities apply in no layer from
  4 on - each depends on the setting effect, which applies first whatever the timestamps (613.8a: Urborg under Magus makes
  nothing a Swamp, either order) - and `Query::lost_all_abilities` answers for it, so every rule its card prints is gone. The
  setting ability itself applies whatever its source loses in layer 6 (613.6: Magus under Eaten by Piranhas), and a land's own
  ability that sets its own subtype (Multiversal Passage) goes on applying and does not count it as having lost its text. A land
  about to enter is judged as it would be on the battlefield (614.12, below): under such an effect its own `enters`
  replacements - "enters tapped", "as this enters" - do not apply.
- **A permanent about to enter is judged as it would exist on the battlefield (614.12)**: `ops::as_it_would_exist` puts it
  there for the moment of the question - under the player it enters under, with the face, copy effect, choices and counters
  the replacements applied so far gave it, at a new (the latest) timestamp - reads its characteristics and
  `lost_all_abilities` through every layer, other objects' static abilities and its own alike, and reverts. One answer serves
  every entering check: `prepare_entry` applies its own `enters` replacements, and daybound's "enters transformed", only if it
  would keep its abilities (asked only while a "loses all abilities" or land-subtype-setting static ability is on the
  battlefield, `layers::abilities_may_be_lost`), and asks again as the copy once a copy replacement chose one; and
  `ops::judged_entering` sets the answer on `Cx::entering` while the replacement effects of its zone change and the counters
  it enters with are judged, so `Query::chars` and `Query::lost_all_abilities` of the entering object answer as it would be
  there ("creatures your opponents control enter tapped" taps a Forest Life and Limb would make a creature; Containment
  Priest exiles a card entering transformed into a creature) - asked only where a replacement effect could apply
  (`replace::possible`) or a counter ban could limit its counters. A resolved effect that sets a land's
  subtype ("target land becomes an Island until end of turn") has no `EffectKind` yet; the copiable values a copy effect gave
  such a land other than abilities stay (KI-51).
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
could match it. "Leaves the battlefield" look-back (CR 603.10) reads the frozen last-known rows.

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
them moves (CR 614.12, 101.4): each one's "as enters" choices (`prepare_entry`, done by the callers first), its zone-change
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
704.3: Thespian's Stage that became a Dark Depths with no ice counters and lost the legend rule). It
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
life (Mana Confluence) can leave only declining. The mana and the life are paid together, and "if they do" asks whether
they paid (CR 118.12). "You may pay {X}" where the text leaves X to the payer (CR 107.3f) is `Game::may_pay_x`
(`actions::may_pay_x`): the same question with its `x`, raised and lowered (`IncreaseX`, `DecreaseX`) while mana is made, any
X from 0 up, and the X paid is what it returns, for "when you do" to carry (Fire Lord Sozin). Not built, because no card in content has one: exchanges of life totals, costs
that gain life, and an effect a spell or ability makes that a player can't lose life (CR 119.8: "can't lose life this turn").

"Damage can't be prevented" (CR 615.12) is asked by `ops::damage_can_be_prevented` before any prevention: a turn an effect said
so (Bonecrusher Giant's Stomp) or a `CardDef::unpreventable_damage` static (Questing Beast's combat damage,
`statics::damage_cant_be_prevented` for Sunspine Lynx and Leyline of Punishment). Every prevention effect asks it.

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
damage if it was (the rulings). Everything about the recipient is judged there: protection, infect's counters or poison,
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
other has nothing left to change (CR 616.1f). Which permission a land play is made by is `actions::land_play_permission`: none
from the hand, or from a graveyard or a library's top a rule lets lands be played from, the same test the offer makes.
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

### Extra turns and "that turn"

An extra turn (CR 500.7) is a `cantrip_state::ExtraTurn` in `GameState::extra_turns`, the most recently created last and taken
first: whose it is and a number that names it (`Game::add_extra_turn`). The turn being taken carries that number in
`Turn::extra` (0 for a turn of the ordinary rotation), which is what "that turn" means to an ability that made it:
`DelayedWhen::ExtraTurnStep { step, turn }` triggers at that step of that turn alone, and is dropped as the next turn begins
once that turn has ended without the step (an effect ended it, CR 724.1e) or will never be taken (skipped, CR 500.11). Final
Fortune and Last Chance's "at the beginning of that turn's end step, you lose the game" is
`cantrip_mechanics::delayed::extra_turn_then_lose`.

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
the permanent it waited for has left (610.4), checked with the returns of exiled cards after the state-based actions. An
attachment that phased out directly phases in unattached if what it was attached to is gone (702.26i), for the state-based
actions. "Until [this] leaves the battlefield" goes on while its source is phased out (`phasing::still_on_battlefield`:
Banishing Light's ruling), "for as long as" durations end (702.26f: `WhileSourceOnBattlefield` asks the battlefield), and a
resolved effect never includes a phased-out permanent (702.26e, `add_effect`). Cleanup removes damage from phased-out
permanents too (514.2). Events: `PhasedOut`, `PhasedIn` (recorded; nothing triggers on them). The phasing keyword itself
(702.26a's phasing out in the untap step) has no card here. Teferi's Protection (`Game::phase_out`) and Oubliette
(`Game::phase_out_until_leaves`) use it; the host shows a phased-out permanent with the battlefield, marked
`ObjectView::phased_out`, and the script dump lists them as `BotN.PhasedOut`.

"Your life total can't change" (CR 119.7, 119.8) is `EffectKind::LifeTotalCantChange`, a player effect `ops::change_life`
asks before any replacement (nothing replaces what can't happen) and `actions::can_pay_life` asks of a cost of more than 0
life. Damage is still dealt; only its loss of life doesn't happen.

### Winning, losing and drawing

A player loses to the state-based actions of CR 704.5a-c through `ops::loses_to_state_based_actions`, which leaves out each
way a static ability says the player doesn't lose (`CardDef::loss_rules`: Platinum Angel's "you can't lose the game",
Phyrexian Unlife's "you don't lose the game for having 0 or less life"); a draw from an empty library is forgotten with the
check that saw it (704.5b: "since the last time state-based actions were checked"). An effect that says a player loses
(`lose_game`, 104.3e) is stopped the same way, and one that says a player wins (`win_game`, 104.2b) does nothing for a player
who can't win (`CardDef::win_rules`) or who has an opponent who can't lose. A concession is the host's and is never stopped
(104.3a). Every loss records `Event::GameLost` with its `LossReason`, and a draw `Event::GameDrawn` with its `DrawReason`,
which the host tells each seat (`PlayerLostEvent`, `GameEndedEvent.DrawReason`).

Loops are `loops.rs`'s (CR 104.4b, 732), watched in `GameState::loop_watch`, journalled like any other write. A loop of
mandatory actions is a draw (CR 104.4b, 732.4). A question with more than one legal answer is a real choice and clears the
watch of mandatory actions; a priority with nothing to do but pass - and tap a mana ability whose whole cost is {T}, when
nothing more mana could buy is castable - is none. After `WATCH_AFTER` such priorities in a row with something on the
stack, each priority's position is fingerprinted up to the identity of objects (objects numbered by where they are,
references to objects that left followed to what they became, timestamps by their order), every amount in it (life,
poison, counters, damage, mana, libraries), and a position seen again is the loop going round: the game is a draw. The
event history is not in the fingerprint; every read of it is counted instead (`cantrip_api::history::note_read`, made by
`History::new`), and a read clears the positions seen, so a loop that "the third time this turn" or "only once each turn"
could end is never found by its positions (`loops::settled_read` asks a question whose answer the rest of the turn can't
change without counting it). A loop that never comes back to a position, or only with the history read between, is
followed by the shape of its positions (the fingerprint less its amounts) once a step has `BUSY_STEP` events in it: a
priority whose shape comes round `ROUNDS` times with no player choosing to act and every amount gone only the way no rule
ends anything by (life gained, counters added) is an endless loop of mandatory actions, a draw; one whose amounts go down is
left to run out. In such a step every question is kept with its shape and the answers given there; a loop its players keep
going by choice goes round `ROUNDS_TO_CHOOSE` times (the engine's count, as no shortcut is proposed) and then the player
chosen as CR 732.3 chooses must choose otherwise for the rest of the step - the answers
they gave that question there are taken out of it (a target, card, player or mode chosen, a priority action, a "may" taken;
never a pass or a decline). A loop whose players only pass or decline can't be stopped by making them act (CR 732.5) and is a
draw after `ROUNDS`. `FORCED_LIMIT` and `STEP_LIMIT` end what is left: a run of mandatory actions that long with no life,
poison or library moving towards its end is a draw, and in a step that long players may only pass and decline. A normal game
pays a comparison per priority and a count per read of the history.

### Casting: when a cost's choices are made

A spell's costs are chosen and paid in the order of CR 601.2b-h, as the C# engine asked them: the permanents a cost
sacrifices or returns, its discards and exiles are chosen before the mana is paid, an additional cost's choices after
it. A cost of tapped permanents (`PermanentCost::tapped`, web-slinging's "return a tapped creature you control", CR
702.188a) is chosen once the mana is paid, since mana abilities come first (CR 601.2g) and one of them may have tapped
the creature (Spider-Sense's ruling): it is offered while a creature is tapped or a mana ability its controller could
activate would tap one, its own {T} or Springleaf Drum's, and a payment that leaves none tapped can't be completed, so
the cast is taken back (CR 601.2h). Only a spell's cost has one yet; an activated ability with one is refused by a
card-table test (`every_activated_ability_has_a_cost_activation_can_pay`) and an assertion in `activate`. The web
client's payment caption for a web-slinging cast says to tap a creature for mana while none is tapped; the cast that
leaves none is still taken back after it is paid.

### Costs that depend on the targets

The total cost of a spell or ability is determined once its modes, X and targets are announced (CR 601.2b-c, 601.2f,
602.2b), so every cost change (`CostChange`) is handed the targets announced, in slot order, and the total is locked in
there: what happens to a target later changes nothing paid. Casting and activating apply the changes after the targets
are chosen (`actions::cast`, `activate`), and the payment asks for that total, which is what the client's payment shows.
Whether an action is worth offering is judged before anything is announced: where its one target is announced with the
action (equip, most single-target spells) and something could change the cost - a cost changer on the battlefield, or
the spell's or ability's own - it is judged for each candidate, and only the candidates it could be paid for are
offered; otherwise the cost is judged once, before any target is looked for, as before. An activated ability's menu row
says what it costs with each candidate where that is not the printed cost (`activation_costs`, the host's
`option_message`: "Equip {4}. ... (costs {1} targeting Dwarven Mauler)"). Where the
targets are chosen later (several slots, a modal spell), a change that reads them is not counted in that advice, so a
reduction that only a target would bring does not make the action offered: no card in content has one. Ajani's Response
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

A cost change may set the least the total comes to (`CostDelta::at_least`: Trinisphere's "each spell that would cost less
than three mana to cast costs three mana to cast"). It is one of the "effects that directly affect the total cost" CR 601.2f
applies once the total is otherwise determined: after the mana cost or alternative cost, every increase and every reduction
(the cast's own too - harmonize's tapped power, a sacrifice for each, emerge's sacrificed mana value), generic mana making up
the difference, before the total is locked in and paid. A spell cast without paying its mana cost (Omniscience) or for an
alternative cost of no mana (Force of Will's) comes to three like any other. A cast takes it apart from the rest
(`apply_cost_changes_before_least`) and applies it last, after its own reductions; the advice applies it with the rest
(`change_total`), before those reductions and a permission's additions (known issue KI-68). The host's row for a cast says what the total comes to whenever the changes in play make it other than the
cost it is cast for (`cast_total`: "(costs {3})", for a card cast from a hand), as it does for a cost that depends on
the target.

A static ability that forbids casting spells with certain qualities ("noncreature spells with mana value 4 or greater can't
be cast", Gaddock Teeg) is `CardDef::spell_rules` (`SpellRule`, CR 101.2, 601.3). Each cast the rules would offer is judged on
the face it casts (CR 601.3e, 709.3a: one half of a split card, an Adventure, a back face), with X as 0, and left out if a
permanent with such a rule forbids it (`castable`, the casts an effect makes as it resolves too); and once a cast's proposal is
complete (CR 601.2a-d) the spell on the stack is judged again, its X announced, and a cast no longer allowed is taken back
(CR 601.6). The battlefield is searched for these only when something could be cast.

A special action with a cost (CR 116.2: plotting, suspending, foretelling, turning a face-down permanent up, unlocking a
door) is costed as `Costed::SpecialAction`: it is no activated ability (CR 116.1), so what changes the cost of activating
abilities (Forensic Gadgeteer, Anointed Peacekeeper) leaves it alone, and only a change that names it applies - Doc Aurlock,
Grizzled Genius's "plotting cards from your hand costs {2} less" (CR 116.2k, 118.7), which knows plot's action by
`ActivatedDef::plot`. `special_action_cost` applies the changes to the printed cost, for the advice and the payment alike;
the host's menu row says what it costs when that differs ("(costs {B}{G})").

"As an additional cost to cast this spell, you may sacrifice any number of nonland permanents. This spell costs {1} less
to cast for each permanent sacrificed this way" (Rottenmouth Viper) is `VariableCost::SacrificeAnyNumber`: how many is
announced as the spell is cast (CR 601.2b), kept as its `x` though its mana cost has no {X}, so the total cost is determined
knowing it (CR 601.2f) - `less_each` generic mana off each, after any increase (CR 118.7a) - and which ones are chosen once
the mana is paid (CR 601.2g-h). The advice counts every permanent that could be sacrificed. The host's announcement says
what the number is (`PayCostsPromptMessage::SacrificeReduces`). A spell's mana value in its `Cast` record counts X once
for each {X} in its mana cost (CR 202.3e), and not at all for an X a cost defines without one.

An additional cost the targets define ("collect evidence X, where X is the total mana value of the permanents this spell
targets", Urgent Necropsy) is `CardDef::cost_of_targets`, asked once the targets are chosen and added to the total then
(CR 601.2c, 601.2f); one its caster can't pay takes the cast back (CR 601.2h). A batch of zone changes moves an object once
however many times it is named (`move_together`), by the first of its moves (a later one to another zone is dropped): a
permanent a spell targets in two slots is destroyed once.

### Ways a permanent gives: from a graveyard or from a hand

A `GrantedWay` names the zone of its controller's own it casts from (`GrantedWay::from`): a graveyard (Lurrus, Serra
Paragon, Underworld Breach's escape, mayhem) or the hand ("you may cast spells from your hand without paying their mana
costs", Omniscience). Each is judged on the face being cast, offered beside the card's other casts with its own way number,
and is the one alternative cost of that cast (CR 118.9a); additional costs and cost increases still apply to it (CR 118.9d).
Whether X is announced follows from the cost the way names (CR 107.3a-b): a cost that is or includes the mana cost has its
{X}; `Cost::NONE`, "without paying its mana cost", leaves X 0. A card with no mana cost may be cast that way (CR 118.6a).
The host marks such a cast `WithoutPayingManaCost`, with the cost it comes to as its `CostText`, and the tactical bot takes it
over the card's ordinary cast. Not built: a way given to cards in a hand used by a cast an effect makes as it resolves
(`cast_ways_during_resolution` offers the mana cost only), and emblems that give one (Tamiyo, Field Researcher's): KI-44.

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

Counters that would be put on a permanent are one `Proposal::Counters` (CR 122.6, 614.1a), whose `CounterPlacement` says who puts
them (`putter`, CR 122.6a: "if you would put", Innkeeper's Talent) and whether an effect does (`effect`: "if an effect would put",
Doubling Season). An effect is a resolving spell's or ability's, or the replacement effect a permanent enters with counters by
(CR 614.1c: a planeswalker's loyalty, CR 306.5b; a battle's defense, CR 310.4b; a Saga's first lore counter, CR 714.3a). A cost's
counters are none (CR 118.3): a loyalty ability's (CR 606.4), "put a page counter on this artifact" (Mazemind Tome) and blight's
(CR 701.68a) go through the replacements with `effect` false (`ops::put_counters`), as does the lore counter a player puts on each
Saga as their precombat main phase begins (CR 714.3c, a turn-based action). So Doubling Season doubles a planeswalker's entering
loyalty and not its +1, and Innkeeper's Talent doubles both. A cost that would put counters a static ability forbids (CR 101.2,
`CardDef::counter_bans`: Solemnity, Melira) can't be paid (CR 118.3, Solemnity's ruling): such a loyalty ability, counter cost or
blight is not offered. Whether damage's -1/-1 counters (infect, CR 120.3d) are an effect's has no ruling; they are put as one.
The permanent's controller orders the replacements (CR 616.1), told the counters as the effects so far have left them
(`RequestKind::ChooseReplacement::counters`, which the host words "1 +1/+1 counter would be put on Walking Ballista"); the
tactical bot has one that adds a counter apply before one that doubles them.

### Blight, counters from among permanents, and casting from exile by a permanent's leave

Blight N (CR 701.68a) is `Cost::blight`, paid by a spell's cost (Requiting Hex) and by an activated ability's (Dawnhand
Dissident; Gristle Glutton and Sting-Slinger print it too). An activation chooses the creature after its mana is paid, as a
spell's additional cost does (CR 601.2g-h by way of 602.2b): from what is there then, and with no creature left the activation is
taken back (CR 701.68b). The creature need not survive the counters (the rulings). 701.68d's "whenever a player blights" has no
event yet: no card in content triggers on it.

"Remove N counters from among [permanents you control]" (CR 118.3, 122.1) is `Cost::remove_counters_among`, a count and a filter
over the payer's permanents: counters of any kinds, from any of them. It is payable while there are that many among them, and
is asked as `RequestKind::RemoveCounters` - each kind on each permanent a `CounterPool` with how many there are, answered with
`Answer::Counters` (how many of which kind from which permanent, adding up to the total, which `validate` checks) - chosen
after the mana like blight and removed as the cost is paid (`ops::add_counters`, so a Siege's last defense counter still
triggers). The host sends it as `RemoveCountersPromptMessage`; the web client offers a button for each kind on each permanent
and takes one of a permanent's counters on a click on it. Dawnhand Dissident's casts pay it; Tayam, Luminous Enigma's ability is
the same cost on an activated ability, which `activate` pays too, though no card in content has it there yet.

A permanent's static ability that lets a player cast cards in exile (CR 601.3, 604.1) is `CardDef::exile_casts`, an `ExileCast`:
whether a card may be cast by it now (asked of the permanent, its controller "you", with the card as the face that would be
cast, so "creature spells" refuses a creature card's instant Adventure) and an additional cost owed on each cast by it (CR 118.8,
601.2f). It is a permission, not a way of paying: `castable` treats the card as permitted, so it is cast for its mana cost or any
alternative cost it has (Solitude's evoke), at the speed its type gives (a flash creature at instant speed, the rulings), and the
additional cost joins `extra` in `actions::cast`, which finds the permission as the cast begins (`exile_cast_permission`), so the
permanent need not stay while the spell is paid for (Dawnhand Dissident's ruling). A permanent that has lost its abilities gives
none, and "cards exiled with this" are those its own ability linked (`Query::exiled_with`, CR 607.2a), so a flickered one finds
none (CR 400.7). A card that a play permission of its own lets be cast, or a card on an Adventure, is cast by that instead; two
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
spell's, is a spell you cast), no `cast_untimely` and no granted way's `gains`. What was chosen and paid as the original
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
abilities once it has it. "Except it's legendary" (CR 707.9b) is `Copiable::legendary`.

### Effects that end when a card is cast from exile

"Until this card is cast from exile" (CR 611.2a, 400.7) is `Duration::UntilCastFromExile(card)`, the object in exile.
`actions::cast` drops those effects as that object becomes cast (CR 601.2i), after its costs are paid, so a mana ability
the effect grants may pay for the cast (Emrakul, the Exigent Doom's ruling). The card leaving exile any other way is a
new object that is never cast as this one, and the effect lasts as long as its affected object.

### Effects that last while a counter is on the object

"For as long as it has a [kind] counter on it" (CR 611.2b) is `Duration::WhileHasCounter(kind)`. `add_effect` refuses to
start one on an object with no counter of that kind (a replacement kept it off), and the effect ends the moment the last
one comes off - `ops::end_counter_durations`, called wherever counters are removed (`add_counters` with a negative amount,
a cost that removes counters, a stun counter replacing an untap, damage to a planeswalker), with the state-based check as
the backstop - and is deleted, so a counter put on again later does not bring it back. Its source leaving changes nothing.
A control effect that ends this way gives the permanent back (CR 613.1b). Eluge, the Shoreless Sea; Aquitect's Will,
Xolatoyac, the Smiling Flood and Quicksilver Fountain print the same words. The paths no printed card can show (a counter
put back without a new effect; an effect begun without a counter) are `crates/cantrip-script/tests/counter_duration.rs`.

### Permissions to play under a condition

A standing permission to play a card (`PlayPermission`, CR 601.3, 305.1) may hold only while a condition does
(`PlayGrant::when`, CR 611.2), worked out each time the permission is asked (`actions::permission_for`), so it is
offered, or not, as the game is then: `PlayCondition::AttackedWithThisTurn(subtype)` ("during any turn you attacked with
a Rogue", Robber of the Rich) asks this turn's history; `PlayCondition::ControlsSubtype(subtype)` ("you may play them if
you control a Wizard", Gandalf, Goblins' Bane's Flameshape; Possibility Technician's Kavu) asks whether the holder controls
a permanent with that subtype now, its characteristics through the layers (a changeling counts). Losing the last one stops
the permission until another arrives; a spell already cast stays cast. The card in exile keeps the permission all along -
only a zone change ends it (CR 400.7). The host says the condition beside the permission ("Playable while it stays here,
if you control a Wizard").

### Restricted mana

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

### Mana spent as though it were of any color

"Spend mana as though it were mana of any color" (CR 609.4b) is a `CardDef::spend_any_color` rule, asked of a payment
(`Spending`: the player, what it is for, whether it is an activation): Agatha's Soul Cauldron's for its controller's
creatures' abilities, Mycosynth Lattice's for every player and payment. Where one says yes, the cost is paid as
`ManaCost::paid_with_any_color` has it - each colored and hybrid symbol by any mana, colorless included, a {C} symbol by
colorless only - by the payment itself (`pay_mana_used`) and by the advice (`Available::covers_for`, which asks once per
player whether the rule holds for every payment, and the activation offers, which ask for their source); the cost, its
mana value and the mana spent are unchanged, and no restriction on the mana is lifted.

### Tapped for mana

`Event::TappedForMana { permanent, player, produced }` is recorded when a mana ability with {T} in its cost resolves
(CR 106.12): `produced` is the mana that ability itself added, every restriction counted, told from the player's pool
before and after it - so a replacement of its mana counts, and the mana a triggered mana ability adds for the same tap
does not (`actions::tapped_for_mana`). The triggered mana abilities it triggers (CR 605.1b) resolve at once, and only when
it produced some mana (CR 106.12a); one that asks (`cantrip_mechanics::mana::add_one_mana_of_any_type_that_land_produced`,
a type of `Label::ManaType`, "Choose which type of mana to add") suspends the activation, which runs again with the
answer. "One mana of any type that land produced" reads `produced`: Lavaleaper, Mana Flare, and Heartbeat of Spring,
Dictate of Karametra, Zendikar Resurgent, Mirari's Wake and Vorinclex, Voice of Hunger as written.

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
and an Omen is shuffled into its owner's library by its controller (CR 720.3d); a copy of an Omen still shuffles its owner's
library and ceases to exist. Countered, or with every target gone, either goes to the graveyard. The host sends the face as
`ObjectView::inset_spell`, printed beside the card. A land card with an Adventure (Lindblum, Industrial Regency) is cast
only as the Adventure, never as itself (CR 305.9), and exiled on its adventure it is played from exile as a land play (CR
715.3d). A static ability about "each X card in your graveyard" (Iroh, Underworld Breach) judges the card, which has only
its normal characteristics there (CR 715.4), and takes the cost from the face cast. Not built: casting the face from the top of a library
(`PlayRule::LibraryTopSpells`, which offers only the card itself).

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
A cost is never a requirement (CR 508.1d): a creature that attacks each combat if able is required only while some attack
open to it costs nothing, and a requirement to attack a certain player (or a goaded creature's other players) only
counts that player while attacking them is free. The board is searched for these only as attackers are declared. The
benchmark measured 5.73 us per decision before and 5.76-5.85 us after (2026-10-16, the same noisy machine).

### Blocked by at most one creature

"Can't be blocked by more than one creature" (CR 509.1b) is `Keyword::CantBeBlockedByMoreThanOne`, an evasion ability, so
"loses all abilities" takes it: printed (a content `keywords` entry: Stalking Tiger, Charging Rhino, Ironhoof Ox, none in
content yet) or given in layer 6 (Meltstrider's Resolve; Familiar Ground, Challenger Troll, Vorrac Battlehorns, Sunder Shaman
and Bristling Boar print the same). `Chars::maximum_blockers` is the counterpart of `minimum_blockers`: a declaration giving an
attacker more blockers is refused (`validate`, CR 509.1b), and an attacker whose minimum is above its maximum (menace as well:
the restrictions are cumulative) is offered no block at all (`turn::declare_blockers`). The question carries both per attacker
(`DeclareBlockersPrompt::maximum_blockers`, 0 for any number); the web client sends at most that many of the creatures picked up
at the attacker clicked and says why the rest are still held; the tactical, random, automatic and language-model seats never
declare more. A multi-pair refusal in a scenario is judged by the engine's check of that declaration.

### Blocks an effect requires

A requirement to block (CR 509.1c) is a `CardDef::block_requirements` entry (`BlockRequirement`), asked of each block the
restrictions allow, with the permanent as its source: Lure's "all creatures able to block enchanted creature do so" requires
every creature able to block the creature it enchants to block it. `static_rules::block_requirements` collects the required
pairs only while a permanent with one is on the battlefield with its abilities; `turn::most_requirements` then finds the most
of them any declaration that breaks no restriction obeys (each blocker blocks one attacker, every block allowed, each attacker
blocked within its minimum and maximum: menace) by a cut search over the blocks of the required attackers, with one such
declaration. The question carries all three (`BlockRequirements`: pairs, most, suggested), `validate` refuses a declaration
that obeys fewer, and the suggestion is every seat's fallback: the script's default, the bench's, `DeclareBlockersPrompt`'s
`Default()`, and the tactical, random, automatic and language-model seats when their own blocks obey too few. The web dock names
a creature that must block and holds the button until the declaration obeys enough. Not built, as no card in content needs
it: "blocks each combat if able", a requirement to block any attacker (a pair with no attacker), and costs to block (CR 509.1d).

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
the Limited events (`BoosterPod`, `DraftPod`) the five basics; a sideboard card the engine does not have is left out of the
game (KI-37), where a deck card it does not have refuses the deal. `cantrip_mechanics::outside::may_put_card_into_hand` is North Wind Avatar's and Death Wish's "you may
put a card you own from outside the game into your hand", asked as `Purpose::PutIntoHand` over the sideboard's cards, which
the client captions as coming from outside the game. Not built, because no card in content needs them: a revealed fetch of
a quality (Burning Wish, Living Wish, Karn, the Great Creator), learn (CR 701.48: a Lesson card from outside the game,
revealed, `Subtype::Lesson`, which would bring it in with this same verb), casting from outside the game (Spawnsire of
Ulamog), and a companion (CR 702.139), which would start in the sideboard and be put into the hand by a special action. The
benchmark is unchanged within the noise (2026-10-16, alternating: 57.4 (an outlier), 12.06, 14.10 us before and 13.87,
12.45, 11.12 us after).

### Keyword counters

A keyword counter gives its permanent the keyword it is named for (CR 122.1b): flying, first strike, double strike,
deathtouch, decayed, haste, hexproof, indestructible, lifelink, menace, reach, shadow, trample and vigilance
(`CounterKind::KEYWORDS`). The ability is a layer-6 effect (CR 613.1f) with the timestamp of its counters: each counter
gets one as it is put on, and every counter of its kind already there gets the same (CR 613.7c). `ops::add_counters`
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

Known gaps:

- Cost reductions by colored mana (`CostDelta::less_mana`, CR 118.7) take off the cost's plain symbols of that color and
  then generic mana. A reduction by an amount that includes hybrid mana, which the payer splits as it is applied (CR
  118.7e), is not built and panics if reached: no card in content reduces a cost by hybrid mana. The power-up cards whose
  own mana cost has a hybrid symbol (She-Hulk, Attorney-at-Law; Brawn, Amadeus Cho; Abomination, Terrifying Titan) would,
  and wait on it. Nor is a colored reduction meeting a hybrid or Phyrexian symbol of its color in the cost built.
- Power-up abilities are not yet a class other cards can name ("power-up abilities of other creatures you control cost
  {3} less", "whenever you activate a power-up ability", "can be activated an additional time"): Hulk, Gamma Goliath,
  Marvel Boy, Wonder Man, Kang, Advancing the Spirit and Quinjet Technician wait on it.
- The web client cannot yet send three answers the engine asks for: naming colorless mana (Squandered Resources),
  choosing which kind of counter to remove (`Label::Counter`, Glen Elendra Guardian), and turning a face-down
  permanent face up (`TurnFaceUp`).
- Serving a scenario's handed-over position to the web client, to look at or play on from, went with the C# engine
  and is not rebuilt.
- Web-slinging cards that wait: Spider-UK ("if two or more creatures entered the battlefield under your control this
  turn") needs the controller a creature entered under, which the history cannot answer once control has changed (no
  event records a change of control); Arachne, Psionic Weaver needs a card type chosen as it enters (`Entering` has a
  color, a subtype, a player, a name, odd or even, no card type) and a prompt for it; Peter Parker // Amazing
  Spider-Man's back face grants web-slinging to the legendary spells its controller casts, a way a permanent gives to
  cards in hand, which only exists for cards in graveyards (`granted_ways_of`).
- `web_slinging::way` is a way from the hand (`ZoneKind::Hand`), like every printed alternative cost. Cast from
  elsewhere at priority by a permission or the card's own say-so, a hand way is offered too (`castable`'s `elsewhere`;
  Spider-Sense from the graveyard under Yawgmoth's Will is a scenario, and sneak's instant-speed way likewise), unless its
  text names the hand (`WayDef::only_from`: warp, CR 702.185a, so a warped card is cast from exile for its mana cost), but a cast made while an effect resolves
  (`cast_ways_during_resolution`) is offered for the card's mana cost and faces only: an effect that lets a spell be cast
  paying its costs ("cast it from exile this turn") as it resolves can't use web-slinging or any other alternative cost.
- Costs to attack are mana only: one that asks for life (Sivitri, Dragon Master's +1), a Phyrexian symbol paid with life
  (Norn's Annex), tapping or sacrificing, and optional costs to attack (CR 508.1g: exert, enlist, which the engine has
  as other things) are not built, nor anything that changes a cost to attack. No card in content needs them.
- No scenario reaches Questing Beast's unpreventable damage, Emrakul's extra turn on cast, or Tamiyo's -7 emblem.
- Suspend does not give a creature cast that way haste (no card in content needs it), and several creatures put onto
  the battlefield one after another by an effect (Ghost Vacuum) enter one at a time rather than together.
- Rooms (CR 709.5) are built: `Marks::doors` holds the unlocked designations, `layers::doors` gives a Room on the
  battlefield the name, mana cost and abilities of its unlocked doors, the unlock special action is an intrinsic ability
  per locked door (`intrinsic::UNLOCK_LEFT`/`UNLOCK_RIGHT`, cost read off the door), `Event::DoorUnlocked` is what
  "when you unlock this door" listens for, and a door's play rules (`CardDef::play_rules`, Steaming Sauna's "You have no
  maximum hand size") apply while it is unlocked (`actions::each_play_rule`). Not yet: a door with static abilities, replacement effects or an enters
  rule (the card table refuses one), instructions to unlock or lock a door (`Game::unlock_door` exists for the first,
  nothing locks), "fully unlocks", and a copy of a Room spell (`resolve_copy_as_token` gives the token its door, but
  no card in content can copy a Room spell to prove it).
  Known deviations from the rules: the unlock is an intrinsic ability entry, so an effect that removes abilities
  (Humility) would remove it, though CR 709.5e says the unlock is not an ability; a copy effect's "except its name is X"
  is overwritten by `layers::doors`, which names a Room by its unlocked doors; `resolve_copy_as_token` drops the copy
  exceptions of a Room spell's copy; a granted cast from a graveyard (Serra Paragon) is judged on the whole Room card, mana
  value 8, never on a door (Known bug scenario `serra-paragon-casts-either-door-of-a-room-in-the-graveyard`).

## Open questions

- **Checkpoints:** how often the live game keeps a full state checkpoint, so that resuming or jumping far through the
  tree does not replay from the root.
- **What a step is for search:** whether the engine exposes plan-level moves (a whole combat) or only primitive
  answers, with plans built above it.
- **Multiplayer:** the rules are written for two players today, and multiplayer (CR 800) must not be designed out.
- **Determinism across machines:** the same results on every platform the server and training run on, which rules
  out depending on hash map iteration order or unordered floating point.
