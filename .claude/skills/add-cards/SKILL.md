---
name: add-cards
description: Add one or more Magic cards to the Cantrip rules engine - each card exact to its Oracle text and the Comprehensive Rules, built from the engine's shared rules verbs and keyword mechanics, with brief scenarios that execute every line of its code and show it observably correct, playable end to end in the web client - and, for many cards, run them through parallel agents and land their work. Use whenever a card is added, implemented or fixed, by anyone.
---

# Adding cards

This is the standard for every card in Cantrip, whoever writes it. A card that does not meet all of it is not added.
This file is what the author of a card reads. **Running many cards through agents (or `import-cards`) starts from
`coordinator.md`.** Beside it: `review.md` (reviewing a card), `client.md` (a card that
adds something the client has never shown or offered), `claims.md` (claims in full: keeping one alive, pull requests,
releasing) and `coordinator.md` (running many cards through agents and landing them). `docs/adding-cards.md` is the
reference for a card file's fields, `docs/engine-design.md` the architecture, CLAUDE.md the repository's rules, and the
`write-game-script` skill the scenario syntax.

## The non-negotiables

1. **The card is exactly what the rules say it is.** Its Oracle text, read with the Comprehensive Rules
   (`docs/rules/MagicCompRules *.txt`) and the card's official rulings, is the whole specification. No approximation,
   no simplification, no house rule, nothing right in the usual case and wrong in a rare one, no "the bots never do
   that" - in what it does, when, what it asks, whom, in what order, or what it lets a player do. If the engine cannot
   do what the card needs, the card waits: the missing rule is built in the core, properly and for every card, or the
   gap is reported. A card that is wrong is worse than a card that is missing.
2. **Rules belong to the engine, never to the card.** A card file holds only what that one card does alone, calling
   the engine's verbs for rules procedures and the shared mechanics for keywords and recipes.
3. **Every line of the card's code is executed by its scenarios**, every branch both ways, each showing the card
   observably doing what its text says. Measured (`tools\card-coverage.cmd`), not claimed. Scenarios are brief.
4. **An independent review against the Oracle text and the rules** before it lands (`review.md`).
5. **The card works end to end, in the client:** a person at a table in the web client sees everything it puts into
   the game, makes every choice it asks with wording true to what the choice does, and takes every action it offers
   with its cost shown. The language-model client and the draft bot, where it reads cards, answer its questions
   sensibly, never by a reading of a question meant for something else. The tactical bot
   (`dotnet/src/Cantrip.Client/Tactics`) is no part of card work: nothing reads,
   changes, runs, tests through or waits on it, and no review raises a finding about how it plays a card.

## Where code goes

Three layers, kept apart by the compiler (`cantrip-api` is all a card can see):

| Layer | Crate | What goes there |
| --- | --- | --- |
| Core | `cantrip-state`, `cantrip-rules`, exposed by `cantrip-api` (`Game`, `Query`) | Every procedure the Comprehensive Rules define: destroy, sacrifice, exile, return to hand, put into a library, mill, discard, damage and its prevention, counters, control, targeting legality, protection/hexproof/ward, the stack, priority, layers, replacement and prevention ordering, triggers, state-based actions, combat, mana, costs, zones and new-object identity (CR 400.7), last-known information, simultaneity (CR 101.4, 603.10). Keywords that change a core procedure (trample, first strike, split second). |
| Shared mechanics | `cantrip-mechanics` | **Every keyword action (CR 701) and keyword ability (CR 702), from the first card that has it**: one module named for the keyword, parameterised (`earthbend`, `evoke`, `storm`, `amass`, `flashback`, ...). Any other recipe two or more printed cards use exactly as written: target filters, trigger matchers, tutors, land templates, tokens, one-line effects, delayed end-step effects. |
| Card | `crates/cantrip-cards/src/cards/<card>.rs` | Only this card's own conditions, values, branching and sequencing, over `Game`, `Query` and the shared mechanics. |

**A card file must never:**

- filter on indestructible, regeneration, protection, hexproof, shroud or ward to decide what a rules action does -
  the verb does that (`destroy_all`, `destroy_target`, targeting);
- build a zone destination the rules dictate (`Zone::graveyard(owner)`, `Zone::hand(owner)`, a library position), or
  call `move_together` for a rules action that has a verb: use `destroy_all`/`destroy_all_no_regen`, `sacrifice`,
  `exile_all`, `return_to_hand`, `put_into_library(.., LibraryPlace::..)`, `mill`, `discard_together`, `discard_hand`,
  `library::search`/`library::find_to_*`;
- shuffle or order a library by hand (`put_into_library(.., LibraryPlace::BottomInRandomOrder)`,
  `bottom_in_random_order`): use `library::rest_to_bottom_in_random_order` or a shuffle. A pick "at random" from a list
  (a card from a hand, an opponent) is `library::random_of`;
- loop over players or objects for something the rules make simultaneous (`damage_together`,
  `each_opponent_loses_life`, `each_opponent_sacrifices`, `destroy_all`, `exile_all`) - a model built for one object
  is not copied for several;
- check whether a player has lost. "Each player" is `apnap()` (or `turn_order_from(p)` for "starting with you") and
  "each opponent" is `opponents(p)`; both leave out players who have left (CR 800.4a), targeting refuses them, and a
  resolving ability's controller is always in the game. `seats()` is every seat, for walking zones by owner, never
  "each player". Only a player named another way (Fecundity's "that creature's controller") is asked `in_game(p)`;
- name a color word, basic land type or creature type its rules text uses without reading it through the words of the
  object whose text it is (`g.words(item.object).subtype(Subtype::Goblin)`, `view.words(source).color(Color::Red)`),
  so a text-changing effect (CR 612) changes what the card does (`docs/adding-cards.md`, "Words of its rules text").
  Mana symbols are not words, and the type line and the tokens it makes follow by themselves;
- build a choice question by hand that `cantrip_mechanics::choose` asks, or write a fallback arm for an answer of the wrong
  shape: the core checks every answer first. A choice of cards or a library search is a **selection rule**, looked up
  first in `cantrip_mechanics::select` (`select::BASIC_LAND_CARD`, `select::basic_land_cards(up_to(2)).with(DIFFERENT_NAMES)`)
  and asked with `choose::select` or `library::search` / `library::find_to_*` (`docs/adding-cards.md`); a rule only this card
  prints is a `SelectFn` part in its file, never a closure or a list of eligible cards worked out first. The older
  `choose::{card, cards, card_from, cards_from, ..}`, `search_library` and `library::Find`/`search_to_*` are kept for
  existing cards, not for new ones;
- read a printed cost where the rules mean mana value (`Query::mana_value`: copies, tokens, face-down, X on the stack);
- implement a keyword, even one only this card has today;
- copy something `cantrip-mechanics` or `cantrip-api` already has (target filters and `targets::object`, trigger
  matchers, `effects`, `tokens`, `delayed`, `library`, `lands`, the selection rules of `select`), or define a token a shared
  definition covers;
- keep a counter or a flag for something that happened earlier: the past is asked of the event history,
  `q.history()` (a one-off query in the card; a question a second card asks moves to `cantrip_mechanics::history`);
- hand-write a stale-object guard (`targets::permanent`, `cantrip_api::is_live`), or guard `sacrifice`, which already
  skips objects that changed zones (CR 400.7);
- produce a different event than the rules name: making Clue tokens is not investigating; moving cards to the
  graveyard is not discarding (and skips madness); moving a card out of a library is not searching, and a "you may search" is asked by
  `library::ask_may_search`, never `ask_may` (a search a spell or ability can't cause is not offered, CR 101.2).

**Adding to the core or to the mechanics.** A core feature mirrors the rule it implements, cites the CR, works for
every card the rule describes, and is guarded on the game state - never on "does any card in the card table have X",
because the table holds every card. A core change is benchmarked (`cargo run --release -p cantrip-bench -- --games
128`, the "engine answering" line before and after, reported with it); a mechanics change is not. A non-keyword
mechanic names in its doc comment at least two printed cards that use it exactly as written; with only one it stays in
that card. A keyword module cites its CR rule and names its cards. Never widen a central switch to encode a card.

**Share what is the same, and only that.** If an existing function does what the card needs, use it. If another card
already does the same thing in its own code - the prior art shows it, you come across it, or `python
tools/maintenance/card-dupes.py "Card Name"` lists it (every function of the card's file whose body another card file
repeats, kinds and numbers masked; run it once the card is written) - move it into one shared function and change every
card that has a copy to call it, in this change: it is part of the card, never a note left for later. Never stretch a shared function over something that only looks alike: a flag or parameter only one
caller uses, or a branch on which card is calling, means it was not the same thing.

**A card never tells the engine what the rules already decide.** Where the rules fix a property of an ability or an
effect from what it is - whether a trigger looks back in time (CR 603.10a) follows from its trigger event - the engine
derives it; a card is never responsible for setting a flag for it, because a flag a card can forget is a rules
deviation waiting to happen. Card files stay as simple as the text they implement. If the engine needs such a flag from
a card to get the rules right, that is a core gap: report it (`blocked`), never set the flag and move on.

## Which card, and claiming it

Every card worked on is a GitHub issue, claimed before any work on it (`claims.md` has it in full):

- **Which cards:** the ones the request names; otherwise a format's list (`docs/formats/<format>.json`, best-played
  first) - ask which format unless told. `python tools/maintenance/card-issues.py next <format>` lists what is free.
  Take them in order and never skip one for being hard.
- **Claim one card:** `card-issues.py claim --format <format>` (or `claim "Card Name"`) prints `#N`; the commit ends
  with `Closes #N`. Exit status 3: someone else holds it or nothing is free - the message says which; never work on a
  card another holds live.
- **Log progress on the issue** with `card-issues.py log <N> <event>` - a fixed comment, nothing to write; its time is
  the measurement (`claims.md`, "The progress log"). The author logs `committed --commit <sha>` when the card is
  committed, `fixes-committed --commit <sha>` after review fixes, `blocked --rule <CR>` when the card waits on an
  engine change, and `unblocked` when work on it resumes.
- **A card that already exists is validated, not rebuilt.** If it already has a content file (here, on a local branch,
  or `claim` says "already implemented"), look for a minute or two: where its text needs code, the card file exists
  and plainly does what the text reads, and its JSON's text is the card's. If so, close the issue as already
  implemented with a comment and take the next card. A doubt about its rules is reported to the coordinator to be
  checked and fixed, not filed away; only code that is missing or plainly wrong sends it through the loop below.

## The loop for one card

1. **`python tools\maintenance\new-card.py "Card Name"` first.** For a card new to content it writes the content JSON
   from Scryfall's current characteristics (one face; a transform or modal double-faced card's `backFace`, with
   `backFacePlayableAsLand` for a modal one whose back is a land; an adventure; the Oracle keywords the core plays as
   `keywords`) - check it, and add any core keyword its text implies
   beyond Oracle's keywords (`CantBeBlocked`, ...); any other layout it prints, and you write
   `content/<dataset>/cards/<card-name>.json` by hand (characteristics only: `name`, `manaCost`, `type`,
   `power`/`toughness`, `loyalty`/`defense`, `oracleText`, core `keywords` from `crates/cantrip-oracle/build.rs`,
   `colorIndicator`, `convoke`/`delve`/`improvise`, faces `backFace`/`adventure`/`omen`/`halves`/`aftermath`). It
   then prints the owner's design for the card if one is left in `docs/card-notes/<card_name>.md` (it is followed,
   and the file deleted in the card's commit), whether the Oracle text is still Scryfall's (fix a "DIFFERS" in the
   JSON first), the official rulings, the CR rule of each keyword and keyword action (the number and where the engine does it, or the whole rule when
   nothing does yet), the scenarios that hold the card, and its prior art; and writes
   `crates/cantrip-cards/src/cards/<card_name>.rs` over the generated constant. A card whose text is only core
   keywords needs no Rust file.
2. **Read the rules before any code.** For each sentence of the Oracle text, the rule that governs it:
   `python tools\maintenance\cr.py 702.21 601.2c "scry"` prints a rule and its subrules by number or a keyword by
   name - cite what it prints, never a number from memory; `scryfall.py "Card Name"` prints the text and rulings
   again. Then what the rules make the card do in the odd cases: no legal target, a target turned illegal, a source
   gone, a copy, a token, face down, an opponent controlling it, two of it, X of zero, an empty library or hand,
   simultaneous events, a replacement effect in the way.
3. **Prior art before writing.** For each ability the prior art lists the implemented cards that print it or its parts
   (cost, trigger, effect), simplest first, with the mechanics and verbs each uses and how its text differs. Where a
   line has a match, read that card's file, build the line the same way, change what the "differs" note names, and
   check every difference ("you"/"each opponent", "may", "another", "you control", one object or several are
   different code). Search by hand only for lines marked `new`: `crates/cantrip-api/src/game.rs` (the verbs) and
   `crates/cantrip-mechanics/src` (the keyword modules, `targets`, `triggers`, `library`, `lands`, `effects`,
   `tokens`, `delayed`, `copy`, `hand`, `chosen_type`, `history`, `combat`, `replacements`), starting from the nearest
   abilities listed. `ability-index.py --text "..."` looks up any text. Most cards need nothing new. Whether a
   wording is printed on a second card - the test for sharing it - is asked of the whole printed pool, not only the
   cards implemented: `python tools/maintenance/scryfall.py --text "regex"` lists every printed card whose Oracle text
   matches.
4. **Write the card.** Behaviour by struct update over the generated constant (`CardDef { spell: Some(..),
   ..oracle::LIGHTNING_BOLT }`). Ask questions with `g.ask(..)` in the order the rules ask them, offering options in
   the order the engine and the implemented cards do. Doc comments quote the Oracle sentence each function implements
   and cite the CR. A replacement that does another thing "instead" leaves its event at nothing and does the other
   thing in its `then` (`ReplacementThen`; Hullbreacher, Soul-Scar Mage) - that is the design, not a workaround.
   Where a resolution makes more than one change with no event (effects beginning, copy effects, control), say between
   each two whether they are one instruction (`instruction(g, |g| ..)`: "gets +2/+2 and gains trample", "each creature
   you control ...") or an instruction ends there (`g.end_sentence()`, at a sentence's end or a "then"; CR 608.2c,
   603.8): `CANTRIP_VERIFY` refuses a resolution that says neither. If
   the engine cannot express the text as the rules have it, and the code would need a stand-in that only behaves
   alike (a value faked to get an effect), stop: that is a core gap -
   keep the claim, log `blocked --rule <CR>` and report the gap, as the non-negotiables say.
5. **Write the scenarios** (below; the `write-game-script` skill) and run them until they pass:
   `cargo run --profile cards -p cantrip-script --bin corpus -- --card "Card Name"`. `new-card.py`'s **test points**
   list what its Oracle text makes a scenario pin down (an amount, a limit, each player's side, a time, "instead", a
   value read as it last existed, things done at once); every one needs an expectation that fails if the card gets it
   wrong. `python tools\maintenance\new-card.py "Card Name" --scenario <what-it-shows>` starts a scenario file: the
   decks, the card cast off free mana, the pool checked, a TODO expectation that fails until you write the real ones.
   Then **break the card**, once for each test point and each thing its own code decides (a number, a limit, a
   condition, whose objects, each narrowing - a card type, a zone, a time): change it, see a scenario go red, restore
   it (`git add` the card file first, and `git checkout -- <file>` restores it). `python
   tools\maintenance\card-mutate.py "Card Name"` makes the mechanical breaks for you - each number one off, each
   comparison and `&&`/`||` flipped, each `!` dropped, each call to the game left out - one build and run each, and
   lists the ones that SURVIVED; the breaks of meaning it cannot make (another filter, the other player's objects, an
   earlier turn) are still yours. A break no scenario catches is a missing expectation, or a change that means the
   same (say which). The report lists each test point with the scenario line that catches it, and what you broke.

   **The breaks of meaning are made by hand and run, every one.** card-mutate's list is not the work: a card that is
   only data (a static `CardDef`, a cost, a filter, an equip) gives it nothing to mutate and still owes every break
   below. For each restriction the Oracle text names, make the wrong card and run its scenarios:
   - a cost's **colour or kind**: `{B}` as `{1}` or `{W}`, an extra `{1}` on a way, the ways' costs swapped;
   - a **filter widened** to the next broader one (`CREATURE_OR_PLANESWALKER` as `PERMANENT`, creature cards as any
     card) and narrowed (the planeswalker half dropped, nonland cards only);
   - **whose and which objects**: the other player's, every player's, all your creatures instead of the equipped or
     targeted one;
   - a **duration**: "until end of turn" or "this turn" made to last forever, "your next end step" as "the next end
     step" (cast on an opponent's turn to tell them apart);
   - **the event**: damage as loss of life, discarding as moving to the graveyard, things done together as one at a
     time (a token that sees the other enter tells them apart), a value read as the ability resolves as one read
     when it triggers.
   An expectation that passes for the wrong reason catches nothing: a refusal for want of mana or for too few
   targets, a count the opening hand already meets, a check on a different object than the one the effect touched.
   A break you reasoned about but did not run is not reported as caught; the report says which line went red.
6. **Citations:** `python tools/maintenance/cr.py check --show <the card's file> <its scenarios>` prints every `CR`
   citation beside the rule it names. Read each pair: a rule that does not say what the comment claims is fixed or
   the citation dropped - a number copied from another card's comment is the usual way a wrong one gets in.
   **Lint:** `python tools/maintenance/card-lint.py "Card Name"` checks the "never" list's mechanical items over the
   card's file. A NEVER finding is fixed; a LOOK finding is read, and either fixed or answered in the report in a line.
7. **Coverage:** `python tools/maintenance/card-coverage.py "Card Name"` reports every line of the card's file
   executed (from a bash shell call it so, not through `tools\card-coverage.cmd`, whose quoting `cmd //c` breaks; a
   file of statics alone has no lines to measure). A line no scenario can reach is wrong code or a
   missing scenario, never an exception. A lone target's guard is `let Some(x) = targets::object(..) else { return Ok(()) };`:
   wrapped in `if let`, its closing brace is a line no scenario runs, since a spell whose only target is illegal never resolves.
8. **The client.** Everything the card puts into the game must be shown, every question it asks captioned truly, and
   every action it offers clickable with its cost. A card that only uses prompts, actions and state the client already
   has needs no client change and no browser test. One that adds a prompt, an action, a cost display or a piece of
   state the client never had makes the client change in the same work and proves it in the browser: `client.md`.
9. **Verify.** (Working for a coordinator, the core - `cantrip-state`, `cantrip-rules`, `cantrip-api` - is closed to
   you: a card that needs a change there is a gap, `blocked --rule <CR>` and a report, never a change you make.)
   Working for a coordinator: run the scenarios of your card and of every card whose file or shared
   mechanic you changed (`corpus -- --card "Name"` each) and the unit tests of a crate you changed - nothing wider -
   and the build shows no warning from your files;
   the coordinator's full run at landing is the gate. Working alone: `cargo run --profile cards -p cantrip-script
   --bin corpus` shows `0 fail`, `tools\test.cmd /engine` is green, `tools\test.cmd` if the core, host, protocol or
   client changed, and the benchmark if the core did.
10. **Review, then commit.** `review.md` - by someone who did not write it. Then `Card: <Name>` - the content JSON, the
   card file and its scenarios - one card per commit, ending `Closes #N`; core, mechanics and client changes in their
   own commits before it (`Engine: ...`, `Mechanics: ...`, `Client: ...`), each building on its own - the new card's file
   goes in the Card commit, since `oracle::` has its constant only once the content JSON is there, then `card-issues.py log <N> committed
   --commit <sha>`. A card agent working for a coordinator does not push; otherwise commit and push as CLAUDE.md says, or open a pull request with the coverage report and the
   review in its description, saying `Closes #N`.

   **Before every call for review** - the first one, and again after each round of fixes - run your card's scenarios
   on the commit as it stands (`cargo run --profile cards -p cantrip-script --bin corpus -- --card "Card Name"`, plus
   `--card` for each card whose file or shared mechanic you changed) and see them all pass. Run only those, never the
   whole corpus. Put the result line (`N pass, 0 fail`) in your report; a report without it is not ready for review.

## Scenarios: what a card owes

A scenario (`scenarios/<what-it-proves>.cantrip`; syntax in the `write-game-script` skill and `scenarios/README.md`) is
a real game: both decklists in order, every answer, and the state it must end in. Positions are forced by the script.
**Never sweep RNG seeds**, and never rely on a bot's choices to reach a position.

Keep them brief: a few short scenarios that run all of the card's code and show it observably correct. A card's
scenarios, together:

- **execute every line and take every branch both ways**: each `if` true and false, each `match` arm, each early
  return, mode, way to cast, cost, ability and face (coverage checks the lines; the branches are on you);
- **show what the text does**, with a refusal (`! Bot1.Hand.Card.Cast`, `! Bot1.Choose(..)`) beside the allowed case
  for each limit the card's own code enforces ("only", "up to", "another", a mana value or type) - an object exactly at
  each numeric limit, both players' objects for whatever "each", "target" or "you control" reads, the same thing on an
  earlier turn for "this turn", and for a cost the pool checked right after the cast, in the same step (`write-game-script`, step 7). A limit the
  engine or a shared mechanic enforces (targeting, timing, equip) is tested there already;
- **test a corner case only where the card's own code could plausibly get an interaction wrong**: it reads a value
  that can change before use, holds last-known information, or combines with another rule in its own code; a card
  complex enough to need very specific testing gets it. Corner cases the engine handles for every card (a fizzled
  target, a source gone, cleanup) are not retested;
- **verify the rules**: each ends with an `[Expect]` of the state the Oracle text, rulings and CR require - **never
  copied from what the engine produced** - at least one of which fails if the card is wrong (the loop's step 5 breaks
  it to see); `?` checks where the position matters before the end;
- **assert precisely**: the keys that prove the behaviour and no others.

Name each scenario for what it proves (`wrath-of-god-destroys-a-creature-regenerated-in-response`) and start it with a
`#` comment: what it shows, why it is arranged so, and the rule. If the harness cannot express something a scenario
needs, extend the harness (`crates/cantrip-script`) and its README; never weaken the scenario to fit.

**A scenario that shows a bug is committed, not dropped.** If the engine or another card gets something wrong that
this change cannot fix, commit the scenario asserting the rules-correct outcome with `# Known bug: <what, and the
rule>` at the top (`scenarios/README.md`, "Known bugs"), and an entry in `docs/known-issues.md`, and report it to the
coordinator: a rules deviation is fixed next, never left waiting (`CLAUDE.md`, "No rules deviations"); the fix removes
both. Never delete, weaken or rewrite a scenario to pass because the engine is wrong.
