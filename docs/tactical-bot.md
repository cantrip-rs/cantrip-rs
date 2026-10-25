# The tactical bot

`TacticalPlay` (`dotnet/src/Cantrip.Client/Tactics`) is the seat meant to be played against. `AutoPlay` takes the one same
line every game and `RandomPlay` draws among what it is offered; neither looks at the board. This one does: it
attacks where the defender's best blocks still leave it ahead, blocks where the trade or the life is worth the
creature, spends removal on what is worth a card, and sends burn at a player once their life total makes that the
better use of it.

Two things were fixed before anything else was chosen, and they shape the rest:

- **It runs entirely locally.** No model server, no API, nothing to download: a decision is arithmetic over the board.
- **It runs wherever a client runs, the browser included.** It lives in `Cantrip.Client`, which the web client is
  built from, so the same seat plays in a console process, inside the Discord bot, and in a player's own tab.

## It is a client

Everything it knows is in the `PlayerView` and the prompt a player's browser would be sent, and every answer is
built from what the prompt offers. It has no rules engine and cannot reach one - `Cantrip.Client` depends on the
protocol and nothing else - so it cannot cheat, cannot see a hidden card, and cannot make a move the server has not
offered. That is also why it can be handed to a player's machine without a second thought.

It costs one thing: the bot cannot try a move and see what happens. Three pieces stand in for that.

**Reading the cards** (`CardReading`). The protocol sends rules text and computed numbers, nothing structured, so the
bot reads a card the way a player skims one: keyword lines, "deals 3 damage to any target", "destroy target
creature", "exile target creature", kicker, the modes of a modal spell, what a creature does as it enters. It
recognises the common shapes and treats the rest as unread: a spell it cannot read that aims at nothing is cast when
there is nothing better to do with the mana, and one that aims at something is not cast at all, since there is no
telling whose things it is good for. A misread costs it a worse choice among legal answers; it cannot make one
illegal.

**Working combat out** (`Combat`). Combat is the one part of the game a client can predict: power, toughness and a
handful of keywords decide it. A creature blocks one attacker, so a combat is a sum of small fights and one life
total, cheap enough to try every block against every attack on an ordinary board and to climb towards a good one on a
crowded one. The attack it makes is the one that is best *after* the defender has blocked as well as it would itself,
and after the attack back: a creature that attacks cannot block on the opponent's turn, and an attack that would
leave it dead on the board is only made when staying home would not save it either.

**Scoring a position** (`TacticalModel`). A linear, symmetric model in log-odds of winning: each side's life, cards
in hand, lands and permanents become the same features - a permanent's being its size, its keywords and its name -
and the position is worth this seat's sum less the opponent's. That shape is what lets the bot use it without
simulating anything: the worth of a creature is a number to subtract when a block would kill it, and the worth of a
life total is a curve to read damage off. Life is banded, so the last five points are worth far more than the fifth
five, which is where chump blocks and burn to the face come from: neither is a rule, both fall out of the sum.

Where it has no way to tell whether something worked, it looks. An activated ability that left the board as it was
(Figure of Destiny's first ability, a second time) is not tried again that turn. And where two bots could feed each
other for ever it stops by count: it copies a spell that offers a copy at most three times a turn, and only one that
removes something, so two of them do not pass Chain of Smog back and forth.

## The weights are fitted

`TacticalModel.Prior()` is a hand-written starting point, good enough to play from. What ships is
`dotnet/src/Cantrip.Client/Tactics/tactical-model.txt`, an embedded resource, made by `tools\bot-train.cmd` in two stages.
Each has to beat what it started from over a few thousand games or nothing is written.

1. **Fit** (`cantrip-botlab train`). The bot plays itself, a few percent of its decisions drawn at random so the
   games differ. Every position is marked with who went on to win and the weights are fitted to that by logistic
   regression, held towards the prior where the games say little. This is where a card gets a weight of its own:
   what Hero of Bladehold is worth beyond being a 3/4.
2. **Tune** (`cantrip-botlab tune`). Simultaneous-perturbation stochastic approximation, as chess engines are tuned:
   every weight is nudged up or down at random, the two nudged bots play each other, and the weights move a little
   towards the winner, hundreds of times. It asks which bot wins rather than which predicts, so it reaches the
   `policy.*` weights the fit cannot - how much an attack back is feared, what a play has to gain to be worth making.

Both run at the session layer - the real session and protocol, no socket - at about thirty games a second on a
many-core machine. RW aggro (`decks/rw-aggro.txt`) is the deck it was trained with. A card the fit has not met has no
weight of its own and is worth what its size and keywords say, so the bot plays any deck and plays the ones it
trained on better; `--deck` trains with others.

## How good it is

`tools\bot-duel.cmd` sets any seat against any other, seats swapped every other game. As shipped (2026-10-03):

| the tactical bot against | deck | won | of |
|---|---|---|---|
| the random bot | RW aggro, both seats | 99.1% | 2000 |
| the deterministic bot (`AutoPlay`) | RW aggro, both seats | 92.8% | 2000 |
| its own hand-written prior | RW aggro, both seats | 53.3% | 4000 |
| the random bot | UB reanimator, both seats | 88.2% | 1000 |
| the deterministic bot | UB reanimator, both seats | 69.4% | 1000 |
| the deterministic bot | RW aggro against UB reanimator | 66.9% | 1000 |
| the random bot | the powered demo decks 1 and 2 | 64.6% | 1000 |

For scale, the deterministic bot beats the random one 90% of the time with RW aggro, so it is the baseline that
means something. Three things the table says. The bot is far better than either where its cards are creatures and
burn. Training bought about three points over the hand-written weights, all of it from the tuning: the fit alone won
49.5%, which is to say that predicting who wins better did not, by itself, play better. And it is weakest where the
game is decided by cards it cannot read - rituals, tutors, reanimation, a Wrath it does not know to hold - which is
the bottom row.

An answer costs it nothing that matters: the same millisecond a random answer does, nearly all of it the game's own
work. Six hundred games of it against itself with the every-card test deck all ended, none stuck.

Read games, not only tallies: `bot-duel.cmd watch` tells one turn by turn with the board at each turn, and
`watch --lost` finds a game the bot lost, which is where its mistakes are.

Its games are also the AI player's first training data: `bot-duel.cmd --a tactical --b tactical --ai-record DIR` has
the native host record every decision of every game that ends as training shards (`docs/ai-player-plan.md`, "Data";
the flags are in `docs/host-protocol.md`), headed `tactical:<hash of tactical-model.txt>`. The bot is deterministic,
so against itself it would play each shuffle's pair of games (seats swapped) the same way twice; a recorded self-duel
deals every game its own shuffle instead (`--each-shuffle`, which any duel can ask for).

The bot is also the AI player's yardstick. A `neural` seat (`neural:decklist` and `neural:hidden` also exist) plays
the network alone, with no search. It answers from `cantrip-ai-host` beside each worker's native host
(`docs/ai-host.md`). An example run is `tools/bot-duel.sh --a neural --b tactical --deck rw-aggro --deck ub-reanimator
--games 160`. On top of the usual tally, the duel splits each answer's time: the host's part and the seat's part for
both seats, and the network's encode, evaluate and round trip. Phase 2's gate result is in `docs/ai-player-plan.md`.

## Measuring a change to it

Against the random bot every version of the pilot wins about nine games in ten on drafted decks, so that tally
says nothing about a change. Three things do, all in `cantrip-botlab` (`tools\bot-duel.cmd`).

**A champion.** A frozen build of the pilot as it last landed, played head to head against today's on a pool of
drafted decks, each deck in both seats:

```
cantrip-botlab champion freeze main-1c9a5242      copies this build's client into .cantrip-lab\champions under that name
cantrip-botlab champion use main-1c9a5242         makes it the seat named "champion"
cantrip-botlab champion referee main-1c9a5242     makes it the seat named "referee" (the draft lab's pinned pilot)
cantrip-botlab champion list
tools\bot-duel.cmd --a tactical --b champion --decks <pool> --games 4000
```

A champion is real earlier code, weights and all: the compiled `Cantrip.Client` of that commit with the protocol it
spoke, loaded beside the current one in a load context of its own. The two builds share no type. The frozen one
plays with its own `GameClient` and its own `TacticalPlay`, and what crosses between them is what would cross a
socket, protocol messages as bytes, so it goes on working when the client's types change and stops only if the wire
format does (`Champion.cs`). It costs about what a second serialization of each message costs. `.cantrip-lab` is
the nearest folder of that name at or above the repository root, so every worktree under one checkout seats the
same champions; `CANTRIP_LAB` names another. Freeze a new one when a change to the pilot lands, and `use` it;
re-pin the referee only at a milestone, and say so, because every drafter number taken with it changes meaning.

**A policy weight.** A new behaviour goes behind a `policy.*` weight where it can, zero meaning the old way, and a
seat can be named with a weight set: `--a tactical --b tactical+policy.crew=0` is one build playing itself with
crewing on and off. `--has "Smuggler's Copter"` keeps only the decks of a pool that have the card the behaviour is
about, which is where a behaviour about one card shows.

**The cards it wins with.** `duel --cards N` says, for each card, how often a seat won the games in which it held
the card at some point against the games with the same decks in which it never did. In a mirror both seats have the
card in their deck, so the deck is no part of the difference; what is left is whether drawing the card helped the
seat that drew it. The typical card comes out a few points up (a card held is also a game that went long enough to
draw it), and a strong card that comes out ten points down is one the pilot leaves in its hand or throws away.
That table is where the worklist comes from: it found that no Mox was ever cast.

## Soaking

`tools\bots-soak.cmd` plays a few thousand games between bots, with the every-card deck (`demo/card-test`) and any
pool given with `--decks`, and reports nothing but the games that did not finish, grouped, each with the
`bot-duel.cmd watch --game N --seed S ...` line that plays it again exactly; it exits 1 if there were any. It is
fuzzing and is meant as that: random shuffles, no position anyone is trying to reach, nothing said about who won. A
game that throws, stalls or never ends is a bug in the engine, the session or a bot's own loop guards, and the
replay line is where a scenario for it starts. It is not how a position is reached for a test, and not a measure of
play.

What it does not do, and would be the next things to teach it:

- It does not plan past the turn. It holds removal for something worth it and keeps blockers back, but it has no
  notion of what the opponent is likely to be holding.
- Combat tricks and protection are used only in answer to something aimed at its creatures (Mother of Runes saves
  what is being bolted; she does not push an attacker through).
- It crews a Vehicle only to attack, never to block, and leaves saddle and the order of triggers to the prompt's
  default. It sends attackers at a planeswalker only when that beats the attack it chose on the player.
- A card whose text it cannot read is worth only its size. `CardReading` is where a new shape is taught. Tutors,
  rituals and storm, and Lion's Eye Diamond are still unread.

Each of these was taught behind a `policy.*` weight (zero is the old way), and each is measured on its own as above:
mana rocks cast for the land they are (`rocks`), a Bauble or a Lantern for the card it holds (`stored`), loyalty
abilities weighed with their loyalty and the emblem (`loyalty`, `emblem`), attacks at planeswalkers and blocks for
its own (`walkers`), crew (`crew`), life for Phyrexian symbols down to a floor (`phyrexian`), the biggest permanent
for a search onto the battlefield (`fetch`), discard as the cards it takes (`discard`), The One Ring left alone at
low life (`burden`), never the last of its life for mana (`lastlife`), creatures cheated into play by Sneak Attack,
Through the Breach, Flash, Exhume, Animate Dead and Necromancy, and Entomb with one of those in hand (`cheat`), and a
counterspell kept for something worth more while the opponent holds cards (`counterbar`, in cards), Chrome Mox and Mox
Diamond cast while the mana is being built (`imprint`), a planeswalker in hand counted for its loyalty and first use
(`walkercast`), and graveyard hate used as that (`graveyard`): Soul-Guide Lantern or Ghost Vacuum exiling a creature
an opponent's spell is bringing back, kept rather than cashed in while their graveyard holds a creature of mana value
six or more, and aimed at their creature cards first. Mox Amber is mana only with a legend out.

## Its own account of a game

`TacticalPlay.Narrator`, off by default, is the seat telling each decision as it makes it: the turn, the step, the
life totals and its hand; what it was asked and what it answered; and what it weighed for it - every candidate at
priority with its gain and its cost, the plan for the turn, the attack's score against staying home and the
blockers it was made into, what a block leaves the attacker. A game lost to a strong player is only worth something
afterwards if the losing side's reasons can be read, and this is where they are:

```
--- Turn 15, mine ---
  | You draw [Counterspell].
  board me  : 5 life, 2 in hand, 26 in library, 5 lands (5 untapped) | Sensei's Divining Top, Sheoldred, the Apocalypse 4/5
  board them: 9 life, 4 in hand, 21 in library, 7 lands (4 untapped) | Quantum Riddler 4/6 (T), Tamiyo, Seasoned Scholar [3 loyalty]
  my hand: Time Spiral, Counterspell
T15 DeclareAttackers mine | life 5-9 hand 2 | attack: Sheoldred, the Apocalypse at Tamiyo, Seasoned Scholar
    attack scored -2.26 against -2.92 for staying home; their untapped blockers: Gollum, Riddle Master 3/1
  | You tap [Sheoldred, the Apocalypse].
...
T16 PrecombatMain theirs | life 5-7 hand 2 | priority: cast Counterspell -> Sheoldred, the Apocalypse
    Counterspell on Sheoldred, the Apocalypse: worth +1.88, at least the bar +0.30: a card and the margin
```

Between its decisions are the game's own log lines as the seat saw them (`|`, "You" being the seat), so what the
opponent did is there too; at the first decision of each turn, both boards and its hand; at a mulligan, the hand it
kept or threw back; a counterspell it held is told with why (under the bar, no mana, nothing it can target); a pass
weighed exactly as the last one that turn is told once; and the end of the game, from hosts that call
`TacticalPlay.NarrateEnd` when their loop stops (the lab does).

`cantrip-bot --tactical` (so `tools\play-game.cmd -Player2 Tactical`) writes it to a file in the temp folder and
says where; `--narrate PATH` names the file and `--narrate none` turns it off. `bot-duel.cmd watch --narrate FILE`
writes the `--a` seat's. The Discord bot's seats write a file each into `--bot-narration DIR` (or
`CANTRIP_BOT_NARRATION`), and the page's own bot seats (`?bots=`, `?bot-tickets=`) write to the browser's console,
each line prefixed with the seat's name, when the page is given `?narrate`. All are off by default, and a record that
cannot be written is given up, never the game. Telling it changes no answer.

## Where it runs

| | |
|---|---|
| `tools\play-vs-bot.cmd` | you against it in the web client. Your own tab plays its seat: `?bots=2` |
| `tools\play-game.cmd -Player2 Tactical` | the same, with every option; with nobody at the table it is `cantrip-bot --tactical` |
| `tools\draft-*.cmd` | the bots you draft against play their games as it does |
| a Discord queue's `bots:N` | the same, inside the Discord bot's process (`BotSeats`) |
| `?auto&bot=tactical` | the page plays its own seat with it |

**In the browser.** A page given `?bots=2` (or `?bots=2,3,4`, or tickets as `?bot-tickets=a,b`) opens a second
connection for each of those seats and answers them itself, one answer a tick (`MainPage.Bots.cs`). The server sees
a seat joined and answered and cannot tell where the answers were worked out; an opponent costs the machine of
whoever wanted one. The seats are remembered across a reload as the page's own is, and a dropped connection is
tried again as the page's own is. A seat the table moves elsewhere, as an event's are between rounds, is let go
rather than followed: one table, for now. A decision is well under a
millisecond natively, so even the interpreted Debug build keeps up.

What is not built yet: a host handing a table's bot tickets to one of its players. The Discord bot still runs its
queues' bots in its own process, where they now play as this bot does; `?bot-tickets=` is the half of that hand-over
that lives in the page.
