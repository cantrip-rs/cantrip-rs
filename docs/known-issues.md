# Known issues

Everything known to be wrong, suspected to be wrong, or unproven in the engine, the client and the harness, in one
place. A scenario that shows a bug is still committed with a `# Known bug:` line (`scenarios/README.md`, "Known bugs");
this file is where that bug is described, ranked and followed. Add an entry when a bug, a suspicion or an unexercised
path is found and not fixed in the same change; when it is fixed, delete the entry in the fixing commit (the history
keeps it).

## Severity

| Severity | Meaning |
| --- | --- |
| Critical | A crash, a hang, a desync between engine and client, or a game that cannot go on. |
| High | A wrong game result in ordinary play: a rule misapplied on a common card or interaction. |
| Medium | A wrong result in a rare interaction, or a choice the client cannot offer or show. |
| Low | Cosmetic, a harness or tooling gap, a suspected bug not yet confirmed, or a path no scenario runs. |

## Entry template

```

### KI-<n>: <one-line description of what is wrong>

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open / Suspected / Unexercised | engine / mechanics / card / client / harness | YYYY-MM-DD, how |

**Notes.** What happens, what the rules require (CR numbers, rulings), where in the code, and what a fix needs.

**Scenarios.**
- [`scenarios/<name>.cantrip`](../scenarios/<name>.cantrip) - what it shows (marked `# Known bug`), or "none yet" and
  which implemented card could reach it.
```

## Summary

| ID | Issue | Severity | Status |
| --- | --- | --- | --- |
| [KI-1](#ki-1-suspend-ignores-a-spell-limit-already-reached) | Suspend ignores a spell limit already reached | Low | Suspected |
| [KI-2](#ki-2-a-harmonize-cost-reduction-cannot-be-checked-in-the-browser) | A harmonize cost reduction cannot be checked in the browser | Low | Open |
| [KI-3](#ki-3-engine-paths-no-scenario-runs) | Engine paths no scenario runs | Low | Unexercised |
| [KI-4](#ki-4-scenarios-waiting-on-cards) | Scenarios waiting on cards the engine does not have | Low | Open |
| [KI-5](#ki-5-a-spell-that-has-left-the-stack-is-judged-by-its-printed-keywords) | A spell that has left the stack is judged by its printed characteristics | Low | Suspected |
| [KI-24](#ki-24-under-bolass-citadel-the-top-of-a-library-is-cast-only-for-life) | Under Bolas's Citadel the top of a library is cast only for life | Medium | Open |
| [KI-25](#ki-25-a-spacecrafts-power-and-toughness-box-is-not-drawn-before-it-is-a-creature) | A Spacecraft's power and toughness box is not drawn before it is a creature | Low | Open |
| [KI-26](#ki-26-an-exile-that-a-replacement-sends-elsewhere-is-not-exercised-by-hex-magic-fireglass-mentor-or-esper-origins) | An exile that a replacement sends elsewhere is not exercised by Hex Magic, Fireglass Mentor or Esper Origins | Low | Unexercised |
| [KI-28](#ki-28-noncombat-damage-plus-n-reads-the-controller-of-a-damage-source-that-is-not-on-the-battlefield-or-the-stack) | Noncombat damage "plus N" reads the controller of a damage source that is not on the battlefield or the stack | Low | Suspected |
| [KI-30](#ki-30-a-permanents-name-is-not-part-of-its-last-known-information) | A permanent's name is not part of its last-known information | Low | Open |
| [KI-31](#ki-31-an-adventure-is-never-cast-from-the-top-of-a-library) | An Adventure is never cast from the top of a library | Medium | Open |
| [KI-33](#ki-33-chain-lightnings-payer-copies-without-being-asked-whether-to) | Chain Lightning's payer copies without being asked whether to | Low | Open |
| [KI-34](#ki-34-a-skip-effects-source-that-has-left-may-go-unnamed-in-the-turn-would-begin-prompt) | A skip effect's source that has left may go unnamed in the "Your turn would begin" prompt | Low | Open |
| [KI-35](#ki-35-the-tactical-bot-counts-no-summoning-sick-creature-as-a-mana-source-under-shang-chi) | The tactical bot counts no summoning-sick creature as a mana source under Shang-Chi | Low | Open |
| [KI-37](#ki-37-a-sideboard-card-the-engine-does-not-have-is-left-out-of-the-game) | A sideboard card the engine does not have is left out of the game | Low | Open |
| [KI-38](#ki-38-an-activated-ability-whose-cost-removes-counters-from-among-permanents-is-unexercised) | An activated ability whose cost removes counters from among permanents is unexercised | Low | Unexercised |
| [KI-39](#ki-39-a-state-that-is-true-only-in-the-middle-of-a-resolution-with-no-event-misses-its-state-trigger) | A state that is true only in the middle of a resolution, with no event, misses its state trigger | Low | Unexercised |
| [KI-10](#ki-10-a-lone-action-whose-cost-depends-on-its-target-shows-only-the-printed-cost-until-the-payment) | A lone action whose cost depends on its target shows only the printed cost until the payment | Low | Open |
| [KI-23](#ki-23-a-creature-fighting-itself-cr-70114c-is-implemented-but-no-scenario-can-reach-it) | A creature fighting itself (CR 701.14c) is implemented but no scenario can reach it | Low | Unexercised |
| [KI-40](#ki-40-a-role-that-could-not-enchant-its-creature-and-a-for-as-long-as-duration-false-from-the-start-are-not-exercised) | A Role that could not enchant its creature, and a "for as long as" duration false from the start, are not exercised | Low | Unexercised |
| [KI-41](#ki-41-a-dependency-through-another-object-is-judged-on-the-object-being-worked-out) | A dependency through another object is judged on the object being worked out | Low | Open |
| [KI-42](#ki-42-a-permissions-enters-tapped-is-judged-from-where-the-land-is-not-from-the-way-it-was-played) | A permission's "enters tapped" is judged from where the land is, not from the way it was played | Low | Unexercised |
| [KI-43](#ki-43-a-modal-double-faced-cards-back-face-cast-by-a-permission-is-not-exercised) | A modal double-faced card's back face cast by a permission is not exercised | Low | Unexercised |
| [KI-44](#ki-44-a-hand-way-is-not-offered-to-a-cast-made-while-an-effect-resolves-and-no-emblem-gives-a-way) | A hand way is not offered to a cast made while an effect resolves, and no emblem gives a way | Low | Open |
| [KI-45](#ki-45-the-number-a-cost-sacrifices-is-kept-as-the-spells-x) | The number a cost sacrifices is kept as the spell's X | Low | Suspected |
| [KI-46](#ki-46-the-lore-counter-a-player-puts-on-a-saga-as-their-main-phase-begins-has-no-player-putting-it) | The lore counter a player puts on a Saga as their main phase begins has no player putting it | Medium | Open |
| [KI-48](#ki-48-emperor-of-bones-puts-its-finality-counter-on-after-the-creature-enters) | Emperor of Bones puts its finality counter on after the creature enters | Low | Unexercised |
| [KI-49](#ki-49-the-tactical-bot-does-not-know-infect-or-poison) | The tactical bot does not know infect or poison | Low | Open |
| [KI-50](#ki-50-an-animation-that-replaces-creature-types-is-unexercised) | An animation that replaces creature types is unexercised | Low | Unexercised |
| [KI-51](#ki-51-a-land-whose-subtype-is-set-keeps-the-copiable-values-other-than-abilities-a-copy-effect-gave-it) | A land whose subtype is set keeps the copiable values other than abilities a copy effect gave it | Low | Exercised; reading unconfirmed |
| [KI-53](#ki-53-a-copy-holds-at-most-four-abilities-added-by-exceptions) | A copy holds at most four abilities added by exceptions | Low | Unexercised |
| [KI-54](#ki-54-a-loop-a-player-could-stop-is-shortcut-after-a-fixed-number-of-rounds-not-one-the-players-name) | A loop a player could stop is shortcut after a fixed number of rounds, not one the players name | Low | Open |
| [KI-55](#ki-55-spellskite-cant-choose-between-two-instances-of-target-that-hold-the-same-object) | Spellskite can't choose between two instances of "target" that hold the same object | Low | Unexercised |
| [KI-56](#ki-56-a-counter-ban-judges-an-entering-permanent-as-the-object-it-was-before-it-moved) | A counter ban judges an entering permanent as the object it was before it moved | Low | Unexercised |
| [KI-60](#ki-60-infects--1-1-counters-are-counters-an-effect-puts) | Infect's -1/-1 counters are counters an effect puts | Low | Suspected |
| [KI-61](#ki-61-limits-of-judging-a-permanent-as-it-would-exist-and-a-land-as-layer-4-leaves-it) | Limits of judging a permanent as it would exist, and a land as layer 4 leaves it | Low | Unexercised |
| [KI-62](#ki-62-a-pool-holds-at-most-255-mana-of-one-kind) | A pool holds at most 255 mana of one kind | Low | Open |
| [KI-63](#ki-63-control-from-a-static-ability-or-a-lapsed-condition-is-settled-only-as-state-based-actions-are-checked) | Control from a static ability, or a lapsed condition, is settled only as state-based actions are checked | Low | Unexercised |
| [KI-64](#ki-64-no-card-in-content-lets-one-control-changing-effect-decide-another) | No card in content lets one control-changing effect decide another | Low | Unexercised |
| [KI-65](#ki-65-a-prepared-permanent-that-phases-out-loses-its-preparation-copy-for-good) | A prepared permanent that phases out loses its preparation copy for good | Low | Unexercised |
| [KI-66](#ki-66-phasing-in-a-multiplayer-game-after-a-player-leaves-is-unbuilt) | Phasing in a multiplayer game after a player leaves is unbuilt | Low | Unexercised |
| [KI-67](#ki-67-a-spell-cast-by-a-resolving-ability-under-split-second-is-unexercised) | A spell cast by a resolving ability under split second is unexercised | Low | Unexercised |
| [KI-68](#ki-68-the-advice-applies-a-total-costs-least-before-a-casts-own-reductions-and-a-permissions-additions) | The advice applies a total cost's least before a cast's own reductions and a permission's additions | Low | Open |
| [KI-69](#ki-69-a-bestowed-spell-has-the-mana-value-of-its-bestow-cost) | A bestowed spell has the mana value of its bestow cost | Low | Open |
| [KI-70](#ki-70-a-block-requirement-is-counted-on-the-declared-block-only-not-through-the-blocked-creatures-band) | A block requirement is counted on the declared block only, not through the blocked creature's band | Low | Open |
| [KI-71](#ki-71-the-tactical-bots-combat-planning-ignores-bands) | The tactical bot's combat planning ignores bands | Low | Open |
| [KI-72](#ki-72-a-face-down-permanent-may-be-unable-to-turn-face-up-under-humility) | A face-down permanent may be unable to turn face up under Humility | Low | Suspected |
| [KI-73](#ki-73-older-you-may-draw-cards-offer-a-draw-a-cant-draw-rule-forbids) | Older "you may draw" cards offer a draw a can't-draw rule forbids | Low | Open |
| [KI-74](#ki-74-a-question-a-replacement-asks-mid-run-is-not-watched-for-loops) | A question a replacement asks mid-run is not watched for loops | Low | Suspected |
| [KI-75](#ki-75-words-of-winds-return-a-permanent-question-may-be-captioned-as-a-cost) | Words of Wind's "return a permanent" question may be captioned as a cost | Low | Suspected |
| [KI-76](#ki-76-a-searchs-world-keeps-what-the-real-game-did-with-hidden-cards-it-cannot-resample) | A search's world keeps what the real game did with hidden cards it cannot resample | Low | Open |
| [KI-77](#ki-77-a-sacrifice-cost-with-nothing-left-to-sacrifice-is-asked-not-taken-back) | A sacrifice cost with nothing left to sacrifice is asked, not taken back | Low | Open |

## Issues

### KI-1: Suspend ignores a spell limit already reached

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-16, review of the cast-limit fix (`d9509b633`) |

**Notes.** Suspend may be used "if you could begin to cast this card by putting it onto the stack from your hand"
(CR 702.62a). The suspend action's `Timing::AsCast` gate in `crates/cantrip-rules/src/actions.rs` checks
`static_rules::may_cast`, but not the spell limit of Archon of Emeria ("each player can't cast more than one spell each
turn", CR 101.2) or the card's own "cast this spell only" condition, which casting checks through `may_begin_cast` and
`cast_only_if_met`. So a player who has cast a spell this turn under Archon may still suspend Lotus Bloom. Confirm the
reading against the suspend rulings first; if it holds, the gate calls `may_begin_cast` like every cast.

**Scenarios.** None yet. Lotus Bloom (suspend 3, {0}) and Archon of Emeria are both implemented and can reach it,
with a positive control that suspends it as the turn's first action.

### KI-2: A harmonize cost reduction cannot be checked in the browser

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | harness | 2026-10-15, Wild Ride's browser scenario |

**Notes.** When the mana pool already covers a spell's cost, no payment prompt appears, so a browser scenario has
nothing on the page to read the reduced total from. The engine side is proven by the headless scenarios; the client
shows the harmonize cost on the graveyard pile's note. A fix is a `Client.` key for the cost a pending cast will pay,
or a scenario arranged so the payment prompt appears.

**Scenarios.**
- [`scenarios/wild-ride-in-the-browser-offers-harmonize-from-the-graveyard-and-the-creature-to-tap.cantrip`](../scenarios/wild-ride-in-the-browser-offers-harmonize-from-the-graveyard-and-the-creature-to-tap.cantrip) - the browser
  scenario that plays it, without the reduced total.

### KI-3: Engine paths no scenario runs

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-15/16, fixes in the U-B-R-G Prowess run and the known-bug fixes |

**Notes.** Code written for every card the rule describes, where no implemented card reaches it yet. Each is a
candidate bug until a scenario runs it; when a card that reaches one is implemented, its scenarios cover the path and
the line goes.

| Path | Covered instead by | Why no scenario |
| --- | --- | --- |
| Retargeting a copy (`ops::retarget`): "up to" slots of a modal spell, each-opponent slots, spliced text (CR 702.47c) and kicked-spell targets | nothing | No modal spell in content has an "up to" mode target, no copyable spell has an each-opponent target, and no scenario copies a spliced or kicked spell. |
| A granted static ability that affects other objects taken by a later "loses all abilities" (`layers::granted_to_others`: the `LoseAllAbilities` clear and the `has.abilities` filter) | The Irencrag's own "loses all other abilities" before its grants | No implemented card makes a noncreature artifact lose all abilities while it stays an Equipment (Oko's +1 makes Everflame an Elk creature, which unattaches it). |
| A granted cast of a back face (`GRANTED_FACE_BACK`, `faces_to_cast`) | nothing | No implemented double-faced card has a castable back face. |
| A flash grant judged for an MDFC back face or a bestowed Aura through `actions::Speed::face`, beyond Sigarda's Aid's bestow | nothing | No implemented double-faced card has a castable noncreature back face; no flash grant names creatures. |
| The transformed cast's `cast_only_if_met` check (`could_cast_transformed_during_resolution`) | nothing | Only Berserk has a "cast only" condition, and it has no back face. |
| A "loses all abilities" effect older than an ability granted later (CR 613.1f, 613.7) | Rust test | No implemented card does both. |
| Gift of an extra turn; a copied permanent spell keeping its gift; "whenever you give a gift" | Rust tests, `crates/cantrip-script/tests/gift.rs` | Perch Protection needs phasing; no implemented card copies a gift permanent or has the trigger. |
| Lost abilities at the flash-rules, instant-activation, damage-by-toughness and ascend sites | nothing | No implemented card loses its abilities while using one. |
| The late-targets / divide and MDFC back-face sites of the minimum-targets offering check (CR 601.2c) | nothing | No implemented card reaches them. |
| The "isn't a mana ability" guards on activation triggers (`triggers::opponent_activates_ability_of_artifact_creature_or_land`, Burning-Tree Shaman's matcher) | nothing | A mana ability's activation is only recorded, never emitted (`EventMask::of`), so no trigger sees one; the guards stay for when it is emitted for the cards that trigger on mana abilities too (Elrond, Moon-Reader, Ceaseless Searblades). |
| A rule that stops abilities triggering (`trigger_rules`) on a permanent entering at the same moment as other permanents: "If Torpor Orb and a creature enter at the same time" (Torpor Orb, Doorkeeper Thrull, Hushbringer, Tocatli Honor Guard, Hushwing Gryff rulings) | Tocatli Honor Guard's own entering stopping Enduring Innocence (the newcomer is on the board the rule is read from); Hushbringer dying beside Worldspine Wurm | No implemented effect puts a chosen suppressor onto the battlefield together with another permanent (several creatures put onto the battlefield by one effect still enter one at a time, `engine-design.md`). |
| The end of an "until" exile (`DelayedWhen::UntilEvent`, Palace Jailer) is never stopped by `trigger_rules` (CR 610.3: the return is a one-shot effect, not a triggered ability) | nothing | The only "until" event is becoming the monarch, which no rule stops. |
| More than `MAX_TARGETS` (16) targets for one spell or ability: a stack item holds 16, so a target question's count is cut to the room left (`choose_slots`, `choose_trigger_targets`), where the rules set no limit - a deviation, in place of a panic. Reachable by Miasma Demon (17+ cards discarded and 17+ creatures), Pest Infestation or Batroc with X or kicks of 17+ | nothing | No scenario holds that many cards and targets. |
| Floodpits Drowner targeting itself (`shuffle_away` shuffles one object, `objects.dedup()`) | nothing | Unreachable in today's pool only by accident: its ability needs it untapped ({T}) and carrying a stun counter, but every stun source (its own trigger, Kaito, Baloth Prime, Cryogen Relic) leaves the permanent tapped, and an untap with a stun counter removes the counter instead (CR 122.1g). A card that puts a stun counter on an untapped permanent would reach it. |
| A creature's own "leaves the battlefield" ability under Hushbringer, not triggering when it dies (the rulings) | earthbend's "When this land dies or is put into exile" (stopped as it dies, not as it is exiled); Daretti's emblem | No implemented creature has a leaves-the-battlefield ability of its own. |
| Dedicated Commons entering at the same time as a planeswalker: that planeswalker isn't considered (ruling; `enters` reads the battlefield as it is, so it is not counted) | nothing | No implemented effect puts a land and a planeswalker onto the battlefield together (several permanents put onto the battlefield by one effect still enter one at a time, `engine-design.md`). |
| Torch the Tower's replacement not made when its damage is prevented (`g.damage(..) > 0` false: a permanent "dealt damage by" the spell only if damage was dealt) | nothing | No implemented effect prevents damage and leaves the target legal: protection from red gained in response makes the target illegal, so the spell does not resolve (a different path), and Impractical Joke and Stomp stop prevention rather than cause it. A damage-prevention card (a Fog, a Circle of Protection) would reach it. |
| A face-down permanent turned face up by an effect (`Game::turn_face_up` with the effect's controller as the player recorded in `Event::TurnedFaceUp`), and a manifested instant or sorcery card that would turn face up, revealed and left face down (`ops::turn_face_up`, CR 701.40g, 701.58g) | the special action (Oblivious Bookworm's and Abhorrent Oculus's scenarios), whose own condition refuses a noncreature card | No implemented card turns a permanent face up as an effect. |
| Ward's life prompt for an amount above 60 (`triggers::ward_life_question`: the static `Question`s name the amount for 0 to 60 only, so a bigger one is asked as "Pay the life", without the number; the payment itself is exact) | Raubahn, Bull of Ala Mhigo's ward at 4 | No implemented creature reaches a power above 60. A `Question` with a runtime amount would remove the table. |

| A copy of a spell paid for with mana that made it uncounterable or gives it haste (Cavern of Souls, Delighted Halfling, Arena of Glory) is counterable and gains nothing (`ops::copy_spell` clears `cant_be_countered` and `gains_haste`: no mana was spent on the copy, CR 707.10, 106.6) | nothing | No implemented card copies a creature spell or a legendary spell, the only spells that mana can pay for. |
| Discover dropping a face whose mana value is over N although the card's is not (`discover::discover`'s `ways` filter; CR 701.57a, "if the resulting spell's mana value is less than or equal to N") | Adventurers and split cards with both faces at or under N (Quintorius Kand's scenarios) | No implemented card has an Adventure or castable back face costing more than a front face of 4 or less. |
| A Case's "To solve" resolving for a Case that became solved while it waited (`solved::become_solved` through `Game::solve`, which leaves a solved permanent as it is; CR 603.4's "this Case is not solved" asked again) | nothing | Only a Case's own "To solve" solves it, once per end step: no implemented card copies a triggered ability or solves a Case another way. |
| A Case that loses all its abilities: it stays solved (CR 719.3b) and its "Solved" ability is gone, a play rule under `PlayRule::While` with it (`each_play_rule` skips a permanent that lost its abilities) | nothing | No implemented card takes the abilities of an enchantment (Oko, The Wondrous Wasp and Tishana's Tidebinder reach only artifacts, creatures and planeswalkers). |
| A "Solved" static ability of the layers (`solved::solved_in_layers`, CR 702.169b) | nothing | No implemented Case has one: Case of the Locked Hothouse's is a play rule. |
| The host showing a player the top card of their library while a permanent says they may look at it any time (`PlayRule::LookAtLibraryTop`, `observable`), whether or not it can be played | nothing | No `Client.` key reads a library pile, and the browser driver plays nothing from a library. Bolas's Citadel, Mystic Forge and a solved Case of the Locked Hothouse have it. |
| Damage redirected (CR 614.1a) to a planeswalker or battle, or away from one: `ops::deal_damage_with` judges loyalty and defense against the recipient the replacement effects leave, and a player's protection from everything is ordered with a redirection (`replace::Origin::Protection`, CR 616.1) | Ancient Adamantoise's scenarios (creatures and players) | Ancient Adamantoise only redirects to itself, a creature, and no implemented card gives its controller protection from everything while it is out; a planeswalker of Bot1's hit while it is out would reach the first half. |
| Planetarium of Wan Shi Tong that left the battlefield and came back the same turn after a card was cast with it: "do this only once each turn" is looked for among the controller's casts with the ability of any Planetarium (CR 603.2h: "its source's controller has not yet taken the indicated action"), so the new object (CR 400.7) does not trigger again; keyed on the object it would | nothing | No implemented card blinks or returns an artifact to the battlefield in a turn the Planetarium has already cast with. |
| Damage prevention ordered against damage replacement (CR 616.1: when replacement and prevention effects both apply to one damage event, the affected player or the affected object's controller chooses which applies first). `ops::deal_damage_with` always runs the replacement chain first (`replace::damage`: Artist's Talent's and Torbran-style "deals that much plus 2 instead") and protection's prevention (CR 702.16e) after it; the player is never asked. For replacements that only change the amount, the order cannot change the result - protection prevents all of that source's damage, whatever the amount. It can for a replacement that redirects the damage to another player or permanent, or an "instead" that changes what is dealt to whom: applied first, protection would have prevented the damage before any redirection, so the player's choice decides it. No content card alters `d.target` (or the source) of a damage proposal today. Damage of 0 never reaches the chain, which is CR 120.8 ("replacement effects that would increase the damage ... have no event to replace"), so Artist's Talent's `d.amount > 0` guard is reached only if an earlier replacement in the chain reduces the damage to 0 (CR 615.6) | nothing | No implemented card prevents a fixed amount of damage, redirects damage by a replacement, or replaces damage down to 0. |
| Beyond the Quiet's "Spacecraft" half (`chars.has_subtype(Subtype::Spacecraft)`): exiling a Spacecraft that is not a creature, which the Oracle text requires and the code does | The creature half of the same filter (a crewed Vehicle exiled, an uncrewed one left) | No Spacecraft card exists in content yet, so no scenario can put one on the battlefield. |
| A static "can attack as though it didn't have defender" of a permanent that has lost all its abilities (`static_rules::may_attack_despite_defender` skips it, CR 613.1f), or of one permanent about another creature | Demon Wall's own rule, asked of itself and refusing Bot2's defender (`demon-wall-lets-only-itself-attack-and-an-opponents-wall-attacks-on-its-own-turn`) | Demon Wall is the only card with the rule and it is its own source: losing its abilities loses its defender too. Rolling Stones, High Alert or Ghalta, the Immovable would reach it. |
| A static ability whose condition reads the history (`LayerQuery::history`) judged while the triggers of the event that would satisfy it are collected: `Cx::emit` collects triggers before it records the event (so a trigger sees the history as it was just before it), so a triggered ability that such a static grants ("as long as you've surveilled this turn, this creature has '...'", Darkblade Agent) is not yet there for the very event that turns the condition on. CR 603.2 looks for triggers once the event has happened; whether the granted ability should see that first event is unconfirmed | nothing | Essence Channeler and Shardmage's Rescue grant only keywords; no implemented card grants a triggered ability on a history condition. |
| "For as long as it has a [kind] counter on it" (`Duration::WhileHasCounter`): a counter put back after the last came off, without a new effect, does not restart it; an effect begun on an object without the counter never applies; the ending through a cost that removes counters, a stun counter replacing an untap, damage removing loyalty, and a control effect ending this way (`ops::end_counter_durations`, `refresh_control`) | Rust tests, `crates/cantrip-script/tests/counter_duration.rs` (the first two); Eluge, the Shoreless Sea's Vampire Hexmage scenario (removal through `add_counters`) | Every printed card that puts a flood counter makes its own effect (Eluge, Aquitect's Will, Quicksilver Fountain, Xolatoyac), and Eluge is the only implemented card with the duration: no flood counter comes off any other way, and no card gives control this way (Shield Broker is not implemented). |
| Eluge, the Shoreless Sea's power and toughness while a player other than its owner controls it (the characteristic-defining ability counts its controller's Islands on the battlefield and the stack, its owner's elsewhere), and its cost reduction's "land" for a flooded permanent that has stopped being a land | nothing | No implemented card gains control of a creature, and nothing in content makes a land stop being a land while it keeps a flood counter. |
| Ba Sing Se entering at the same time as basic lands: those lands are not counted for "enters tapped unless you control a basic land" (ruling 2025-10-02; `lands::enters_tapped_unless_you_control_basic_land` reads the battlefield as it is) | Abandoned Air Temple-style scenarios with a basic land already in play | No implemented effect puts a land and a basic land onto the battlefield together (several permanents put onto the battlefield by one effect still enter one at a time, `engine-design.md`). |
| Demonic Pact under another controller: the modes it chose before count whoever chose them (ruling 2015-06-22; `history::mode_not_chosen_before` is keyed on the Pact object, not the player) | Two Pacts choosing independently (`demonic-pact-two-pacts-each-have-all-four-modes-to-choose`) | No scenario steals a Demonic Pact between upkeeps. |
| Demonic Pact with only modes that need a target left and no legal target for them (ruling 2015-06-22: the ability is removed from the stack with no effect, CR 603.3c, 700.2) | Gollum, Riddle Master with every mode spent (`gollum-spends-all-three-modes-and-then-riddles-no-more`: the same removal, for lack of modes) | Its targeted modes take any target and target opponent, which are always legal while an opponent is in the game, so no position lacks a target. |
| A spell no longer allowed once its proposal is complete (CR 601.6: `actions::cast` asks `CardDef::spell_rules` again of the spell on the stack, its X announced, and takes the cast back): a fused split spell's combined mana value, or a back face cast transformed, whose mana value is its front face's (CR 202.3b) where the card it is judged as before the cast reads the back face alone | the check before the cast (every Gaddock Teeg scenario) | No implemented spell's qualities that a rule judges change between the offer and the proposal: Gaddock Teeg's {X} clause already forbids every noncreature spell with {X}, no implemented card has fuse, and the one implemented Siege's back face (Lightshield Array) is mana value 2. |

**Scenarios.** None, by definition.

### KI-4: Scenarios waiting on cards

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | cards | `corpus` reports them as "need cards" |

**Notes.** These scenarios name a card no file defines, so they cannot run. Each tests an interaction already
written into the engine that is unproven until the card exists: Diabolic Edict, Quest for Ancient Secrets, Doomed
Traveler, Innocent Blood.

**Scenarios.**
- [`scenarios/quest-for-ancient-secrets-sees-a-spell-that-paused-for-its-opponents-choice-land-in-the-graveyard.cantrip`](../scenarios/quest-for-ancient-secrets-sees-a-spell-that-paused-for-its-opponents-choice-land-in-the-graveyard.cantrip)
- [`scenarios/quest-for-ancient-secrets-counts-both-an-innocent-blood-sacrifice-and-the-spell-itself.cantrip`](../scenarios/quest-for-ancient-secrets-counts-both-an-innocent-blood-sacrifice-and-the-spell-itself.cantrip)
- [`scenarios/dying-pest-tokens-hand-over-their-trigger-order.cantrip`](../scenarios/dying-pest-tokens-hand-over-their-trigger-order.cantrip)

### KI-5: A spell that has left the stack is judged by its printed keywords

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-16, review of Momo, Friendly Flier |

**Notes.** `layers::chars_before` computes `spell` from the object's zone being the stack, and applies continuous effects only to
a spell or a permanent on the battlefield. An earlier cast, read through `q.chars(c.spell)` after the spell resolved, was countered or
left the stack (its old object is a card row in another zone or `Gone`), therefore has its printed characteristics, not the ones it
had as it was cast (CR 601.2i, 400.7). Momo, Friendly Flier asks "was an earlier spell this turn a non-Lemur creature spell with
flying" this way: a creature spell given flying (or stripped of it) while it was on the stack is misjudged afterwards. The history's
`Cast` record keeps types, colors and mana value but not subtypes or keywords; recording them there, or keeping last-known
information for a spell that leaves the stack, fixes it. No implemented card grants or removes a keyword on a spell on the stack, so
no scenario can reach it yet.

**Scenarios.** None yet; the first card that gives a creature spell flying on the stack (then a second flier cast under Momo) reaches it.

### KI-25: A Spacecraft's power and toughness box is not drawn before it is a creature

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-16, station (Wurmwall Sweeper) |

**Notes.** A station card's [P/T] box belongs to its striation (CR 721.2b): the permanent has that base power and
toughness only while it has N or more charge counters, and a station card has none in any other zone (CR 721.2c). So its
content file prints no `power`/`toughness`, and the striation's static gives them (`station::base_pt`). The page draws the
2/2 once the Sweeper is a creature, but a player looking at it in hand, or on the battlefield below the threshold, reads only
"It's an artifact creature at 4+" and "4+ | Flying" - not what size it will be. A fix carries the box to the client as
presentation (a `stationPt` beside the Oracle text, drawn in the striation), never as a characteristic.

**Scenarios.**
- [`scenarios/wurmwall-sweeper-is-a-flying-artifact-creature-only-while-it-has-four-charge-counters.cantrip`](../scenarios/wurmwall-sweeper-is-a-flying-artifact-creature-only-while-it-has-four-charge-counters.cantrip) - the engine side (no power below four, 2/2 at four).

### KI-10: A lone action whose cost depends on its target shows only the printed cost until the payment

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-16, review of target-aware costs |

**Notes.** The total cost of a spell or ability is determined once its target is announced (CR 601.2c, 601.2f, 602.2b).
The host words what an activation or a cast costs with each candidate into the action's description ("Equip {2} (costs
{0} targeting Dwarven Mauler, {2} targeting Ornithopter)", `prompt.rs` `costs_by_target`), which the card menu shows when a
card has two or more actions. A card with a single action (Dragonfire Blade's equip, Ajani's Response in hand) goes
straight to its targets when clicked, with no menu row: the targeting caption ("Target for Dragonfire Blade") does not say
what each target would cost, and the real total first shows at the payment ("{1} of {1} still to pay"), where the player
may still cancel. Targets the cost could not be paid for are not offered. A fix shows the description in the targeting
caption, or the cost on each candidate as it is pointed at.

**Scenarios.**
- [`scenarios/dragonfire-blade-equip-shows-its-reduced-cost-and-the-granted-hexproof-in-the-browser.cantrip`](../scenarios/dragonfire-blade-equip-shows-its-reduced-cost-and-the-granted-hexproof-in-the-browser.cantrip) - the menu row of a two-action card (Shadowspear) and the payment of a one-action card (the Blade).

### KI-23: A creature fighting itself (CR 701.14c) is implemented but no scenario can reach it

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, review of Bushwhack |

**Notes.** `ops::fight` makes a creature that fights itself deal damage to itself equal to twice its power (CR 701.14c), and
the two damages of an ordinary fight are one simultaneous `damage_together`. No card in the engine can name one creature for
both fighters (Bushwhack's targets are a creature you control and a creature you don't; Mawloc and Wolverine say "other"), so
only the simultaneous damage is covered, by the fight scenarios. The day a card can fight itself it needs a scenario, and this
entry is deleted.

**Scenarios.** none yet.

### KI-24: Under Bolas's Citadel the top of a library is cast only for life

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Medium | Open | engine | 2026-10-16, review of the Cases |

**Notes.** A player who controls Bolas's Citadel (`PlayRule::LibraryTop`) and also Mystic Forge or a solved Case of the Locked
Hothouse (`PlayRule::LibraryTopSpells`) may cast the top card either way: for life by the Citadel, or for its mana cost by the
other permission (CR 601.3, 118.9; the Citadel's life payment is only for a spell cast "this way"). `castable` in
`crates/cantrip-rules/src/actions.rs` offers the mana-cost cast only when no permanent gives `LibraryTop`, and the cast decides
life or mana from `land_rules(..).top_for_life` alone, so only the life cast is offered. A fix offers both, each as its own way,
and has the cast pay what that way says.

**Scenarios.** None yet: Bolas's Citadel with Mystic Forge, or with a solved Case of the Locked Hothouse, and a creature on top.

### KI-26: An exile that a replacement sends elsewhere is not exercised by Hex Magic, Fireglass Mentor or Esper Origins

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | card | 2026-10-16, review of Hex Magic |

**Notes.** Hex Magic exiles the whole hand, then draws "that many" cards and lets the player play "cards exiled this way"; Fireglass
Mentor exiles two cards and lets the player choose one "of them". Both keep only the cards that really reached exile
(`g.obj(card).zone.is(ZoneKind::Exile)`), as a replacement effect could send one elsewhere (CR 614.6, 406.2), and draw or grant
play for those alone. No card in the engine replaces an exile from a hand or a library today, so the false arm of that guard
(hex_magic.rs, fireglass_mentor.rs) runs in no scenario. Esper Origins likewise exiles itself as it resolves and puts onto the
battlefield transformed only a card that reached exile ("exile it, then put it onto the battlefield transformed", CR 608.2c,
712.14a); the false arm of that guard (esper_origins.rs, `effect`) is the one line of the card no scenario runs, and no card
replaces an exile from the stack. The day such a card exists each card needs a scenario, and this entry is deleted.

**Scenarios.** None yet.

### KI-28: Noncombat damage "plus N" reads the controller of a damage source that is not on the battlefield or the stack

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | mechanics | 2026-10-16, review of Tomik, Izzet Sparkmage |

**Notes.** `replacements::noncombat_damage_you_deal_to_an_opponent_or_their_permanent` (Tomik, Izzet Sparkmage; Artist's Talent
at level 3) asks `q.controller(d.source)` for "a source you control". A damage source that is a card outside the battlefield and the
stack has no controller (CR 108.4a); for an ability of such a card the controller is the player who controls the ability
(CR 110.2, 113.8), and for the damage of a spell it is the spell's controller. `q.controller` of such a card answers its owner, so
damage dealt by an ability a card has in a graveyard or exile (an activated ability that works from there, a Cycling-style
trigger) is judged by its owner rather than by the player who activated or controls the ability. No card in the engine deals
noncombat damage from such an ability while an opponent controls Tomik or a level 3 Artist's Talent, so nothing shows it. A fix
reads the controller of the stack item that is dealing the damage (the `DamageEvent` would have to carry it) and keeps
`q.controller` for permanents.

**Scenarios.** none yet.

### KI-30: A permanent's name is not part of its last-known information

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-16, review of The End |

**Notes.** `LastKnown` (`crates/cantrip-state/src/object.rs`) records types, subtypes, colors, power, toughness, keywords,
abilities and words but no name, so "the same name as that permanent" asked after it has left the battlefield (CR 608.2h) cannot
use last-known information: a card, copy effect, face-down or transformed state is gone with the old object (CR 400.7). Cards
read `library::object_names` before moving the object (The End, Deadly Cover-Up), which is right for them; a card that must ask
after the move needs the name kept in `LastKnown`.
The same holds for a name an effect set (`EffectKind::SetName`, CR 612.8): The Irencrag that left as Everflame, Heroes'
Legacy reads as The Irencrag afterwards.

**Scenarios.** None yet.

### KI-31: An Adventure is never cast from the top of a library

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Medium | Open | engine | 2026-10-16, review of the land-with-an-Adventure change |

**Notes.** A card may be cast as its Adventure wherever the card itself may be cast (CR 715.3), the top of a library
included: Bolas's Citadel could cast Stomp or Mage Siege off the top for life, and Mystic Forge an Adventure that is an
artifact or colorless spell. `actions::castable`'s two library-top passes (`PlayRule::LibraryTopSpells`, and
`top_for_life`) offer only the card itself, never `inset_faces` (also noted in `docs/engine-design.md`, "Spells in an
inset frame"). Traveling Chocobo's ruling (Lindblum on top: the land may be played, the Adventure not cast) holds today
only because no face is offered; a fix judges each inset face with the filter, as `Speed::face` does for flash.

**Scenarios.**
- [`scenarios/traveling-chocobo-plays-lindblum-off-the-top-but-not-its-adventure.cantrip`](../scenarios/traveling-chocobo-plays-lindblum-off-the-top-but-not-its-adventure.cantrip) - the case that must stay refused; none yet for one that must be offered (Bolas's Citadel with Bonecrusher Giant on top).

### KI-33: Chain Lightning's payer copies without being asked whether to

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | card | 2026-10-16, review of the payment prompt (`Game::may_pay`) |

**Notes.** Chain Lightning: "that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this
spell and may choose a new target for that copy." Paying is one choice and copying a second (CR 118.12, 707.10c), but
`chain_lightning.rs` copies as soon as the payment is made. Only a player who pays {R}{R} and then would rather not copy is
affected. A fix asks a "may copy" (`effects::ask_may`) after a paid `may_pay`, and a scenario pays and declines the copy.

**Scenarios.** None yet.

### KI-34: A skip effect's source that has left may go unnamed in the "Your turn would begin" prompt

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-16, review of Ral Zarek, Guest Lecturer |

**Notes.** When an effect that makes a player skip a turn (`EffectKind::SkipNextTurn`) and a "you may skip that turn" (Time
Vault) both apply, the player chooses which applies (CR 616.1, `turn.rs::skip_turn`), and the prompt names each effect by its
source. The skip effect's source is usually gone by then (Ral Zarek's −7 from 7 loyalty puts Ral into the graveyard,
CR 704.5i), and `prompt.rs::source_handle` names a gone object only through a handle the seat's view already has. A seat
that never saw the source (a reconnect with a fresh handle space) gets no handle, and its button reads "An effect: you
skip this turn". The choice is still right; only the name is missing. A fix carries the source's name with the prompt.

**Scenarios.**
- [`scenarios/ral-zarek-guest-lecturer-skipped-turn-and-time-vault-in-the-browser.cantrip`](../scenarios/ral-zarek-guest-lecturer-skipped-turn-and-time-vault-in-the-browser.cantrip) -
  the named case, the page having seen Ral; the unnamed one needs a reconnect, which no scenario does.

### KI-35: The tactical bot counts no summoning-sick creature as a mana source under Shang-Chi

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-16, review of Shang-Chi, Master of Kung Fu |

**Notes.** Under Shang-Chi, Master of Kung Fu ("You may activate abilities of creatures you control as though those creatures
had haste", CR 302.6; `CardDef::activate_as_though_haste`) the engine offers the {T} mana abilities of creatures that came
this turn, Shang-Chi's own among them, but the tactical bot's mana planning
(`dotnet/src/Cantrip.Client/Tactics/TacticalPlay.Priority.cs`, the source counts that skip `SummoningSick` creatures) leaves
them out, so it plans with less mana than it has. The view has no word for the permission; a fix counts a sick creature as
a source when the actions offered for it include a mana ability, or the view says its {T} abilities may be activated.

**Scenarios.** none: a bot's plan is not scenario'd. Shang-Chi's scenarios show the engine side.

### KI-37: A sideboard card the engine does not have is left out of the game

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-16, review of cards outside the game |

**Notes.** A seat's sideboard is the cards it owns outside the game (CR 400.11a), which North Wind Avatar chooses among. The
host refuses a deck naming a card the engine has not ported, but leaves such a card out of a sideboard instead
(`Session::new`, with a line on stderr), so that a constructed decklist whose sideboard names one (`decks/standard/jeskai-lessons.txt`'s
Riverchurn Monument, `mono-black-combo.txt`'s Rottenmouth Viper) is still played. In such a game a card that brings one in from
outside the game cannot choose it. Limited pools are the cube's ported cards, so an event's games are whole. A fix ports the
card, or has the deck loader refuse the list.

**Scenarios.** `cantrip-host`'s `a_sideboard_card_the_engine_does_not_have_is_left_out`.

### KI-38: An activated ability whose cost removes counters from among permanents is unexercised

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, building Dawnhand Dissident's casts |

**Notes.** `Cost::remove_counters_among` ("remove N counters from among creatures you control", CR 118.3) is chosen and paid by
`actions::activate` as well as by `actions::cast`, but only a cast pays it in content (Dawnhand Dissident's permission). Tayam,
Luminous Enigma's "{3}, Remove three counters from among creatures you control" is the printed activated ability it is built for;
the activation path chooses the counters after the mana, as blight's, and removes them with the rest of the cost. When a card with
it is added, its scenarios exercise this path.

**Scenarios.** None yet: no card in content has the cost on an activated ability.

### KI-39: A state that is true only in the middle of a resolution, with no event, misses its state trigger

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, review of the state triggers |

**Notes.** CR 603.8: a state trigger "triggers as soon as the game state matches the condition". The engine asks a state
trigger's condition after every event (`settle::consider`, inside `collect_batch`) and each time a player would receive
priority (`settle::state_triggers`). A state that becomes true and false again within one resolution through changes that
record no event - a continuous effect starting and ending, a characteristic changing in the layers - is never seen, so the
ability doesn't trigger where the rules say it has. No card in content reaches it: Mazemind Tome's state changes only by
counters, which are events. A fix asks the conditions after every change to the game state, or makes every such change
an event.

**Scenarios.** None can be written with the cards in content.

### KI-40: A Role that could not enchant its creature, and a "for as long as" duration false from the start, are not exercised

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, review of Royal Treatment and Eluge, the Shoreless Sea |

**Notes.** `create_tokens_onto` in `crates/cantrip-rules/src/ops.rs` does not create an Aura token "attached to" a
permanent it could not legally enchant (CR 303.4i), and attaches an Equipment token only to what it could equip (CR
301.5e); `add_effect` in `crates/cantrip-rules/src/lib.rs` refuses a `Duration::WhileCounter`/`WhileTapped` effect whose
condition is already false (CR 611.2b). No card in content gives protection from enchantments or keeps a counter from
being put on a land, and none creates an Equipment token attached to something, so those arms run in no scenario. The day
such a card exists each needs a scenario (Royal Treatment on a creature with protection from enchantments; Eluge's flood
counter on a land that can't have counters put on it), and this entry is deleted.

**Scenarios.** None yet.

### KI-41: A dependency through another object is judged on the object being worked out

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | core (layers) | 2026-10-16, the dependency system (`layers::by_dependency`, CR 613.8) |

**Notes.** CR 613.8a asks whether applying one effect would change what another applies to, which is a set of objects.
The layers work out one object at a time, so `layers::ready` asks it of that object: whether applying the other changes
whether the first applies to it, now or once the other's own prerequisites have applied (which covers a chain such as
"creatures are artifacts" after "lands are creatures"). An effect that depends on another only through some other object
- the other changes the first's reach on that object and not on this one - is applied to this one in timestamp order. The
result differs only where the two do not commute on this object (one sets what the other adds), and no two cards the
engine has meet that way: every multi-object static ability it has adds types, abilities or power, and the resolved
effects that set types (Oko's Elk) name their object. A fix asks the question over every object the first could affect,
at the cost of their characteristics part-way through the layer.

**Scenarios.** None: no pair of implemented cards reaches it.

### KI-42: A permission's "enters tapped" is judged from where the land is, not from the way it was played

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, review of Lightstall Inquisitor |

**Notes.** "Each land played this way enters tapped" (CR 305.1, 614.1c; `PlayGrant::enters_tapped`) is the core's replacement
`replace::Origin::PlayedTapped`, which works out from the land's zone and the permissions on it (`actions::land_play_permission`)
whether the land play was made by the permission: the action does not carry the way it was played. Two paths would get it wrong,
and no printed card in the engine reaches either: a land with such a permission played by a resolving effect instead
(`actions::play_land_during_resolution`, which would enter tapped though it was not "played this way"), and a land in a graveyard
with such a permission played by a way a permanent grants (`granted_slot`), which `played_by_permission_from` does not know of.
Of several ways that could play the same land, the engine also takes the one that lets it enter untapped rather than asking. A fix
carries the permission used on the PlayLand move into the entry's `ZoneChange`.

**Scenarios.** None yet: no card in the engine both plays a land by an effect and could face a Lightstall Inquisitor's card.

### KI-43: A modal double-faced card's back face cast by a permission is not exercised

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-16, review of Inside Information |

**Notes.** A permission to cast a card from where it is (CR 601.3) now lets a modal double-faced card be cast as its back face
(CR 712.11b) and a split card as either half (CR 709.3, an aftermath half only from a graveyard), for what the permission says
to pay (`actions::castable`, `face_cast`). The split halves are proven (Inside Information with Life // Death); no card in
content is a modal double-faced card whose back face is a spell rather than a land, so the back-face cast from exile runs in no
scenario. (Two transforming cards, Jennifer Walters and Norman Osborn, give their back faces a mana cost in content, which makes
`back.has_cost` true for them; whether that offers a back-face cast it should not is worth a look.) A scenario is owed when a
spell-backed modal double-faced card is added.

**Scenarios.** None yet.

### KI-44: A hand way is not offered to a cast made while an effect resolves, and no emblem gives a way

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-16, review of Omniscience |

**Notes.** Omniscience's "you may cast spells from your hand without paying their mana costs" is a `GrantedWay` from the hand
(CR 118.9), offered with priority (`actions::castable`). A cast an effect makes as it resolves from its controller's hand
(CR 608.2g) is offered only for the card's mana cost and faces (`cast_ways_during_resolution`), so Omniscience's free cast is
not among its ways there, though the rules allow any alternative cost. Ways are found on the battlefield's permanents only
(`granted_ways_of`), so an emblem that gives one - Tamiyo, Field Researcher's "You may cast spells from your hand without paying
their mana costs" (CR 114.4) - could not be built. No card in content casts from a hand as it resolves under such a way.

**Scenarios.** none yet.

### KI-45: The number a cost sacrifices is kept as the spell's X

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-16, review of Rottenmouth Viper |

**Notes.** `VariableCost::SacrificeAnyNumber` (Rottenmouth Viper) and `SacrificeX` (Immoral Bargain) keep the number announced
as the spell's `x` (`StackItem::x`, `Obj::x`), though Viper's mana cost has no {X}. Its mana value is right (`Query::mana_value`
and the `Cast` record count X once per {X} in the mana cost, CR 202.3e), but anything that reads `x` as "the value of X" -
"X is the value of X paid" on a permanent, a copy's X (CR 707.10), the client's X shown on a spell - would see the number
sacrificed for a spell that has no X. No card in content reads X of a Viper. A fix keeps the number apart from X.

**Scenarios.** none: no card reaches it.

### KI-46: The lore counter a player puts on a Saga as their main phase begins has no player putting it

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Medium | Open | engine | 2026-10-17, scenarios of Esper Origins |

**Notes.** CR 714.3c: "As a player's precombat main phase begins, that player puts a lore counter on each Saga they control."
`ops::add_lore_counters` runs as a turn-based action whose cause is the rules, so `replace::counters` finds no putter
(`Cx::actor` is `None` for `Cause::Rules`), and "If you would put one or more counters on a permanent or player" (Innkeeper's
Talent at level 3, `replacements::double_counters_applies`, keyed to the putter) does not double it. The lore counter a Saga
enters with is doubled, as its controller puts it (CR 122.6a). A fix names the active player as the putter of the turn's lore
counters.

**Scenarios.**
- [`scenarios/innkeepers-talent-doubles-the-lore-counter-a-player-puts-on-summon-esper-maduin-as-their-main-phase-begins.cantrip`](../scenarios/innkeepers-talent-doubles-the-lore-counter-a-player-puts-on-summon-esper-maduin-as-their-main-phase-begins.cantrip)
  (marked `# Known bug`): one lore counter becomes two, so chapters II and III trigger together.

### KI-48: Emperor of Bones puts its finality counter on after the creature enters

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | card | 2026-10-17, review of Esper Origins |

**Notes.** "Put a creature card exiled with this creature onto the battlefield under your control with a finality counter on it"
is one entry with the counter on it (CR 122.6, 614.1c). `emperor_of_bones.rs` calls `put_onto_battlefield` and then
`add_counters`, though `put_onto_battlefield_with_counters` does both, as Scavenger's Talent and Zoraline use (and
`put_onto_battlefield_transformed_with_counters` for a card entering transformed, as Esper Origins uses). Nothing can happen
between the two inside the resolution; the difference is to an ability that triggers on a permanent entering with a counter on
it (no card in content has one) and to who puts the counter (CR 122.6a: the permanent's controller, which for the Emperor is
also the ability's controller). A fix calls `put_onto_battlefield_with_counters`.

**Scenarios.** none: no card reaches it.

### KI-49: The tactical bot does not know infect or poison

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-17, review of Inkmoth Nexus |

**Notes.** The engine is right: damage from a source with infect is -1/-1 counters to a creature and poison counters to a
player (CR 702.90b-c, 120.3b, 120.3d), and ten poison counters lose the game (CR 704.5c). The tactical bot
(`dotnet/src/Cantrip.Client/Tactics`) has no notion of either: `CardReading.cs` passes "infect" over as a keyword it does
not know, and nothing in the bot reads a player's poison counters, so it scores an infect creature's damage (Inkmoth
Nexus, Blightsteel Colossus) as life lost, races and blocks as if it were, and does not see a player near ten poison. A fix
reads infect into `Keywords` and gives the bot's damage and race arithmetic a poison clock beside the life one. The web
client shows poison on the player rail and the language-model client in its text, so a person and the model are not
affected.

**Scenarios.** none: a bot's judgement is not a rules outcome; a bot-lab duel with infect decks would show it.

### KI-50: An animation that replaces creature types is unexercised

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | mechanics | 2026-10-17, review of Inkmoth Nexus |

**Notes.** An effect that makes an object "a [creature types] artifact creature" without saying it is still what it was
keeps its card types and other subtypes but replaces its creature types with those named (CR 205.1b). `Animation` in
`cantrip-mechanics` has `replaces_creature_types` for it (a `LoseAllCreatureTypes` effect before the named types), set by
Iron Man Armor ("a 0/0 Construct Hero artifact creature"). The line runs in the Armor's scenarios, but the Armor has no
creature types of its own to lose, so no scenario shows a creature type being replaced. It would need the Armor to have a
creature type before it animates (an effect that gives every permanent a creature type), which no card in content does.

**Scenarios.** none yet: Iron Man Armor could reach it with such an effect.

### KI-51: A land whose subtype is set keeps the copiable values other than abilities a copy effect gave it

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Exercised; the reading is the CR's grammar, no ruling | engine | 2026-10-17, Magus of the Moon |

**Notes.** CR 305.7: a land whose subtype an effect sets "loses all abilities generated from its rules text, its old land types,
and any copiable effects affecting that land". The engine reads the sentence as naming the three sources of the abilities the land
loses - its rules text, its old land types (their mana abilities, CR 305.6) and any copiable effects (a copied card's abilities
and keywords, a copy's "except it has ...") - so `Chars::set_land_subtypes` takes, in layer 4, every ability the land has by
then, and what else a copy effect made of it (the name, supertypes, card types, colors, power and toughness of what it copies,
CR 707.2) stays, as layer 1 applied it. The sentence's first clause already says the land "no longer has its old land type", so
"loses ... its old land types" read as a list of things lost would repeat it; read as sources of abilities it does not. The
other reading - the copy effect itself is lost - has the land stop being what made it a land at all in the case of a Clone
copying a Dryad Arbor, and no ruling of Blood Moon, Magus of the Moon, Vesuva or Thespian's Stage says so; the judges' articles
consulted (Blood Moon, Progenitor of Dependency; a 2013 Judge Forum thread on Thespian's Stage under Blood Moon, which predates
the "copiable effects" wording and kept the Stage's ability) settle nothing either way. A ruling that the copy effect goes
entirely would start the land's characteristics from its own card when `layers::land_text_lost` holds it.

**Scenarios.** vesuva-copying-dark-depths-under-blood-moon-is-a-legendary-mountain-named-dark-depths-with-no-abilities,
thespians-stage-copying-urborg-under-blood-moon-keeps-the-name-and-loses-the-ability-it-kept,
clone-copying-dryad-arbor-under-blood-moon-is-a-green-dryad-creature-mountain: they assert this reading and pass.

### KI-53: A copy holds at most four abilities added by exceptions

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, Thespian's Stage |

**Notes.** `Copiable::extra_abilities` (`crates/cantrip-state/src/object.rs`) is a fixed array of four; adding a fifth
"except it has [ability]" (CR 707.9b) is silently dropped. Each Thespian's Stage copying a Stage-copy adds one more instance of the
Stage ability (CR 113.2c), so a chain of five Stages copying one another loses one. No other card in content adds more than one.
A fix grows the array or refuses the copy loudly; instances beyond the second of the same ability change no play.

**Scenarios.** none: no scenario builds the chain.

### KI-54: A loop a player could stop is shortcut after a fixed number of rounds, not one the players name

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine / client | 2026-10-17, CR 104.4b loop detection; narrowed by the loop hardening |

**Notes.** `loops.rs` makes every loop end (its module comment is the design): a loop of mandatory actions is a draw at
once when a position comes round with nothing asking the history in between, and otherwise after `ROUNDS` rounds of its
shape with no amount heading for an end; a loop its players keep going by choice goes round `ROUNDS_TO_CHOOSE` times and
then the player CR 732.3 names must choose otherwise; one whose players only pass or decline is a draw after `ROUNDS`
(CR 732.5, 732.4). What is still not the rules' procedure:
- No shortcut is proposed (CR 732.2a-c): no player names a number of iterations, and none can shorten one. The engine takes
  `ROUNDS_TO_CHOOSE` as the number every player chose; a human who wants more rounds (mana past 32 rounds of an infinite
  mana loop) can't have them, and one who wants fewer stops by choosing otherwise, as the rules allow. A fix is a shortcut
  proposal the client offers (a number of iterations) and the bots answer.
- The player made to choose otherwise is chosen as CR 732.3 chooses, but the engine applies it to every loop of choices
  going nowhere - also one whose amounts rise each round (mana floated), where no position is "reached multiple times" - and
  for the rest of the step. What is made to differ is every answer they gave to the same question in the same shape (a
  target, card, player or mode chosen alone, a priority action - any activation of that ability, whatever its targets - or a
  "may" taken): more than "a different game choice", though never anything but choosing less or another of the same.
- A pass or a decline is never refused: a loop whose only choice is a "may" declined each time round (Fecundity's draw) is a
  draw after `ROUNDS`. CR 732.5 would let the player be held to that action, a loop object's; the engine's policy is not to,
  as taking it seldom stops the loop and may only harm them.
- A choice of several targets or cards at once, an order of triggers, a number, and a "may" asked in a staged ability
  (`Game::ask_then`) can't be refused, so a loop going round through them is treated as one of passes: a draw after
  `ROUNDS`. Where the other order or choice would have ended the loop, that is a draw the rules don't make (CR 104.4b's last
  sentence): the price of ending the loop at all.
- A loop whose rounds read the history (a card's "this turn" question each time round) is found only by its shape, after
  `BUSY_STEP` events and `ROUNDS` rounds, not at its second round: the right result, later. `ROUNDS` (256) is taken to be past
  any number a card counts the history or an amount to.
- A loop that never comes back to a shape - its zones growing, a token or an effect more each time round - is ended only by
  `FORCED_LIMIT` (a run of mandatory actions: a draw if no life total, poison, library, the stack or the board moved towards
  an end in its second half), or in a step of choices by `STEP_LIMIT` (players may then only pass and decline, and every
  answer counts as passing) and at last by twice `STEP_LIMIT` events in the step, a draw by the engine's judgement and no
  rule's - each slower as the zones grow. An amount going down from far away (a million life lost one at a time) is played
  out to its end.
- Cost: in a step being watched every question clones and journals the watch (its kept questions and answers), so a loop
  that runs to `ROUNDS` costs seconds, and a minute or more under `CANTRIP_VERIFY` (the growth and Fecundity draw
  scenarios). A fix keeps the questions in smaller journal entries.
- A mana ability whose whole cost is {T} is treated as no choice when nothing more mana could pay for is castable, even when
  its effect does more than add mana (a painland's damage).

**Scenarios.**
- [`scenarios/oblivion-ring-loop-a-player-can-break-with-an-instant-is-not-a-draw.cantrip`](../scenarios/oblivion-ring-loop-a-player-can-break-with-an-instant-is-not-a-draw.cantrip)
  and [`scenarios/worldgorger-dragon-loop-ends-when-animate-dead-chooses-another-creature-card.cantrip`](../scenarios/worldgorger-dragon-loop-ends-when-animate-dead-chooses-another-creature-card.cantrip):
  loops with a choice, played round by hand until the script breaks them.
- [`scenarios/oblivion-ring-loop-a-player-never-breaks-with-the-instant-they-hold-is-a-draw.cantrip`](../scenarios/oblivion-ring-loop-a-player-never-breaks-with-the-instant-they-hold-is-a-draw.cantrip),
  [`scenarios/worldgorger-dragon-loop-whose-player-keeps-choosing-the-dragon-must-choose-the-other-creature-card.cantrip`](../scenarios/worldgorger-dragon-loop-whose-player-keeps-choosing-the-dragon-must-choose-the-other-creature-card.cantrip)
  and [`scenarios/worldgorger-dragon-loop-whose-player-always-declines-fecundity-is-a-draw.cantrip`](../scenarios/worldgorger-dragon-loop-whose-player-always-declines-fecundity-is-a-draw.cantrip):
  the fixed count at work (not marked: they show rules-correct outcomes).

### KI-55: Spellskite can't choose between two instances of "target" that hold the same object

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, Spellskite |

**Notes.** "Change a target of target spell or ability to [this]" (CR 115.7b, `ops::change_a_target_to`) asks which target to
change as a choice among the targets. When the same object is the target of two different instances of "target" on the spell
(CR 115.3) and both could be changed, the choice names that object once, and the first instance is the one changed; the rules
let Spellskite's controller choose either (the 2020-08-07 ruling), and the results differ when the instances do different
things. A fix asks the choice by instance (each labelled with its slot's text). No spell in content with two instances of
"target" that may share an object has been put under a Spellskite.

**Scenarios.** none yet.

### KI-56: A counter ban judges an entering permanent as the object it was before it moved

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, Melira, Sylvok Outcast |

**Notes.** `CardDef::counter_bans` (Melira's "creatures you control can't have -1/-1 counters put on them") is asked of a
permanent entering with counters (CR 122.6) with the player it enters under. Its characteristics should be those of the
permanent as it would exist on the battlefield (CR 614.12): `ops::judged_entering` now answers `Query::chars` of the entering
object so while its counters are judged, for Solemnity's ban (artifacts, creatures, enchantments, lands) and the replacements
of +1/+1 counters keyed to the permanent's types (`cantrip_mechanics::replacements::plus_one_counters_applies`: Hardened
Scales, Corpsejack Menace, Ozolith, Michelangelo) as well, which should cover a card entering as a copy of a creature or a
noncreature card entering as a creature; but no scenario reaches either yet: no implemented effect returns a noncreature card
with a -1/-1 counter, and a persisting Clone is a creature card. Moonshadow, a persisting Kitchen Finks, a Walking Ballista, a
Saga, Dark Depths and a planeswalker entering as themselves are judged rightly.

**Scenarios.** none yet.

### KI-57: The tactical bot does not know islandwalk

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-17, adding Lord of Atlantis |

**Notes.** The engine is right: a creature with islandwalk can't be blocked while the defending player controls an Island
(CR 702.14c, `turn::can_block`), and the blockers question offers only the pairs the rules allow, which the bots follow. The
tactical bot's own combat arithmetic (`dotnet/src/Cantrip.Client/Tactics`, `Unit.CouldBlock` and `Exposure`) reads no
landwalk: `CardReading.cs` passes "Islandwalk" over, and a unit knows nothing of the defending player's lands, so the bot
plans its attacks as if an islandwalker could be blocked by an opponent with an Island, and counts its own blockers against
one. A fix reads islandwalk into `Keywords` and makes a landwalker unblockable in the bot's planning when the defending
seat controls a land of its type. The web client shows the keyword on the card, and the language-model client reads it.

**Scenarios.** none: a bot's judgement is not a rules outcome.

### KI-60: Infect's -1/-1 counters are counters an effect puts

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-17, Doubling Season |

**Notes.** Doubling Season doubles counters "an effect" puts (`CounterPlacement::effect`), and not those a cost or a turn-based
action puts (CR 118.3, 714.3c). The -1/-1 counters damage from a source with infect puts on a creature (CR 120.3d, 702.90c) are
put as an effect's (`ops::add_counters`), combat damage's included, though combat damage is dealt by a turn-based action
(CR 510.2). No ruling says which they are; a wither source would ask the same (none is in content).

**Scenarios.** none yet: an infect creature dealing combat damage to a creature whose controller has a Doubling Season.

### KI-61: Limits of judging a permanent as it would exist, and a land as layer 4 leaves it

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, review of the CR 614.12 change |

**Notes.** `ops::as_it_would_exist` (CR 614.12) and `layers::land_text_lost` (CR 305.7) leave these cases, none of which an
implemented card reaches:
- `ops::judged_entering` works the entering permanent out once, before its zone-change replacements apply, so one that changes
  how it enters ("enters under your control instead") is not taken into account by those after it (614.12: "taking into
  account replacement effects that have already modified how it enters"), and one that sends it elsewhere (Containment Priest)
  leaves the rest still judging a permanent.
- A permanent leaving at the same moment as one enters (`sacrifice_and_return`, Goblin Welder) is still on the battlefield
  while the entering one is judged, so its static abilities count among those that would apply to it.
- The trial object does not carry its `tapped`, a Room's cast door (CR 709.5d) or a battle's protector, and a token copy of a
  permanent spell is judged without the text change `carry_effects` gives it.
- `prepare_entry`'s guard (`layers::abilities_may_be_lost`) looks at the battlefield only: a permanent whose own static
  ability would take its own abilities or set its own land subtype is not asked.
- A replacement of the entering object's own that functions from every zone (`own_everywhere`) is read off its card where it
  is, while `Query::lost_all_abilities` of it answers as it would exist.
- `land_text_lost` judges a land-subtype-setting ability on the object as layer 4 leaves it. For an ability keyed on the subtype
  it overwrites ("All Mountains are Plains": Conversion, Glaciers) that misses the lands it applied to; in a dependency loop
  (CR 613.8b) the ability may have applied before an effect that changed the object's land-ness; and while the question is
  being answered (`finding`), the layer-4 abilities of lands that lose their text still apply. A fix records which setting
  abilities applied to the object as its layer 4 is worked out.

**Scenarios.** none: no implemented card reaches them.
### KI-62: A pool holds at most 255 mana of one kind

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-17, the loop hardening |

**Notes.** A pool holds each kind of mana as a `u8` (`ManaPool`, and each restriction's and each source's share of it), and
`GameState::add_mana` adds no more than brings a kind to 255: what more a mana ability would add is lost, where the rules
have no limit (CR 106.4). Before, the count wrapped round (mana lost, the pool no longer the sum of its `mana_from`
entries) in a release build and panicked in a debug one. Only an infinite-mana loop gets there: a seat that taps its lands
each time round a Worldgorger Dragon loop is made to stop only after some hundred rounds (`loops::BUSY_STEP` events, then
`ROUNDS_TO_CHOOSE` rounds), by which three lands of one color have made 255. A fix widens the counts.

**Scenarios.** none yet.
### KI-63: Control from a static ability, or a lapsed condition, is settled only as state-based actions are checked

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, review of the control-changing static abilities (Mind Control, Old Man of the Sea) |

**Notes.** A resolved control effect changes the controller as it is created (`Game::add_effect`), and the untap step settles
control after untapping (an Old Man of the Sea untapped there gives its creature back before the upkeep's triggers). But a
static control-changing ability that starts or stops applying (`control_changing`: an Aura attached, moved or gone) and a
`Duration::AsLongAs` condition that turns false partway through a resolution are worked out by `ops::settle_control` and
`ops::end_lapsed_conditions` only when the state-based actions are next checked (CR 613.1b and 611.2b say at once). Anything
read in between - later in the same resolution, or a trigger looking at the controller at the event - sees the old
controller. No card in content reads a controller in that window that a scenario reaches.

### KI-64: No card in content lets one control-changing effect decide another

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, review of the control-changing static abilities |

**Notes.** `ops::settle_control` works the controllers out again while any changes, since an Aura's controller is whom it
gives control to (gain control of a Mind Control, and the creature it enchants follows). No card in content can take control
of an Aura that gives control: Opalescence leaves Auras alone, and the stealing cards target creatures. The repeat has no
scenario.

### KI-65: A prepared permanent that phases out loses its preparation copy for good

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, Teferi's Protection |

**Notes.** A prepare spell's copy in exile ceases to exist once what prepared it is no longer on the battlefield and prepared
(`settle::state_based_actions`, CR 722.3c), and a phased-out permanent is not on the battlefield's list, so phasing a
prepared permanent out (Teferi's Protection, Oubliette) ends its copy. CR 722.3c says that "as a permanent ... phases in
prepared, its controller creates a copy" again; `phasing::phase_in` makes none. A fix creates the copy as a prepared permanent
phases in (`ops::become_prepared`'s copy-making, without the event of becoming prepared).

**Scenarios.** none yet: a prepare card under Teferi's Protection would show it.

### KI-66: Phasing in a multiplayer game after a player leaves is unbuilt

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, Teferi's Protection |

**Notes.** A phased-out permanent its owner owns leaves the game with them (CR 702.26k: `ops::leave_game` walks every zone list,
the phased-out one included). But one that phased out under the control of a player who has left phases in "during the next
untap step after that player's next turn would have begun" (CR 702.26n), and `phasing::untap_step` phases in only what phased
out under the active player, so it never phases in; and a permanent another player owns that the leaving player controlled
while it was phased out is not exiled with what they control (CR 800.4a reads the battlefield). No scenario has a third player
with phased-out permanents.

**Scenarios.** none yet.

### KI-67: A spell cast by a resolving ability under split second is unexercised

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Unexercised | engine | 2026-10-17, review of Krosan Grip |

**Notes.** Krosan Grip's ruling: "If the resolution of a triggered ability involves casting a spell, that spell can't be cast if
a spell with split second is on the stack" (CR 702.61a). `actions::cast_ways_during_resolution` offers no way to cast while a
spell with split second is on the stack under what resolves, but no scenario has a triggered ability that casts (cascade,
discover) resolve above a split-second spell.

### KI-68: The advice applies a total cost's least before a cast's own reductions and a permission's additions

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-17, review of Trinisphere |

**Notes.** CR 601.2f applies what directly affects the total cost (Trinisphere's "costs three mana", `CostDelta::at_least`)
last. `actions::cast` does (`apply_cost_changes_before_least`, then harmonize, a sacrifice for each and emerge, then
`at_least`). The advice that decides whether a cast is offered does not: `least_cost`/`finish` apply it inside
`change_total`, before Rottenmouth Viper's sacrifice-for-each reduction and before `way_covered`'s emerge and harmonize
reductions, so under Trinisphere such a cast may be offered for less than it costs and given back at the payment (Wild
Ride's harmonize with a 4-power creature is advised at {R}, costs {2}{R}); and `castable_ways` adds a permission's `more`
and additional mana after it, so a cast it could pay may be withheld (a Bolt costing {2} more by a permission is advised at
{4}{R}, costs {2}{R}). A fix gives the advice the cast's order: `change_parts`, the cast's own reductions and the
permission's additions, then the least.

**Scenarios.** none yet (Trinisphere with Wild Ride, Winternight Stories, Crabomination, Rottenmouth Viper or a permission
with a tax would show it).

### KI-69: A bestowed spell has the mana value of its bestow cost

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-17, review of Chalice of the Void and Gaddock Teeg |

**Notes.** `bestow::bestowed` makes the bestowed face with the bestow cost as its `cost` where the two differ (Detective's
Phoenix: `{R}`), and the spell's mana value on the stack is read from that face, so a bestowed Detective's Phoenix is mana
value 1 rather than 3. Bestow is an alternative cost (CR 702.103a, 118.9c): the mana value is the card's mana cost's.
Chalice of the Void with one counter would counter it (and with three would not), and a `CardDef::spell_rules` ban judges it
by the bestow cost too (no implemented bestow card's mana value reaches Gaddock Teeg's 4). A fix keeps the mana cost on the
face and the bestow cost apart.

### KI-73: Older "you may draw" cards offer a draw a can't-draw rule forbids

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | cards | 2026-10-17, the draw-replacement batch's review |

**Notes.** "If an effect says that a player can't draw cards and another effect offers that player the choice to draw a card,
that player can't choose to do so" (CR 121.3), and under "can't draw more than one card each turn" a choice to draw two or
more can't be taken at all (CR 121.2b, Narset, Parter of Veils' ruling). `cantrip_mechanics::effects::may_draw` does this
(Leovold, Consecrated Sphinx, Sylvan Library), but Fecundity, Jeskai Ascendancy, Oblivious Bookworm, Smuggler's Copter and
Terrasymbiosis still ask their "you may draw" by hand: under Spirit of the Labyrinth or Narset they ask, and the draw then does
not happen (the core refuses it), where the rules say they should not ask. The fix moves them onto `may_draw`.

**Scenarios.** none yet.

### KI-74: A question a replacement asks mid-run is not watched for loops

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-17, the draw-replacement batch's review |

**Notes.** A run that goes on past a question it could not stop at (a replacement effect's order, a dredge or Abundance "may",
Chains of Mephistopheles' discard) is wound back to where the question was asked (`Cx::asked_at`). The mark is taken before
`loops::waiting` registers the question with the loop watch, so the revert undoes the registration, and such a question is
not watched for a loop of choices (CR 732). The old revert to the run's start did the same. A loop through repeated draw
replacements is the only way to meet it.

**Scenarios.** none yet.

### KI-70: A block requirement is counted on the declared block only, not through the blocked creature's band

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-17, review of Lure and banding |

**Notes.** Lure's requirement (CR 509.1c) is obeyed by blocking the Lured creature. If it is in an attacking band, a creature
blocking another member of the band blocks it too (CR 702.22h), which obeys the requirement; `BlockRequirements::obeyed`,
`turn::most_requirements` and `validate` count only the declared (blocker, attacker) pairs, so they refuse that legal
declaration (the blocker must be declared on the Lured creature itself, which is also legal and has the same result). A fix
expands each declared block through its band before counting, in the search and in the check. Relatedly, the band members a
blocker blocks only by 702.22h get no `Event::Blocks` (CR 509.3a's "whenever this creature blocks" correctly triggers once);
no card in content triggers on "blocks a creature" or "becomes blocked by a creature" per pair (CR 509.3b, 509.3d).

**Scenarios.** none yet: no card in content puts Lure on a creature with banding or bands with a Lured creature.

### KI-71: The tactical bot's combat planning ignores bands

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | client | 2026-10-17, review of banding |

**Notes.** The engine is right (CR 702.22h-k). The tactical bot's blocks and attacks (`TacticalPlay.Combat.cs`, `Combat`) are
judged pair by pair: it does not know that blocking one member of a band blocks all of it, nor that the opponent divides its
blocker's damage among the band (702.22k) or its attacker's damage among banding blockers (702.22j). Its band choice and the
divisions it makes are on purpose (`TacticalPlay.Choices.cs`, the engine's sparing suggestion).

**Scenarios.** none: a bot's judgement is not a rules outcome.

### KI-72: A face-down permanent may be unable to turn face up under Humility

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | engine | 2026-10-17, card-faces batch |

**Notes.** Turning a face-down permanent face up is a special action (CR 116.2b, 702.37e, 701.40a), allowed by the face-up card's
morph cost or, for a manifested creature card, its mana cost; it is not an ability of the face-down permanent, so "loses all
abilities" (Humility, Dress Down) should not stop it (Whisperwood Elemental's ruling: a manifested creature card can still be
turned face up after losing its abilities). The engine offers it through `intrinsic::FACE_DOWN` / `intrinsic::can_turn_face_up`,
which sit on the face-down values and may be taken by Humility's layer-6 effect. Separately, there is no verb to turn a
permanent face down (CR 708.2a) and no "manifested" mark, so a creature an effect turned face down (Ixidron) would wrongly be
offered turning face up for its mana cost (CR 701.40b, 702.37e). Neither is reachable by a card in content today.

**Scenarios.** none yet: a face-down creature under Humility is the first to write.

### KI-75: Words of Wind's "return a permanent" question may be captioned as a cost

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Suspected | client | 2026-10-17, draw-replacement batch |

**Notes.** Words of Wind's replacement ("each player returns a permanent they control to its owner's hand instead") asks with
`Purpose::ReturnToHand`, which the server's prompt captioning treats as a cost (the karoo lands' "return a land you control ...
to pay for it"). The caption would then say the return pays for something, which it does not. A fix gives the effect's return its
own purpose or caption. Not checked in the browser.

**Scenarios.** none yet.

### KI-76: A search's world keeps what the real game did with hidden cards it cannot resample

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-18, building `cantrip_ai::determinize` |

**Notes.** `determinize` (`crates/cantrip-ai/src/worlds.rs`, docs/ai-player-plan.md "Worlds") deals the hidden cards of a
fork again, without journalling, and asks the waiting question again (`Game::ask_again`). Some facts of the real game stay
in the world, and a seat's search could read them:
- **Which hidden object an event named.** If the opponent discards a card, the history says whether it was the card just
  drawn or one it had kept (two different objects). The world keeps that object, with a new card. Two real games that
  differ only there make different worlds. The redaction test skips dealing a world for such a pair ("histories apart":
  10 positions of 176 pairs, `accumulate-wisdom` among them). A fix renumbers the seat's unseen hand objects too, or
  deals the hidden objects as a set.
- A library the journal tail rearranges (a resolution in progress drew from it or put a card in it) keeps its real order
  (`zone_order_touched`).
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

### KI-77: A sacrifice cost with nothing left to sacrifice is asked, not taken back

| Severity | Status | Area | Found |
| --- | --- | --- | --- |
| Low | Open | engine | 2026-10-18, the redaction test's long run (Fanatical Offering and Black Lotus in a Hidden-mode prior deck) |

**Notes.** A player casts Fanatical Offering ("As an additional cost to cast this spell, sacrifice an artifact or creature")
and, while paying its mana, sacrifices their only artifact, Black Lotus, for mana. The cost can't be paid, so the cast is
taken back (CR 601.2h). The engine asks the sacrifice anyway: `actions::choose_permanents` puts a `ChooseCards` with
nothing eligible, count 1, and cancel allowed. Only a cancel answers it. A cost of tapped permanents with too few left is
taken back without asking; this one is not, because the guard reads `p.tapped`. The bench's random seat
(`cantrip_bench::decide`) now cancels such a question; before, it answered with no cards, which was rejected. A fix
takes the cast back without asking whenever too few are eligible, which changes what a real game asks.

**Scenarios.** none yet: Fanatical Offering cast with Black Lotus as the only artifact, Black Lotus activated while paying.
