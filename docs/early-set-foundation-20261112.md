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
