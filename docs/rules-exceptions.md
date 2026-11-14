# Rules exceptions

The engine and every card follow the game's rules - the Comprehensive Rules and the official rulings - exactly. This
file lists the only deviations a human has allowed. Nothing else is an exception: a deviation not listed here is a bug,
and blocks any change that would bring it in (`CLAUDE.md`, "No rules deviations").

An automated agent never allows a deviation and never adds an entry here. It asks a human, and only when the rule is
one of these:

- **Nonsensical for the domain:** a rule that only makes sense for paper play (ante, physical card handling).
- **Extremely niche and extremely complicated to build:** sub-games (Shahrazad), stickers and the like.
- **Only reaching banned cards:** a rule that affects only cards banned in every format Cantrip supports.

A decision recorded here may be applied to a later case of the same kind; the later case is added to its entry.

## Entry template

```
### RX-<n>: <the rule, and what the engine does instead>

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR <number> | paper-only / niche and complex / banned cards only | <person> | YYYY-MM-DD |

**What the rules say.** ...

**What the engine does instead, and why.** ...

**Cards and cases it covers.** ...
```

## Exceptions

### RX-1: A game that reaches the engine's maximum number of events ends in a draw

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 104.4b, 732.4-5 | niche and complex / beyond practical scale | jschibel | 2026-10-21 |

**What the rules say.** A loop of mandatory actions is a draw (CR 104.4b); a loop the players keep going by their own
choices goes on as long as they choose, and no player can be made to break it with an action outside the loop
(CR 732.5). A run of play that never repeats exactly - a token, an effect or a card more each time round, while the
other players only pass - is neither recognised as a loop nor ever ended by the rules.

**What the engine does instead, and why.** The engine keeps recognising every loop it can tell exactly, as the rules
say. Beyond that it has a practical maximum on the game's history (`MAX_GAME_EVENTS`, in events recorded), sized to
hold the longest and most complicated multiplayer game with headroom; a game that reaches it ends in a draw. The
engine's earlier judgements of "a run going nowhere" (a step-length limit, a forced-run limit) are replaced by this one
bound.

**Cards and cases it covers.** Any combination that grows without repeating while nobody acts to end it.

### RX-2 (not a deviation, recorded as a decision): a "may" declined every time round a loop is left to the clock

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 732.5 | - | jschibel | 2026-10-21 |

**What the rules say.** No player can be forced to perform an action that would end a loop other than those called for
by objects in the loop (CR 732.5); a player who declines a "may" each time round keeps the loop going by choice.

**What the engine does.** It does not end such a loop as a draw after a number of rounds. Every iteration is played by
hand, and the match clock runs down until the player times out, as for any other loop kept going by choice.

### RX-3 (not a deviation, recorded as an interpretation): a dependency loop with a dependency outside it

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 613.8b | - | jschibel | 2026-10-21 |

**What the rules say.** If several dependent effects form a dependency loop, "this rule is ignored and the effects in
the dependency loop are applied in timestamp order" (CR 613.8b). The rules do not say what happens when a member of
the loop also depends on an effect outside it, and no ruling settles it.

**How the engine reads it.** Only the dependencies that make up the loop are set aside: the cycle is broken by falling
back to timestamp order among its members, and every other dependency - including a loop member's dependency on an
effect outside the loop - stays in place, so the order is a DAG. Dependencies are judged on the working state at each
step (CR 613.8c). Example: Life and Limb, Arcane Adaptation naming Saproling, Blood Moon (in that order), Taiga and
Llanowar Elves: Arcane Adaptation, then Life and Limb, then Blood Moon.

### RX-4: Loops are left to the players, the clock and draw offers; only a loop of passes is drawn by the engine

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 104.4b, 732.3-5 | niche and complex | jschibel | 2026-10-21 |

**What the rules say.** A loop of mandatory actions is a draw (CR 104.4b, 732.4). In a loop of optional actions taken by
several players, the active player (or the first player in turn order involved) must make a different choice (CR 732.3).
No player can be made to take an action outside the loop to end it (CR 732.5).

**What the engine does instead, and why.** The engine detects one kind of loop only: the same exact position comes round
again with every player having only passed priority and nothing having read the game's history in between. It lets
such a loop go round a few times - a player holding an answer outside the loop (a Disenchant in hand) may still choose to
end it - and then ends the game in a draw. Every other loop is played by hand: no player is made to choose otherwise
(CR 732.3 is not applied), and the loop goes on while each player's clock runs, until someone runs out of time or the
players agree to a draw. Any player may offer a draw while they have priority (the game menu's "Offer Draw"): the offer
passes priority, and while it stands each of the offering player's later priorities runs no clock for its first three
seconds (a courtesy, so an offer can't be used to run an opponent's clock down); anything other than passing withdraws
it. Each other player is asked to accept or decline when it is made, and may accept later at any priority (the same
menu entry, then "Accept Draw"); the game is a draw when every other player still in it has accepted. A game that
reaches the event cap is a draw (RX-1). A loop detector for the rest would have to catch far too many cases to be
tractable; this is the closest to the spirit of the paper rules.

**Cards and cases it covers.** Fragmented loops (CR 732.3); loops whose players keep choosing to go on; a loop of
mandatory actions while a player holds an unused response outside it (drawn after a few rounds of passes).


### RX-5: Numbers keep their current widths

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| Any rule that counts (kicker and multikicker, player counters) | beyond practical scale | jschibel | 2026-10-21 |

**What the rules say.** A number in the game has no upper limit.

**What the engine does instead, and why.** The engine keeps the widths it has today for the times a kicker or multikicker
cost was paid (`Obj::kicked`), the mana one payment records from one source (`ManaTally`) and player counters (poison,
energy and experience, and toxic; KI-103 and KI-104, closed by this decision), with no rollover handling. If one of them
rolls over in a real game, an issue is opened then. A mana cost is no longer part of this: its generic mana and each
count of its symbols, the cost changes added to it and the mana a spell's casting spent (`Obj::spent`) are 32-bit, with
plain arithmetic (2026-11-01). Power, toughness, counters, damage and X are 32-bit, with plain arithmetic (PR #2406,
2026-11-05).

The same decision keeps today's capacities (KI-105, closed): an object's 32 abilities (`MAX_ABILITIES`), a copy's six
abilities added by exceptions, and the special actions given once layer 6 is done taking the first free ability slot.
Text-changing effects are no longer part of this: an object keeps every one (`Words`, 2026-11-01). A full list takes
no more and never panics; a repeatable modal counts each mode to 15.

Narrowed 2026-11-01 by jschibel: mana costs and text changes are exact; RX-5 now covers only what it still lists.
Narrowed 2026-11-05 by jschibel: power, toughness, counters, damage and X are exact (32-bit, plain arithmetic, PR #2406);
RX-5 now covers only what it still lists.

### RX-6: A player keeps after their seventh mulligan

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 103.5 | beyond practical scale | jschibel | 2026-10-21 |

**What the rules say.** A player may mulligan as many times as they like.

**What the engine does instead, and why.** After seven counted mulligans the player keeps (`turn.rs`); in a multiplayer
game the first mulligan is free and does not count (CR 103.5c), so that is after eight there. A player who has taken
seven counted mulligans has an empty opening hand to put cards back from, so another mulligan makes no practical
difference.

### RX-7: A card an ability is activated from in a hidden zone is revealed once, not kept revealed while the ability is on the stack

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 701.20a (with 602.2a) | paper-only | jschibel | 2026-10-31 |

**What the rules say.** Activating an ability from a hidden zone reveals the card that has it (CR 602.2a), and "the card
remains revealed from the time the spell or ability is announced until the time it leaves the stack" (CR 701.20a).

**What the engine does instead, and why.** The reveal is one atomic event, made once by the activation, as the ability is
announced and put on the stack and before its costs are paid (`Event::Revealed`, by the activator; `reveal_if_hidden` in
`cantrip-rules/src/actions.rs`). The card is not treated as seen by every player afterwards while the ability waits on the
stack: not by the look-back rule (CR 603.10a, `ops::seen_by_all`) and not by what a seat is shown
(`cantrip-engine/src/visibility.rs`). Keeping the card revealed is a logistical aid for paper games, so players can track
what is on the stack: nothing triggers on it, and everything that must be public is made public by the event, which every
player sees and the history keeps. The one rule that reads the lasting reveal is the look-back of CR 603.10a: a card moved
from the hand or library into a hand or a library while its ability waits is judged hidden as it moves, so an ability that
triggers on an object all players can see being put there does not look back for it. That case does not arise in
practice: nearly every such ability moves its card to a public zone as part of its cost (cycling and channel discard
it, a Spirit Guide exiles it, Urban Retreat and its kind put it onto the battlefield). Ninjutsu is the exception, and a
single reveal serves it fully: a player who wants to respond to it, or stifle it, does so with exactly the information a
lasting reveal would give.

**Cards and cases it covers.** Every activated ability activated from a hand or a library, mana abilities included:
cycling and typecycling, channel, ninjutsu, transmute, the Spirit Guides, Urban Retreat and the other "from your hand"
abilities. The lasting reveals of behold, splice and miracle (also CR 701.20a) are not covered.

### RX-8: Game of Chaos stops offering "flip again" once the doubled stake would take a life total past 2^24

| Rule | Category | Decided by | Date |
| --- | --- | --- | --- |
| CR 119.3, 705.1; Game of Chaos's text | beyond practical scale | jschibel | 2026-11-14 |

**What the rules say.** Game of Chaos goes on for as long as each flip's winner chooses to flip again, and the life
stakes double with each flip, with no limit: after 31 flips a stake no longer fits in 32 bits.

**What the engine does instead, and why.** After each flip, the winner is asked whether to flip again only while the
next, doubled stake - gained or lost by either player - would leave both players' life totals within 2^24 (16,777,216)
of zero; otherwise the spell ends as if the winner had stopped. Every flip that happens is exact. The bound is reached
only after some 23 flips from 20 life, and it keeps each total far inside the engine's 32-bit life totals and life
changes with room for what may follow: the difference of two totals (an exchange of life totals, Mirror Universe), and
several gain or loss doublers (Boon Reflection, Bloodletter of Aclazotz) applied to a stake. The bound is computed in the card's own file
(`game_of_chaos.rs`, `LIFE_BOUND`); nothing shared is widened or changed for it.

**Cards and cases it covers.** Game of Chaos only. A later card whose own doubling stakes can push life past 32 bits
within a game may cite this decision.
