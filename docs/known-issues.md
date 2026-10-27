# Known issues

Problems being tracked in the client, the tooling and the harness, and engine paths no scenario runs yet, in one place.
A scenario that shows a bug is still committed with a `# Known bug:` line (`scenarios/README.md`, "Known bugs"); this
file is where that bug is described, ranked and followed. When it is fixed, delete the entry in the fixing commit (the
history keeps it).

**This file never accepts a deviation from the game's rules** (`CLAUDE.md`, "No rules deviations"). A rules deviation
found in review blocks the change that has it. One found in existing code is fixed, or reported to a human at once; it
appears here only while its fix is under way, marked as such, and is never left to wait for "a card that needs it". A
path no scenario runs gets a test - a Rust test with test-only cards when no printed card reaches it - rather than an
entry. The only allowed deviations are the human decisions in `docs/rules-exceptions.md`, and limits of scale beyond
any practical game or card (`CLAUDE.md`, "Scale is the one standing exception"), each listed here as "Won't fix:
beyond practical scale" with the limit, what happens past it, and why play cannot reach it.

## Priority

| Priority | Meaning |
| --- | --- |
| Pri-0 | A crash, a hang, a desync between engine and client, or a game that cannot go on. A blocker. |
| Pri-1 | Any rules violation, however rare, that a card legal in a supported format (implemented or not) or a card the engine implements (legal or not) can reach. A blocker. |
| Pri-2 | A choice the client cannot offer or show, or a wrong display of a right game state. |
| Pri-3 | Cosmetic, a harness or tooling gap, or a path no scenario runs yet. |

Pri-0 and pri-1 are fixed before any other work is started or landed (`CLAUDE.md`, "Bug priority"); one is listed here
only while its fix is under way ("Fix queued" or "Fix under way"), and the list of them is kept at zero. A limit of scale
beyond any practical game or card is "Won't fix" (`CLAUDE.md`, "Scale is the one standing exception"). The tactical bot's
own judgement is pri-3: the engine is right, and the bot only plays less well.

## Entry template

```

### KI-<n>: <one-line description of what is wrong>

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open / Suspected / Unexercised | engine / mechanics / card / client / harness | YYYY-MM-DD, how |

**Notes.** What happens, what the rules require (CR numbers, rulings), where in the code, and what a fix needs.

**Scenarios.**
- [`scenarios/<name>.cantrip`](../scenarios/<name>.cantrip) - what it shows (marked `# Known bug`), or "none yet" and
  which implemented card could reach it.
```

## Summary

| ID | Issue | Priority | Status |
| --- | --- | --- | --- |
| [KI-98](#ki-98-a-station-permanent-lists-its-reached-striations-keyword-again-as-granted) | A station permanent lists its reached striation's keyword again, as granted | Pri-2 | Open |
| [KI-101](#ki-101-a-player-out-of-a-game-that-goes-on-sees-it-a-step-late) | A player out of a game that goes on sees it a step late | Pri-2 | Open |
| [KI-108](#ki-108-what-can-be-cast-is-over-offered-where-a-mana-abilitys-yield-is-unknown-or-its-cost-is-shared) | What can be cast is over-offered where a mana ability's yield is unknown or its cost is shared | Pri-2 | Open |
| [KI-114](#ki-114-the-web-client-cannot-name-colorless-mana-for-a-sacrificed-land-that-could-also-produce-a-color) | The web client cannot name colorless mana for a sacrificed land that could also produce a color | Pri-2 | Open |
| [KI-116](#ki-116-a-resolutions-unless-pays-question-shows-only-the-mana-and-life-parts-of-its-cost) | A resolution's "unless pays" question shows only the mana and life parts of its cost | Pri-2 | Open |
| [KI-3](#ki-3-engine-paths-no-scenario-runs) | Engine paths no scenario runs | Pri-3 | Fix queued |
| [KI-35](#ki-35-the-tactical-bot-counts-no-summoning-sick-creature-as-a-mana-source-under-shang-chi) | The tactical bot counts no summoning-sick creature as a mana source under Shang-Chi | Pri-3 | Open |
| [KI-49](#ki-49-the-tactical-bot-does-not-know-infect-or-poison) | The tactical bot does not know infect or poison | Pri-3 | Open |
| [KI-57](#ki-57-the-tactical-bot-does-not-know-islandwalk) | The tactical bot does not know islandwalk | Pri-3 | Open |
| [KI-71](#ki-71-the-tactical-bots-combat-planning-ignores-bands) | The tactical bot's combat planning ignores bands | Pri-3 | Open |
| [KI-78](#ki-78-the-tactical-bot-attacks-a-player-whom-protection-keeps-from-being-dealt-damage) | The tactical bot attacks a player whom protection keeps from being dealt damage | Pri-3 | Open |
| [KI-117](#ki-117-a-searchs-world-keeps-what-the-real-game-did-with-hidden-cards-it-cannot-resample) | A search's world keeps what the real game did with hidden cards it cannot resample | Pri-3 | Open |
| [KI-118](#ki-118-a-fix-agents-report-can-be-empty-or-claim-work-its-commit-does-not-hold) | A fix agent's report can be empty or claim work its commit does not hold | Pri-3 | Open |
| [KI-119](#ki-119-a-triggered-abilitys-text-in-a-card-file-is-not-always-its-oracle-text) | A triggered ability's text in a card file is not always its Oracle text | Pri-2 | Open |
| [KI-121](#ki-121-the-host-never-marks-a-kicker-x-as-variable) | The host never marks a kicker {X} as variable | Pri-2 | Open |
| [KI-122](#ki-122-a-declaration-taken-back-for-an-enlisted-creature-tapped-for-mana-says-nothing) | A declaration taken back for an enlisted creature tapped for mana says nothing | Pri-2 | Open |
| [KI-124](#ki-124-what-a-land-could-produce-is-judged-for-at-most-256-answers-to-its-mana-abilitys-questions) | What a land could produce is judged for at most 256 answers to its mana ability's questions | Pri-3 | Won't fix: beyond practical scale |
| [KI-125](#ki-125-a-mana-abilitys-row-can-show-what-its-ability-lists-or-the-best-of-several-outcomes-under-replacement-effects-on-its-mana) | A mana ability's row can show what its ability lists, or the best of several outcomes, under replacement effects on its mana | Pri-2 | Open |
| [KI-128](#ki-128-the-non-tactical-bots-drop-an-attack-whose-phyrexian-cost-only-life-could-pay) | The non-tactical bots drop an attack whose Phyrexian cost only life could pay | Pri-2 | Open |
| [KI-129](#ki-129-a-cost-holds-at-most-six-discard-instructions) | A cost holds at most six discard instructions | Pri-3 | Won't fix: beyond practical scale |
| [KI-130](#ki-130-an-unless-pays-prompts-remaining-hint-lists-a-phyrexian-symbol-under-its-color-when-any-color-may-pay-it) | An "unless pays" prompt's remaining hint lists a Phyrexian symbol under its color when any color may pay it | Pri-2 | Open |
| [KI-131](#ki-131-harmonizes-tap-a-creature-caption-says-up-to-or-an-exact-amount-a-little-off-in-rare-costs) | Harmonize's tap-a-creature caption says "up to", or an exact amount, a little off in rare costs | Pri-2 | Open |
| [KI-134](#ki-134-the-browser-scenario-for-lands-planned-for-x-fails-on-main) | The browser scenario for lands planned for X fails on main | Pri-2 | Open |
| [KI-136](#ki-136-a-choice-listing-an-opponents-face-down-permanent-records-it-as-shown-to-the-chooser) | A choice listing an opponent's face-down permanent records it as shown to the chooser | Pri-2 | Open |
| [KI-135](#ki-135-a-permanent-keeps-the-names-of-at-most-two-abilities-that-chose-one) | A permanent keeps the names of at most two abilities that chose one | Pri-3 | Won't fix: beyond practical scale |
| [KI-137](#ki-137-the-host-shows-an-attacker-attacking-nothing-as-attacking-a-player-at-seat-0) | The host shows an attacker attacking nothing as attacking a player at seat 0 | Pri-2 | Open |
| [KI-138](#ki-138-a-cast-menu-rows-cost-reads-the-card-where-it-is-not-the-spell-it-would-be) | A cast menu row's cost reads the card where it is, not the spell it would be | Pri-2 | Open |
| [KI-139](#ki-139-the-language-model-clients-dack-fayden-prompt-runs-words-together) | The language-model client's Dack Fayden prompt runs words together | Pri-2 | Open |
| [KI-140](#ki-140-every-pending-effect-tooltip-says-it-ends-with-the-turn-if-no-spell-is-cast) | Every pending-effect tooltip says it ends with the turn if no spell is cast | Pri-2 | Open |
| [KI-141](#ki-141-a-creature-type-chosen-past-the-encoders-128-tokens-makes-no-training-sample) | A creature type chosen past the encoder's 128 tokens makes no training sample | Pri-3 | Open |
| [KI-142](#ki-142-the-ai-encoder-cannot-tell-apart-options-that-differ-only-by-their-grant-or-a-companion-from-exiling-to-pay) | The AI encoder cannot tell apart options that differ only by their grant, or a companion from exiling to pay | Pri-3 | Open |
| [KI-143](#ki-143-a-scenario-check-right-after-an-activation-can-read-the-pool-before-its-payment-finishes) | A scenario check right after an activation can read the pool before its payment finishes | Pri-2 | Open |

## Issues

### KI-114: The web client cannot name colorless mana for a sacrificed land that could also produce a color

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-21, moved from `docs/engine-design.md`'s known gaps |

**Notes.** Squandered Resources adds a type the land it sacrifices could produce (CR 106.7). The page offers {C} only where
colorless is the land's only type (`CardActions.ChoosesColorless`); a land that could produce {C} and a color is offered
its colors alone. No one is stuck: the ability is offered, as a mana ability is, and its other types answer it. Choosing
which kind of counter to remove (`Label::Counter`, Glen Elendra Guardian) is a modes question the page's modes dock answers,
and turning a face-down permanent face up is a card action it offers (`MainPage.Prompts.cs`, `TurnFaceUp`).

**Scenarios.** None.

### KI-3: Engine paths no scenario runs

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Fix queued | engine | 2026-10-15/16, fixes in the U-B-R-G Prowess run and the known-bug fixes |

**Notes.** Code written for every card the rule describes, where no implemented card reaches it yet. Each is a
candidate bug until something runs it; when a card that reaches one is implemented, its scenarios cover the path and
the line goes. So does a line whose path a Rust test drives through the real engine with test cards (the tests under
`crates/cantrip-script/tests`, `real_cards` among them with every card the engine has, and the rules crate's own), where
no printed card can reach it.

| Path | Covered instead by | Why no scenario |
| --- | --- | --- |
| A fused split spell no longer allowed once its proposal is complete (CR 601.6: `actions::cast` asks `CardDef::spell_rules` again of the spell on the stack), its mana value the sum of its halves' | the check before the cast (every Gaddock Teeg scenario); a back face cast transformed, whose spell has its front face's mana value (CR 712.8c), in a Rust test (`transformed_cast_tests.rs`) | Fuse is built (Turn // Burn): a scenario casting it fused under Gaddock Teeg-style rule now covers this, queued. |

**Scenarios.** None, by definition.

### KI-35: The tactical bot counts no summoning-sick creature as a mana source under Shang-Chi

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | client | 2026-10-16, review of Shang-Chi, Master of Kung Fu |

**Notes.** Under Shang-Chi, Master of Kung Fu ("You may activate abilities of creatures you control as though those creatures
had haste", CR 302.6; `CardDef::activate_as_though_haste`) the engine offers the {T} mana abilities of creatures that came
this turn, Shang-Chi's own among them, but the tactical bot's mana planning
(`dotnet/src/Cantrip.Client/Tactics/TacticalPlay.Priority.cs`, the source counts that skip `SummoningSick` creatures) leaves
them out, so it plans with less mana than it has. The view has no word for the permission; a fix counts a sick creature as
a source when the actions offered for it include a mana ability, or the view says its {T} abilities may be activated.

**Scenarios.** none: a bot's plan is not scenario'd. Shang-Chi's scenarios show the engine side.

### KI-49: The tactical bot does not know infect or poison

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | client | 2026-10-17, review of Inkmoth Nexus |

**Notes.** The engine is right: damage from a source with infect is -1/-1 counters to a creature and poison counters to a
player (CR 702.90b-c, 120.3b, 120.3d), and ten poison counters lose the game (CR 704.5c). The tactical bot
(`dotnet/src/Cantrip.Client/Tactics`) has no notion of either: `CardReading.cs` passes "infect" over as a keyword it does
not know, and nothing in the bot reads a player's poison counters, so it scores an infect creature's damage (Inkmoth
Nexus, Blightsteel Colossus) as life lost, races and blocks as if it were, and does not see a player near ten poison. A fix
reads infect into `Keywords` and gives the bot's damage and race arithmetic a poison clock beside the life one. The web
client shows poison on the player rail and the language-model client in its text, so a person and the model are not
affected.

**Scenarios.** none: a bot's judgement is not a rules outcome; a bot-lab duel with infect decks would show it.

### KI-57: The tactical bot does not know islandwalk

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | client | 2026-10-17, adding Lord of Atlantis |

**Notes.** The engine is right: a creature with islandwalk can't be blocked while the defending player controls an Island
(CR 702.14c, `turn::can_block`), and the blockers question offers only the pairs the rules allow, which the bots follow. The
tactical bot's own combat arithmetic (`dotnet/src/Cantrip.Client/Tactics`, `Unit.CouldBlock` and `Exposure`) reads no
landwalk: `CardReading.cs` passes "Islandwalk" over, and a unit knows nothing of the defending player's lands, so the bot
plans its attacks as if an islandwalker could be blocked by an opponent with an Island, and counts its own blockers against
one. A fix reads islandwalk into `Keywords` and makes a landwalker unblockable in the bot's planning when the defending
seat controls a land of its type. The web client shows the keyword on the card, and the language-model client reads it.

**Scenarios.** none: a bot's judgement is not a rules outcome.

### KI-71: The tactical bot's combat planning ignores bands

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | client | 2026-10-17, review of banding |

**Notes.** The engine is right (CR 702.22h-k). The tactical bot's blocks and attacks (`TacticalPlay.Combat.cs`, `Combat`) are
judged pair by pair: it does not know that blocking one member of a band blocks all of it, nor that the opponent divides its
blocker's damage among the band (702.22k) or its attacker's damage among banding blockers (702.22j). Its band choice and the
divisions it makes are on purpose (`TacticalPlay.Choices.cs`, the engine's sparing suggestion).

**Scenarios.** none: a bot's judgement is not a rules outcome.

### KI-78: The tactical bot attacks a player whom protection keeps from being dealt damage

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | client | 2026-10-18, Absolute Virtue |

**Notes.** The engine is right: damage from an opponent's sources to a player with protection from them, or from everything,
is prevented (CR 702.16e, 702.16j, 702.16k: Absolute Virtue, Teferi's Protection, The One Ring), and the host tells both seats
("They have protection from Bot1."). Targeting is the engine's, so the bot never aims a spell at such a player. But its attack
planning (`TacticalPlay.Combat.cs`, `Combat`) scores the damage an unblocked attacker would deal that player as life lost, so it
may attack into a protected player for nothing and leave itself open. A fix tells the bot, in the view, whom each player is
protected from (a field on `PlayerSummaryView` rather than the text), and has the attack plan count no damage to them.

**Scenarios.** none yet.

### KI-98: A station permanent lists its reached striation's keyword again, as granted

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client / host | 2026-10-20, Wurmwall Sweeper's browser scenario (KI-25) |

**Notes.** A wrong display of a right game state. A station card's striation abilities (CR 721.2a) are its own static
abilities, which give the permanent the keyword while it has the charge counters (`station::charged`,
`statics::have_flying`). The host's `ability_changes` counts every keyword the permanent has and its card definition does
not as granted, so Wurmwall Sweeper at four or more charge counters reads "... / 4+ | Flying | 2/2 / Flying": the
striation's flying said a second time, as though another effect gave it. A fix leaves out of `granted_abilities` what the
permanent's own reached striation prints.

**Scenarios.** `wurmwall-sweeper-shows-its-striation-power-and-toughness-in-the-browser` (browser) checks the face in hand and
below the threshold only, for this reason.

### KI-101: A player out of a game that goes on sees it a step late

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | host / client | 2026-10-21, a player out of time in a game of four (`TableClockTests`) |

**Notes.** A wrong display of a right game state. When a player of a game of more than two leaves it - their clock ran out
(`GameSession.TimeOut`), or they lost - the game goes on for the rest (CR 800.4a), but that player's own page is told only
once the run being played is published: the host shows a run in progress to its answering seat alone, and every other seat
keeps the position from before it (`session.rs`, `view_for`). Until then the page shows the player still in, and its last
question; `GameClient` drops the question once its own view shows it out. The server judges who is still in from every
seat's view and its own leaves, so the rules are unaffected (`GameSession.InGame`, `TimeOutTests`). A fix sends a seat that
has left its own view at once, as the answering seat's is.

**Scenarios.** None; `TableClockTests.In_a_game_of_four_a_player_out_of_time_leaves_and_the_others_play_on` answers the others'
questions until the departed seat is told.

### KI-108: What can be cast is over-offered where a mana ability's yield is unknown or its cost is shared

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | engine | 2026-10-21, review of the exact payability check |

**Notes.** A choice the client offers that cannot be made; the payment takes the cast back (CR 733.1), so no game state is
wrong. A mana ability whose yield is not stated (Sunken Ruins, X costs, Phyrexian or {2/W} symbols in its cost) makes
every cost count as payable. Life, energy and sacrifices that several mana abilities would share are judged for each
alone: three Mana Confluences at 1 or 2 life count as three mana, so Raffine is offered. And the search's time budget
falls back to offering the cast where the mana is hard to judge (about 1 ms of search: several Caverns of Souls and
Chromatic Stars under two Mana Flares with a Black Lotus), as the walk over the target announcements a cost change could
read does past `ANNOUNCEMENT_BUDGET` (`some_cast`): only where a cost change tells apart the candidates of a slot of many
targets among more than a dozen (one that reads each target, as "{1} less for each target you control" does), since
candidates no change tells apart are judged by how many of them there are (Fireball). A kicker's
or optional cost's parts other than mana are left out of that judgment, which may offer a kicked announcement whose
life or other cost then can't be paid. A permanent Urza, Lord High Artificer could tap that also has a mana ability
without {T} counts Urza's tap apart from its own abilities, as if both could be used (`available_without`, `Repeat::Tap`).

The filter lands now share this unstated yield: `lands::filter_two` (Sunken Ruins, Mystic Gate, Fetid Heath, Cascade
Bluffs, Fire-Lit Thicket, Flooded Grove, Graven Cairns, Rugged Prairie, Twilight Mire, Wooded Bastion) and Cascading
Cataracts' {5} ability set no `yields_any`, so each counts every cost as payable while it is untapped. A plain
`yields_any(2)` would misdescribe the {X}{X}/{X}{Y}/{Y}{Y} choice, so a fix needs a yield the payability check can read
for a choice of colour pairs. The review of PR #564 saw it at its sharpest: Mystic Gate's filter edited to cost `{W/U}`
without {T} was offered with only a Swamp and the Gate on the battlefield, though nothing there could pay {W/U} - the
unknown yield counted the filter as able to pay its own cost.

**Scenarios.** None yet.

### KI-116: A resolution's "unless pays" question shows only the mana and life parts of its cost

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-21, KI-113's fix (resolution costs with any part) |

**Notes.** A wrong display of a right game state. The engine now pays every part of a cost a resolving spell or ability
asks for (`pay_during_resolution`, KI-113's fix): permanents, discard, exile, counters, behold and the rest. But the
client's `{cost}` text in the `UnlessPays` question is built from the mana and life parts alone (`Messages.cs`,
`CostText`), so a card whose cost has other parts must spell them out in its own question text. A fix sends the cost's
other parts with the prompt and has the client write them.

**Scenarios.** None yet.

### KI-117: A search's world keeps what the real game did with hidden cards it cannot resample

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | harness | 2026-10-18, building `cantrip_ai::determinize` |

**Notes.** `determinize` (`crates/cantrip-ai/src/worlds.rs`, docs/ai-player-plan.md "Worlds") deals the hidden cards of a
fork again, without journalling, and asks the waiting question again (`Game::ask_again`). Some facts of the real game stay
in the world, and a seat's search could read them:
- **Which hidden object an event named.** If the opponent discards a card, the history says whether it was the card just
  drawn or one it had kept (two different objects). The world keeps that object, with a new card. Two real games that
  differ only there make different worlds. The redaction test skips dealing a world for such a pair ("histories apart":
  10 positions of 176 pairs, `accumulate-wisdom` among them). A fix renumbers the seat's unseen hand objects too, or
  deals the hidden objects as a set.
- In hidden mode, the cards that began outside the game (sideboards) stay what they are when the prior has no list for
  them.
- A face-down object (morph, manifest) may be rebound to a card that could not have been face down that way.
- A staged question (`Game::ask_then`, Duress's choice) is not asked again, so its options are those of the real cards.
  So are the runs of the lower frames, and the answers `ask_again` replays may not fit the new cards.
- What the opponent has looked at in its library (Ponder) is not kept as a constraint. The seat does not know it either,
  but the opponent's play in the world assumes a library it never looked at.
- `loop_watch` keeps the hashes of the real positions, so a world catches only loops that begin after the fork.
- Dead rows the seat never saw keep their real timestamps. On live objects, a timestamp whose move an event
  recorded is kept, and an unrecorded one becomes 0.
- Under `CANTRIP_VERIFY`, the pool sizes are asserted. An object from a library that went to a zone no seat sees (Gone)
  would make that assert fire. Whether any card in content does that has not been checked.

**Scenarios.** none: these are properties of a search's world, checked by `crates/cantrip-ai/tests/redaction.rs`.


### KI-118: A fix agent's report can be empty or claim work its commit does not hold

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | tooling (agents) | 2026-10-22, reviewing the card pull requests #547-#564 (`review-card-pr`) |

**Notes.** The Sonnet agents that apply review fixes (`review-card-pr` step 3, `coordinator.md` step 2) do not always
report what they did. Twice (Sulfurous Springs, PR #553; Underground River, PR #554) the final report was the single word
"placeholder", though the commit was right and the issue log written - the second time after the prompt asked for "the
actual report, not a placeholder". Once (Hunter's Axe, PR #547) the report said the content JSON's reminder text was
fixed and the commit held only the scenario changes; the re-review caught it, and the agent then made the edit. Nothing
wrong landed: the coordinator read each commit (`git show --stat`, the diff, `git status --short`, the issue log) before
landing. A fix makes that check part of the procedure rather than the coordinator's habit: `review-card-pr` and
`coordinator.md` say to verify every fix commit's contents against the findings before the re-review and the landing,
whatever the report says - and find why the hand-back carries "placeholder" (the agent's final message, or the harness).

**Scenarios.** None: a property of the agent workflow, not of the engine.

### KI-119: A triggered ability's text in a card file is not always its Oracle text

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | cards | 2026-10-22, writing the CR 603.10 look-back change |

**Notes.** The text a card file gives a triggered ability (what the client shows on the stack and in prompts) differs
from the card's Oracle text for 159 abilities: an old self-reference instead of "this creature", one Oracle sentence
split into two abilities ("When X enters or dies"), modal bullets flattened into a line, reminder text, and granted or
delayed abilities' texts. Look-back now reads "from anywhere" and "leaves a graveyard" from that text (CR 603.6c,
603.10a); `crates/cantrip-cards/tests/zone_words_match_oracle.rs` holds those words to the Oracle text, so the rules
are safe. A fix makes every ability's text a sentence or clause of its card's Oracle text and tightens that test to
say so.

**Scenarios.** None: display text.

### KI-121: The host never marks a kicker {X} as variable

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | host, client | 2026-10-22, building "X can't be 0" (CR 107.3a) |

**Notes.** `PayCostsPrompt.variable_kicker` (`crates/cantrip-host/src/protocol.rs`) is never set, so it is always
false. The web client reads it for a kicker {X} (`Announcement.TotalAt`, the stepper's labels in
`MainPage.Prompts.cs`), so Thieving Skydiver's kicker is shown as a fixed kicker: its totals and button captions are
likely wrong. The engine's announcement itself is right; what is paid and put on the stack does not change.

**Scenarios.** None: display.

### KI-122: A declaration taken back for an enlisted creature tapped for mana says nothing

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-22, reviewing optional costs to attack (CR 508.1g-j) |

**Notes.** While the mana costs to attack are paid (CR 508.1i), the payment offers the mana abilities of a creature
chosen to be enlisted. Using one is legal and makes the declaration illegal, so the engine takes it back and asks for
attackers again (CR 508.1, 733.1) - correctly, but the page starts the declaration over without saying why. The page
should say the enlisted creature was tapped.

**Scenarios.** None: display.

### KI-124: What a land could produce is judged for at most 256 answers to its mana ability's questions

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Won't fix: beyond practical scale | engine | 2026-10-22, review of the CR 106.7 effect-decided yields |

**Notes.** The types a permanent could produce (CR 106.7, `actions::producible_types`) count a mana ability whose mana is
its own effect's by running the effect for every answer to the questions it asks (`effect_productions`): each color the
activation may name, each multiset of "N mana in any combination of colors", each "you may", mode or player. Past 256 runs
it is not judged: its listed mana stands for it, and with `CANTRIP_VERIFY` it panics naming the card. "Could produce" is
asked only of lands (Squandered Resources' sacrifice, `colors_from_sacrifice`), and no printed land comes near: the most
is Cascading Cataracts, five mana in any combination of colors, 126 runs; a filter land is 3 runs, a single color 5. Seven
mana in any combination of five colors (330) on one land's ability, or two such questions on one, is what would reach it.

**Scenarios.** None: no printed card reaches it.

### KI-129: A cost holds at most six discard instructions

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Won't fix: beyond practical scale | engine | 2026-10-24, review of the CR 701.9c discard costs |

**Notes.** A cost keeps each discard instruction that names a characteristic apart from the others (`cantrip_api::Discards`:
retrace's "discard a land card" and escalate's "discard a card" are a land card and any card, CR 702.81a, 702.120a), the
instructions of any cards adding up into one, in six parts. Past six, the cost asks for a discard no hand can pay, so the
spell or ability can't be cast or activated: never a wrong discard. A cast merges at most six sources of cost (its own
additional cost, a permission's such as retrace, a chosen additional cost, an optional cost, escalate and a cost its targets
define), and no printed cost has two discard instructions, so play cannot reach it.

**Scenarios.** None: no printed card reaches it.

### KI-125: A mana ability's row can show what its ability lists, or the best of several outcomes, under replacement effects on its mana

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | host, client | 2026-10-22, building replacement effects on a tap for mana (CR 106.12b) |

**Notes.** The host's row for a mana ability (`prompt.rs`, `row_yield` over `replaced_mana_yields`) says what a tap would
produce once the replacement effects on its mana apply, where its `produces` and `produces_of_any_one_color` can say it:
fixed mana, and N of the color the activation names. Two cases they can't:
- the color named changes nothing the ability adds (Contamination's or Infernal Darkness's {B} for an any-color land): the
  activation must still name a color, and the client names one only while `ProducesOfAnyOneColor > 0`, so the row keeps
  what the ability lists ("1 of W/U/B/R/G");
- the order the player chooses the effects in gives different mana (Damping Sphere and Mana Reflection on Ancient Tomb:
  {C}{C} or {C}): the row shows the outcome with the most mana.

The engine is exact in both: the mana produced, the order asked (CR 616.1) and what the player has to pay with
(`available_without`) all go through the same effects. Only what the row says, and the client's auto-tapping hints that
read it (`AutoMana`, `AutoPlay`, the tactical bot), can be off. A fix is a protocol field for "the activation names a
color" apart from how much of it the ability adds, or the outcomes listed per color.

**Scenarios.** None: display. `prompt::row_yield_tests` shows which rows fall back.

### KI-130: An "unless pays" prompt's remaining hint lists a Phyrexian symbol under its color when any color may pay it

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | engine (display) | 2026-10-24, review of Oath of Nissa |

**Notes.** Whether an "unless pays" cost is offered as payable is right: `pay_during_resolution` asks
`spend_any_color_for(.., SpendingFor::Other)` and `pool_pays` converts the cost to any color after its Phyrexian and {2/C}
symbols are announced (CR 609.4b, 118.12). But the prompt's `remaining` hint, the mana still owed that the client and bots
read, is built with `paid_with_any_color`, which leaves Phyrexian symbols alone, so under Mycosynth Lattice a {B/P} is still
listed as needing black. A fix converts the Phyrexian symbols' colored halves too when any color may pay them.

**Scenarios.** None: display.

### KI-131: Harmonize's tap-a-creature caption says "up to", or an exact amount, a little off in rare costs

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | engine (display) | 2026-10-24, review of Nature's Rhythm |

**Notes.** The reduction harmonize's tapped creature makes is right (its power off the total cost after X and increases,
before a minimum; CR 702.180a, 601.2f); only what `choose_reducer` shows can be imprecise. (1) A targeted harmonize spell
says "up to" whenever any cost changer is in play, though most changers don't read targets; `Watching` / `deltas_watched`
already knows which do. A spell with the Arcane subtype says "up to" with no splice card in hand. (2) It shows an exact
amount though the cost can still change when the harmonize cost has {2/C} symbols, an `exile_from_hand_for_less`
reduction or a `VariableCost::SacrificeAnyNumber { less_each }`. No printed harmonize card reaches (2), and (1) is never
false, only less precise.

**Scenarios.** None: display. `HarmonizeTests` covers the X case.

### KI-139: The language-model client's Dack Fayden prompt runs words together

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | language-model client | 2026-10-25, PR #576 (Dack Fayden, Helping Hand) |

**Notes.** The ChoosePlayer prompt's new `subject` reads "...to gaincontrolof Llanowar Elves" in
`dotnet/src/Cantrip.Client.Llm/Decisions.cs` (~1581): the caption's words are joined without spaces. The web client's
caption is right. A fix builds the sentence with its spaces.

**Scenarios.** None: the language-model client has no scenario runner.

### KI-138: A cast menu row's cost reads the card where it is, not the spell it would be

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | host / client | 2026-10-25, review of the as-cast cost changes |

**Notes.** What the engine offers and what a cast is paid with read the spell the card would be on the stack (CR
601.3e, 601.2f). The menu's cost figures - `cast_costs`, `cast_total` and `alternative_cast_total`, read by
`crates/cantrip-host/src/prompt.rs` - still read the card where it is, so a reduction that applies only to the spell
(Herald's Horn with Arcane Adaptation, on an opponent's card you cast) shows the unreduced figure on the row. No
implemented card combination reaches it. A fix reads those helpers through the same `AsSpell` judgement.

**Scenarios.** None yet: no implemented combination shows it.

### KI-137: The host shows an attacker attacking nothing as attacking a player at seat 0

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | host / client | 2026-10-24, review of the CR 506.4c removal from combat |

**Notes.** A creature attacking a planeswalker or battle that is removed from combat goes on attacking nothing (CR
506.4c; `Attacker::target` is `Target::None`). `crates/cantrip-host/src/view.rs` (`combat`, ~857) sends it to the client
through its catch-all arm as `AttackTargetKindView::Player` with seat 0, so the client is told it attacks a player, at a
seat no player has. The game state is right; only the display is wrong. A fix adds a "nothing" kind to
`AttackTargetKindView` (and its C# mirror) and has the client draw the attacker as attacking nothing.

**Scenarios.** None: the engine's own tests (`removed_from_combat_tests.rs`) show the state; a host view test can pin it.

### KI-136: A choice listing an opponent's face-down permanent records it as shown to the chooser

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | engine (AI knowledge) | 2026-10-24, review of PR #565 |

**Notes.** When a `ChooseCards` or `ChooseTarget` request lists an opponent's face-down permanent, the engine records
`Event::Shown` for the chooser (`crates/cantrip-rules/src/lib.rs` ~1068-1072 and ~2327-2335), though `hidden_from` already
treats that permanent as hidden from them (CR 708.5): Liliana of the Veil's controller separating piles that include an
opponent's morphs "learns" them in the history. Only `cantrip-ai` (`knowledge.rs`, `worlds.rs`) reads `Shown`, and
nothing that plays a real game uses its search; the host and the client never read it. A fix leaves out of `shown`
whatever `hidden_from` hides from the chooser.

**Scenarios.** None: the history record only; a `cantrip-ai` redaction test can pin it.
### KI-135: A permanent keeps the names of at most two abilities that chose one

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Won't fix: beyond practical scale | engine | 2026-10-24, review of the chosen-choice links |

**Notes.** A card name chosen for a permanent is kept with the ability that chose it (`cantrip_state::Choices`, CR 607.2d),
one place for each such ability, `CHOSEN_NAMES` (two) in all; the same ability choosing again replaces its own. A third
ability's name is not kept, so that ability reads no chosen name: never another ability's. An ability is told apart by
its text and how the permanent acquired it, so the same card's ability given at two different times - two effects, two
timestamps - is two abilities, each with its own place, the earlier one's never read again once its effect has ended.
Reaching it takes three such abilities that each choose a name, all on one permanent that stays the same object: a name
chosen as it entered (Anointed Peacekeeper) and as abilities of it resolved (Petrified Hamlet's, Medomai's Prophecy's) - a
copy effect does not make it enter or its enters abilities trigger again. No printed card reaches it.

**Scenarios.** None: no printed card reaches it.

### KI-134: The browser scenario for lands planned for X fails on main

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-24, the Dauntless Bodyguard work (`test.cmd` browser run) |

**Notes.** `ScenarioInBrowserTests.Lands_planned_for_x_raise_it_and_are_tapped_as_the_spell_is_paid_for` fails on a clean
origin/main (a3276106b), with the host rebuilt: the page does not tap the Mountains it planned for X itself
("Mountain#2.Activate ... answered by the page itself"). The engine side is unaffected; what the page does with lands it
planned for X as the spell is paid for has regressed. Needs bisecting.

**Scenarios.** The browser scenario named above.

### KI-128: The non-tactical bots drop an attack whose Phyrexian cost only life could pay

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-23, review of Norn's Annex |

**Notes.** The engine is right: a cost to attack's {W/P} (Norn's Annex) is paid with {W} or 2 life (CR 107.4f), asked as the
attack is paid. But the declare-attackers prompt's `CanPay` is the mana the player could make (`turn.rs`,
`available(..).most_unrestricted()`), and `AttackCosts.Within` (used by `AutoPlay` and `RandomPlay`) trims an attack to it
by mana value, so those bots never attack into Norn's Annex with no white mana though 2 life would pay. The web client's
attackers dock says "(you can make up to N mana)", which is true but says nothing of life. A fix gives the prompt the life
that could pay the Phyrexian symbols beside the mana, and `Within` counts each symbol payable either way.

### KI-140: Every pending-effect tooltip says it ends with the turn if no spell is cast

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | client | 2026-10-25, review of Way of the Paradox |

**Notes.** `PlayerRail.cs` appends " It ends with the turn if no spell is cast." to the tooltip of every line in a player's
`PendingEffects`, which is true only of "the next spell you cast can't be countered". It is wrong for "You may play an additional
land this turn." (it ends with the turn whether or not a spell is cast), and already for the skip-turn, legend-rule and
control-of-a-player lines. A fix gives each line its own tooltip text (or none), from the host, beside the line.

**Scenarios.** None yet: a browser check of the tooltip would show it; the host test
`an_additional_land_play_is_shown_to_both_seats_and_gone_after_the_turn` covers the line only.

### KI-141: A creature type chosen past the encoder's 128 tokens makes no training sample

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | harness | 2026-10-19, the recorder's whole-corpus test (`crates/cantrip-ai/tests/record.rs`, ignored) |

**Notes.** A "choose a creature type" question (Arcane Adaptation, Conspiracy, Artificial Evolution and the like) offers
every creature type, some 300 options. `cantrip_obs::encode` holds a position in at most 128 tokens and cuts options
past what fits; when the type chosen is one of those cut, the network cannot be shown the answer, and the recorder
(`cantrip_ai::record`) skips that decision with "chosen option cut by the cap". The whole corpus has 58 such answers;
the test lists the kind in `EXPECTED`. Generation 0's decks ask no such question. The network will equally be unable to
pick a cut type when it plays. A fix splits a long list of names into steps (a first letter, then the name) in
`cantrip_obs::factor`, or ranks the types by the creatures in the game before cutting.

**Scenarios.** the corpus's creature-type choices whose type falls past the cut, which the recorder's ignored
whole-corpus run counts (`cargo test --profile cards -p cantrip-ai --test record -- --ignored`).

### KI-142: The AI encoder cannot tell apart options that differ only by their grant, or a companion from exiling to pay

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-3 | Open | harness (AI player) | 2026-10-25, bringing the AI player up to main |

**Notes.** The engine is right; only what the network sees is coarser. `cantrip_obs::encode` gives each option its
kind, source and ability index. Two options that differ only by `granted_by` or `paid_by` (Kili, the Resourceful's free
first equip beside the paid one; two permissions to cast the same card) encode alike, and `ActionKind::TakeCompanion`,
newer than the encoder's action vocabulary (`ACTIONS`, 12 kinds), is encoded as `ExileToPay`. Answers are still exact:
`OptionObs` keeps the kind and both grants, so the canonical order and the way back to the engine's option tell them apart,
and the search plays each out. A fix adds the grants and the companion to the encoding, which changes the layout and
`encoder_hash`, so it waits for the next model that is trained from scratch, and its tiny test model is exported again.

**Scenarios.** `kili-the-resourceful-the-first-equip-each-turn-is-free-with-an-enduring-story` (the factoring's round
trip, `crates/cantrip-ai/tests/factoring.rs`).

### KI-143: A scenario check right after an activation can read the pool before its payment finishes

| Priority | Status | Area | Found |
| --- | --- | --- | --- |
| Pri-2 | Open | harness (`crates/cantrip-script`) | 2026-10-27, review of the morph and disguise cards |

**Notes.** A `? Bot1.Mana =` line placed right after an `Activate(..)` line whose cost the pool could pay in more than
one way (`Spend: R, G` for a hybrid cost) runs at the payment's pending `AssignMana` question, before the default answers
finish paying, so it reads the whole pool (`{B}{B}{B}{B}{R}{G}`) rather than what is left. The scenarios of Riftburst
Hellion put a refusal line between the activation and the check, which lands at the next priority after the payment.
A fix either finishes the pending payment before a `?` check runs or refuses a check made mid-payment, so a check can't
silently read a state no player would see.
