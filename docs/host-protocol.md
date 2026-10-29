# The native host protocol

`cantrip-native-host` (`crates/cantrip-host`) is one process per server, holding any number of games.
`Cantrip.Server` talks to it over stdin and stdout, one JSON object per line, one reply per request, in order; stderr is
the host's log. Every payload inside is `protocol.rs`'s JSON (PascalCase, `$type` tags), so the C# side deserialises it
into `Cantrip.Protocol` types and forwards. The C# half is `dotnet/src/Cantrip.Server/Rules/NativeRulesHost.cs` over
`NativeHostProcess.cs` (which finds the binary: `CANTRIP_NATIVE_HOST`, else the workspace's `target/cards`), behind
`IRulesHost`, which is the whole of what `GameSession` asks of a rules engine. The rules themselves are the native
engine's: `docs/engine-design.md`.

Requests and replies (a reply is `{"Ok": {...}}` or `{"Rejected": {"Code": "...", "Detail": "..."}}`; codes are
`Cantrip.Protocol.ErrorCode` member names):

| Request | Reply |
| --- | --- |
| `{"Cmd":"NewGame","Seats":[{"Name":"Alice","Deck":["Island", ...],"Sideboard":["Negate", ...],"Companion":"Lurrus of the Dream-Den"}, ...],"Seed":123,"Shuffle":true,"FirstPlayer":1,"Chooser":null,"BasicLandsOutside":[],"MinimumDeckSize":60}` | `{"GameId":1,"Decklists":{"1":[ObjectView, ...], ...}}` - decks are card names, top first when unshuffled; `Sideboard` is the cards the seat owns outside the game (CR 100.4, 400.11a), which only that seat sees (`ZoneKindView.Sideboard`, a zone per seat in each view) and which a card that brings one into the game chooses among (North Wind Avatar); `Companion` (or null) is the seat's companion chosen at deck building, dealt into its sideboard and revealed before the game (below); `BasicLandsOutside` gives a Limited game's sideboards any number of those basic lands; `MinimumDeckSize` is the format's minimum deck size, which the server says from the table's format (CR 100.2a-b) and a deal without is `Rejected` with `InvalidSetup`; `FirstPlayer` fixes who goes first, `Chooser` asks that seat instead (CR 103.1); a printed `ObjectView` per deck card, in deck order, for the welcome. A card the engine has not ported, in a deck or a sideboard, is `Rejected` with `InvalidSetup`, naming it, as is a deal of fewer than two seats or more than eight (`MAX_SEATS`) |
| `{"Cmd":"NewGame","Seats":[{"Name":"You","Deck":[]}, ...],"Seed":1,"Scenario":{"Name":"...","Text":"...","Driven":[1]}}` | the same: a table playing a `.cantrip` scenario, its decks, order and dice the scenario's; the `Driven` seats are answered by the table and every other by the script, and each step carries a `Scenario` object (`docs/scenarios-in-the-browser.md`) |
| `{"Cmd":"Start","GameId":1}` | a step |
| `{"Cmd":"Answer","GameId":1,"RequestId":7,"Seat":1,"Answer":{AnswerPayload}}` | a step, or `Rejected` with `IllegalAnswer`, `StaleReply`, `UnknownHandle` or `NotYourTurnToAnswer`, after which the request is still outstanding |
| `{"Cmd":"Check","GameId":1,"RequestId":7,"Seat":1,"Selected":[12, 15]}` | `{"Complete":true,"Addable":[18, 21]}` - what the rule of the outstanding choice of cards says of a partial selection (below); nothing advances. `Rejected` with `StaleReply`, `NotYourTurnToAnswer`, `UnknownHandle`, or `IllegalAnswer` for a question that picks no cards |
| `{"Cmd":"CheckBlocks","GameId":1,"RequestId":7,"Seat":1,"Blocks":[{"Blocker":12,"Attacker":3}]}` | `{"Complete":false,"Addable":[{"Blocker":15,"Attacker":3}],"Issue":{"Creature":3,"Words":"can't be blocked except by two or more creatures","Other":0,"Rule":"509.1b"},"Cost":""}` (captioned "Boggart Brute can't be blocked except by two or more creatures": Grizzly Bears, 12, blocks the menace Boggart Brute, 3, alone, and Llanowar Elves, 15, may join it) - what the rules make of a declaration of blockers for the outstanding declaration (below); nothing advances. `Rejected` with `StaleReply`, `NotYourTurnToAnswer`, `UnknownHandle`, or `IllegalAnswer` for a question that is not a declaration of blockers |
| `{"Cmd":"CheckAttacks","GameId":1,"RequestId":7,"Seat":1,"Attacks":[]}` | `{"Complete":false,"Addable":[{"Attacker":12,"TargetPlayer":2,"TargetPermanent":0},{"Attacker":12,"TargetPlayer":3,"TargetPermanent":0}],"Issue":{"Creature":12,"Words":"must attack","OtherPlayer":0,"OtherPermanent":0,"Rule":"508.1d"},"Cost":""}` (captioned "Ursine Monstrosity must attack": goaded by seat 2 and told to attack seat 2, it may attack either opponent) - what the rules make of a declaration of attackers for the outstanding declaration (below); nothing advances. `Rejected` as for `CheckBlocks` |
| `{"Cmd":"Concede","GameId":1,"Seat":1}` | a step |
| `{"Cmd":"End","GameId":1,"Winner":2}` | a step - the game ended from outside its rules, won by seat `Winner`, or a draw with `"Winner":null` (a draw agreed to, a match clock run out): every face-down permanent and spell is revealed (CR 708.9), told with the game's end. `Rejected` with `IllegalAnswer` for a game already over or a seat not in it |
| `{"Cmd":"View","GameId":1,"Seat":1}` | `{"View":PlayerView}` - the seat's current view, for resynchronising a client |
| `{"Cmd":"Oracle","Name":"Lightning Bolt"}` | `{"Card":ObjectView}` or `{"Card":null}` - a card as printed |
| `{"Cmd":"Companions","Deck":["Swamp", ...],"Sideboard":["Lurrus of the Dream-Den", ...],"MinimumDeckSize":60}` | `{"Companions":[{"Card":"Lurrus of the Dream-Den","Fulfilled":true}]}` - each card with companion in the sideboard, in its order, and whether the deck (a starting deck, CR 702.139b) fulfils its condition: the rules' judgement, which the server asks before it deals a deck with a companion (`DeckValidation`). A card the engine has not ported is `Rejected` with `InvalidSetup` |
| `{"Cmd":"Close","GameId":1}` | `{}` |

A **step** is what one advance of the game produced:

```
{"Requests":[{"RequestId":7,"Seat":1,"ForSeat":1,"Prompt":{PromptPayload},"MayCancel":false}, ...],
 "Views":{"1":{PlayerView},"2":{PlayerView}},
 "Events":{"1":[GameEventView, ...],"2":[...]},
 "Finished":false,"Outcome":"InProgress","WinningTeam":0}
```

`Requests` are every question outstanding, not only new ones; request ids are the host's and never reused within a game.
A run in progress - a spell or ability being announced and paid for (CR 601.2a-h), a resolution waiting on a question -
is told to nobody until it completes, since it may yet be taken back: its events wait, and every seat but the one
answering is sent the view it was last sent. An opponent sees no card leave the hand, no land tapped and no spell on the
stack until the spell is cast (CR 601.2i), and nothing at all of a cast that is given up.
Each event object carries a `"Sequence"` beside its `$type`, the host's game-wide event number, the same for every seat
told of it: it is what `EventsMessage.Sequences`, `AbilityResolvedEvent.First` and the log's line sharing run on. A
request may carry a `"Presentation"` (`PromptPresentationContext`); without one the C# side derives what it can from the
prompt and the seat's view. Seats are numbered from one. A game is resumed the way it always was: a fresh `NewGame` with
the same seed and decks, `Start`, and the recorded answers sent again in order, so the host is deterministic for a seed
and nothing in it is a snapshot.

Each option of a priority, pay-costs or unless-pays prompt carries an `OptionId`, its place among the question's options
from 1, and a `TakeActionAnswer` echoes it. Two options may be alike in `Kind`, `Source` and `AbilityIndex` - a card two
permanents' permissions would each let be cast, or one cast for its mana cost and for an alternative cost another permanent
gives - and the id says which is taken; what makes each possible (the engine's `granted_by` and `paid_by`) stays engine-side,
so nothing about hidden objects goes to the client. An id the question does not have, or one of an option with another kind,
source or index, is refused with `IllegalAnswer`; an answer without one (0) is taken where only one option has its kind,
source and index, and refused where several do. The rows themselves say what tells them apart (`Description`: "Pay {E} rather
than its mana cost (Lurrus of the Dream-Den)", "Mystic Forge"). The C# protocol is version 194 for this.

A pay-costs prompt that `AnnouncesXOrKicks` (CR 601.2b) is answered with an `AnnounceAnswer` whose `Value` is the X, or the
times kicked, from the prompt's `LeastAnnounced` to its `MostAnnounced` - one answer however high it goes, so a page steps the
value itself and sends nothing until it is chosen. A value past `MostAnnounced` is refused with `IllegalAnswer`. Its `IncreaseX` and
`DecreaseX` options still step it one answer at a time, as the bots do. Version 202. `LeastAnnounced` is 1 for an X that
can't be 0 (CR 107.3a: "X can't be 0"), where the announcement starts at 1, `DecreaseX` stops there, and 0 is refused; else 0.
Version 204. `XGoverns` is, for an X that is not paid in mana, what the cost says X governs - casualty X's "the least power of the
creature you sacrifice" (CR 702.153a) - and empty for any other; a page captions the announcement with it in place of the mana
wording ("not enough mana for it" would be false of a creature's power). Version 206.

**Checking a selection.** A choice of cards (`ChooseCardsPromptMessage`) and a library search (`SearchLibraryPromptMessage`)
are judged by the question's selection rule (docs/engine-design.md, "Selection rules"), which the client cannot run: the
prompt says how many it takes for its caption (`Count`, `CountKind`: `Exactly`, `UpTo`, `AnyNumber`, `AsManyAsCan`), its
noun and its parts' words (`Kind`, `Words`: "with different names"), a running total a part keeps for the player
(`MaximumTotalManaValue` with each card's `ManaValues`, for "total mana value 6 or less": `Tally`, display alone), and the
rule's `Verdict` on none chosen yet - `Complete`, whether that is an answer, and `Addable`, the cards that may be picked first, and for a
question that takes one card at most `CompleteAlone`, the addable cards that are an answer picked alone (a one-card pick of
those is sent without asking; every other selection waits for the host's verdict as it is made). There is no eligible list beside
it. Every other selection is asked of the host as it changes: the client sends `CheckSelectionMessage { RequestId, Selected }`,
the server asks the host's `Check` and answers `SelectionVerdictMessage { RequestId, Selected, Verdict }`, to that seat
alone. The answer is still checked when it is given. A cost's tapping for power, collecting evidence and crafting, which
the same prompt carries, are checked the same way. `GameClient.Answer` settles any answer to such a question with the host
before sending it - checked, and if refused mended a card at a time from what may be added - so every client, the bots
included, answers what the rules take. A check the host refuses is answered with an `ErrorMessage` whose `RequestId` names the
request, so a client reacts only to the error of its own check. Version 203.

**Targets judged together.** A `ChooseTargetPromptMessage` whose slot judges its targets together (`TargetSpec::together`:
"any number of target creature cards that each have a different mana value X or less") carries its rule's `Words` ("with
different mana values", "with mana value 3 or less") and its `Verdict` on none chosen over the object candidates; a rule that
keeps a running total of mana value still sends `MaximumTotalManaValue` and `ManaValues` beside them, display alone. Every
other selection of the objects among the targets picked is checked as a choice of cards is - `CheckSelectionMessage` with
their handles, answered with `SelectionVerdictMessage` - a player among the targets being no card to the rule.
`GameClient.Answer` settles a `TargetsChosenAnswer` to such a prompt with the host before it sends it, the players chosen
kept as they are. `Verdict` is null for a slot that asks nothing of its targets together. Version 210.

**Checking a declaration of attackers.** A declaration of attackers (`DeclareAttackersPromptMessage`) is judged the same way, by
the engine's one validator (`AttackRules::judge`, `crates/cantrip-state/src/attacks.rs`): each attacker eligible and attacking
something offered, no more creatures attacking a player, planeswalker or battle than a static ability caps it at (CR
508.1c: "No more than one creature can attack you each combat"), and as many of the requirements obeyed as any declaration
within those caps can attacking for free (CR 508.1d). The caps are not in the prompt: a client learns of one from the
verdict, whose `Addable` never offers an attack past a cap and whose `Free` is false wherever a target is capped. The prompt's
`Requirements` are one per source, for showing the player - an `Attacker` that must attack (`TargetPlayer` `0`,
`TargetPermanent` `0`), that must attack a seat or permanent, or that must attack a player other than `OtherThan` (a goad's,
two per goader) - with `MostRequirements`, the engine's own declaration `Suggested`, and the `Verdict` on no attacks
(`Complete`, `Issue`, `Addable`, `Cost`). Any other declaration is asked with `CheckAttacksMessage { RequestId, Attacks }`,
answered with `AttacksVerdictMessage { RequestId, Attacks, Verdict }`. `GameClient.Answer` settles every declaration of
attackers with the host as it does blocks, and a client offers the attacks the verdict does. Version 205.

**Optional costs to attack.** Once a declaration of attackers is sent and its creatures tapped (CR 508.1f), the attacking
seat is asked about each optional cost to attack its attackers have (CR 508.1g), one question at a time, before any cost is
paid: enlist (CR 702.154a) as a `ChooseCardsPromptMessage` whose `Purpose` is `Enlist` and `Source` the attacker - up to
one of the creatures it may tap, none chosen paying nothing - and "you may exert this creature as it attacks" (CR 701.43d)
as a yes-or-no `MayPromptMessage` whose question's verb is `Exert`. A creature chosen for one enlist ability is not offered
for another. Then the costs are locked in and paid (CR 508.1h-j); a creature to be enlisted that was tapped for mana
meanwhile makes the declaration illegal, and the declaration of attackers is asked again (CR 733.1). Version 207.

**Checking a declaration of blockers.** A declaration of blockers (`DeclareBlockersPromptMessage`) is judged by one validator
in the engine (`BlockRequirements::judge`, `crates/cantrip-state/src/blocks.rs`) against every restriction, requirement and
cost (CR 509.1a-d): each block allowed, each creature blocking as many attackers as it may, each attacker within its fewest
and most blockers (menace, "can't be blocked by more than one creature"), and as many requirements obeyed as any declaration
can without paying a cost. A requirement is a `BlockMessage` in `Requirements`, one for each effect or source: a blocker and an
attacker (Lure), a blocker with `Attacker` `0` ("blocks this turn if able", obeyed by blocking any attacker), or an attacker
with `Blocker` `0` ("must be blocked if able", obeyed by any creature blocking it). The prompt carries the rules' `Verdict` on
no blocks: `Complete`, whether that may be declared; `Issue`, why not (`Creature`, `Words`, `Other` - `0` for none - and
`Rule`, the CR number, to caption as "Grizzly Bears must block Goblin Guide"); `Addable`, the blocks some legal declaration
has; and `Cost`, what the declaration costs (CR 509.1d, "{2}"; empty when free), which is paid once it is sent through the
payment prompt attack costs use. Addable is not "legal now": one blocker of a menace creature is addable where a second can
join it, and a block no legal declaration has alongside those declared is not. Every other declaration is asked of the host
as it changes: the client sends `CheckBlocksMessage { RequestId, Blocks }`, the server asks the host's `CheckBlocks` and
answers `BlocksVerdictMessage { RequestId, Blocks, Verdict }`, to that seat alone. A client holds any set of blocks - a
creature blocking more than one attacker included - and offers exactly the verdict's `Addable`; `GameClient.Answer` settles
every declaration with the host before sending it - checked, and if refused mended a block at a time from what may be added,
the blocks meant first and the engine's own declaration (`Suggested`) next; a check the host refuses sends nothing, and the
question is asked again unless it is no longer outstanding - and answered with the engine's own declaration if it is
answered so again, so an automatic seat never goes round forever. So no client works out a requirement, restriction or cost itself,
and a new one changes the engine alone. `MinimumBlockers`, `MaximumBlockers`, `Requirements` and `MostRequirements` are for
showing the player. The answer is still checked when it is given. Version 205.

An unrestricted `SeparatePiles` selection (`AnyNumber`, no parts or candidate restriction) uses the existing non-targeted
`ChooseTargetPromptMessage` presentation; its `TargetsChosenAnswer` is translated to the engine's card selection and
checked by the same validator. A pile-choice mode names its members, with an empty pile named "nothing" (CR 700.3d). The same
holds for a `SeparateFaceDown` selection (the chosen cards are the face-down pile, the prompt says so). A face-up pile is named by its
cards ("Face-up pile: Lightning Bolt, Counterspell") and a face-down pile by how many cards it holds only ("Face-down pile: 2 cards"),
since the player choosing between the piles may not look at them.

A mana ability can be taken back while nothing hidden has been shown and nothing has followed from it. While the last
answer a seat gave activated one, and it and everything since only tapped, sacrificed from the battlefield, removed
counters, added mana or cost life (a land, Black Lotus, Lotus Petal, Pentad Prism, a Treasure), that seat's next priority
or pay-costs prompt offers an `UndoManaAbility` option on the permanent; taking it moves the game back to before the
activation through the game-log tree (`Game::goto`), each seat's handles back with it, and sends an
`EventsRetractedEvent` naming the sequences told of it. Never taken back: an ability whose cost discards (Lion's Eye
Diamond shows the hand), and one that triggered anything (Chromatic Star draws). Any other answer - casting a spell
included - a concession, or anything else happening clears what can be taken back. An undo is an answer like any other,
so a resumed game replays it.

**Controlling another player.** While a player controls another (CR 723: Mindslaver, Emrakul, the Promised End), every
question asked of the controlled player goes to the controlling seat: a request's `Seat` is the seat that answers it and
`ForSeat` the seat whose decision it is (the same seat but for this), its prompt in the answering seat's handles, and an
answer, check or undo from any other seat is `NotYourTurnToAnswer`. The server passes it on as `PromptMessage.ForSeat` (zero
for the seat's own), and never lets the controlling seat offer a draw with the controlled player's priority. Each view says
who controls whom (`PlayerSummaryView.ControlledBy`, and a line in `PendingEffects`, as for an effect still waiting for the
player's next turn), and the controlling seat's view holds what the controlled player sees (CR 723.4): their hand
(`ContentsVisible`), the faces they know, what their questions name - not their sideboard. Conceding is each seat's own (CR
723.6). Nothing in a prompt is specific to the card that gave control, so a later one needs no protocol change. Version 208.

**Companion.** A seat's companion is chosen at deck building (`Companion` on its seat; CR 702.139a) and the server has
checked the deck against its format before the deal, the companion's condition with the `Companions` command. Before the
game, once the starting player is known, each seat's companion whose condition the starting deck fulfils is revealed, with
nothing asked of anyone (CR 103.2b), so nothing a seat sees says what another chose; one whose condition it does not fulfil
stays a hidden sideboard card. A revealed companion stays in its owner's sideboard, revealed: every seat's view holds it as an
object, names it as `PlayerSummaryView.Companion` (none once it is in the game), and another seat's `Sideboard` zone, closed
(`ContentsVisible` false, the whole `Count`), lists it alone in `Contents`. Once a game, with priority in a main phase of
their turn and the stack empty, its owner is offered a `TakeCompanion` option on it (CR 116.2g), described "{3}: Put [card]
in hand."; taking it pays {3} as a special action and puts the card into their hand, where it is a card like any other (CR
702.139c). Version 209.

**Naming a card.** A `NameCardPromptMessage` lists in `Names` only the names the question allows, and its `Kind` is what
the name must be the name of in the card's own words, article and all (CR 201.4a): "a card name", "a nonland card name", "a
card name other than a basic land card name" (Necromentia), "a noncreature, nonland card name". A client captions the question
"[source]: choose [Kind]"; the host refuses a name outside `Names` with `IllegalAnswer`. Version 215.

**A player chosen for one of several objects.** A `ChoosePlayerPromptMessage` whose effect chooses a player for each of
several objects in turn names the one it is asking about as `Subject` (no handle otherwise): Dack Fayden, Helping Hand's "for
each of those permanents, choose a different opponent", verb `GainControlOf`, the permanent the chosen opponent gains
control of. A client captions it with the subject's name, "The player you choose gains control of [Subject]". Version 216.

**Face down, prototyped, assisted.** A cast by morph or disguise (CR 702.37c, 702.168b) is a `CastSpell` option with
`CastFaceDown`: a 2/2 face-down spell with no name or text (ward {2} for disguise) for `CostText`, {3}, its `Description` the
keyword line ("Morph {3}{W}"), whether the card is in a hand or a permission lets it be cast from elsewhere. The face-down spell
is `FaceDown` on the stack, `Known` with its `FaceDownName` to its controller alone (CR 708.5); a face-down spell or permanent
that leaves for anywhere but the battlefield is revealed (`CardsRevealedEvent`, CR 708.9), to a seat that may never have held a
handle for it. A manifested or cloaked instant or sorcery an effect would turn face up is revealed and stays face down (CR
701.40g, 701.58g): every seat's view of the step that tells its `CardsRevealedEvent` has it `Known` with its `FaceDownName`,
and the opponents' views after that step no longer do (CR 708.5). A face-down permanent offers one `TurnFaceUp` per way to turn it up: for its morph or disguise cost
(`Description` "Turn face up: Morph {3}{W}", `CostText`, `HasX`, `LifeCost` that cost's) or for its mana cost, a manifested or
cloaked creature card's (`Description` the ability's own words, "Turn this permanent face up by paying its mana cost."). A cast prototyped (CR 718.3) has `CastPrototyped`, `Description` the keyword line
("Prototype {2}{R} — 3/2") and `CostText` the prototype's mana cost; `ObjectView.Prototype` is that line for a prototype card in
any zone, and `Prototyped` marks a prototyped spell or permanent, or a copy of one, whose cost, power, toughness and colors in
the view are the prototype's. Assist (CR 702.132a) asks the caster a `May` with the verb `Assist` (a `ChoosePlayer`, verb
`Assist`, with more than one other player), then `PayCostsPromptMessage`s with an `AssistStep`: `HelperActivates` and
`CasterActivates` (a window for mana abilities closed by `ConfirmCasting`; `Remaining` is the whole of what is owed),
`HelperAnnounces` (how much of the generic mana the chosen player pays, up to `MostAnnounced`), `HelperPays`, and `CasterPays`
(no mana ability offered; `Assisted` is what the other player paid). The chosen player's steps offer no `CancelCasting`, and
their default answer closes the window, announces nothing more, or assigns the first pool mana offered. Version 217.

**Squad is not kicker.** A `PayCostsPromptMessage` whose optional additional cost is squad (CR 702.157a) sets `Squad`
beside `Multikicker`: `Kicker` is then the whole cost in words, the mana and the rest ("Exile four other cards from your
graveyard"), and the number of times it can be paid is `MostAnnounced`, set by the cards there are, not by mana. A client
says "pay squad for [Spell]", never "kick", and does not speak of mana for it. Version 219.

A draw the players agree to (`docs/rules-exceptions.md`, RX-4) is not a request of the host's: the offer and the answers to it
are the server's (`GameSession.OfferDraw`, `AnswerDraw`, over the C# protocol's `OfferDrawMessage`, `AnswerDrawMessage` and
`DrawOfferMessage`, version 201). Offering passes the offerer's priority, which the host is sent as any `PassAnswer`; once
every other player still in the game has accepted, the session ends the game as a draw from outside the host, as a match
clock running out does (`GameSession.Forfeit`), and nothing more is asked of it.

**Recording for the AI player.** `cantrip-native-host --ai-record DIR` writes every game that ends - won, drawn,
conceded, or closed before its end (a cap: every outcome 0) - into zstd training shards in `DIR`
(`crates/cantrip-ai/src/shard.rs`; `docs/ai-player-plan.md`, "Data"), each decision as the answering seat observed it
and the option it chose, decomposed into the policy's steps. A game still unfinished when the host's input ends is
dropped and counted. Recording is done by worker threads off the request path, and changes no reply
(`crates/cantrip-host/tests/ai_record.rs`); once stdin closes the host finishes the queue, closes its shards, and writes
`DIR/report-<pid>-<ms>.json` (games, samples, bytes, shards, unfinished games, and the answers that made no sample by
request kind and why) before exiting. The recorder is built only with the host's `ai-record` feature (`cargo build -p
cantrip-host --features ai-record`, which `tools/bot-duel.sh`, and `tools\bot-duel.cmd` given `--ai-record`, do), so the
shipping host compiles neither the AI player nor zstd; a host built without it refuses the flags. The flags:

| Flag | Default | |
| --- | --- | --- |
| `--ai-record DIR` | off | where shards go, `cantrip-w<i>-...shard.zst` (a `.part` until closed) |
| `--ai-record-mode full\|decklist\|hidden` | `full` | the information each observation holds; the shard header names it, and the card index its rows are of (`cards`, the engine's own card names) |
| `--ai-record-producer TEXT` | `unknown` | what played the games (the bot lab writes `tactical:<model hash>`) |
| `--ai-record-seeds TEXT` | empty | the seeds the games were dealt from, as the caller describes them |
| `--ai-record-threads N` | 2 | recording workers, each with its own shard |
| `--ai-record-games N` | 0 (no limit) | games a shard holds before the next is started |
| `--ai-record-mb N` | 64 | compressed megabytes a shard holds before the next is started |

A shard recorded here labels each step with the option chosen: its visit field is a one-hot. Self-play shards come
from `cantrip-selfplay` instead (`docs/ai-host.md`, "Self-play"), not from this host. They have the same layout, with
the search's root visit distribution in the visit field and each searched decision's root value in the game's meta
(`root_values`, empty here).

The C# side starts the shared host with `NativeHostProcess.SharedArguments` and stops it with
`NativeHostProcess.CloseShared(wait)`, which the bot lab's `duel --ai-record DIR` uses.

**Playing as the AI.** The AI player answers from a process of its own, `cantrip-ai-host`. It holds a mirror of each
game the table hosts and is sent every command the table is sent. It speaks this protocol plus `AiStart` /
`AiResult`: `docs/ai-host.md`.

The cards a host knows are the ones the native engine has ported (`docs/adding-cards.md`); a deck naming any other
is refused at the deal. The decks under `decks/demo` and `decks/spike` are kept to ported cards, and
`NativeHostGameTests` deals them through the real binary.
