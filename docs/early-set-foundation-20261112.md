# Foundation for blocked early-set cards

Base: upstream `6814db9c8c564e8c51c6f698c1f487189baf48f7`.
Branch: `engine/early-set-foundation-20261112`.

Keep each rules feature in a focused commit, with its native tests. Card recipes
and generator adoption follow the shared foundation rather than duplicating it
on separate card branches. Preserve the bounded quantity contract: power,
toughness, counters and life remain signed 32-bit values, damage and X unsigned
32-bit values. Do not restart the paused card campaign as part of this work.

The initial priority list comes from MTGish's November 11 coverage receipt.
Counts below are affected missing cards / cards with no other recorded blocker;
they are not cards already implemented or independently verified end to end.

| Feature | Affected / otherwise clear | Status |
| --- | ---: | --- |
| Blocking as though landwalk were absent | 9 / 9 | Native tests passed (13 landwalk tests) |
| Set only power or only toughness | 4 / 4 | Native tests passed (17 base-P/T tests) |
| World supertype and world rule | 12 / 8 | Native tests passed (8 world-rule tests) |
| Bands with other, including grants and removal | 8 / 8 | Queued |
| No combat damage assignment after an earlier choice | 8 / 8 | Native tests passed (21 assignment tests) |
| Other players activating abilities | 8 / 5 | Native tests passed (9 activation-player tests) |
| Step-bound effect and permission lifetimes | 6 / 5 | Native tests passed (6 step-deadline tests); generator adoption in progress |
| Computed limits on announced X | 5 / 3 | Queued |
| Persistent public visibility | 5 / 2 | Queued |

## Blocking exceptions for landwalk

`CardDef::landwalk_exceptions` contains callbacks returning the landwalks ignored
for a particular blocker/attacker pair (CR 509.1b, 609.4, 702.14). Great Wall and
Deadfall name a single landwalk; Staff of the Ages names every landwalk. Callbacks
receive the source's `Asker`, retaining component links and controller context,
and read type words through the source's current text. A named basic landwalk
reference includes its snow counterpart, consistently with other keyword
references.

The engine subtracts the exception only from its local landwalk check. It does
not remove a keyword, change the attacker's characteristics or bypass flying,
protection, another landwalk, or an independent blocking restriction. Exceptions
come from live battlefield sources with functioning abilities, through the same
source list used for ordinary blocking rules. Copies, ability loss, phasing and
control changes use that existing authority. There is no global card-table guard
and no per-pair battlefield scan just to discover exception sources.

Tests cover named and snow-qualified exceptions, another landwalk continuing to
restrict blocking, flying and ordinary blocking restrictions, copied sources,
text changes, ability loss, phasing, controller-dependent scope, and an actual
declaration offering the newly legal block.


## Independent base power and toughness

`EffectKind::SetPower(i32)` and `EffectKind::SetToughness(i32)` are layer-7b
settings (CR 613.4b). Island of Wak-Wak and Singing Tree set power; Sentinel and
Wall of Tombstones set toughness. The other axis is never copied at resolution:
a changing CDA, a later copy, another layer-7b setting, counters, bonuses and
switching power/toughness continue to apply in their own layers and timestamp
order. The existing signed 32-bit bounds and journalled effect storage are
unchanged. Tests include negative and extreme values, changed underlying P/T,
counters, a power/toughness switch, effect ordering and rollback/loop identity.


## No combat damage assignment

`EffectKind::NoCombatDamageAssignment` records a resolved rule about the exact
creature (CR 510.1a, 609.4). Gaze of Pain and Cloak of Confusion choose their
alternative in their earlier triggered ability. The damage step does not ask an
additional question. It excludes that creature from assignment and division
prompts, without removing it from combat, preventing damage, changing power, or
preventing its noncombat damage. Other combatants can still assign damage to it.
A first/double striker still creates a first-strike damage step even when it
assigns no damage; the rule applies in both damage steps and survives ability
loss for its duration. Native tests cover the attacking and blocking roles,
noncombat damage, double strike, and journalled rollback.


## World supertype and world rule

The oracle builder, native characteristics, host view and raw observation now
retain `world` (observation supertype bit 3; existing basic/legendary/snow bits are
unchanged). The host protocol already defines `SupertypeView::World`. Explicit
`Supertypes` initializers receive the required false field; those compatibility
edits do not change the affected cards' behavior.

`Obj::world_since` is the timestamp of its current uninterrupted World interval,
not a record of how often something happened. The core synchronizes it after a
whole instruction and after changes visible in event history. Simultaneous
acquisitions share one value. Copies that remain World, control, attachments and
other layer timestamp changes do not refresh it. Losing and regaining World
does; phasing preserves it while the permanent is treated as nonexistent.

The state-based action (CR 704.5k) keeps only the World with the newest interval,
across all controllers. A tie for newest puts every World into its owner's
graveyard, including older Worlds. It neither destroys nor sacrifices and asks
no choice. The normal state-based-action batch still applies other simultaneous
actions on that same position.

The clock uses journalled object updates. Loop identity includes its relative
order and ties, not absolute clock values. The presence of world-sensitive live
definitions is cached by state epoch; actual characteristics are recalculated
when event history changes, so a history-dependent type-layer ability remains
correct. Games without relevant live sources do not perform a characteristics
sweep. This cache is derived and reset when cloning or making a hypothetical
game, rather than serialized or used as a table-wide guard.

Tests cover successive Worlds under different controllers, simultaneous entry
and ties including an older World, indestructibility, copies and continuous
intervals, phasing, simultaneous type-layer changes, history-dependent type
changes and atomic event batches, and rollback/canonical loop identity.

## First-batch validation

The four feature commits through `1617f06d8` passed the complete Rust workspace
via `tools/test.sh --engine`: 22,556 tests passed and 15 were ignored, with
`CANTRIP_VERIFY=1`. The full test script was also attempted; it stopped before
tests because the .NET SDK is unavailable on this machine. No .NET pass is claimed.

The 128-game benchmark measured 45.24 microseconds per engine decision before
and 45.44 after the batch (about 0.4% on this shared machine), with reported
allocation unchanged at 26.74 KB per decision. A run during concurrent builds
measured 45.73 microseconds.

MTGish commit `6a9d523` adds capability-gated adapters without advancing its normal
upstream API pin or changing AST schema 190. Of 33 affected early-set card probes,
29 have no remaining recorded feature gap. All 33 modules type-check with a
names-only Cube-counter prerequisite overlay; no Cube declaration is part of
this engine branch. Twenty-five generated-card scenarios pass against the
pristine foundation, and the older 21-scenario corpus still passes against the
previous engine target. The generator suite passes 1,569 tests (22 ignored),
and the default target's Rust bodies and gap lists remain identical for all
33,529 corpus records. These are generator probes, not newly installed cards.

Reproducible probes and the detailed validation receipt are in MTGish's
`docs/cantrip-foundation-adoption-20261112.md` and accompanying proof JSON.
Local logs are under `/mnt/vizier/early-set-foundation-20261112`. The five queued
feature groups above remain separate follow-up commits.

## Different activators for an ability

`ActivatedDef::activators` and the builders `.activated_by(predicate)` /
`.any_player_may_activate()` express CR 602.2 permissions for one ability.
Armageddon Clock and Infinite Hourglass allow every player during upkeep; Clergy
of the Holy Nimbus permits only opponents; Personal Incarnation permits its
owner. Permission is evaluated as the source's current controller, with the
proposed activator as a separate argument. It replaces the default permission
and may exclude the source's controller. Conditions and the resulting stack
ability instead use the actual activator (CR 113.8, 602.2a).

Ordinary activation, mana activation and mana affordability use the same test.
Live source texts and the existing acquisition guard find opposing sources
without querying the characteristics of ordinary opposing permanents. Copies,
granted abilities, ability loss and phasing follow the existing layers. Other
players' hidden zones are not searched. Conditions, timing and once-per-turn
limits remain in force; the latter still belong to the ability instance, not
each possible activator. Permission is rechecked when executing an activation.

The activator pays from their own mana, life and hand; a source's own tap symbol
is authorized, with summoning sickness still read from the source's controller
(CR 302.6, 107.5). A source may pay explicit return/unattach costs, and an exert
cost skips the payer's next untap (CR 701.43a). Sacrifice still requires control
(CR 701.21a). An opposing artifact is never thereby an improvise/convoke payer.
Counter-placement prohibitions use the permanent's controller, independently
of the player paying the cost. Nine forced native tests cover these boundaries.

The roadmap's initial 8/5 count precedes the World foundation; together these
features remove the recorded gaps on six of these eight cards, including Land's
Edge. Merseine still requires its computed mana cost, and Personal Incarnation
its damage redirection. Card-level generation and runtime probes are separate
from these core fixtures.

## Five-feature validation receipt

Through `de790242f`, the complete native Rust workspace passes 22,565 tests,
with 15 ignored (`tools/test.sh --engine`, verification enabled). MTGish
`c6a9ad2` enables the new permission interface only for the candidate foundation
profile. Its suite passes 1,570 tests (22 ignored); 27 generated-card scenarios
pass against the pristine engine, including an opponent paying for Land's Edge
and Clergy of the Holy Nimbus. Forty affected modules type-check with an
explicit names-only Cube/Doom declaration overlay. That overlay is not part of
this branch; its sources/build cache were removed after archiving the patch.

Thirty-five of those 40 card candidates now have no recorded generator feature
gap. Caverns of Despair, Field of Dreams, Revelation, Merseine and Personal
Incarnation retain their other requirements. No card implementation was added,
no reservation changed, and the campaign remains paused. The four roadmap
groups still marked queued remain future focused commits.

The 128-game benchmark still reports 26.74 KB allocated per decision. Timings
are noisy on this shared machine: original baseline 45.24 microseconds per
engine decision, initial five-feature after-run 48.81 during builds, additional
before runs 44.12/46.67 and after runs 42.55/43.47. Additional runs overlapped
in their copy/presentation phases; these are not controlled evidence of a
speedup or regression. The release rebuild needed RUST_MIN_STACK=16777216,
matching native validation. The .NET SDK remains unavailable, so no .NET pass
is claimed.

MTGish's `docs/cantrip-foundation-activators-20261112.proof.json` records the
exact revision, candidate API fingerprint, card list, counter-only patch and
log hashes. It keeps the normal upstream API pin unchanged. Local evidence
is `/mnt/vizier/early-set-foundation-20261112`.

## Step deadlines and counted untap steps

`Duration::UntilNextStepOf` expires before a matching step's beginning triggers
(CR 500.4); `UntilEndOfNextStepOf` becomes `EndOfStep` at that beginning and
expires at its boundary, even if an effect ends the turn/combat early. A player
selector is fixed when the effect is created; `None` means the next such step
for anyone. Effects made during the step wait for its next beginning. Skipped
steps never trigger expiry (CR 500.11). Effects naming a departed player's next
step expire when that player's turn would have begun (CR 800.4m).

`Expiry::UntilYourNextStep` supplies the same start boundary for a play
permission. Erhnam Djinn, Gabriel Angelfire and Xenic Poltergeist use the next
upkeep; Elkin Bottle also expires a play permission then. Halfdane lasts through
its controller's next upkeep. Control is settled before the incoming step's
triggers inspect the battlefield. The untap boundary is handled without adding
an event or priority window to that step.

`DuringControllersNextUntapSteps(n)` is a remaining-duration state, not a count
of past events. Telekinesis uses two; Frost Breath uses one. Only actual untap
steps under the affected permanent's current controller consume a step, even
if it was already untapped. Independent effects advance together. Changes of
control and ability loss do not remove it; ordinary spell/ability untapping
still works. A zero-step duration never begins. Six native tests include
journal rollback, departed-player deadlines, early turn endings, permission
expiry and the beginning-trigger view. Workspace type-checking also passed.

## Explicit loss of unspent mana

`Game::empty_mana_pool(player) -> ManaPool` makes the named player lose all
unspent mana, as Mana Short, Power Sink and Drain Power instruct (CR 106.4,
609.1). This is not a step/phase boundary: Upwelling/Omnath-style boundary keep
rules do not apply. Normal and restricted pools, mana lasting through combat or
the turn, snow/source provenance and cached lasting amounts are all cleared
through the state's existing journalled emptying primitive. Another player's
pool is unaffected. Returned amounts name the types lost, within the native
per-type quantity bound, for a later instruction; no quantity type is widened.

Two native tests exercise the keep-rule distinction, mixed unrestricted and
restricted mana, all supported intrinsic lifetimes, snow/source metadata, a
second empty loss, journal rollback and producing fresh mana afterwards.
Drain Power's forced land-mana-activation procedure remains a separate gap;
this primitive does not tap lands or claim that procedure is implemented.
The generator's old coarse requirement label also covers mana doubling, which
is deliberately not enabled by detecting this loss-only method.

## Random discard costs

`Cost::discard_at_random` is an exact byte-sized count (Coral Helm, Draconian
Cylix, Stormbind). CR 601.2h requires every nonrandom cost to be paid before it;
the common cost scheduler marks it as random and offers no card-selection
prompt. The special-action path also pays random components last, including
separately declared components. Selection is uniform without replacement,
through the core's recorded/journalled random-choice API (CR 701.9b). Taking
the entire hand needs no random draw, zero does nothing, and an insufficient
hand cannot pay. Discarded objects retain ordinary typed cost receipts.

Availability counts the exact hand resources but does not pretend a random
discard can supply an earlier nonrandom graveyard-exile cost. Cost combination,
repetition bounds and tap-only classification include the new component.
Four focused tests pass: payment order/no card choice, selection without
replacement and random-state rollback, whole-hand/shortfall cases, the
nonrandom graveyard-cost dependency, and a separate special-action component.
The generator only enables constant unrestricted random discards under this
capability; variable-X random discards remain explicitly unsupported.

## Eight-feature batch validation

The complete Rust workspace through `4deb96071` passes 22,577 tests (15 ignored),
with verification enabled. MTGish's generator suite passes 1,573 tests (22
ignored), and its generated-card runtime probe passes 34 scenarios. Within
the frozen 286 unsupported early-set cards, 178 now have no recorded generator
feature gap and 108 remain gapped. This is not a claim that those cards are
installed or independently reviewed.

The 128-game benchmark still allocates 26.74 KB per engine decision. Sequential
control runs measured 49.88 microseconds before and 44.79 after; a previous
after-run during concurrent builds measured 77.01. Shared-machine variability
precludes a statistical speed claim. No .NET pass is claimed: the SDK remains
unavailable. MTGish's docs/cantrip-foundation-progress-20261112.json records
the per-card ledger, exact native revision, API fingerprint and log hashes.

## Typed keyword-family queries

`TriggeredDef::keyword_family` and `Query::has_keyword_family` identify rampage
and cumulative upkeep without parsing display text or comparing function
pointers (CR 702.23, 702.24). Their shared keyword factories supply the tag;
queries read the object's current layered abilities, including grants and
copies, so ability loss removes the family and a later granted instance
restores it. These are compact static tags, not new game-history counters or
permanent allocations. Ward/flanking's existing metadata is unchanged.

Rapid Fire asks whether its creature already has rampage; Balduvian Shaman
requires a target without cumulative upkeep. Two focused rules tests cover
labels that disagree with the typed tag, copies, grants and ability loss. An
Oracle-consistency test checks every currently implemented card's printed
rampage/cumulative-upkeep lines against its actual tags and passes. Production
classification never reads those lines. No existing card file needed edits.

## Simultaneous untap limits

`CardDef::untap_limits` holds typed player/filter/amount rules for Smoke,
Damping Field and Winter Orb (CR 502.3). The active player chooses one
inclusion-maximal set satisfying every current limit; this does not force the
largest-cardinality set. Smoke's ruling permits choosing a single animated land
instead of a separate creature and land. Duplicate limits do not add together.
The existing selection validator enforces the intersection for answers and
partial client selections. Unrestricted permanents untap without a question.

All choices precede the simultaneous untap. Tapped Winter Orb cannot turn on
halfway through that action. Ability loss disables a source; copied and merged
texts retain their normal links, using the existing text walk factored into
`CardTable::texts_of` so the query validator needs no allocated metadata copy.
An impossible untap cannot consume a slot; a stun replacement does. Optional
untaps are decided before the quota selection. Other players' simultaneous
untaps and effects outside the untap step retain their existing behavior.

Seven focused native tests and two generated runtime scenarios (the complete
runtime probe passes 40) cover those rules, rejected answers and journal undo.
No permanent data or historical counters are added. Games without a live limit
allocate no feature-specific list and do not compute extra characteristics for
this rule. The batched workspace gate and benchmark follow this feature commit.

## Ten-feature batch validation

The complete native Rust workspace through `d8d0d26b7` passes 22587 tests
(15 ignored), verification enabled. MTGish's complete generator suite passes
1,577 tests (22 ignored), and its generated runtime probe passes 40 scenarios.
Of the frozen 286 missing cards, 185 generate without feature gaps and 101
remain gapped. A direct native compile covers 183 of those modules; the other
two require only the missing Doom and Cube counter-name declarations. They are
not silently aliased to other counter kinds, and no overlay is used for the
183-module receipt. None of these cards is installed by this work.

A sequential 128-game benchmark measured 59.36 microseconds per engine decision
on the original base, 60.40 on this foundation; allocations remain 26.74 KB per
decision and both runs finished all games. The shared host is busy, so this is
not a statistical speed claim. The .NET SDK is unavailable; only the complete
native gate is claimed. The generator's old Earthlore expectation was updated
to expect its now-supported named tap cost before the successful full rerun.

## Named-counter delivery prerequisites

Append Cube (91) for Delif's Cube and Doom (92) for Armageddon Clock, preserving
all existing IDs 0–90. These are distinct named counters with no intrinsic
power/toughness meaning. The normal catalog walks provide host names, scenario
tokens and proliferate support; no card-specific rules are added to the core.
Thirteen vocabulary tests pass, including the dense-ID/name/token consistency
checks. This catalog uses the engine's existing dense counter storage, so two
additional i32 entries add eight bytes to an object; quantity widths do not
change. This is a data prerequisite, not an additional rules primitive. The
next batched benchmark includes this catalog growth.

## Ordinary observers of mana taps

`EventMask::TAPPED_FOR_MANA` exposes the existing event to ordinary triggered
abilities, including Manabarbs and Burning Earth (CR 106.12, 603.3). They queue
for the stack after a mana ability completes. Triggered mana abilities still
resolve immediately within the activation (CR 605.4a), and the ordinary trigger
collector excludes them even if they use the new mask. A production replaced
by zero triggers ordinary observers, but no additional-mana trigger.

An interest guard examines actual game objects, their possible copied/granted
texts, delayed abilities and functioning zones before collecting observers.
A game with none keeps the history-only path. This reuses the activation
observer guard's procedure with separate per-card metadata, rather than a
whole-table presence check or an allocation on every tap. Verification asserts
that the guard never misses a live observer. Seven focused tests pass, covering
immediate/stack timing, payment windows, zero production, irrelevant taps,
grants/loss, other zones and a table whose observers are absent from the game.
The generated runtime suite passes 41 scenarios, including actual Manabarbs
and Burning Earth with basic/nonbasic lands, artifacts and a different payer.

The generator adapter deliberately leaves mixed mana-producing triggers such
as Overabundance and Snowfall gated: they require correct immediate-mana
classification/advice, not an ordinary stack trigger. The previous complete
native gate is through the ten-feature batch; this feature's full batch and
performance receipt follow with the next group of changes.

## Attack-only summoning-sickness exceptions

`CardDef::attacks_as_though_haste` grants permission for one creature/defender
pair (CR 302.6, 508.1a, 609.4), as Instill Energy and Chaos Lord require. It
gives no haste keyword and no permission to pay tap/untap-symbol costs or to
enlist. Tapped status, controller, defender and independent attack bans still
apply. Losing an external permission's recipient's abilities leaves that rule;
losing the source's abilities removes it. Copied source texts participate.

Sources are collected lazily only when an otherwise eligible attacker is sick;
no collection is allocated in the ordinary path. Target-specific exceptions
feed the existing declaration validator, so invalid defender choices are
rejected by the core and its client-facing verdict. Six focused tests pass,
including three-player defender choices. The generated runtime suite passes
43 scenarios, including an actual Instill Energy on a sick mana creature and
Chaos Lord distinguishing a new entry from a control change this turn. Quantity
bounds and the AST schema remain unchanged. The next complete gate/benchmark
batches this feature with ordinary mana-tap observers and counter metadata.

The next generator-only recovery, Glyph of Delusion, also requires the named
Glyph counter, appended at 93. Existing IDs 0–92 remain unchanged. The original
source's false end-of-turn grant duration is corrected in MTGish; the native
prerequisite supplies only its name. The three appended entries now occupy
12 additional bytes in the existing dense counter array, before object padding.

## Additional shared rules and the fifteen-group receipt

The same branch now also supplies these capabilities:

| Group | Native contract and witnesses |
| --- | --- |
| Ordinary mana-tap observers | A separate `TAPPED_FOR_MANA` event interest queues Manabarbs/Burning Earth on the stack, while triggered mana abilities remain immediate. A live-state interest guard preserves the common path. |
| Attack-only summoning-sickness exceptions | `attacks_as_though_haste` permits Instill Energy/Chaos Lord to attack without granting haste, changing tap-symbol costs, or bypassing independent restrictions. |
| Skipped beginning-phase steps | `skip_steps` handles Stasis/Necropotence. The turn still begins when untap is skipped; step actions, triggers, priority and deadlines wait. The starting player's first draw step is entirely skipped. Other step kinds are rejected until their procedures support skipping. |
| Named players' hand sizes | `hand_size_rules` merges setters, adjustments and unlimited sizes with existing resolved/granted rules in timestamp order. Cursed Rack names its chosen opponent; Jin-Gitaxias names all opponents. Negative intermediate values are retained until the final cleanup limit. |
| Reselected linked values | `remember_chosen` replaces a permanent's chosen color or number while preserving the choosing text's link. Shapeshifter and Chromatic Armor reuse existing entry-choice carriers; copied/granted text stays separate. No new object fields or heap storage. |

Glyph of Delusion's runtime also exposed an existing availability bug: required
related target slots were tested without earlier target choices. The repair
reuses the bounded ordered announcement search (also needed by Goblin Welder),
including optional slot positions and conservative pre-X bounds. The source
reference's unrelated erroneous end-of-turn grant duration is repaired in
MTGish and reported as i5jb/mtgish#269. The native counter catalog appends Cube,
Doom and Glyph without changing old IDs; its dense storage grows by three i32
entries. Quantity contracts remain unchanged.

Native `cfac93547` passes the full engine workspace gate with verification:
22,616 passed, zero failed, 15 ignored, across 236 test targets. Four AI golden
files needed only their diagnostic answer indices reduced by two after the
first draw step stopped offering priority. Their encoded tensors are byte for
byte unchanged. No other old tests or golden payloads were modified.

MTGish's complete generator suite passes 1,584 tests (22 ignored). All 193
currently gap-free modules in the fixed 286-card early-set audit compile
against this foundation, and 48 generated runtime scenarios pass. The remaining
93 cards retain explicit gaps. These are generation candidates, not installed
or fully reviewed card implementations. The detailed per-card ledger lives in
MTGish's `docs/cantrip-foundation-progress-20261112.json`.

The 128-game sequential benchmark controls measured 49.44 us/decision and
26.74 KB allocated on the original base, versus 50.16 us and 27.04 KB on
`994d7b122` (the same production code as `cfac93547`). Both finished all games.
The corrected first draw step changes the random seats' game paths, so these
are whole-run observations rather than isolated API costs; shared-host timing
supports no statistical speed claim. The .NET SDK is unavailable, so no .NET
validation is claimed.

Evidence is retained under `/mnt/vizier/early-set-foundation-20261112`, including
`workspace-fifteen-tests-rerun.log`, `generator-fifteen-suite.log`,
`generator-player-values-native-runtime-final.log`, the 193-module compile log
and the sequential `benchmark-fifteen-*-control.log` files.

## Fixed nonmana attack costs

Flooded Woodlands, Reclamation and Leviathan share a declaration payment
procedure, exposed as `CardDef::attack_cost_rules: &[AttackPayment]`. Each
matching attack owes its own fixed `Cost`. The engine keeps each rule's source
and linked text, locks the total after attackers tap and optional costs are
announced, opens the mana window when the total includes mana, then uses the
existing cost scheduler to pay every component. An abandoned declaration rolls
back sacrifices, taps, mana payments and payment receipts together. A cost never
forces an otherwise required attacker to attack.

The scheduler now recognizes different overlapping removal choices, such as a
land and an Island, as order-sensitive. Enlist joins that scheduler when a fixed
attack cost is owed: a land creature can be tapped before it is sacrificed.
Old object identities cannot pay later taps, and a chosen attacker that left the
battlefield during payment never becomes attacking. Each combat payment and its
object receipts are recorded in history, including their imposing source and
attacker; they are distinct from spell and resolution receipts.

Eleven focused regressions and all 1,490 rules tests pass (one existing test
ignored). MTGish's generated runtime probe covers all three printed cards and
62 earlier scenarios. Its fixed audit now has 215 generation-ready candidates
and 71 gapped cards; these candidates are not installed card implementations.
The normal declaration advice still describes the mana part of attack costs;
nonmana valuation in client/AI advice is not supplied by this primitive. An
entry with zero mana still denotes a cost for requirement validation.

No per-object storage or quantity width changes. Locking scans live static
sources and chosen attacks, allocating no lists when there is no active tax.
Evidence and final receipts are kept in MTGish's
`docs/cantrip-foundation-nonmana-combat-20261112.md` and proof JSON. Complete
workspace test gates remain batched for the end of the shared-family milestone.
