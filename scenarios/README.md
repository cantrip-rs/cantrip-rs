# Game scripts

Each `*.cantrip` file here is one test case of the engine's scenario corpus. A script is two ordered decklists, a
sparse list of actions, and the state the game should end in. The harness is `crates/cantrip-script`
(parser, runner, state dump): `cargo test --profile cards --workspace` plays every scenario whose cards the engine has
(`crates/cantrip-script/tests/scenarios.rs`), and `cargo run --profile cards -p cantrip-script --bin corpus -- --card "Name"` runs one card's
scenarios and prints the full report of any that fail (`docs/adding-cards.md`). The corpus began as the C# engine's
test suite; the C# engine and its harness are gone, and this folder is the engine-independent record of what a card does.

```
# comment lines start with #

[Bot1 Deck]            top of the library first; the first seven cards are the opening hand
Island
4 Island               (or: Island x4)

[Bot2 Deck]
...

[Actions]
Bot1.Hand.Island.Play
Bot2.Hand.Duress.Cast(Bot1)
[T5 Bot1.Main2]        wait for a step: [Bot1.Main2], or with a turn number
? Stack.Count = 1      check the state right here
! Bot1.Hand.Island.Play   Bot1's next priority must not offer this

[Expect]
Bot1.Graveyard = Counterspell
```

Bot1 goes first and libraries are not shuffled, unless the script has an `[Opening]` section (below).

A line answers for the player it names, as the rules ask that player: while another player controls them (CR 723:
Mindslaver), `Bot2.Hand.Shock.Cast(Bot2)` is still Bot2's line, though Bot1 makes the choice; a table drives it from
the seat answering (Bot1's, when Bot1 is a page).

A game of three or four players names more decks, `[Bot3 Deck]` and `[Bot4 Deck]`, and everything that names a player
(`Bot3.Hand.Island.Play`, `Bot1.ChoosePlayer(Bot4)`, `Bot3.Life`) takes them in turn order after Bot2. The first player of a
game of more than two draws on their first turn (CR 103.8a). A script with no `[Bot3 Deck]` is the duel it always was.


## In the browser: `# Browser` and `Client.`

A scenario also plays at a real table with Bot1 a browser page (`docs/scenarios-in-the-browser.md`): the table plays the
script, and the page is clicked through each of Bot1's answers, which must be exactly what the line means - so every
line is shown to be reachable by a person clicking. A `# Browser` line puts a scenario in that suite (a wall-clock test:
`tools\test.cmd /filter "FullyQualifiedName~ScenarioInBrowser"`); `tools\scenario-in-browser.cmd name` plays any one in a
window to watch.

A key starting `Client.` is about what the page shows, not the game's state, and is skipped by a headless run:

```
? Client.Bot1.Stack.Count = 1            while an equip ability is paid for, the stack row shows it alone
? Client.Bot1.Arrows = player 2; Blood Token   what a resolving ability's arrows point at
```

| Key | The page for that player shows |
| --- | --- |
| `Client.Bot1.Stack`, `.Hand`, `.Battlefield`, `.OpponentBattlefield` | the cards in that row, compared as a set; each with `.Count` |
| `Client.Bot1.OpponentHand` | the cards known in the opponent's hand: empty when nothing in it has been shown |
| `Client.Bot1.Button.Undo` | `shown` or `absent`: a button whose label holds the text |
| `Client.Bot1.Button.No blocks.Enabled` | `true` or `false`: whether the dock's button whose label holds the text can be pressed (a disabled one is drawn but waits, as the blockers' button does for a requirement unmet); `absent` with no such dock button |
| `Client.Bot1.Arrows` | what the latest resolution's arrows point at: card names, or `player 2` |
| `Client.Bot1.TargetLabels` | the labels on the arrows from the spell on the stack to its targets, as drawn: which part of the spell each target is for (`Turn; Burn` for a fused split spell's halves), empty where none says |
| `Client.Bot1.Over` | `true` once the page says the game is over |
| `Client.Bot1.Prompt.Title`, `.Detail` | the dock's caption and its line under it for the question asked now: what the choice says it does |
| `Client.Bot1.Prompt.Modes` | the options of a choice of modes, as buttons: an additional cost's, with its cost in words |
| `Client.Bot1.Confirm` | the label of the button the last choice of cards was confirmed with ("Behold it"); nothing when the last click answered by itself |
| `Client.Bot1.Log.Has` | the game log has a line with exactly this text (cards in it in brackets), shown as the value or as nothing |
| `Client.Bot1.<Row>.<Card>.TypeLine`, `.Rules`, `.Status`, `.Annotation`, `.Counters`, `.Damage`, `.FaceDownName`, `.Halves` | what the face of that card says: its type line (`Kindred Artifact - Shapeshifter, every creature type`), its rules lines joined by ` / ` (granted keywords after the printed text, a printed line that was lost prefixed `(grey) `, any other lost ability as `Lost: ...`), the status words at its foot (`CAN'T BE COUNTERED`), the line over its art (`Playable this turn`, or a permanent's one chosen quality: the name Petrified Hamlet chose). `.Counters` is the pills drawn across its art, `3 × incubation` (several joined by `; `), empty with none. `.FaceDownName` is what a face-down card really is, to the page that may look at it (CR 406.3; empty to one that may not). `.Damage` is the marked damage drawn beside its power and toughness, `-3`, empty with none. `.Halves` is a split card's halves as its face draws them side by side, `Turn; Burn`: a fused split spell on the stack is both, a half cast alone none. `<Row>` is `Stack`, `Hand`, `Battlefield`, `OpponentBattlefield`, `Exile`, `OpponentExile` (a hidden card is `(hidden)`) or `Graveyard`, which the page opens to read, as a player does |
| `Client.Bot1.Mana` | the mana pool's chips as drawn, `2 Blue (land abilities); 1 Red`: how many of each kind, and the caption of mana a restriction limits (CR 106.6) |
| `Client.Bot1.Facts`, `.OpponentFacts` | the facts beside a player's life total, as a set: `The next spell you cast this turn can't be countered` |
| `Client.Bot1.Exile.Lit`, `.Caption`, `.Tooltip` (and `OpponentExile.`) | the Exile button over a player's piles: `true` when a card in it may be played, what it says, its tooltip |
| `Client.Bot1.Sideboard.Caption`, `.Tooltip` (and `OpponentSideboard.Caption`) | the Sideboard button over a player's piles, into the cards they own outside the game: what it says (`Sideboard  2`) and its tooltip; nothing where there is no button, as on the opponent's rail, whose sideboard is hidden |
| `Client.Bot1.Graveyard.Lit`, `.Note` | the graveyard pile: lit when a card in it can be cast, and the note its tooltip adds |
| `Client.Bot1.Dock` | the dock's buttons in order |
| `Client.Bot1.Menu` | the rows of the menu the last card action opened, empty when it opened none (a card with one thing to do); a row is judged when the menu opens, not after the click |
| `Client.Bot1.Targeting` | what the dock said while the last card action's targets were picked, its title and its line: `Target for Dragonfire Blade / Costs {1} targeting Dwarven Mauler  ·  1 possible target`; empty for an action with no targets |

Only Bot1 is a page for now, so only `Client.Bot1.` keys can be judged.

## Known bugs

A scenario asserts what the rules require, never what the engine happens to do. When the engine or a card gets it
wrong, the scenario is still committed - asserting the correct outcome - with a marker line at the top:

```
# Known bug: Clarion Conqueror bars an ability of a card in hand (CR 109.2).
```

A marked scenario is expected to fail (a panic counts): `cargo test` passes it while it fails, and `corpus` counts it
under "known bugs", apart from the failures. The day it passes, both report it as fixed - an error until the marker
line is removed in the same change as the fix. A scenario is never deleted, weakened or rewritten to pass because the
engine is wrong, and never kept outside `scenarios/`.

Every known bug also has an entry in `docs/known-issues.md` - its severity, notes and the scenarios that show it -
added with the marked scenario and deleted with the fix.

## Outside the game: `[Bot1 Sideboard]` and `[Basic Lands Outside]`

```
[Bot1 Sideboard]       the cards Bot1 owns outside the game (CR 100.4, 400.11a), as a deck is written
Lightning Bolt
2 Counterspell

[Basic Lands Outside]  a Limited game: every sideboard holds any number of these basic lands
Island
Mountain
```

A player's sideboard is what an effect that brings a card into the game from outside it chooses among (CR 400.11b: North
Wind Avatar). Only its owner may look at it (`BotN.Sideboard` is the dump's, which sees everything). A card brought in is
in the game until it ends; a basic land of `[Basic Lands Outside]` brought in leaves another of it outside, since a Limited
sideboard holds any number (Magic Tournament Rules). A player with no section has nothing outside the game.

```
[Bot1 Companion]       the companion Bot1 chose at deck building (CR 702.139a): one card, dealt into its sideboard
Lurrus of the Dream-Den

[Minimum Deck Size]    the format's minimum deck size, which a companion's condition may name: sixty in Constructed
60
```

A companion chosen at deck building is revealed before the game, nothing asked, if the deck fulfils its condition (CR 103.2b);
otherwise it stays a sideboard card. A scenario names no format: it is Freeform, with no deck-construction restriction and
no minimum deck size, unless a `[Minimum Deck Size]` section gives one (sixty in Constructed, CR 100.2a; forty in Limited,
CR 100.2b). `Bot1.Sideboard.Lurrus of the Dream-Den.TakeCompanion` takes the special action that puts the revealed
companion into the hand for {3} (CR 116.2g), paid with the mana abilities that follow as a cast is; `BotN.Companion` is the
card BotN revealed, empty for none. In the browser, `Client.Bot1.Companion` and `OpponentCompanion` are the cards in the
companion windows, and their `.Corner` where each stands.

## A real start: `[Opening]`

```
[Opening]
Chooser: Bot2          optional; Bot1 chooses otherwise
```

With an `[Opening]` section the game starts as a real one does (CR 103): nobody is fixed to go first, the chooser is
asked to play or draw first **before any hand is dealt**, and then each player, the starting player first, keeps or
mulligans (CR 103.5, the London mulligan). The hands are still the first seven of each decklist; a mulligan shuffles
the hand back with the game's own generator, so after one a script names counts, not cards — or uses a deck of one card
repeated, so the shuffle cannot matter.

| Line | Answers |
| --- | --- |
| `Bot1.PlayFirst` · `Bot1.DrawFirst` | who takes the first turn, asked of the chooser (default: play first); `DrawFirst` hands it to the next seat, which in a duel is the other player |
| `Bot1.Keep` · `Bot1.Mulligan` | keep the hand or take a mulligan (default: keep) |
| `Bot1.Choose(Island)` | after keeping with mulligans taken, the cards put on the bottom, the last named lowest (default: the first in hand) |

`[Handover Bot1.PlayFirst]` and `[Handover Bot1.Mulligan]` stop on those questions for the web client.

## Dice: `[Dice]`

Randomness belongs in fuzzing, so a scenario says what a die shows. A `[Dice]` section lists the faces (1 to 20) the dice show
in the order they are rolled, `6; 1; 3`; once they are used up dice are random again. Comet, Stellar Pup rolls one.
A coin flip is a two-sided die (CR 705.1) and takes its face from the same list: face 1 wins the flip, face 2 loses it. So does a
choice made at random (CR 701.9c): one of N things is a die of N sides (up to 20), face k choosing the k-th (the first card of
a graveyard, the first opponent in turn order). Every roll, flip and random choice goes through the game (`roll_die`, `flip_coin`,
`choose_at_random`) and is recorded as `DieRolled`, `CoinFlipped` or `ChosenAtRandom` in the event history, where triggers
("whenever you flip a coin", "win a flip", "roll a die") see it.

## Starting life: `[Life]`

A scenario that needs a player near death says so instead of playing the life away: a `[Life]` section gives a starting life
total (not the starting life of the format, so "half your starting life total" is unchanged), one player per line, `Bot1: 1`.
A player it does not name starts at 20.

## The event limit: `[Event Limit]`

A game that reaches the most events the engine records (`MAX_GAME_EVENTS`) is a draw (`docs/rules-exceptions.md`, RX-1),
which only a loop played for a very long time reaches. A scenario of such a loop sets a small limit instead, one number,
`5000`: the game is drawn between two frames once its history is that long, with `DrawReason = EventLimit`.

## Starting conditions and the board

A scenario may set its starting conditions - life totals (`[Life]`) and dice (`[Dice]`) - but every counter and every
piece of board state comes from game actions: a player near ten poison counters gets them from infect or toxic damage or
a card that gives them, played in the script, never from a section that writes them.

## How lines are played

There is **one cursor for the whole list**. When the engine asks a player something, the line under the cursor is
tried. If the line belongs to that player and the prompt offers what it describes, it is taken and the cursor moves on.
Otherwise the player gives a default answer: pass, no attackers or blockers, decline to pay, discard the newest cards.

Because of that, a line happens **as soon as it can**. Use a wait (`[Bot1.Main2]`) when that is too early, or
`Bot1.Pass` to give up priority once. For example, to tap lands while Mana Leak resolves rather than in response to it.

A `? Key = Value` check runs at the next prompt after the line before it. Right after a cast, the spell is still on
the stack, so check the battlefield after a later line or a wait. A cast with `X:` or `Kicked:` stops at its pay
prompt first, where the spell is still in hand and the stack empty, so check it after a wait.

A cast an effect makes as it resolves — cascade's, Etali's — has no `Cast` line to say an X or a kicker on, so it is
announced with X of 0 and unkicked, and paid out of the pool; its other questions (a target, teamwork's creatures)
are answered by lines as usual. Where a "you may" makes the cast, `Bot1.Accept(X: 5)` announces that X (a miracle cost
that includes {X}, CR 107.3a).

A **refusal**, `! Bot1.Graveyard.Embereth Shieldbreaker.Cast`, says what must *not* be offered; see [Refusals](#refusals).

`Bot1.Undo` takes back the latest mana ability Bot1 activated, as the table offers it (`cantrip_engine::undo`): the
game goes back to before it and Bot1 is asked again. It is offered while that activation and everything since only
tapped, sacrificed from the battlefield, removed counters, added mana or cost life (a land, Black Lotus, Lotus Petal,
Pentad Prism, a Treasure); never for an ability whose cost discards (Lion's Eye Diamond) or one that triggered
anything (Chromatic Star). Any other answer clears it, a pass or a cast spell included, so tap after the spell you
want to pay with has resolved. `! Bot1.Undo` says nothing may be taken back at Bot1's next priority or payment. `! Bot1.PayCost` says Bot1's next "unless pays" question does not offer paying (the mana they hold cannot pay it); `! Bot1.PayCost(Times: 4)` says the next question to pay a cost a number of times does not allow that many (past "up to three").

The script ends when every line is played and the stack is empty. A line that isn't possible within 4 turns fails.
The failure report shows what each player was last offered, written as script lines, followed by a trace and a full
state dump.

The cursor is matched against whatever question is outstanding, not against a turn, so the **opening-hand window**
(CR 103.6) is scripted with ordinary lines placed before any turn's. A card in an opening hand that offers an action
— Gemstone Caverns' "you may begin the game with this card on the battlefield" — asks its holder before turn one
begins, and `Bot2.Accept` / `Bot2.Decline` answers it; the trace writes those moments as `T0 Untap`. Unanswered, the
default declines, so a scenario that does not care about such a card need say nothing.

## Actions

| Line | Answers |
| --- | --- |
| `Bot1.Hand.Island.Play` | play a land |
| `Bot1.Hand.Lightning Bolt.Cast(Bot2)` | cast; arguments are targets |
| `Bot1.Hand.Mind Twist.Cast(Bot2; X: 3)` | X is raised and pool mana applied automatically |
| `Bot1.Hand.Wheel of Fortune.Cast` with `{R}{R}{R}{B}` floating | when the pool could pay in more than one way, the engine asks which mana; a script takes the first kind offered, which is the engine's own preference (restricted mana, then colourless, then colours the cost does not name) |
| `Bot1.Battlefield.Chromatic Star.Activate(Color: Red; Spend: B)` | `Spend:` (on any cast or activation) names the pool mana to put towards a cost the pool could pay in more than one way, one per question in the order asked, comma-separated — a symbol (`W U B R G C`) for unrestricted mana, or the symbol and what restricts it (`R GainsHaste`, `C ArtifactSpells`, `R InstantOrSorcerySpells`, `B PlaneswalkerSpells`, `U Abilities`, `U LandAbilities`, `C ArtifactAbilities`, and for subtypes `R DragonSpells`, `G DragonOrOmenSpells`, `G ElfCreatureSpells`, `G ElfCreatureSpells, CantBeCountered`, `G NoTypeCreatureSpells` for a chosen type never chosen, `U NoncreatureSpells`, `B ManaValue4OrGreaterSpells`, `G ManaValue5OrGreaterOrXSpells`, `R ElementalSpellsOrAbilities`, `G ElementalCreatureSpellsOrAbilities`, `R CreatureSpells`, `G CreatureAbilities`, `C AbilitiesOfArtifacts`, `G CreatureSpellsOrAbilitiesOfCreatures`, `G CreatureSpellsOrAbilities`, `C ArtifactSpellsOrAbilitiesOfArtifacts`, `R AnyOf(ArtifactSpells+Abilities)`, `R ElementalSpellsOrAbilitiesOfPermanents`, `R SpellsFromExile`, `C NotSpellsFromHand`; the dump writes `Mana` the same way), or for ordinary mana a rule tells apart by the permanent that produced it (CR 106.3) the symbol and `from` that permanent (`W from Barracks of the Thousand`; a bare `W` is then the mana no rule tells apart) — for a spell's cost, a mana ability's own mana cost (CR 602.2b), or an "unless pays" cost on `PayCost(Spend: B, R)`; unsaid takes the first kind offered |
| `Bot1.Battlefield.Island.Activate` | activate an ability, including a mana ability while paying: a mana ability the line under the cursor offers is activated before the pool pays anything, so it can change what the pool's mana may pay for (a source it sacrifices or changes). A cost's other parts are asked before its mana (CR 601.2h lets them be paid in any order): a sacrifice or discard cost's `Choose(..)` comes right after the `Cast`/`Activate` line, and the mana abilities that pay come after it - or tap them before the line, leaving the mana floating |
| `Bot1.Sideboard.Lurrus of the Dream-Den.TakeCompanion` | pay {3} and put the companion Bot1 revealed into its hand (CR 116.2g), then pay with lines as for a cast; also in a refusal |
| `Bot1.Battlefield.Cathar Commando.Activate(Bot2.Battlefield.Sol Ring)` | arguments are targets, as for `Cast` |
| `Bot1.Battlefield.Tundra.Activate(Ability: {U})` | pick one of several abilities by part of its text |
| `Bot1.Battlefield.Basking Rootwalla.Activate(Ability: #2)` | pick the source's Nth ability, mana abilities first, where two read the same — one it prints and an identical one it acquired (CR 602.5c); for a cast, the card's Nth own way (`#1` its mana cost), not one paying an alternative cost another permanent gives; also in a refusal |
| `Bot1.Battlefield.Candelabra of Tawnos.Activate(X: 2)` | X in an ability's cost, raised the way a cast's is |
| `Bot1.Hand.Batroc the Leaper.Cast(Kicked: 2)` | kicker or multikicker paid that many times; unsaid is not kicked |
| `Bot1.Hand.Hogaak, Arisen Necropolis.Cast(Convoke: Llanowar Elves, Ornithopter; Delve: 2)` | what pays a cost instead of mana (CR 601.2h): `Convoke:` taps the named creatures, one per question in the order given (CR 702.51a), and `Delve:` exiles that many graveyard cards, the first offered each time (CR 702.66a) |
| `Bot1.Graveyard.Unholy Annex // Ritual Chamber.Cast(Ability: Unholy Annex: Once during each of your turns)` | a cast a permanent grants (Serra Paragon, Lurrus, Underworld Breach) of a card with more than one face is offered once per face (a Room's two doors, an Adventure) and judged on that face, so its text reads `Face name: the way's text`; name the face first |
| `Bot1.Hand.Daze.Cast(Duress; Ability: Return)` | cast for an alternative cost by part of its text, as its Adventure with `Ability: Adventure`, or as one half of a split card with `Ability: Death`; without `Ability:`, the mana cost — which a split card, never cast as itself, does not offer |
| `Bot1.Battlefield.Black Lotus.Activate(Color: Blue)` | mana of any one color |
| `Bot1.Battlefield.Urza, Lord High Artificer.Activate(Ability: Add; Tap: Sol Ring)` | choose the untapped permanent paying a mana ability's tap cost |
| `Bot1.Channel.Activate` | a mana ability the player has of their own (CR 611.2a), named by the card that gave it and no zone: it has no source, so it is offered wherever that card has gone |
| `Bot1.Battlefield.Squandered Resources.Activate(Sacrifice: Strip Mine; Color: C)` | a cost that sacrifices one of several permanents names which; `Color: C` (or `Colorless`) answers colorless where the ability's type can be it (CR 106.7) |
| `Bot3.Concede` | the player concedes the game (CR 104.3a: at any time) the moment the cursor reaches the line, whoever is asked; the question outstanding is asked again of the players left. Fails if the player has already left. Place it after the passes that bring the game to the question it is to interrupt |
| `Bot1.Pass` · `Bot1.PayCost` · `Bot1.Decline` · `Bot1.Undo` · `Bot1.Cancel` | priority, unless-costs, mana undo, cancel a spell or ability at any question before it is on the stack |
| `Bot1.PayCost(X: 3)` | "you may pay {X}" with X chosen as it is paid (CR 107.3f): X is raised (or lowered) to the number one step at a time, then paid from the pool like any `PayCost` |
| `Bot1.PayCost(Times: 2)` | "you may pay {G} up to three times" / "any number of times" (CR 603.12a): the number of times is raised (or lowered) from 1 to the number one step at a time, then the cost that many times over is paid like any `PayCost`; `Times:` on an X question, or `X:` on a number of times, fails |
| `Bot1.Accept` · `Bot1.Decline` | a "you may" choice, such as Path to Exile's search (default: decline); `Accept` also pays a payment of life alone (a shockland's 2 life, ward's), which has no mana to make first |
| `Bot1.Choose(Lightning Bolt; Island)` | Duress, Brainstorm (last chosen ends on top) |
| `Bot1.Choose(Burst Lightning; Kicked: 1)` | the card chosen to be cast as an effect resolves ("you may cast a spell from your hand without paying its mana cost", Buster Sword) announces that X or those kicks, paid from the pool like any cast's additional cost (CR 118.9d); unsaid, X is 0 and it is unkicked |
| `Bot1.Choose(Graveyard.Black Lotus)` | craft's materials (CR 702.167a), permanents and graveyard cards in one question: qualify the zone where a name is in both (default: the fewest it takes, the first offered); a refusal `! Bot1.Choose(Ornithopter)` holds when no such material may be chosen, and one naming only choosable materials holds when that is no answer the question accepts (one of "two creatures") |
| `Bot1.Discard(Island)` · `Bot1.Search(Black Lotus)` | hand size, library search (`Search()` finds nothing) |
| `Bot1.Arrange(Top1; Top2 \| Bottom1)` · `Bot1.Arrange(Shuffle)` | cards looked at and placed: one `\|`-separated group per destination, in the order the prompt lists them — scry (top \| bottom), surveil (top \| graveyard), Ponder (top), Stock Up (hand \| bottom); and the order of cards put into a graveyard together, top first, which their owner is asked while a card in the game reads a graveyard's order (CR 404.3: `Bot1.Arrange(Llanowar Elves; Ornithopter)`) |
| `Bot1.Attack(Grizzly Bears; Hill Giant > Bot2)` | attackers |
| `Bot2.Block(Serra Angel > Grizzly Bears)` | blockers |
| `Bot1.AssignDamage(Ox > Lurker: 1; Ox > Raider: 3)` · `Bot1.AssignDamage(Ox > Bot2: 3)` | how a gang-blocked attacker, a trampler, or a blocker on two attackers divides its combat damage (CR 510.1); a creature the line leaves out keeps the engine's own division (default: the engine's division, lethal to each in turn) |
| `Bot1.Divide(Grizzly Bears: 2; Bot2.Battlefield.Hill Giant: 1)` · `Bot1.Divide()` | a divided slot's targets and each one's share, chosen together as the spell or ability is announced (CR 601.2d): Fire Covenant's damage, Court of Garenbrig's counters; the shares add up to the total, and an empty line chooses no target where the slot says "up to" (default: the whole total on the first candidate) |
| `Bot1.RemoveCounters(Sylvan Caryatid: 2; Walking Ballista: 1)` · `Bot1.RemoveCounters(Sylvan Caryatid: Flying 1; Dawnhand Dissident: 1)` | counters removed from among permanents as a cost (CR 118.3: the casts Dawnhand Dissident permits, "removing three counters from among creatures you control"): each permanent with how many come off it, and the kind as the dump spells it (`PlusOnePlusOne`, `MinusOneMinusOne`, `Flying`) where it has more than one kind; the amounts add up to the total (default: as many as there are from each kind on each permanent in turn, the first offered first). For an effect's "remove up to N counters" (CR 608.2d: Heartless Act), the amounts add up to N or fewer, and `Bot1.RemoveCounters()` removes none |
| `Bot1.Target(Sol Ring)` · `Bot1.Target()` | a triggered ability's target as it goes on the stack, or a chosen mode's as a modal spell is cast; `Target()` chooses none for "up to one" (default: none if optional, else the first candidate) |
| `Bot1.Target(Island; Mountain)` | a slot that takes several — "Untap X target lands", asked once the mana is paid — takes exactly that many, and they must be different permanents; "up to two target creatures" takes as many as the line gives, at most two |
| `Bot1.Modes(1; 4)` · `Bot1.Modes(Destroy target artifact)` | a modal spell's modes as it is cast, or a modal triggered ability's as it goes on the stack, by number from 1 or part of their text (default: the first eligible); also the next room of a dungeon when there are two, `Bot1.Modes(Lost Well)` |
| `Bot1.Name(Lightning Bolt)` | a card name, chosen as a permanent enters (Anointed Peacekeeper) or as a spell or ability resolves (Petrified Hamlet's land card name); any card the game knows of the kind asked for (default: the name the card suggests, such as the costliest nonland card in the hand looked at) |
| `Bot1.Colors(Red; Green)` | colors chosen as a spell or ability resolves (CR 202.2b), by name and in the order asked for; a color repeats where the ability allows it, as Coalition Relic's one mana per charge counter does (default: the first on offer) |
| `Bot1.Replace(Archon of Emeria)` | which of two applicable replacement effects applies first (CR 616.1), by its source card's name or part of its hint (default: the first offered) |
| `Bot1.ChoosePlayer(Bot2)` | a player chosen without targeting (CR 608.2d, 614.12a), as True-Name Nemesis asks as it enters (default: the first in turn order) |

A preparation card's spell (CR 722.3c) is named in a line by its own name, `Bot1.Exile.All Aboard.Cast(...)`, though no deck can hold it; it exists only as the copy in exile.

Arguments are separated by `;`, because card names contain commas. Names can be qualified when they are ambiguous:
`Stack.Duress`, `Bot2.Battlefield.Grizzly Bears`, `Island#2`. Copies are numbered in battlefield order, and top first
for libraries, graveyards and the stack (`Stack.Duress#1` is the topmost Duress; unnumbered, a name takes the first
candidate the question offers, which for two spells of that name on the stack is the bottom-most). An activated or
triggered ability on the stack is named for its source, `Stack.Locus of Enlightenment` the topmost of that source's and
`Stack.Locus of Enlightenment#2` the one under it. A target shared by two players' permanents must be qualified. Same-named cards
in one place are interchangeable, so the first one offered is used.

A permanent is named by the name it shows, so a transformed one is written by its **back face's** name:
`Bot1.Battlefield.Concealing Curtains.Activate` turns it over and `Bot1.Battlefield.Revealing Eye.Attack` plays it
afterwards. Either face's name reaches the card in a decklist too.

A token is named by its rules name (CR 111.4), in lines and dump keys alike: the name the text that made it gives it
(`Boo`, `Marit Lage`, `Moloid`, the Roles' `Royal`), and otherwise its subtypes followed by `Token` -
`Bot1.Battlefield.Spirit Token#2`, `Bot1.Attack(Phyrexian Hydra Token > Bot2)`, `Bot1.Battlefield = Treasure Token;
Island`. There is no short alias: a token named for its subtypes never shares a name with a card (Phyrexian Hydra the
card, Phyrexian Hydra Token the token), and the scenario says which. A text change of the text that made it renames it
with its subtypes (CR 612.2a): Artificial Evolution changing Hydra to Elf makes `Phyrexian Elf Token`s. A trigger or an
ability picked by its words (`Triggers(Soldier; battle cry)`, `Activate(Ability: Spirit)`) is still picked by its text.

A face-down permanent (CR 708.2a) has no name: `Bot1.Battlefield.#1.Activate` turns the first one face up for its card's mana cost (a
special action, paid like a spell's cost with the mana abilities that follow), and `#2` the second; its dump keys are
`Bot1.Battlefield.#1.FaceDown` and `.FaceDownName` (what it really is), or with a lone one `Bot1.Battlefield..FaceDown`.

Morph, megamorph and disguise (CR 702.37, 702.168): a card is cast face down for {3} by naming its way,
`Bot1.Hand.Hidden Dragonslayer.Cast(Ability: Megamorph)` (`Ability: Morph`, `Ability: Disguise`: the way's keyword line,
"Megamorph {2}{W}", contains it), paid like any cast. A face-down permanent is turned face up for its morph (megamorph's
included) or disguise cost with `Bot1.Battlefield.#1.Activate(Ability: morph cost)` or `Activate(Ability: disguise cost)` - the
special actions read "Turn this permanent face up by paying its morph cost." and "... its disguise cost."; a manifested or
cloaked one with morph has both that and `Activate(Ability: mana cost)` (CR 701.40c, 701.58c), so name which. A cost that is
not mana ("Morph—Discard a Zombie card", "Return a Bird you control to its owner's hand") is answered by the lines that
follow, as an activated ability's cost is. Refused as any action is: `! Bot1.Battlefield.#1.Activate(Ability: morph cost)`.

Surge and freerunning (CR 702.117a, 702.173a) are alternative costs of the card's own, cast by their keyword line:
`Bot1.Hand.Goblin Freerunner.Cast(Ability: Surge)`, `Bot1.Hand.Brotherhood Ambushers.Cast(Ability: Freerunning)`, offered
only while the condition holds (another spell Bot1 cast this turn; combat damage an Assassin creature Bot1 controlled dealt a
player this turn, as it was then), so `! Bot1.Hand.Goblin Freerunner.Cast(Ability: Surge)` shows it refused before.

Assist (CR 702.132a): after the `Cast` line (and its cost's choices), `Bot1.Accept` chooses another player to help pay
(unsaid, Bot1 chooses no one), and with more than two players `Bot1.ChoosePlayer(Bot3)` says whom. That player's mana
abilities come first: `Bot2.Battlefield.Forest.Activate` lines; then the caster's, `Bot1.Battlefield.Forest.Activate` -
the last chance the caster has (CR 601.2g: once paying begins no mana ability is offered, so tap everything the rest needs
here, or float it before casting); then `Bot2.PayCost(X: 2)` is how much of the generic mana Bot2 pays (at most the generic
mana and what their pool holds; `Spend:` as on any cast), unsaid 0; then Bot1 pays the rest from the pool. A window with no
line for it is closed.

Prototype (CR 702.160, 718): `Bot1.Hand.Blitz Automaton.Cast(Ability: Prototype)` casts it prototyped (the option reads its
keyword line, "Prototype {2}{R} — 3/2"), paid like any cast; with an alternative cost another permanent gives, the option
reads "Prototype {2}{R} — 3/2: Without paying its mana cost". `Stack.Blitz Automaton.Prototyped` and
`Bot1.Battlefield.Blitz Automaton.Prototyped` are `true` for a prototyped spell or permanent, or a copy of one (absent
otherwise); its `.Power`, `.Toughness`, `.Colors` and `.ManaValue` are the prototype's there, the card's anywhere else.

## Refusals

A line starting with `!` says what the game must **not** offer, which no ordinary line can prove — a line never offered
only fails after four turns, without saying why:

```
[T5 Bot1.Main1]
! Bot1.Exile.Lightning Bolt.Cast                      Thor's permission has run out
! Bot1.Battlefield.The Wandering Emperor.Activate     no second loyalty ability this turn
! Bot2.Block(Grizzly Bears > True-Name Nemesis)        protection from Bot2
```

A play, cast or activation is checked the next time that player has priority (a refused mana ability sooner, at a
payment that asks for mana, below), without answering it, and fails
listing what was offered, written as script lines. It is matched as the line would be: the card in that zone, and
with `Ability:` the option whose text contains it. A refused `Cast` without `Ability:` refuses every way of casting
the card, not only for its mana cost. A cast or activation the player cannot pay for is not offered at all (CR 601.2g,
602.2b), so a refusal that holds only for want of mana proves nothing of any other rule: each refused cast or activation
(special actions too) is judged again with a thousand mana of every type in the player's pool, and one that would then be
offered fails as proving nothing - give the player the mana, and refuse for the rule in question. Where the cost is what
the refusal is about (a cost increase, a reduction that does not apply, a color no source can make, mana restricted to
other things, a filter land with nothing to pay its hybrid), say so with `Mana: short`:
`! Bot1.Hand.Lightning Bolt.Cast(Bot2; Mana: short)` holds only if the action is not offered now and would be with mana to
spare, and fails if it is refused even then. A refused activation
with `Color:` (`! Bot1.Battlefield.Chrome Mox.Activate(Color: Green)`) is offered only where that color is among the colors the
ability may add: the colors it lets the player choose from, or the fixed mana it adds (`! Bot1.Battlefield.Llanowar Elves.Activate(Color: Blue)`);
`Color: Colorless` (or `C`) likewise asks whether it may add colorless mana. One with `Sacrifice:`
(`! Bot1.Battlefield.Krark-Clan Ironworks.Activate(Sacrifice: Hithlain Rope)`) refuses only a mana ability whose sacrifice
that permanent could pay.

A refused question to a player who has lost the game (`! Bot2.Accept`) holds once the stack is empty and they were asked
nothing: a player who has left the game is given no priority and no choice (CR 800.4a).

A refusal with `X:`, `! Bot1.Hand.Fire Covenant.Cast(X: 21)`, written right after the cast or activation that announces X,
says X cannot be announced that high (life to pay, creatures to sacrifice); with `Kicked:`, `! Bot1.Hand.Ruthless Radrat.Cast(Kicked: 3)`,
that the spell cannot be kicked that many times (cards to exile for a squad cost): it is judged there, by the most the announcement
allows, and holds if that is short of the number and fails if it reaches it. The card announcing must be the one named. Written where
nothing is announcing, it is an ordinary refusal of the action (the `X:` is only part of the line).

A block, `! Bot2.Block(Blocker > Attacker)`, is checked the next time that player declares blockers, and fails if
that pair is allowed. Both creatures must be in the question (an untapped blocker, an attacker attacking that
player), or the refusal fails as proving nothing. The engine does not ask for blockers when no block is possible at
all, so a refusal of the only possible block needs some other attacker the defender could block. The one exception is a
block an effect forbids outright (Headliner Scarlett's "creatures target player controls can't block"): when the question
never comes, the refusal holds if, with priority in the declare-blockers step, the blocker is an untapped creature of that
player, the attacker is attacking that player, and no such block was made; otherwise it fails as proving nothing.
The pairs of one refusal are one declaration: `! Bot2.Block(Bear > Moonshadow)` holds for an attacker with menace (a lone blocker is not
legal, CR 702.110b), while naming two blockers for it, `! Bot2.Block(Bear#1 > Moonshadow; Bear#2 > Moonshadow)`, would fail.
Pairs each allowed alone are judged together by the engine's own check of the declaration: `! Bot2.Block(Bear#1 > Rover; Bear#2 >
Rover)` holds for an attacker that "can't be blocked by more than one creature" (CR 509.1b). Where an effect requires blocks
(CR 509.1c: Lure's "all creatures able to block enchanted creature do so"), a line of blocks each allowed alone holds when the
engine's own check refuses the declaration as a whole - it obeys fewer of the requirements than a declaration could - and
`! Bot2.Block(Count: 0)` says declaring no blockers at all is refused at the next declaration. The default answer to a
declaration with requirements is the engine's own, which obeys them.

`! Bot1.Attack()` says declaring no attackers at all is refused at the next declaration (a creature must attack, CR 508.1d).
A refused activation (`! Bot1.Battlefield.Llanowar Elves.Activate`) is checked at the player's next priority, and fails if
that ability is offered then. One that names only mana abilities is checked sooner at a payment that asks for mana, where
mana abilities may be activated (CR 605.3a): a spell's or ability's cost, a cost to attack after an `Attack` line, or an
"unless pays" cost with mana in it. A payment of life alone (a shock land's 2 life) does not count, and a refusal of a
non-mana ability is always checked at priority. Wherever it is checked, it is judged with mana to spare as above, and
needs `Mana: short` where it holds only for want of mana.
An attack, `! Bot1.Attack(Sylvan Caryatid > Bot2)`, is checked at the next declaration of attackers and fails if that attacker may attack the
target. The attacker must be one the question offers, or else one that only a restriction (Defender, "can't attack", summoning sickness) can have left
out: the asked player's own, untapped, with another creature of theirs offered in the same question. A pair the question
offers still holds as a refusal when no declaration can make it: the rules refuse that attack together with every other
creature that must attack sent where attacking is free (a creature that must attack a certain player sent at their
planeswalker while attacking the player costs nothing, CR 508.1d).
The pairs of one refusal are one declaration, judged by the engine's own check: a cap each attack alone obeys
holds as a refusal once they are named together - `! Bot1.Attack(Bear > Wrenn; Giant > Wrenn)` for Tomik's "no more
than one creature can attack this planeswalker" (CR 508.1c). Written as separate refusal lines each would fail, since each attack alone is allowed. The refusal also holds when
the pairs named fall short of a requirement (a creature that must attack, CR 508.1d), as a refusal of blocks does.

A search, `! Bot1.Search(Plains)`, is checked at that player's next library search, and fails if a Plains is among
the cards it may find. The card must be in the library searched, or the refusal fails as proving nothing. A search for
a quality may always find nothing (CR 701.23b), so this is the only way to show a card is not one; the search itself
is still answered by the next line, `Bot1.Search()` or another card. Several names that are all among the cards it may
find are instead one answer, which the search's rule must refuse: `! Bot1.Search(A; B; C)` for "up to two", or two cards
with a name in common for "with different names".

`Choose(..)` and `Search(..)` lines are answered through the question's own selection rule (docs/engine-design.md,
"Selection rules"), as a player picks: each name is the first card of that name that may be added to the ones named before
it, and the cards named together must be an answer; a refusal of one name holds when no card of that name may be picked
first.

A target, `! Bot1.Target(Lightning Bolt)`, is checked at that player's next target question and fails if the named object is among its
candidates; it fails as proving nothing when no such card exists anywhere in the game. It shows a spell or ability cannot target something
(a creature card, not an instant, in "target creature card in your graveyard", or one worth more than a slot's "total mana value X or less" by
itself); pair it with the legal target named on the next line.
Several names that are all among the candidates are instead one answer, which the question must refuse:
`! Bot1.Target(Ornithopter#1; Ornithopter#2; Ornithopter#3)` for "up to two target creatures" (more than the slot takes), or one
object named twice, `! Bot1.Target(Ornithopter#1; Ornithopter#1)`, as many as the slot takes, so that the repetition is what is
refused (CR 115.3), or targets whose mana values add up to more than the slot's total allows (CR 601.2c). The engine's own check of the answer decides (`check_answer`).
A lone name that is a legal target is one answer too when the slot takes at least two, and is refused as too few:
`! Bot1.Target(Bot2.Battlefield.Memnite)` for Kitsune's "two other target creatures" (CR 601.2c, 115.1).
`! Bot1.Target(Count: N)` fails instead if the question takes N targets, and proves nothing when it has fewer candidates than
that; `Count: 0` is choosing none, which no answer naming targets can say. It shows a slot takes at least one target (The Eagles Are Coming! unkicked, where kicked it takes any number).

A mode choice, `! Bot1.Modes(1; 2)`, is checked at that player's next modes question and fails if the question would accept exactly
those modes (too many for "choose one", a mode that is not eligible): the way to show a spell offers fewer modes than another case
does ("choose one, or both if ..."). A mode named by text that the question does not have at all holds too: it is not offered
(`! Bot1.Modes(Fuse)`, a fused cast over a mana-value limit) - but only when it is a mode the question's source could offer
somewhere (its faces and halves, a fuse card's fused cast, its modes, ways, alternative costs and labels); any other text
is a typo, and fails. `! Bot1.Modes(Count: 0)` refuses choosing no mode at all ("choose one or both" takes at least one). Pair it with the same line where the condition holds.

A card choice, `! Bot1.Choose(Thopter)`, is checked at that player's next choice of cards (a sacrifice or discard cost, a card
chosen from a revealed hand) and fails if the named card is among those it may take; it fails as proving nothing when no such
card exists in the game. It shows a cost or choice leaves something out (a token as "a nontoken artifact"); pair it with the
`Bot1.Choose(Thopter Foundry)` that takes the legal card on the next line. `! Bot1.Choose()` says choosing no card at all is an answer the question must refuse (a mandatory choice). Without parentheses it is the bare refusal below.
Several names that are all among the cards it may take are instead one answer, which the question must refuse: `! Bot1.Choose(A; B; C)` for "up to two", or `! Bot1.Choose(A; B)` for creatures a crew, saddle or teamwork cost may tap whose total power falls short (`! Bot1.Choose(A)` for a single one) (a negative power takes from it, CR 107.1b).
A list of names that are not all among them (for `Choose(..)` and `Search(..)` alike) is read name by name instead: it fails if any card it names may be taken or found, so a refusal of one card that may not be is written with that name alone.

A removal of counters, `! Bot1.RemoveCounters(Dyadrine, Synthesis Amalgam: 2)`, is checked at that player's next question of counters to remove
(CR 118.3, 122.1) and fails if the rules' own check accepts it as an answer (more counters from one permanent than the question lets come
off it: Dyadrine's one counter from each of two creatures); written like the `RemoveCounters` that answers it, whose next line is the
positive control.

A name, `! Bot1.Name(Lightning Bolt)`, is checked at that player's next card name question and fails if the question would accept
that name (CR 201.4a: a nonland name where a land card name is asked for passes the refusal); pair it with the `Bot1.Name(...)` that
answers it on the next line.

A color choice, `! Bot1.Colors(Blue)`, is checked at that player's next color question and fails if the named color is on offer
(a replacement color that may not be the one being replaced); pair it with the `Bot1.Colors(...)` that answers it on the next line.

A "you may" choice, `! Bot1.Accept`, fails if that player is asked one before their next priority (or before the question
the line after it answers, if that comes first: `! Bot1.Accept` then `Bot1.Choose(Mox Pearl)` for the sacrifice the same
resolution asks): the way to show a choice is *not* offered (a choice made impossible, CR 608.2d), which a default Decline
would otherwise hide. Put `Bot1.Pass` lines before it for the priorities that come first, so it lands on the resolution
that would ask.

The same holds for the other questions, written bare: `! Bot1.Choose`, `! Bot1.Decline`, `! Bot1.Discard`, `! Bot1.Colors`, `! Bot1.Name`,
`! Bot1.Arrange`, `! Bot1.ChoosePlayer`, `! Bot1.Modes` and `! Bot1.Target` (without arguments) fail if that player is asked that kind
of question before their next priority: a discard, a search result, a placement or a target that should not be asked for (an
empty hand, a lone candidate, nothing to reveal). Pair each with a scenario that does ask it.

Such a refusal must cover a point where its question could have been asked, or it fails as proving nothing. Its window
opens where the game stood before the answer to the line above it (or, under a step header, as the step began; a refusal
straight after another, checks between, shares that one's window) and closes where it is judged. For `Modes` and `Target`,
asked as a triggered ability is put on the stack, anything at all happening in it is such a point, the beginning of the
step under the header too ("at the beginning of"). For every other kind it is one of: a spell or ability leaving the stack
(resolved, countered, or with no legal target) or resolving; a step ending, every player having passed with the stack
empty (a trigger that should not have been there would have resolved first); a spell cast, a non-mana ability activated (a
mana ability only for `Colors`), a land played, a spell copied, a permanent turned face up, a door unlocked; a declaration of
attackers with banding among them, for the attacking player's `Choose` (bands); and the beginning of the step under the
header where its turn-based action asks that question (`Block` in the declare blockers step, `Discard` at cleanup, a draw replacement's `Accept`/`Decline` in the draw step). So
`! Bot1.Accept` written right after a step header, with the trigger that would ask still on the stack, fails: pass priority
first (`Bot1.Pass`, and the other player's `Pass` when the refusal is theirs, since their own priority would judge it). So
does a refusal of a combat damage trigger's question written after the `Attack` or `Block` line: it is judged in the
declare attackers or blockers step; put it under `[T3 Bot1.CombatDamage]` after a `Bot1.Pass`. The rule sees that
something happened, not that it was the thing meant: a refusal of a death trigger's question after the pass that resolves
the removal spell still has the trigger above it, so pass once more.

`! Bot1.Arrange(A; B | )` with arguments is a distribution the question must refuse (two cards to a hand that takes one, a
card left unplaced), judged by the rules' own check of the answer at the next placement question; place it before the
`Arrange` that answers, beside which it is the positive control. `! Bot1.Arrange(Shuffle)` says the next placement question
does not let the cards be shuffled instead (Ponder's does, Thassa's Oracle's does not).
`! Bot1.Discard` is the discard to hand size at cleanup (no maximum hand size): a refusal is checked until the player's next priority, so
place it after the `Bot1.Pass` of the end step (`[T5 Bot1.End]`, `Bot1.Pass`) or the end step's own priority ends it first.

A bare `! Bot2.Block` is the same for the declaration of blockers: it fails if Bot2 is asked to declare blockers before their next
priority, which shows that no block is possible at all ("creatures can't block this turn" leaves nothing to refuse with
`! Bot2.Block(Blocker > Attacker)`, since the engine then asks nothing). Put `[T3 Bot1.DeclareBlockers]` before it, because Bot2
gets priority earlier in the turn, in the declare attackers step; pair it with a scenario in which a block is possible.

A refusal answers nothing, and the player goes on to the next line. Because it lands on the *next* question of its
kind, place it where the action could otherwise be taken, with a wait or `BotN.Pass` before it:

- Any priority counts, upkeep included, and a sorcery-speed action is never offered in an upkeep or with a spell on
  the stack, so a refusal placed there holds for that reason alone.
- Targets are not considered unless the line names them, but a spell with no legal target is not offered at all. Give a
  targeted spell something to target, or it goes unoffered for the wrong reason. A refusal that names its targets,
  `! Bot2.Hand.Lightning Bolt.Cast(Bot1.Battlefield.Figure of Fable)`, refuses only the option that can be announced with
  them (here: protection or hexproof keeps Figure of Fable out of the Bolt's candidates) and fails as proving nothing if no
  object has that name; the Bolt cast at something else is the positive control.
- **Pair every refusal with a positive control**: the same action, or the same pair, taken or offered where the rule
  under test does not apply (Lightning Bolt cast before Teferi arrives; the blocker blocking another attacker). Without
  one, a refusal also passes when the card is simply not where the line says, or never could have done it.

## Expectations

`Key = Value` against the dump. A failing script prints every key. The keys are:

- `Turn`, `Step`, `ActivePlayer`, `PriorityPlayer`, `Result` (`InProgress`, `Bot1 won`, `Draw`)
- `DrawReason` (`MandatoryLoop`, `AllLost`, `EventLimit`), why a drawn game is drawn (a loop every player only passed in,
  come round three times, `docs/rules-exceptions.md` RX-4; CR 104.4a: every player left lost at once; the event limit,
  RX-1), from the history; absent for a game not drawn
- `Monarch` (`Bot1`), once a player has become the monarch (CR 725) and absent until then; `Initiative` likewise (CR 726)
- `DayNight` (`Day`, `Night`), once the game has a designation (CR 731.1) and absent while it is neither. The game's
  own, so it names no player.
- `Stack`, `Exile`, `Combat.Attackers`, `Combat.Blocks`, each with `.Count`: an attacker as `Grizzly Bears > Bot2`, one in an
  attacking band (CR 702.22c) as `Benalish Hero > Bot2 (band 1)`, bands numbered in the order announced; a block as
  `Blocker > Attacker`, one for each member of a band a creature blocks (CR 702.22h)
- `Stack.<Name>.ManaValue` for each spell on the stack (CR 202.3e: its X counted as announced) and `Stack.<Name>.X`, the X it
  was cast with, absent where there was none; named and numbered as `Stack` lists them, top first. A spell cast face down
  (morph, disguise: CR 708.4) has no name, so it is listed as an empty name and keyed `Stack..ManaValue` (`Stack.#2.ManaValue`
  with two nameless spells, top first), with `Stack..FaceDown` (`true`) and `Stack..FaceDownName` (the card it is)
- `Delayed` (with `.Count`): the delayed triggered abilities still waiting (CR 603.7), each named for the object it came from:
  one used up or ended with its turn is gone from it
- `Combat.AttackCost` (`{2}`), `.Remaining` and `.Attackers` (with `.Count`), while a declaration's cost to attack is being
  paid (CR 508.1h-j: "creatures can't attack you unless their controller pays ..."): the total, what the pool has not yet
  covered, and the attackers it is for; absent at any other time. A `? Combat.AttackCost = {2}` right after an `Attack`
  line proves the payment was asked for, and `Bot1.Cancel` then takes the declaration back, to be declared again
- `LeftBattlefield` (with `.Count`): every permanent that left the battlefield, in the order the history recorded it, as it
  last was there (CR 608.2h): `Expedition Map (tapped)` when it was tapped as it left. A tap the history records only after
  the permanent had left does not count (CR 701.26a), which is how a cost that taps and moves its source shows it paid the tap
  first (CR 602.2b, 601.2h)
- `Prevented` (with `.Count`): every prevention of damage (CR 615.6), in the order the history recorded it: how much, its source,
  whom it would have been dealt to and whose prevention effect it was, `3 Lightning Bolt > Bot1 by Healing Salve` (protection's is
  the protected permanent's, or the source of a player's protection). Absent until damage has been prevented
- `Attached` (with `.Count`): every time an Aura, Equipment or Fortification became attached to something (CR 701.3a), in the
  order the history recorded it: `Bonesplitter > Grizzly Bears`, a player an Aura enchants as `Bot2`. An attach to what it is
  already attached to does nothing and is not one (CR 701.3b). Absent until something has become attached
- `Exile` lists a card exiled face down as `(face down)` (CR 406.3), and `Exile.FaceDown` says what those cards are (the dump sees
  everything; a player's view does not)
- `BotN.Life`, `.Poison`, `.Energy`, `.Status` (`Lost`, once the player has lost the game and absent until then), `.CantWin` (`true` while an effect keeps the player from winning the game, CR 104.2b - Platinum Angel's "your opponents can't win" - absent otherwise), `.Mana` (`{U}{U}`), `.LandsPlayed`
- `BotN.LostTo` (`Life`, `EmptyLibrary`, `Poison`, `Effect`), how the player lost the game (CR 104.3), from the history; absent
  until they have
- `BotN.ManaFrom` (with `.Count`), the pool's mana by the source that produced it (CR 106.3), restricted mana too and without
  its restriction: `{W} Barracks of the Thousand; {W}{W} Plains` (`none` for a player's own mana ability); absent while the pool is empty
- `BotN.Experience` — counters on the player by name (CR 122.1), one key per kind and only for a kind they have any
  of, spelled as `CounterKind.Token` spells it (`.Experience`, `.Rad`). Poison and energy keep their own keys above.
- `BotN.CitysBlessing` and `BotN.EnduringStory` (`true`), once the player has the designation (CR 702.131c, 702.195b) and
  absent until then
- `BotN.TurnsToSkip` (`3`), the turns an effect still makes the player skip (CR 500.11, 614.10), absent when none
- `BotN.ControlledBy` (`Bot1`), the player controlling them now (CR 723), absent while they make their own decisions; and
  `BotN.NextTurnControlledBy` (`Bot1, Bot3+extra`), the effects waiting to control them during their next turn, oldest
  first, the last the one that works (CR 723.1a), `+extra` on one that gives an extra turn after it; absent when none
- `BotN.Speed` (`1` to `4`), once the player has a speed (CR 702.179) and absent while they have none (CR 702.179b)
- `BotN.Dungeon` (`Undercity: Forge`) while in a dungeon, and `BotN.DungeonsCompleted` once one is completed (CR 309.7)
- `BotN.Looked`, the cards the player has looked at, in the order they looked (a look at one card of a hand, Urza's Bauble),
  once they have looked at any and absent until then
- `BotN.Hand`, `.Library`, `.Graveyard`, `.Battlefield`, `.Emblems` (CR 114, from the command zone), each with `.Count`
- `BotN.Companion`: the card the player revealed as their companion before the game (CR 103.2b), wherever it has gone since;
  empty for a player who revealed none.
- `BotN.Sideboard` (with `.Count`): the cards the player owns outside the game (CR 400.11a), in the order the sideboard lists
  them - what `[BotN Sideboard]` and `[Basic Lands Outside]` gave them, less what has been brought into the game - and
  `BotN.Sideboard.<Name>.Colors` for each (CR 400.11c: nothing but its own characteristic-defining abilities changes them)
- `BotN.Hand.<Name>.Power`, `.Toughness` and `.Subtypes` (absent with none), and the same for `BotN.Graveyard.<Name>` and `BotN.Library.<Name>`, for a creature card there (CR 604.3: a characteristic-defining ability such as Sandman's works in every zone; a static ability that works in every zone, Arcane Adaptation's chosen type, too); a name held twice there is `Name#1`, `Name#2` in zone order: a library's top first, a hand's and a graveyard's first arrived first (so a graveyard's bottom first, though the `Graveyard` list is top first)
- `BotN.Hand.<Name>.Colors`, `BotN.Graveyard.<Name>.Colors` and `BotN.Library.<Name>.Colors` (computed, sorted, empty when colorless), for every card there: an effect that works in every zone may change them (Mycosynth Lattice)
- `BotN.Exile.<Name>`, the cards BotN owns in exile, by what each is (face down too), with the same keys as a card in a hand:
  `.Colors`, and `.Power`, `.Toughness`, `.Subtypes` for a creature card
- `BotN.<Zone>.<Name>.Counters.PlusOnePlusOne` (or any `CounterKind`) for a card in a hand, a graveyard, a library, a sideboard
  or exile that has counters of that kind (suspend's time counters in exile); `(absent)` shows a card has none, such as one a
  replacement exiled instead of letting it enter, which "put a counter on each of them" must not reach
- `BotN.Battlefield.<Name>.ManaValue` for every permanent (CR 202.3): a transformed permanent's is its front face's
  (CR 202.3b), a copy of a back face's or a melded permanent's is 0 (CR 202.3b-c), a face-down one's 0 (CR 708.2a)
- `BotN.Battlefield.<Name>.X` for a permanent whose spell was cast with an X above 0 (CR 107.3): absent otherwise
- `BotN.Battlefield.<Name>.Doors` and `.ManaValue` for a Room (CR 709.5): the names of its unlocked doors, left first
  (empty when both are locked), and the mana value they leave it. The Room is named by its unlocked doors (`Unholy Annex`,
  `Unholy Annex // Ritual Chamber`), and by nothing at all when neither is (`Bot1.Battlefield.#1`). A locked door is
  unlocked by `Bot1.Battlefield.Unholy Annex.Activate(Ability: right door)`, a special action paid with the mana abilities that follow.
- `BotN.Revealed` (present once the player has revealed a card): the cards they revealed, in order, from the history; a beheld
  card (CR 701.4a) is revealed from a hand, a beheld permanent is not
- `BotN.Crimes` (present once the player has committed a crime, CR 700.13): the spells, and the sources of the abilities,
  by which they committed one, in order, from the history - one per spell or ability however many of its targets were crimes
- `BotN.PhasedOut` (with `.Count`): the phased-out permanents the player controlled as they phased out (CR 702.26b), apart
  from `BotN.Battlefield`, each with the same keys as a permanent there (`BotN.PhasedOut.<Name>.Tapped`, `.Counters...`,
  `.AttachedTo`)
- `BotN.Battlefield.<Name>.Tapped` and `.Types` (computed card types, `Land; Creature` for an animated land); for
  creatures also `.Power`, `.Toughness`, `.Damage` and `.SummoningSick`
- `BotN.Battlefield.<Name>.Token` (`true`) for a token, absent for a card
- `BotN.Battlefield.<Name>.Transformed` (`true`) for a double-faced permanent with its back face up (CR 701.27g), absent
  front face up - shown even where a copy effect makes it look like something else (CR 707.2)
- `BotN.Battlefield.<Name>.Supertypes` (computed: `Legendary`, `Basic; Snow`), absent when it has none
- `BotN.Battlefield.<Name>.Subtypes` (computed, sorted: `Hero; Wizard`), absent when it has none; every creature type (Changeling) is the one entry `(every creature type)`
- `BotN.Battlefield.<Name>.Colors` (computed, sorted: `Green; White`), absent when it is colorless
- `BotN.Battlefield.<Name>.Counters.PlusOnePlusOne` (or `.Lifelink`, any `CounterKind`), for a permanent that has
  counters of that kind
- `BotN.Battlefield.<Name>.Keywords` (`Indestructible; Lifelink`, sorted), computed keywords, absent when it has none
- `BotN.Battlefield.<Name>.Toxic` (`3`), its total toxic value (CR 702.164b), printed and granted, absent when it has none
- `BotN.Battlefield.<Name>.HexproofFrom` (`Black; White`, `Monocolored`, sorted), the qualities of its "hexproof from"
  abilities (CR 702.11d), printed or granted, absent when it has none
- `BotN.Battlefield.<Name>.ProtectionFrom` (`Black; Red`, sorted), the colors of its "protection from [color]" abilities
  (CR 702.16a), printed or granted, absent when it has none
- `BotN.Battlefield.<Name>.ProtectionFromSubtypes` (`Demon; Dragon`, sorted), the creature types it has protection from
  (CR 702.16a), absent when it has none; `ProtectionFromManaValue`, the least mana value it has protection from ("mana value
  3 or greater" is `3`), absent when it has none
- `BotN.Battlefield.<Name>.Monstrous`, `true` for a monstrous permanent (CR 701.37b) and absent otherwise
- `BotN.Battlefield.<Name>.Solved`, `true` for a solved permanent (a Case, CR 719.3b) and absent otherwise
- `BotN.Battlefield.<Name>.Suspected`, `true` for a suspected permanent (CR 701.60b) and absent otherwise
- `BotN.Battlefield.<Name>.Saddled`, `true` for a permanent saddled this turn (CR 702.171b) and absent otherwise
- `BotN.Battlefield.<Name>.Prepared`, `true` for a prepared permanent (CR 722.3a) and absent otherwise; while it is
  set, that permanent's preparation card is the copy in `Exile`
- `BotN.Battlefield.<Name>.Chosen<Key>`, one per quality chosen about a permanent and kept on it (CR 614.12a, 607.2d), as an
  ability of the text it has now reads it: a choice made by a text it no longer has as its own (it has since become a copy
  of something else, CR 607.5) is not dumped, and `ChosenPlayer` and `ChosenObject` are the player and the object chosen:
  `ChosenName` for a card name (chosen as it entered, or for it as its ability resolved: Petrified Hamlet), `ChosenOption` for the word chosen as it entered (CR 607.2f: Gollum's `Odd` or `Even`, a Siege's `Khans`), `ChosenColor` for a chosen colour,
  `ChosenSubtype` for a chosen creature type (Three Tree City), `ChosenObject` for the permanent chosen as it entered
  (Dauntless Bodyguard's creature), while that permanent is still on the battlefield (CR 400.7); and `ChosenModes`, the modes its modal abilities have
  chosen, numbered from 1 and read from the history (`1, 3` for Gollum's counter and draw)
- `BotN.Battlefield.<Name>.Exerted`, `true` for an exerted permanent (CR 701.43a) and absent otherwise
- `BotN.Battlefield.<Name>.GoadedBy` (`Bot2`), the players who goaded a creature (CR 701.15a), absent otherwise
- `BotN.Battlefield.<Name>.MustAttack` (`Bot2`, `Bot2; Bot3`, `Gideon, Battle-Forged`), the player or permanent each
  "attacks [that player or permanent] if able" effect on a creature names (CR 508.1d), one entry per effect while it lasts -
  one "during its controller's next turn" before that turn as well - and absent otherwise
- `BotN.Battlefield.<Name>.CantBeBlocked`, `true` while a resolved spell or ability's "it can't be blocked this turn" is on
  a creature (CR 509.1b, 611.2c: an effect, not an ability, so it is not in `Keywords` and losing its abilities leaves it),
  absent otherwise; a printed or granted "can't be blocked" is the `CantBeBlocked` of `Keywords`
- `BotN.Battlefield.<Name>.BlockRestriction` (`Can't be blocked except by creatures with haste`), each restriction on
  blocking a resolved spell or ability put on a permanent (CR 509.1b, 611.2c: Gingerbrute's), in its card's words, sorted
  and joined with `; `, while it lasts, and absent otherwise; like `CantBeBlocked`, an effect and not in `Keywords`
- `BotN.Battlefield.<Name>.AttachedTo` (`Grizzly Bears`), what an Aura or Equipment enchants or equips, or the player an
  Aura enchants (`Bot2`, CR 303.4), absent otherwise

Lists use `;`. Library, graveyard and stack are listed top first and compared in order. Hand, battlefield and exile
are compared as sets.

`Key != Value` holds when the key is in the dump with any other value: `Bot2.Library != Lightning Bolt; Island; ...`, the
library's order before a "then shuffle", shows the library was shuffled (Ashiok, Dream Render's forbidden search still
shuffles). Use it only where no positive value can be known; never for a `Client.` key or `(absent)`.

A key described above as absent is asserted absent with `(absent)`: `Bot2.Battlefield.Deep-Cavern Bat.Keywords =
(absent)` holds when it has no keywords at all, and fails if the key is there. It also fails when the dump has no
other key on the same object (`Bot2.Battlefield.Deep-Cavern Bat.`, or for a `.Counters.` key the permanent itself),
so a misspelt or departed object cannot pass vacuously, and on a key the dump never writes - a parent such as
`.Counters` (the dump writes `.Counters.MinusOneMinusOne`, one key per kind) or a misspelt property - which would be absent
whatever the game did ("not a key the dump has"). An object itself, `Bot1.Battlefield.Ornithopter = (absent)`, holds when no
object of that name is there (no `Ornithopter.` key, nor `Ornithopter#1.`). Anywhere else a missing key fails.

## Handing a position over

`[Handover Bot1.AssignDamage]` stops the script with that question outstanding instead of answering it. Named by the
verb that would otherwise have answered it, and by the player being asked.

A script that ends this way is two things at once. Run as a test it plays to the handover and checks its `[Expect]`
block, which is how it proves it reached the position it describes. It was also a position to deal to a real session for
a person or a bot to answer from, through the C# engine, which is gone; a table plays a scenario now
(`docs/scenarios-in-the-browser.md`), but stopping at a handover for a person to take the seat is not built yet.

Name the question. A bare `[Handover]` stops at whatever is asked next, which is usually one or two questions early
and silently so: priority passes in the declare-blockers step come before combat damage is divided. Until the named
question is asked, the seats go on answering by default, and a handover that never arrives fails the script after
four turns like any other line.

Expectations then describe the position mid-game rather than a finished one, which is the point: `Step = Damage`,
`Combat.Blocks = ...`, and every `.Damage` still `0` says the division is outstanding and not already answered.
