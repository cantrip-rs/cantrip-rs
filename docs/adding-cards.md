# Adding a card

How a card gets into the engine ([engine-design.md](engine-design.md)). A card is done when it is exact - every line
of its Oracle text, by the rules - and every scenario in `scenarios/` that uses it passes. Every card in content was
ported from the old C# engine this way; new cards follow the same loop.

## The loop

```
python tools\maintenance\new-card.py "Mind Stone"                          # read everything, write the file to start from
cargo run --profile cards -p cantrip-script --bin corpus -- --card "Mind Stone"   # its scenarios, full reports (~1 s)
cargo run --profile cards -p cantrip-script --bin corpus                         # whole corpus, one pass/fail line
set CANTRIP_VERIFY=1 & cargo test --profile cards -p cantrip-script             # every scenario, rollbacks checked
cargo run --profile cards -p cantrip-script --bin corpus -- --missing 40          # which cards block the most scenarios
```

The standard every card meets - exact to the rules, no deviation; every line of its file executed by its scenarios
(`tools\card-coverage.cmd "Name"`); its corner cases and interactions covered; an independent review - is the `add-cards` skill (`.claude/skills/add-cards/SKILL.md`).

The `cards` profile is release code with incremental compilation: an edit to one card rebuilds in about a second.
The whole corpus runs in a fraction of a second, so run it after every card rather than a part of it.

1. **Read the card**: `new-card.py` prints `content/<dataset>/cards/<name>.json` (its characteristics and Oracle
   text), whether that is still Scryfall's current Oracle text and the card's official rulings (`scryfall.py`), the
   CR rule of each keyword and keyword action on it - the whole rule when the engine has nothing for it yet (`cr.py`),
   the scenarios whose decks hold it, and its prior art: for each ability, the implemented cards that already
   print it, or each of its parts (cost, trigger condition, effect sentence), matched exactly, then with numbers and
   mana as wildcards, then card types, colours and subtypes too - with the file, the shared mechanics and `Game`
   verbs it uses, and how its text differs (`tools/maintenance/ability-index.py`). A new card's JSON is its characteristics only; the
   build generates the Rust constant from it, so there is nothing to run. The scenarios say what has to be true, and they name the
   options of a question in the order the engine offers them ("the first one offered"), so a card asks its questions
   in the order the rules and the cards already implemented do.
2. **Write the card's file** in `crates/cantrip-cards/src/cards/<card_name>.rs` (`new-card.py` writes it
   with the characteristics only). Nothing else has to change: the registry is generated from the files in that folder.
   Adding a card creates files only (its JSON, its Rust file, its scenarios) and edits none that another card's
   addition also edits.
3. **Run its scenarios** with `corpus --card`. A scenario runs only when every card in both decks exists; the rest
   are listed with what they still need. When the card's scenarios pass, they are already tests: `cargo test` makes one per file in `scenarios/`.
4. A scenario that is wrong (rules-incorrect) is not "fixed" by matching the mistake: fix the scenario, with the
   rule.

## A card's file

Characteristics are generated at build time (`crates/cantrip-oracle/build.rs` reads `content/*/cards/*.json`). Never type a
name, a cost, types or P/T. A card's file adds only behavior, with struct update syntax over its generated constant:

```rust
use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    ..oracle::LIGHTNING_BOLT
};
```

Abilities are written cost first, then what the ability does, then the printed text (`crates/cantrip-api/src/ability.rs`):

```rust
abilities: &[
    mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
    ability(cost("{2}{G}"), pump_team, "{2}{G}: Creatures you control get +0/+1 until end of turn."),
    ability(loyalty(1), plus_one, "+1: Target player draws two cards, then discards two cards.").targets(&[targets::PLAYER]),
    triggered(THIS_ENTERS, draw_a_card, "When this creature enters, draw a card."),
],
```

- **Costs**: `TAP`, `SACRIFICE_THIS`, `cost("{1}{U}")` (mana symbols as printed, read at compile time; `cost("{1}, {T}")`
  includes the tap; an `{X}` in it is announced and read as `item.x`, and "X can't be 0" is `ability(..).x_cant_be_zero()` on
  the ability, `x_cant_be_zero: true` on a spell's `CardDef`, never a check in the effect), `loyalty(n)` (and `loyalty_minus_x()` for "−X", X read as `item.x`), `life(n)`, `sacrifice(n, filter)`, `exile_permanents(n, filter)` ("exile a creature you control", CR 118.3: the exiled permanents are the ability's `q.cost_objects(item)`, read with `chars` as they last were, and "exiled with" its source, CR 607.2a; a spell's additional cost is `Cost::exile_permanents`, asked of the history as Fling's sacrifice is), `discard(n, filter)`, `remove_counters(kind, n)`, `remove_counters_among(n, filter)` ("remove N counters from among creatures you control", any kinds), `blight::cost(n)` (blight N, CR 701.68a),
  `DISCARD_THIS`, `EXILE_THIS`, `EXERT`, joined with `.and(..)`; `return_tapped_to_hand(n, filter)` is "return N tapped
  [permanents]", whose permanents are chosen once the mana is paid, so one tapped for mana while paying may be among
  them (web-slinging, `cantrip_mechanics::web_slinging::way(cost, text)`); craft (CR 702.167a) is
  `cantrip_mechanics::craft::ability(mana("{3}{R}{R}"), &craft::ARTIFACT, "Craft with artifact {3}{R}{R}")`, its materials a
  `Materials` (`craft::materials(count, or_more, filter, quality)` for a card's own, `craft::materials_each(parts, ..)` for
  materials named one by one), and "the exiled cards used to craft it" (CR 702.167c) is
  `craft::exiled_cards_used_to_craft(q, permanent)` (`craft::used_to_craft(state, ..)` in a characteristic-defining
  ability, with `LayerQuery::chars_with_defined_pt` for their power); anything rarer is a `Cost { .., ..Cost::NONE }`. A bare
  mana cost (equip, flashback, kicker, madness) is `mana("{2}{R}")`, or `generic(n)` for a number the card works out.
  What an activated ability's cost tapped, sacrificed or returned ("the tapped creature's power", "the sacrificed
  creature") is `g.cost_objects(item)`, in the order chosen, read with `chars` (last-known once gone, CR 608.2h).
  Station (CR 702.184a) is `cantrip_mechanics::station::ability(text)`, and each "N+ |" striation a static with
  `station::charged::<N>` (`has::<INDEX>` for its abilities, `is_a_creature` and `base_pt::<P, T>` for its P/T box).
- **Mana**: `add(Mana::G)`, `add_n(Mana::C, 2)`, `add_each(&[Mana::W, Mana::U])`, `add_any_color(1)`,
  `add_one_of(&[Color::Red, Color::Green])`, `.only_for(restriction)`. What these list is what is added; a card whose
  mana ability does more writes the effect itself with `.by(f)` (the listing then only says what it can add) or
  `add_by(f)` when nothing can be listed (Gaea's Cradle).
- **Triggers**: `cantrip_mechanics::triggers` names the common ones (`THIS_ENTERS`, `THIS_DIES`, `YOUR_UPKEEP`,
  `THIS_ENTERS_OR_ATTACKS` - one ability with two trigger events, never two abilities, ...); a
  card's own is `when(EventMask::DAMAGE, my_match_fn)`. A Saga's chapter is `chapter(n, effect, text)`. A trigger event
  that says "you" is `when_asked(mask, f)`, `f` taking the `Asker` and reading `me.controller`, never
  `q.controller(source)`: a delayed triggered ability's "you" is the player who controlled the spell or ability that made
  it (CR 603.7d-e), whoever controls its source now or wherever it has gone.
  "Whenever you scry", "whenever you surveil" and "whenever you scry or surveil" are `triggers::YOU_SCRY`, `YOU_SURVEIL`
  and `YOU_SCRY_OR_SURVEIL`: `cantrip_mechanics::scry` and `surveil` record `Event::Scried` / `Event::Surveilled` once
  the action is complete, an empty library included (CR 701.22d, 701.25d), and never for scry 0 or surveil 0 (CR
  701.22b, 701.25c), so every scry and surveil goes through them; the trigger's `data` is the number and the cards looked at.
  "Whenever ... activate(s) an ability" listens to `EventMask::ABILITY_ACTIVATED` and reads the event with
  `triggers::activation` (the object, the activating player, the `ActivatedDef`: `is_loyalty_ability()`,
  `is_mana_ability()`, `from`); only abilities that use the stack are activations a trigger sees - a mana ability's is
  not emitted yet.
  "Whenever you commit a crime" (CR 700.13) is `triggers::YOU_COMMIT_A_CRIME`, on `Event::CrimeCommitted`: the core
  records one for each spell cast, ability activated or triggered ability put on the stack that targets an opponent,
  anything an opponent controls or a card in an opponent's graveyard (never a copy, never a changed target); "if you've
  committed a crime this turn" asks the history for one.
- **The rest** is a method after the call: `.targets(..)`, `.sorcery_speed()`, `.once_each_turn()`, `.condition(..)`
  (an activation restriction, or a trigger's intervening "if"), `.modal(..)`, and the
  others in `ability.rs`. Whether a trigger looks back in time (CR 603.10) is never written: the rules decide it from
  the event and the ability's own words (`CardTable::looks_back`). Nor is where a trigger condition functions when it
  can't trigger from the battlefield (CR 113.6k): "when you cast this spell" is `triggers::CAST_THIS_SPELL` (or
  `when_this_spell_is_cast(..)` for one that says more) and functions from the stack; "when this card is put into a
  graveyard from anywhere" is `triggers::PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE`. Only an ability whose own text puts it in a
  zone says so, `.functions_in(IN_A_GRAVEYARD)`: one that returns "this card from your graveyard" (CR 113.6m) or says
  "if this card is in your graveyard" (CR 113.6b). The engine asks it there and nowhere else, so its matcher never checks
  the zone; `tests/functioning_zones_match_text.rs` holds every ability's zones to its words. A loyalty ability is already sorcery speed (CR 606.3).
  `.only_as_an_instant()` requires priority even for a mana ability (CR 304.5, 602.5e): it cannot be activated during
  payment, though its mana floated beforehand remains spendable.
- **Shared effects with an argument** are called in place: `triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), ..)`.

The rest of a card is written the same way, one constructor each, the rare parts as methods:

```rust
spell: spell(&[targets::CREATURE], effect),                       // `spell(&[], effect)` for none
target(filter), up_to_one(filter), up_to(3, filter),               // target slots; `targets::X.optional()`, `.between(min, max)`, `.counted()`,
                                                                  // `.if_kicked(0, TargetSpec::ANY_NUMBER)` ("if this spell was kicked, instead choose any number of ...")
                                                                  // slots in printed order, a slot of several targets anywhere: `item.spec_targets(n)` is where slot n's targets are
static MODAL: ModalDef = modes(&[mode(deal_3, "...").targets(&[targets::CREATURE]), mode(gain_3, "...")]).choose(1, 2);
statics: &[static_ability(Layer::PtModify, attached_object, plus_one, "Equipped creature gets +1/+1.")],
statics: &[characteristic_defining(Layer::PtDefine, power_is_creatures, "...")],
statics: &[loses_all_abilities(attached_object, "Enchanted creature loses all abilities.")],  // never a hand-made layer-6 clear:
                                                                  // the core must know (cost changes, replacements, triggers)
statics: &[static_ability(Layer::Type, others, creature_too, T), static_ability(Layer::PtSet, others, base_pt, T).continues()],
                                                                  // one ability over several layers (CR 613.6): the later parts
                                                                  // `.continues()` the one before, apply to what the first part
                                                                  // applied to, and outlive their source's abilities
replacements: &[replacement(ReplacementKind::ZoneChange, applies, apply, "shuffles it into its owner's library").own_everywhere()],
ways: &[way(cost("{1}{B}"), ZoneKind::Hand, "Pay {1}{B}; an opponent draws a card")],  // keywords: flashback::way(cost(..), text)
kicker: kicker(mana("{2}{B}")),                                   // `multikicker(..)`
```

Definitions a card brings with it (a token, a copy's exception) go in `pub static EXTRA: &[&CardDef]` in the same
file. The generator's constant name is the card name upper-cased with every run of non-alphanumerics as `_`.

A token's name is the text's (CR 111.4), never written by hand: one the text doesn't name is built on
`token!(Phyrexian Hydra)` instead of `CardDef::BLANK` - its subtypes, once, in the text's order - and is named "Phyrexian
Hydra Token" (a text change of the creating text renames it with its subtypes, CR 612.2a). Only a token the text names
("a ... token named Moloid", "create Boo, a legendary ... token") writes `name`, on `CardDef::BLANK`.
`token_names_match_oracle` holds every token in an `EXTRA` to the text of the card that makes it.

What a `CardDef` can carry (see `crates/cantrip-api/src/card.rs`):

| Field | For |
| --- | --- |
| `spell: spell(targets, effect)` | an instant's or sorcery's effect, and its target slots |
| `abilities: &[AbilityDef]` | its activated, mana and triggered abilities, written as above. **Mana abilities first**: the C# engine numbers them first. |
| `ways: &[way(..)]` | alternative costs (flashback from the graveyard, Fireblast's sacrifice, Gush's return, web-slinging's tapped creature), way 1 onward; `instant_speed` offers one whenever an instant could be cast, whatever the card's type (sneak); `enters_attacking` has a permanent spell cast that way enter tapped and attacking what the creature its cost returned was attacking (sneak, CR 702.190b); `kind(WayKind::..)` names the keyword a card asks about ("cast using web-slinging": `web_slinging::was_cast_using_web_slinging`). Surge and freerunning are `surge::way(mana("{1}{R}"), "Surge {1}{R}")` and `freerunning::way(mana("{3}{B}"), "Freerunning {3}{B}")`, their conditions asked of the history (`surge::surged` for "if its surge cost was paid"). Morph, megamorph and disguise are ways too: `morph::way(Cost::mana(mana("{4}{W}")), "Morph {4}{W}")`, `megamorph::way(..)`, `disguise::way(..)`, the cost any `Cost` ("Morph—Pay 5 life": `Cost { life: 5, ..Cost::NONE }`); casting face down for {3} and turning face up are the core's, so a card with nothing else needs only that line |
| `enters: Some(fn)` | "as this enters" and "enters with": edits the `Entering` record (tapped, counters) before it is on the battlefield - never a copy, which a verifying run refuses. The core orders it with the other replacements of the entry, by the entering permanent's controller (CR 616.1), offering it in the card's own line of text that says how it enters. A permanent chosen as it enters ("choose another creature you control") is `choose::object_as_enters`, and "the chosen creature" `choose::chosen_object` (CR 607.2d). The engine keeps every choice with the text that made it (`Chosen`); "the chosen [color, type, player, name]" is read through `GameState::chosen_color` and its siblings with the asking ability's `Linked` - `g.linked(item)` resolving, `q.linked_to(asker)` for a function asked with an `Asker` (a triggered ability's matcher that reads a choice is `when_asked`, so it is asked with one, never `text_linked`), `LayerQuery::linked_to(source)` in a static ability - and the object's fields can't be read, so a permanent that has become a copy of something else, or an ability an effect gave it, reads no choice that ability did not make (CR 607.5) |
| `enters_as_copy: Some(fn)` | "you may have this creature enter as a copy of ...": sets `Entering::copy`, built with `cantrip_mechanics::copy::enter_as_copy*` (Clone, Vesuva); applied before every other replacement of how it enters (CR 616.1c) |
| `as_transforms: Some(fn)` | "As this creature transforms into [this face], ..." (CR 712.20), on the face it names: `fn(g, asker) -> Result<(), Suspend>`, run by the core as the permanent turns over to that face, whatever transforms it, `asker.controller` its "you"; never as it enters transformed (CR 712.14a). Sephiroth, One-Winged Angel |
| `delayed: &[AbilityDef]` | abilities its effects set up to trigger later (`Game::delayed_trigger` with `AbilityRef::delayed(card, n)`), or give an object with `EffectKind::AddAbility`; a quoted static ability it gives is `granted_static(layer, apply, text)` (layer 7 only; its source is the object that has it) |
| `replacements: &[replacement(..)]` | its replacement effects (CR 614): a `hint`, the event kind (`ZoneChange`, `Counters`, `DrawInstruction`, `Draw`, `Damage`, `TokenCreation`, `LifeChange`: a signed `amount` of life, a gain above zero; the hint is worded to the player whose life it is, who orders them; `ManaProduction`: the mana a permanent tapped for mana produces, CR 106.12b, its `mana` changed in type and amount, the player who tapped it ordering them), `applies(q, asker, &Proposal)` and `apply(q, asker, &mut Proposal)`, both pure; `own_everywhere` for "if this card would ... from anywhere". Several that apply to one event are ordered by the affected player (`Replace(...)` in a scenario). The shared "+1/+1 counters, plus one", "twice that many tokens" and the life ones ("you gain twice that much life", "that much life plus 1", "an opponent loses that much life instead") are in `cantrip_mechanics::replacements` |
| `cast_fact: Some(f)` | "if you controlled a Faerie as you cast this spell" (CR 601.2i): `fn(q, asker) -> u32`, asked of the spell and its caster once its costs are paid and kept in its `Cast` record of the history; the effect reads `g.history().fact_as_cast(item.object)`, which is 0 for a copy (not cast, CR 707.10). Faerie Fencing |
| `cast_limit: N` | "each player can't cast more than N spells each turn" (CR 101.2), while it is on the battlefield |
| `trigger_rules: &[..]` | "[permanents] entering don't cause abilities to trigger" (CR 603.2), while it is on the battlefield: `fn(q, asker, &Event, &Triggering) -> bool`, true stops that ability triggering on that event. `cantrip_mechanics::statics::creatures_entering_dont_trigger` is Torpor Orb's; `statics::entering(event)` is the permanent an event put onto the battlefield |
| `cant_gain_life: &[..]` | "players can't gain life" (CR 119.7), while it is on the battlefield: `fn(q, asker, player) -> bool`, true stops that player gaining life. `cantrip_mechanics::statics::players_cant_gain_life` (Sunspine Lynx) and `opponents_cant_gain_life` (Knight of Dusk's Shadow) are shared; "damage can't be prevented" is `unpreventable_damage: &[statics::damage_cant_be_prevented]`. A life total that "becomes N" is `Game::set_life_total` (CR 119.5) |
| `legend_rule_exempt: &[..]` | "the 'legend rule' doesn't apply" (CR 704.5j), while it is on the battlefield with its abilities: `fn(q, asker, legend) -> bool`, true keeps that legendary permanent out of the legend rule (Mirror Gallery). An effect that lifts the rule for a player is `EffectKind::NoLegendRule` (Hall of Echoes) |
| `granted_miracles: &[GrantedMiracle { .. }]` | "each instant and sorcery card in your hand has miracle {2}" (CR 702.94a), while it is on the battlefield: `has(q, asker, card)` says which cards in a hand have it (`cantrip_mechanics::miracle::in_your_hand` and the card's own test), `cost(q, face)` its miracle cost for the card or the face of it being cast (none for one that can't be paid, CR 118.6), `less` the generic mana that cost is reduced by (Aminatou's {4}), `text` what the choice between several miracle abilities calls it. Lorehold, the Historian, Molecule Man, Aminatou, Veil Piercer |
| `spell_alternatives: &[SpellAlternative { .. }]` | "you may cast Dragon spells without paying their mana costs" (Dracogenesis), "you may pay {W}{U}{B}{R}{G} rather than pay the mana cost for spells that you cast" (Jodah, Fist of Suns), "you may cast spells from your hand without paying their mana costs" (Omniscience): an alternative cost (CR 118.9, 601.2b) for the spells a player casts, while it is on the battlefield with its abilities. `applies(q, asker, spell, player)` judges the face being cast in the zone it is cast from (`player == asker.controller` for "you cast"; Omniscience asks for the hand), `cost` is `InsteadOfManaCost::WithoutPaying`, `::Mana(mana("..."))` or `::Energy(n)` (Primal Prayers' {E}), `flash: true` for "if you cast a spell this way, you may cast it as though it had flash", `text` the menu row (and `Ability:`). It is no permission: the core offers it beside every cast the player could already make for the face's mana cost, from any zone and by any permission (each such cast's `paid_by`), and as an effect resolves "you may cast that card" asks which cost (`Label::ManaCost`, `Label::Way`, `Label::AlternativeCost`); never with another alternative cost (CR 118.9a); additional costs and cost changes still apply, X is 0 |
| `assist` (content) | assist (CR 702.132): the content's `"assist": true` (`PayWith::assist`, as convoke's), which `new-card.py` drafts; the core asks the caster to choose a player and has them pay (`cantrip_mechanics::assist`), so a card with nothing else is `CardDef { ..oracle::CHARGING_BINOX }` |
| `prototype` (content) | prototype (CR 702.160, 718): the content's `"prototype": {"manaCost": "{2}{R}", "power": 3, "toughness": 2}`, which `new-card.py` drafts from Scryfall and the oracle checks against the Oracle line; the core does the rest (the prototyped face, its cast, its characteristics on the stack and the battlefield), so a card with nothing else is `CardDef { ..oracle::BLITZ_AUTOMATON }` and one that prints more writes only that (`cantrip_mechanics::prototype`) |
| `granted_ways: &[GrantedWay { .. }]` | a way a permanent gives cards in its controller's graveyard or hand to be cast: `cost(q, asker, card, player)` the cost for the face being cast, or none where it does not apply. `permission: true` where it is only a permission for the mana cost ("you may cast a permanent spell with mana value 2 or less from your graveyard": Lurrus, Serra Paragon), so alternative costs may be paid by it; otherwise it is the cast's alternative cost too (escape, mayhem). A keyword's way is made by its module (`escape::granted(cost)`, `mayhem::GRANTED`); escape's carries `kind: WayKind::Escape`, as the card's own escape way does, so a spell cast by either "escaped" (`escape::escaped`, CR 702.138b). "Once during each of your turns" is a history question: `history::permission_used_this_turn(q, asker.source)`, since every cast and land play records what made it possible (`Cast::granted_by`, `Event::LandPlayed`); never a marker of your own |
| `additional_triggers: &[..]` | "that ability triggers an additional time" (CR 603.2d), while it is on the battlefield: the same `fn(q, asker, &Event, &Triggering) -> bool`, true makes that ability trigger once more on that event. `Triggering::of_a_permanent_controlled_by(asker.controller)` is "a triggered ability of a permanent you control"; `statics::permanent_entering_triggers_again` (Elesh Norn, Yarok), `creature_dying_triggers_again` (Teysa Karlov, Drivnod) and `creature_attacking_triggers_again` (Isshin) are shared |
| `mana_told_apart: &[..]` | an ability that asks where the mana spent came from (CR 106.3), while the permanent is on the battlefield with its abilities: `fn(q, asker, source) -> bool`, true tells mana `source` produced apart from other mana, so its payer spends it only by choosing it (a pool chip of its own, `Spend: W from <source>` in a scenario). The ability itself reads the history: `g.history().cast_using_mana_from(spell, asker.source)` ("whenever you cast a spell using mana produced by [this]"), or `mana_spent_to_cast(spell)` for each source and its mana. Barracks of the Thousand |
| `cant_become_untapped: &[..]` | "enchanted creature ... can't become untapped" (CR 101.2, 701.26b), while it is on the battlefield with its abilities: `fn(q, asker, permanent) -> bool`, true keeping that permanent tapped through every untap - the untap step, an effect, a stun counter's replacement. The rule is the source's, no ability of the permanent (CR 113.1a), so one that loses its abilities is still under it. `attach::attached_to_it` is "enchanted/equipped creature" (Frozen in Ice; Infinite Coursework's `untap_bans`) |
| `damage_stays: &[..]` | "damage isn't removed from this creature during cleanup steps" (CR 514.2), while it is on the battlefield: `fn(q, asker, object) -> bool`, true keeps the damage marked on that permanent through cleanup. Regeneration still removes it. Ancient Adamantoise |
| `replacements: &[replacements::DAMAGE_TO_YOU_AND_YOUR_OTHER_PERMANENTS_IS_DEALT_TO_THIS]` | "all damage that would be dealt to you and other permanents you control is dealt to this creature instead" (CR 614.1a): a `Damage` replacement may set `DamageEvent::target`, and the core deals the damage there, judging protection, infect, lifelink and deathtouch against the new recipient (Ancient Adamantoise, Palisade Giant) |
| `preventions: &[PreventionDef { hint, applies }]` | a static ability's prevention effect (CR 615): "prevent all damage that would be dealt to [this]", `applies(q, asker, &DamageEvent)`. The core orders it with the replacement effects of the same damage (CR 616.1), skips it for damage that can't be prevented (CR 615.12) and records what it prevents (`Event::DamagePrevented`, the dump's `Prevented`) (Black Panther, Hope Enduring) |
| `granted_preventions: &[prevention::ANY_DAMAGE]` | the prevention effects the card's spell or abilities make for a duration (CR 615.3): `cantrip_mechanics::prevention::this_turn(g, item, to, index, Shield::All \| Shield::Next(n))` makes the `index`th, of damage to an object, a player or anyone; `next_damage_to_target` and `all_combat_damage_this_turn` are the printed recipes (Healing Salve's shield, CR 615.7; Fog) |
| `loss_rules: &[..]`, `win_rules: &[..]` | "you can't lose the game", "you don't lose the game for having 0 or less life" (CR 104.3, 704.5a-c), "your opponents can't win the game" (CR 104.2b), while it is on the battlefield with its abilities: `fn(q, asker, player, LossReason) -> bool` (true: that player doesn't lose that way) and `fn(q, asker, player) -> bool` (true: that player can't win). The core asks them at every state-based action check that would make a player lose, and in `lose_game`/`win_game`; a concession still loses. Platinum Angel, Phyrexian Unlife |
| `infect_damage_to: &[..]` | "all damage is dealt to you as though its source had infect" (CR 702.90b): `fn(q, asker, player) -> bool`, judged once per damage event for each player, before it changes their life (CR 120.2). Phyrexian Unlife |
| `counter_bans: &[..]`, `player_counter_bans: &[..]` | "creatures you control can't have -1/-1 counters put on them", "you can't get poison counters" (CR 122.6, 122.1, 101.2), while it is on the battlefield with its abilities: `fn(q, asker, object, controller, CounterKind) -> bool` and `fn(q, asker, player, PlayerCounter) -> bool`, true keeping those counters off. Every placement is asked, a permanent entering with counters included (persist, "enters with"), before and after the replacement effects; damage from a source with infect or wither that puts no counter is still damage dealt. Melira, Sylvok Outcast |
| `enter_rules: &[..]` | "creature cards in graveyards and libraries can't enter the battlefield" (CR 101.2), while it is on the battlefield with its abilities: `fn(q, asker, card) -> bool`, false keeping that card where it is. Asked of every card put onto the battlefield from anywhere but the stack, by any effect, and of every land that could be played (CR 305.1); the card is judged as it is where it is (a card turned face down to be manifested or cloaked as that face-down card, CR 701.40f), and nothing of its entering is asked. Grafdigger's Cage |
| `cost_bans: &[..]` | "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities" (CR 101.2, 118.3, 601.2h, 602.2b), while it is on the battlefield with its abilities: `fn(q, asker, payer, CostFor, CostPayment) -> bool`, true forbidding that payment for a cost paid to cast a spell (`CostFor::Spell`) or activate an ability (`Ability`, `ManaAbility`). A cost only a forbidden payment pays is not payable, so its spell or ability isn't offered; what a resolving spell or ability has a player pay is untouched. Yasharn, Implacable Earth |
| `sacrifice_bans: &[..]` | "equipped creature ... can't be sacrificed", "creatures you control but don't own ... can't be sacrificed" (CR 101.2, 701.21a), while it is on the battlefield with its abilities: `fn(q, asker, permanent) -> bool`, true keeping that permanent from being sacrificed by anything - costs, instructions and choices. A permanent's own "this permanent can't be sacrificed", printed or granted, is `Keyword::CantBeSacrificed`; "it can't be sacrificed this turn" is `EffectKind::CantBeSacrificed`. Assault Suit, Garland, Royal Kidnapper |
| `attacks_despite_defender: &[..]` | "[this creature / creatures you control] can attack as though [it / they] didn't have defender" (CR 702.3b, 609.4), while it is on the battlefield with its abilities: `fn(q, asker, creature) -> bool`, true lets that creature with defender attack, which keeps defender for everything else and gets past no other restriction. Demon Wall's "as long as this creature has a counter on it". A resolved "can attack this turn as though it didn't have defender" is `EffectKind::CanAttackDespiteDefender` (Assault Formation) |
| `attack_costs: &[..]` | "creatures can't attack you unless their controller pays {2} for each ..." (CR 508.1d, 508.1h), while it is on the battlefield: `fn(q, asker, attacker, Target) -> Option<ManaCost>`, the cost of that creature attacking that player, planeswalker or battle. `statics::attacking_you_costs::<N>` (Ghostly Prison, Propaganda) and `attacking_you_or_your_planeswalkers_costs::<N>` (Baird, Archon of Absolution) are shared; the core adds every cost up, asks for it as attackers are declared and never makes a requirement pay it |
| `attack_limits: &[..]` | "No more than one creature can attack this planeswalker each combat", "No more than one creature can attack you each combat" (CR 508.1c), while it is on the battlefield with its abilities: `fn(q, asker, Target) -> Option<u32>`, the most creatures that may be declared attacking that player, planeswalker or battle, the smallest where several abilities limit one. The cap is on the declaration alone: an effect can still put another creature there attacking (CR 508.4c), or change what an attacker attacks (Tomik, Orzhov Lawmage). A cap the ability gives the target (Tomik's "Planeswalkers you control have ...") is the target's ability, which layer 6 gives and takes in timestamp order (CR 613.7): grant it with `statics::grant_delayed_ability` and have the rule ask whether the target has that `AbilityRef` (`q.chars(id).abilities`), not `q.lost_all_abilities(id)`, which a loss older than the grant would wrongly answer; any condition on the source is the rule's too (Mirri, Weatherlight Duelist while tapped); Judoon Enforcers, The Eternal Wanderer |

## Card code

An effect is `fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>`. It reads the game through `Query` (`g.state()`,
`g.chars(id)`, `g.obj(id)`, `g.permanents_of(p)`...) and changes it only through `Game`'s operations (`draw`,
`damage`, `move_together`, `put_onto_battlefield`, `create_tokens`, `counter`, `may_pay`, `add_effect`...). The
full list is `crates/cantrip-api/src/game.rs`.

**Asking a question:** `let answer = g.ask(player, RequestKind::...)?;`. The first time it is reached, the effect
stops there (`?` carries the `Suspend` out). Once the question is answered, the effect runs again from the start and
`ask` returns the answer. So:

- Card code must be deterministic: the same game and answers do the same thing. Use the game's RNG (`g.shuffle`), no
  globals, no clocks, no `HashMap` iteration.
- Everything before the question is done again on the re-run. That is correct and cheap (the engine rolls the first
  run back). Never keep state across runs anywhere but the game.
- Ask what the rules ask, when they ask it, the way the cards already implemented do. A question with one possible answer
  is skipped in some places and not in others (a lone copy candidate is taken; a sacrifice of exactly as many as
  there are is taken; a discard is always asked unless the hand is empty); match the existing cards, since the
  scenarios rely on it.
- A "you may" is worded by the card: `effects::ask_may(g, player, source, Purpose::Ask(&ASK))` with a `const ASK: Question`
  beside it - the question in the card's words ("pay 2 life?", shown after the source's name), what it means beyond its
  buttons, and the buttons named for what each answer does ("Pay 2 life", "Enters tapped"), not Yes and No; a verb the
  bots read it by (`Draw`, `SearchLibrary`); `.no_default()` when saying yes costs or risks something.
  `Purpose::Optional` shows only the card's text and Yes and No.
- A payment as the effect resolves - "you may pay {W}{B} and 2 life. If you do / When you do", "unless [a player] pays
  {3}" (CR 118.12) - is `g.may_pay(payer, source, subject, cost("{W}{B}").and(life(2)), &ASK)`, never a life check, a
  "may" and a `lose_life` by hand: it asks the whole cost as one payment, offers mana abilities while there is mana to
  pay, judges the life as it is paid (CR 119.4) and says whether it was paid. Its `Question` says what paying does, with
  `{cost}` and `{subject}` where the cost and the subject's name go ("pay {cost} to untap {subject}?", "Pay {cost}",
  "Leave it tapped"), and its verb is what the bots weigh the cost against: `Countered` and `Sacrificed` (paying keeps
  the subject), `CopySpell`, `UntapSource`, `ReturnToHand`, `ReturnToBattlefield`, `ReturnPermanentCard` (a card chosen
  once paid, its greatest mana value the `amount`), `CreateCopyToken`, `Transform`, `PutOntoBattlefield`, `LookAgain`,
  `EnterUntapped`. A new verb gets its weighing in the tactical bot's `WorthPaying`.
- Most questions are asked through `cantrip_mechanics::choose`, which returns the thing chosen. The core checks every
  answer against its question before the card sees it, so a card never matches on an answer of the wrong shape.
- **A choice of cards, or a library search, is a selection rule** (docs/engine-design.md, "Selection rules"): look in
  `cantrip_mechanics::select` first for the wording the card prints, and ask it with `choose::select` (cards shown to
  choose among) or `library::search` / `library::find_to_hand`, `find_to_battlefield`, `find_to_graveyard`, `find_to_top`:

  ```rust
  use cantrip_mechanics::{library, select::{self, up_to, DIFFERENT_NAMES}};

  // "Search your library for a basic land card, reveal it, put it into your hand, then shuffle."
  library::find_to_hand(g, you, item.object, select::BASIC_LAND_CARD, true)?;
  // "... for up to X basic land cards with different names ..."
  library::find_to_hand(g, you, item.object, select::basic_land_cards(up_to(x)).with(DIFFERENT_NAMES), true)?;
  // "You may choose a nonland card from it." (a revealed hand)
  let chosen = choose::select(g, you, item.object, Purpose::Discard, hand, select::NONLAND_CARD)?;
  ```

  A quality only this card asks is a `SelectFn` in its file - `fn noncreature_nonland(q: &dyn Query, choice: &Choice, _: u32)
  -> Verdict { Verdict::each(choice, |card| ..) }` - made a part with `select::part(&(noncreature_nonland as SelectFn), 0,
  "")` (Duress); one a second printed card asks moves into `select`. Never a closure, never a list worked out before asking:
  the core judges the answer, and the client each pick, by the rule. The older `choose::{card, cards, card_from, cards_from,
  cards_from_kind, any_number_of, cards_within_total}`, `search_library` and `library::Find` / `search_to_*` are kept for
  existing cards.
- **Targets judged together are a set rule** (`TargetSpec::together`, CR 601.2c): "any number of target creature cards
  that each have a different mana value X or less" is a filter for "creature card in your graveyard with mana value X or
  less" - a quality of each target, reading `q.announced_x()` (none while only asked whether some X could have a target, and always none in a
  triggered ability, which announces no X: a trigger's "X or less" quality needs another source of X, never this one) -
  and `TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(filter) }.together(select::DIFFERENT_MANA_VALUES)`; "with
  total mana value X or less" is `.together(select::total_mana_value_at_most(TargetSpec::COUNTED))`, `COUNTED` the X
  announced. Only a rule of the targets together goes there (`select::set_part`; anything else does not compile): a
  quality of each target is the filter's, so that a target that loses it is illegal alone. The core offers, checks and
  rechecks the set on resolution (CR 608.2b); the card reads its legal targets as for any slot (`targets::legal_objects`).

**The questions** (`RequestKind`, `crates/cantrip-state/src/decision.rs`), and the script verb that answers each:

| Question | `RequestKind` | Script verb |
| --- | --- | --- |
| choose cards | `ChooseCards { purpose, cards, rule }` (`choose::select`, a rule from `select`) | `Choose(...)` |
| a choice, or a trigger's targets | `ChooseTarget { candidates, optional, count, any_number }` | `Target(...)` |
| modes, piles, options | `ChooseModes { modes: Vec<Label>, eligible, count }` | `Modes(...)` |
| colors | `ChooseColors { choices, count }` | `Colors(...)` |
| may | `May { purpose }` | `Accept` / `Decline` |
| a payment ("you may pay", "unless pays") | `g.may_pay(...)` asks it (`UnlessPays`) | `PayCost` (`Accept` for one of life alone) / `Decline` |
| "you may pay {X}", X the payer's (CR 107.3f) | `g.may_pay_x(...)` asks it (`UnlessPays` with `x`), the X paid or `None` | `PayCost(X: 3)` / `Decline` |
| "you may pay {G} up to three times", "any number of times" (CR 603.12a) | `g.may_pay_times(.., mana, most, ..)` asks it as one payment (`UnlessPays` with `times`), the number of times paid or 0; "when you pay this cost one or more times" is `reflexive_trigger_counted` with it, and "choose up to that many" `modes(..).up_to_that_many()` on a `modal_delayed_trigger` | `PayCost(Times: 2)` / `Decline` |
| distribute cards | `DistributeCards { cards, destinations, may_shuffle }` | `Arrange(... \| ...)` |
| search a library | `SearchLibrary { library, rule, destination }` (use `library::search` or a `find_to_*` wrapper with a rule from `select`, which names where the cards found go: the question says it; a search the effect can't cause, `Game::may_search`, finds nothing) | `Search(...)` |
| "you may search" | `library::ask_may_search(..)`, never `ask_may`: not asked when the search can't be made (CR 101.2), and the search and its shuffle run only on a yes | `Accept` / `Decline` |

What a player is told about a question is part of the card (non-negotiable 5 of the `add-cards` skill): a `TargetSpec`
with two slots says what each is for with `.labelled("Return target spell or permanent to its owner's hand")` (`{player}` stands
for the opponent of an each-opponent slot), a cast during resolution asks with the `Purpose` that says what it costs (`CastFree`,
`CastCopyFree`, `Cast` for one paid for), and a question is never asked with a `Purpose` that means something else.

Their defaults (what the runner answers when no line applies) are in `crates/cantrip-script/src/run.rs`, `default`.

**Words of its rules text: read through `words`.** A text-changing effect (CR 612: Trait Doctoring, Artificial
Evolution) replaces a color word, basic land type or creature type in an object's text, and the card then does what
the changed text says. So every color word, basic land type and creature type a card's rules text uses as one is read
through the words of the object whose text it is: `g.words(item.object).color(Color::Blue)` for "if it's blue",
`view.words(source).subtype(Subtype::Goblin)` in a static ability for "other Goblins", `q.words(asker.source)` in a
condition. Mana symbols (`{U}`) are not words, and a card's name is never changed (CR 612.2), nor is a keyword's
reminder text. The layers change the type line, and a token gets the colors and creature types of the text that makes
it without the card doing anything (CR 612.4); a token a keyword's own rules make (For Mirrodin!'s Rebel) is created
with `create_tokens_worded(.., Words::NONE)`, and so is every keyword's added later (afterlife's Spirits, fabricate's
Servos). An object that has left the battlefield keeps the words it last had
(`words` reads its last-known information), so a sacrificed source's ability reads them too.

## Asking about the past: the event history

Every rules event since the game began is in the history, in order: `q.history()` (`cantrip_api::history`). Each
`Record` has its serial, turn, step, whose turn it was, its cause, and the `Event`; records reference objects and copy
nothing out of them, so what an object was is read from its row (never deleted; frozen as it left) or its last-known
snapshot. A trigger matching an event, and a replacement deciding about one, see the history as it was just before
that event; a static ability that reads the history ("as long as you've surveilled this turn") already sees it when the
triggers of that event are looked for (CR 603.10), so an ability it grants triggers on the event that granted it.

Any question about the past is answered there, never with a new field in the state:

1. **A one-off goes in the card's file.** Use `history.select(EventQuery::kind(..).player(..).this_turn()...)`, the
   typed helpers on `History` (`casts`, `casts_by`, `cast_of`, `drawn`, `life_gained`, `life_lost`, `resolutions`,
   `triggerings`, `done`, `discarded_this_turn`, `attacked_this_turn`), or a plain iterator over `history.this_turn()`
   or `history.all()` with whatever closure the card needs.
2. **A question a second printed card asks in the same words moves to `cantrip_mechanics::history`** as a named
   function, and both cards call it (`first_draw_in_own_draw_step` for Orcish Bowmasters and Hullbreacher,
   `permanent_left_battlefield_this_turn` for revolt, `put_into_graveyard_from_battlefield_this_turn`,
   `cards_left_graveyard_this_turn`, `spells_cast_this_turn` for storm, `opponents_who_lost_life_this_turn`,
   `entered_this_turn`, `drawn_this_turn`).
3. **The core only records.** If an event a card needs is not recorded, record it (an `Event` variant emitted where
   the rules do the thing, or `Cx::record` for one no ability triggers on); never add a counter.

A static ability's condition asks it too: its functions are handed a `LayerQuery`, whose `history()` is the same
history (`view.history().life_lost(you) > 0` for Essence Channeler's "as long as you've lost life this turn",
`history::entered_this_turn(view.history(), source)` for Shardmage's Rescue). The layers ask it again whenever
characteristics are computed (CR 611.3a), so the effect starts the moment the event is recorded and stops as the turn ends.

**A card file that is only `CardDef { ..oracle::X }` is a stub**, and `crates/cantrip-cards/tests/no_stub_cards.rs` fails on
one for a card whose Oracle text is more than core keywords (a card that is all core keywords needs no file). The
exceptions are a draft-only card (`draftAbilities` in its JSON) and a file that starts a comment `// Characteristics only:`
saying why every line of the card's text is already read from its JSON.

## Where code goes

**A keyword is never written in a card's file.** Every keyword action (CR 701: earthbend, amass, adapt, manifest
dread, vote, ...) and keyword ability (CR 702: evoke, persist, ripple, fading, boast, backup, ...) is a function or
module in `cantrip-mechanics` named for it and parameterised, from the first card that has it, and the card calls it
(`mechanics::earthbend(g, target, 1)`). Look there first; if it is missing, add it there, citing the CR rule, and say
in its doc comment which printed cards use it. Only a keyword that changes a core procedure (trample, first strike,
split second) goes in the core.

The layering of [engine-design.md](engine-design.md) holds, and CLAUDE.md's rule with it:

1. **The card's file** for everything one card does. That is the default.
2. **`cantrip-mechanics`** for a recipe a second already-printed card would use exactly as written (connive, cycling,
   search, a target slot). Name the second card in the doc comment.
3. **`cantrip-rules`** (the core) only for rules authority: a procedure of the Comprehensive Rules, a kind of
   question, a cost type, a zone-change rule. Never a card's behavior. A core change is reviewed on its own.

Card crates depend on `cantrip-api`, `cantrip-mechanics` and `cantrip-oracle` only. The compiler enforces it.

## What already exists

Before writing anything, look at what the cards already implemented do (`crates/cantrip-cards/src/cards/`). Most
shapes have an example:

- a targeted spell (Lightning Bolt);
- a choice from a revealed hand (Duress);
- "each player" choices in turn order (Liliana's +1);
- piles (Liliana's −6);
- an "unless" cost (Mana Leak, Miscalculation);
- alternative costs (Fireblast, Gush, Firebolt's flashback);
- cycling (Miscalculation);
- an ETB trigger (Satyr Wayfinder);
- a "becomes the target" trigger (Phantasmal Image's exception);
- a copy as it enters (Phantasmal Image);
- an any-color mana ability (Black Lotus);
- a stack ability with a mid-resolution color choice (Deathrite Shaman);
- library search and shuffle (Fabled Passage, Demonic Tutor);
- look and arrange (Ponder);
- a planeswalker (Liliana).

## Designations, dungeons and the rules that limit casting

- **The monarch and the initiative** (CR 725, 726) are `Player.monarch` / `Player.initiative`; a card says
  `g.become_monarch(p)` or `g.take_initiative(p)`. Their inherent triggers (the end-step draw, the steal by combat
  damage, the venture on taking the initiative and at upkeep) are the rules' own abilities (`cantrip_api::intrinsic`),
  put on the stack from an object in no zone (`designations.rs`). `Query` reads `state().player(p).monarch`.
- **The city's blessing and an enduring story** (CR 702.131, 702.195) are `Player.citys_blessing` /
  `Player.enduring_story`, had for the rest of the game; a card with the keyword says `ascend: true` or `storied: true`,
  and a card asks `g.state().player(p).enduring_story`. The rules give them (`designations::keyword_designations`)
  with the state-based actions and as each event happens, before what it triggers is looked for (CR 702.131d,
  702.195c), so a token that is the tenth permanent gives the blessing in time for the rest of the ability that made it.
  The scenario keys are `BotN.CitysBlessing` and `BotN.EnduringStory`.
- **Speed** (CR 702.179) is `Player.speed`, none until set. A card with start your engines! says
  `start_your_engines: true`, and the rules give its controller speed 1 as a state-based action (CR 704.5aa,
  `speed::engines_start`); the player's inherent "whenever one or more opponents lose life during your turn" ability is
  the rules' own (`intrinsic::SPEED_UP`, queued from `change_life`, once a turn by the history's `Triggered` records), and
  `g.increase_speed(p, n)` raises it, never past 4. Each change is `Event::SpeedChanged`. A card reads it with
  `start_your_engines::speed` (0 for none, CR 702.179f) or `has_max_speed`; "Max speed — [ability]" is
  `statics: &[max_speed::max_speed(text)]` with the ability as the card's `delayed[0]` (CR 702.178a). `BotN.Speed`.
- **Venturing** (CR 701.49, 309) walks `Player.room` through a dungeon card (`CardDef.dungeon`: the arrows of each room;
  the ability of the room is the ability of the same index, its label "Name — text"). The Undercity is
  `cards/undercity.rs`; `g.venture(p)` enters or moves on, and the rules complete it (CR 704.5t, `dungeons_completed`).
- **Cases** (CR 719): `cantrip_mechanics::solved`. A Case's file lists `solved::to_solve(condition, text)` with its own
  condition (an intervening "if" asked of the Case and its controller) and its "Solved" ability as `solved::triggered(trigger,
  effect, text)`, `solved::ability(cost, effect, text)` or, for play rules, `play_rules: &[solved::play_rules(&[..])]`. Solved
  is `Marks::solved` (`q.is_solved(id)`, scenario key `.Solved`). **Suspect** (CR 701.60) is `g.suspect(creature)`
  (`q.is_suspected(id)`, scenario key `.Suspected`).
- **Revolt** is `cantrip_mechanics::history::permanent_left_battlefield_this_turn(q, p)`. **Harness** (CR 701.64) is an effect:
  `g.add_effect(id, id, EffectKind::Harnessed, Duration::Indefinite)`, read by a static that grants the infinity ability
  (The Mind Stone). `Duration::UntilNextTurnOf(p)` is "until your next turn".
- **Cost changes** are `CardDef.cost_changes` (Thalia-likes, Bilbo, Anointed Peacekeeper), a spell's own
  `own_cost_changes` (affinity), and an ability's own `.cost_change(f)` ("this ability costs ... less", power-up). Each
  returns a `CostDelta`: `more(n)` generic, `less(n)` generic, `less_mana(cost)` for an amount that may be colored,
  whose colored part comes off that color and any excess off generic (CR 118.7), or `at_least(n)` for the least the total
  comes to once every increase and reduction is applied (Trinisphere, CR 601.2f). Each is also handed the targets announced for the
  spell or ability (CR 601.2c): the total cost is determined after them (CR 601.2f), so "equip abilities you activate
  that target this creature cost {2} less" (`attach::equip_targeting_this_costs_less::<2>`, Dwarven Mauler) and "costs
  {1} less for each color of the creature it targets" (Dragonfire Blade) read them; an action whose target is announced
  with it is offered only with the targets its cost could then be paid for. A change a resolving spell or ability makes
  for the turn ("artifact spells you cast this turn cost {1} less to cast": Armor Wars, Urza, Planeswalker, Academy
  Loremaster's "spells they cast this turn cost {2} more") is a `LastingCostChange { text, change }` in the card's
  `lasting_cost_changes`, made with `g.spells_cost_this_turn(item, player, index)`: it changes `player`'s spells (only
  spells, and only theirs - the core checks that) until the turn ends, whether or not its source is still around, so its
  `change` never asks who controls its `source`; its `text` is shown beside that player's life total while it lasts.
  A card's own convoke, delve and improvise are characteristics (`pay_with`, from the content JSON); a static ability that
  gives them to the spells a player casts ("noncreature spells you cast have improvise": Ironheart, Clever Champion; Chief
  Engineer's convoke; Teval's delve) is a `PayWithGrant` in `CardDef.grants_pay_with`,
  `fn(q, source, caster, spell) -> PayWith`, asked of the spell as it is cast and of the card (as the face it would be cast
  as) when the cast is judged; it checks "you cast" itself (`q.controller(source) == caster`).
  "Activate only once" is `.once()`;
  **cast limits and bans** are
  `CardDef.cast_rules` (`fn(&dyn Query, &Asker, caster, from: ZoneKind) -> bool`, false forbids casting from that zone:
  Teferi, Drannith Magistrate's "from anywhere other than their hands"); a ban on spells with certain qualities is
  `CardDef.spell_rules` (`fn(&dyn Query, &Asker, caster, spell) -> bool`, false forbids it, the spell read through the query
  as the face it would be cast as, on the stack under the caster: Gaddock Teeg's "noncreature spells with mana value 4 or greater can't be cast"); and a resolved spell's bar on a player casting from
  some zones is `EffectKind::CantCastSpellsFrom(player, ZoneKinds)` (Avatar's Wrath), both read by the core wherever a
  cast is begun, during a resolution too (CR 101.2, 601.3); **as though it had flash** is
  `EffectKind::CastAsThoughFlash(types)` for a resolved spell's effect and `CardDef.flash_rules` for a static ability
  ("You may cast Aura and Equipment spells as though they had flash", Sigarda's Aid; `fn(q, asker, spell, caster)`, the spell
  read through the query as it would be on the stack under the caster), and a card's own condition ("You may cast this spell as
  though it had flash if you control a Human", Bard's Company) is `CardDef.flash_if` (`fn(&dyn Query, &Asker) -> bool`, the card as
  source and the player who would cast it as controller, judged as the casting begins); a static lifting a loyalty ability's timing is `CardDef.instant_activation`
  (The Wandering Emperor), and a resolved spell's "you may activate loyalty abilities of [Jace] planeswalkers you control
  ... any time you could cast an instant" is `EffectKind::InstantLoyalty(Some(Subtype::Jace))` (Jace's Machinations; `None`
  for every planeswalker), which changes only the timing (CR 606.3). The board is searched for these only when something is cast or activated.
- **Mana**: `PlayGrant.any_type` / `CastTerms.any_type` (mana of any type pays, CR 609.4b). Mana with a restriction on
  what it may be spent on (CR 106.6) is added by a mana ability with `.only_for(restriction)` (or `g.add_restricted_mana(..)`
  from an effect): `ManaRestriction::Spells(types)` is "spend this mana only to cast [type] spells" (`ARTIFACT_SPELLS`,
  `INSTANT_OR_SORCERY_SPELLS`, `PLANESWALKER_SPELLS`, `CREATURE_SPELLS`), `ManaRestriction::Abilities(types, of)` "spend this mana only to
  activate abilities [of [type] sources]" (`ABILITIES` for any source, `LAND_ABILITIES`, `ARTIFACT_ABILITIES`, `CREATURE_ABILITIES`)
  - with `AbilitiesOf::Permanents` for "of [type]s", which are permanents only (CR 109.2: `ABILITIES_OF_ARTIFACTS`, Soldevi
  Machinist) and `AbilitiesOf::Sources` for "of [type] sources" in any zone (CR 109.2c) -, `ManaRestriction::SpellsOrAbilities(types,
  of)` "spend this mana only to cast [type] spells or activate abilities of [type]s | [type] sources"
  (`CREATURE_SPELLS_OR_ABILITIES_OF_CREATURES` for Castle Garenbrig, `CREATURE_SPELLS_OR_ABILITIES` for Gwenna's "creature
  sources", `ARTIFACT_SPELLS_OR_ABILITIES_OF_ARTIFACTS`),
  `SPELLS_FROM_EXILE` "spend this mana only to cast spells from exile", `NOT_SPELLS_FROM_HAND` "this mana can't be
  spent to cast spells from your hand",
  `ManaRestriction::Subtypes(..)` "spend this mana only to cast [subtype] [creature] spells" (`SubtypeSpells::of(Dragon)`,
  `::either(Dragon, Omen)`, `SubtypeSpells::CHOSEN_TYPE_CREATURE` for "a creature spell of the chosen type",
  `.uncounterable()` for Cavern of Souls, `.or_abilities()` for "or activate abilities of [subtype] sources",
  `.or_abilities_of_permanents()` for "or activate abilities of [subtype]s" (Smokebraider),
  `SubtypeSpells::CHOSEN_TYPE_SPELLS_OR_ABILITIES` for Eclipsed Realms, `CHOSEN_TYPE_CREATURE_SPELLS_OR_ABILITIES` for Secluded
  Courtyard), `ManaRestriction::NONCREATURE_SPELLS` "spend this mana only to cast a noncreature
  spell", `ManaRestriction::mana_value_at_least(4)` "spells with mana value 4 or greater" (and `mana_value_at_least_or_x(5)`
  "... or spells with {X} in their mana costs"), and `NotNonartifactSpells`, `GainsHaste` and `LegendaryUncounterable` the
  rest. A mana ability's subtype is read through its source's words and its chosen type as it is activated; an effect
  that adds the mana reads `g.words(item.object).subtype(..)` itself. A new restriction of card types is added to
  `ManaRestriction::NAMED` (the build fails on a card naming one that is not there), and the host gives it a word on the
  wire (`restriction_view`). "N mana in any combination of colors" is `cantrip_mechanics::choose_colors`. "Whenever a player taps a land
  for mana, that player adds one mana of any type that land produced" is `mana::a_player_taps_a_land_for_mana` (or a matcher
  of the card's own over `mana::land_tapped_for_mana`) with `mana::add_one_mana_of_any_type_that_land_produced`, as a
  `.mana_ability(TriggeredMana::OneOfProduced)` trigger: every triggered mana ability says what it adds (`TriggeredMana`), which
  is what the advice of whether a cost can be paid counts before anything is tapped. "You may activate abilities of creatures you control as though those creatures had haste" is
  `CardDef::activate_as_though_haste`.
- **Gift** (CR 702.174) is `CardDef.gift`, one of `cantrip_mechanics::gift`'s constants (a card, a Food, a tapped Fish, a
  Treasure, an Octopus, an extra turn); the promise and the opponent are asked before the targets, the spell's effect reads
  `item.gift.is_some()` and a target filter `gift::promised(q, asker.source)`. An instant or sorcery gives it as it
  resolves (the core); a permanent lists `gift::trigger(text)`. Tokens a gift makes are listed in the card's `EXTRA`. **Player hexproof** a resolved effect gives is `EffectKind::PlayerHexproof`; a permanent's static "You have shroud" (or hexproof) is `CardDef::player_keywords` (`cantrip_mechanics::statics::YOU_HAVE_SHROUD`), which targeting and the host read. **Player protection** from everything a resolved effect gives is `EffectKind::PlayerProtectionFromEverything` (CR 702.16j); a permanent's static protection from players ("You have protection from each of your opponents", CR 702.16k) is `CardDef::player_protection` (a `PlayerProtectionRule`: source, protected player, player it is from). Both go through `layers::player_protection`, which targeting, damage prevention (and its ordering, CR 616.1), the Aura state-based action (CR 702.16c) and the host read. **Hexproof from [quality]** (CR 702.11d) printed on a card is
  `CardDef.hexproof_from` (`HexproofFrom::color(Color::Black)`, `HexproofFrom::MONOCOLORED`, card types with
  `HexproofFrom::card_type`; a color in it is a word of the text, CR 612.2); given by a static ability it is
  `cantrip_mechanics::hexproof_from::gives::<Q>` (a resolved effect that gives it, Mondo Gecko's, is not built). The core judges it
  as a target is chosen, loses it with hexproof (CR 702.11e) and the host shows a granted one on the card.
  **"Can't be the target of [quality] spells or abilities ..."** on a permanent (Thrun, Breaker of Silence) is a
  `TargetingRule { kind: TargetingRuleKind::Forbid(f), applies }` in `CardDef::targeting_rules`: `applies` names the
  objects, `f` reads the `Asker` (`spell` or ability, its controller) and its source's characteristics, a color word
  through the words of the card's text (engine-design.md, "Can't be the target of" restrictions).
- **Names and players**: `RequestKind::ChooseName` (a card name, answered `Answer::Name`; its `kind` is what the name must
  be the name of, CR 201.4a, and the core refuses any other: card types the card has, card types it has none of, and
  "other than a basic land card name" - `NameKind::ANY`, `NONLAND`, `LAND`, `OTHER_THAN_BASIC_LAND`, or a `NameKind { .. }`
  for any other printed one, "a noncreature, nonland card name") and `ChoosePlayer`;
  a name chosen as a permanent enters is `Entering.chosen_card`, kept in `Obj.marks.chosen_card`, and one chosen for a
  permanent as its ability resolves ("when this land enters, choose a land card name", a Saga's "choose a card name") is
  `Game::remember_chosen_name`, kept there too (CR 607.2d): Petrified Hamlet, Medomai's Prophecy.
- **Modal abilities** are `.modal(..)`. A modal spell whose number of modes is decided by its optional additional cost
  or its kicker ("If this spell's additional cost was paid, choose both instead", Pyrrhic Strike) says so with
  `ModalDef::when_paid(min, max)`: the core then announces the cost before the modes (CR 601.2b) and offers paying an
  optional additional cost only when that many modes could be chosen (CR 700.2a); a kicker is not gated yet. **Playing from the top of a library for life** is
  `PlayRule::LibraryTop` (Bolas's Citadel).

## Rules bricks added with the second wave of cards

- **Control** (CR 613.1b): `Game::gain_control(source, permanent, player, duration)` and `exchange_control` for a resolved
  effect (`effects::gain_control_untap_haste` is Act of Treason's whole text); `control_changing(affects, text)` in `statics`
  for "You control enchanted creature" (Mind Control, with `attach::attached_object`). A duration the card words itself
  ("for as long as this creature remains tapped and ...") is `Duration::AsLongAs(g.card_id(&CARD), i)` with the condition
  the card's `lasting[i]`, its `text` the client's words (`{source}` for the source's name). "You may choose not to untap
  this ... during your untap step" is `may_not_untap: true`.
- **Printed "protection from [color]"** is `CardDef::protection` (White Knight); a text change reaches it, and a printed
  landwalk (Bog Wraith), in layer 3 (CR 612.2).

- **Entering with counters** (CR 122.6, 614.1c): `Game::put_onto_battlefield_with_counters` and
  `create_tokens_with_counters`; `mechanics::tokens::incubate` (CR 701.53). **`Game::win_game`** (CR 104.2b).
- **Combat**: planeswalkers are attackable (`DeclareAttackers.planeswalkers`); `Keyword::CantBeAttacked` (CR 508.1c), `EffectKind::CantAttack` and `CantBlock` for a resolved "can't attack or block" (CR 508.1c, 509.1b; the host writes each on the creature with its duration),
  `CantBeBlockedByPowerTwoOrLess`, `CantBeBlockedExceptByThreeOrMore` (`Chars::minimum_blockers`, menace's two too),
  `CantBeBlockedByMoreThanOne` (`Chars::maximum_blockers`; a JSON `keywords` entry when printed, a layer-6 grant otherwise).
  Landwalk (CR 702.14): a keyword per basic land type (`Keyword::Islandwalk`, `Swampwalk`, ...), read by the blocking
  rule with its land type (`Keyword::LANDWALKS`); a card that grants one names it through its words,
  `Keyword::landwalk(q.words(source).subtype(Subtype::Island))` (Lord of Atlantis).
  A scenario refuses an attack or a block with `! Bot2.Attack(A > B)` / `! Bot2.Block(A > B)`.
  **Toxic N** (CR 702.164) is the core's: printed, a JSON `keywords` entry `"Toxic 2"` (`CardDef::toxic`); granted, `EffectKind::Toxic(n)` or a layer-6 static's `chars.toxic += n`. `Chars::toxic` is the total toxic value (CR 702.164b), which combat damage to a player gives as poison counters (`ops::deal_damage_with`); the dump's `.Toxic` key shows it.
- **Reflexive triggers** (CR 603.12, "when you do"): `Game::reflexive_trigger`; one whose quantity is what the first part
  did ("tap up to that many target creatures") is `Game::reflexive_trigger_counted(.., count)`, and its slot's
  `TargetSpec::COUNTED` bound and its stack item's `x` are that count (Seasoned Cryomancer, Miasma Demon, Loamcrafter
  Faun, Swashbuckler Extraordinaire).
- **Counters**: `Event::CountersPut` carries `by` (CR 122.6a), the player who put them, and `total`, how many of the kind
  the permanent has once they are on it; a permanent entering with counters announces each kind it has, lore included.
  "When the Nth [kind] counter is put on this" (the Plans, Shang-Chi and the Ten Rings) is
  `when_asked(EventMask::COUNTERS, triggers::nth_counter_put_on_this::<{ CounterKind::Plan as u8 }, 4>)`. A new kind of counter
  is one line in `counter_kinds!` (`cantrip-state/src/vocab.rs`); the host and the client name it from there. A kind the
  rules give an effect is the core's: keyword counters (CR 122.1b: every one but exalted, each kind's keyword given in layer 6
  at the timestamp of its counters, CR 613.7c, so a card only puts them), and hone counters (CR 122.1j), whose +1/+0 the layers
  give the creature an Equipment with them is attached to (7c), so a card only puts them (Dwalin, Weaponmaster; Sting,
  Bilbo's Sword).
  **Costs**: `Cost.unattach_self`.
- **Until a card is cast from exile** (CR 611.2a, 400.7): `Duration::UntilCastFromExile(card)` ends as that object, a
  card in exile, becomes cast (CR 601.2i, after its costs are paid); the card leaving exile any other way is a new object
  and the effect goes on. `cantrip_mechanics::land_until_cast::ability::<N>(cost, text)` is the whole "{N}, Exile this
  card from your hand: Target land gains "{T}: Add ..." until this card is cast from exile. You may cast this card for as
  long as it remains exiled" (Emrakul, the Exigent Doom; Glamorous Outlaw and its cycle).
- **Casting cards in exile by a permanent's static ability** (CR 601.3, 604.1): `CardDef.exile_casts`, an `ExileCast { text,
  allows, additional, any_type, lands, life_for_mana_value }` - `allows(q, asker, card, player)` asked of the permanent with the card as the face that would be cast
  ("you may cast creature spells from among cards you own exiled with this creature": `q.exiled_with(q.linked_to(asker.source))`, its
  controller, its owner, the turn), and `additional` the cost owed on top of whatever it is cast for (Dawnhand Dissident's
  `remove_counters_among(3, targets::filters::creature)`; `Cost::NONE` for Rona, Disciple of Gix). The card's own alternative
  costs stay open to it. `any_type: true` is "and mana of any type can be spent to cast that spell" (CR 609.4b, Null
  Summoner); an "as long as" condition on the game state (threshold) goes in `allows`. `lands: true` is "you may play"
  rather than "cast": a land it allows is played from exile as a land play (CR 305.1); `life_for_mana_value: true` is "if
  you cast a spell this way, pay life equal to its mana value rather than pay its mana cost" (CR 118.9, 119.4), the one
  alternative cost, additional costs paid as usual (Valgavoth, Terror Eater).
- **Ward with a cost other than mana** (CR 702.21a, 118.12): `triggers::ward_life::<N>`, `ward_discard`, and
  `ward_sacrifice::<N>` ("Sacrifice N permanents": refused with fewer, and under Tamiyo, Collector of Tales),
  `ward_sacrifice_nonland::<N>` ("Sacrifice N nonland permanents") and `ward_sacrifice_a_food`.
- **Turns**: `DelayedWhen::UntilNextTurnOf(p)`; `DelayedWhen::EachStep` for "at the beginning of each [step] ... for the
  rest of the game", never used up and on no object (paradigm, CR 702.192a); `CardDef.turn_begin` skips a turn (Time Vault, CR 614.10); "end the
  turn" is `g.end_the_turn()?` (CR 724.1, Time Stop), after which the rest of the effect still runs (Glorious End), and
  "the player whose turn it is may end the turn" asks with `Purpose::EndTurn` (Obeka). "Activate only during your turn"
  is `cantrip_mechanics::your_turn`.
- **Spells**: copies of permanent spells become tokens (CR 707.10f), `Event::Copied` (magecraft); a spell that resolved
  (a copy too) is `Event::Resolved` with `AbilityRef::spell(card)` in the history (paradigm's "the first time a spell you
  control with this spell's name has resolved"); `Game::create_copy_in_exile` makes a copy of an object, gone or not (CR 707.12); abilities a static grants
  to a spell on the stack are seen by the cast-trigger scan; `Game::play_land_during_resolution`; `mechanics::hideaway`.
- **Mana**: an `{X}` in a mana ability's cost is announced and read as `Obj::x` of its source; an `any_color` ability with
  no color to choose is still offered and adds nothing (CR 106.5).
- **Layers and rules**: `Layer::PtSwitch` / `EffectKind::SwitchPt` (CR 613.4d), `EffectKind::PlayerProtectionFromEverything`
  (CR 702.16j), `CardDef.unpreventable_damage` (CR 615.12), `CardDef.cause_rules` (what an opponent's spells and
  abilities can't make you discard, sacrifice or search, CR 101.2: `CauseKind`), `Query::event_subject` for a trigger's target filter, `Query::announced_x` (`targets::mana_value_is_x`) for a target "with mana value X" of a spell or an activated ability, each its own X (CR 107.3k), and
  miracle (`CardDef.miracle`, CR 702.94), and miracle a permanent gives cards in a hand (`CardDef.granted_miracles`): a card
  with several miracle abilities is revealed with the one its player chooses (`Label::Miracle`), and only that one's linked
  ability triggers, carrying which it was (`Miracle::data`, CR 607.5), so its cost still holds once the permanent has gone
  (CR 113.7a).
