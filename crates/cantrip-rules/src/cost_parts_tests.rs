//! Cost parts the engine once stopped at, over test cards: an activated ability's behold (CR 701.4a,
//! 602.2b), a mana ability whose cost returns a tapped permanent (CR 605.3a, 601.2g-h), and a cost paid as a spell
//! resolves that has parts other than mana and life (CR 118.3, 118.12).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn is_bear(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Bear"
}

fn is_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Cost Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Cost Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "{T}: Add {G}."
static DORK: CardDef = CardDef {
    name: "Cost Test Dork",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

/// "{1}, Behold a Bear: You gain 3 life."
static SEER: CardDef = CardDef {
    name: "Cost Test Seer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}").and(Cost { behold: Some((1, is_bear, "Bear")), ..Cost::NONE }), gain_three, "{1}, Behold a Bear: You gain 3 life.")],
    ..CardDef::BLANK
};

/// "Behold a creature: You gain 3 life." A creature itself, it may behold itself.
static GAZER: CardDef = CardDef {
    name: "Cost Test Gazer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(Cost { behold: Some((1, is_creature, "creature")), ..Cost::NONE }, gain_three, "Behold a creature: You gain 3 life.")],
    ..CardDef::BLANK
};

/// "{1}, Return a tapped creature you control to its owner's hand: Add {G}{G}{G}."
static RETURNER: CardDef = CardDef {
    name: "Cost Test Returner",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}").and(Cost::return_tapped_to_hand(1, is_creature)), add_n(Mana::G, 3), "{1}, Return a tapped creature you control to its owner's hand: Add {G}{G}{G}.")],
    ..CardDef::BLANK
};

/// "{T}, Return a tapped creature you control to its owner's hand: You gain 3 life."
static RECOILER: CardDef = CardDef {
    name: "Cost Test Recoiler",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(TAP.and(Cost::return_tapped_to_hand(1, is_creature)), gain_three, "{T}, Return a tapped creature you control to its owner's hand: You gain 3 life.")],
    ..CardDef::BLANK
};

// Two distinct cost components cannot share a filter merely because both sacrifice a permanent.
fn is_dork(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Dork"
}
static MULTIKICK_DIFFERENT_COSTS: CardDef = CardDef {
    name: "Multikick different costs", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, is_bear)),
    kicker: Some(Kicker { cost: ManaCost::ZERO, extra: Some(Cost::return_to_hand(1, is_dork)), multi: true, waterbend: false }),
    ..CardDef::BLANK
};
static CHOSEN_DIFFERENT_COSTS: CardDef = CardDef {
    name: "Chosen different activation costs", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::sacrifice(1, is_bear), gain_three, "Sacrifice a Bear and return a Dork: Gain 3 life.")
        .pay_one_of(&[ExtraCost { text: "Return a Dork", cost: Cost::return_to_hand(1, is_dork) }])],
    ..CardDef::BLANK
};
static TWO_DISTINCT_COSTS: CardDef = CardDef {
    name: "Cost Test Two Distinct Costs", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::sequence(&[Cost::sacrifice(1, is_bear), Cost::sacrifice(1, is_dork)]), gain_three, "Sacrifice a Bear and a Dork: You gain 3 life.")],
    ..CardDef::BLANK
};

fn life_star_power(q: &dyn LayerQuery, id: ObjectId, chars: &mut Chars) {
    chars.power = Some((q.state().player(q.state().obj(id).controller).life / 2) as i32);
    chars.toughness = Some(1);
}
static LIFE_STAR: CardDef = CardDef {
    name: "Cost Test Life Star", types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, life_star_power, "Its power is half your life total.")], ..CardDef::BLANK
};
fn is_life_star(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).name == "Cost Test Life Star" }
fn damage_from_paid_power(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let object = g.history().spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).next().unwrap();
    let power = g.chars(object).power.unwrap_or(0).max(0) as u32;
    g.damage(item.object, Target::Player(PlayerId(1)), power);
    Ok(())
}
fn reverse_life_power(q: &dyn LayerQuery, id: ObjectId, chars: &mut Chars) {
    chars.power = Some((18 - q.state().player(q.state().obj(id).controller).life) as i32);
    chars.toughness = Some(1);
}
static REVERSE_LIFE_STAR: CardDef = CardDef {
    name: "Cost Test Reverse Life Star", types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, reverse_life_power, "Its power is 18 minus your life total.")], ..CardDef::BLANK
};
fn positive_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|n| n >= 1)
}
static SPELL_WITH_LIFE_AND_POWER_COST: CardDef = CardDef {
    name: "Cost Test Dynamic Power Cost", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], damage_from_paid_power),
    additional_cost: Some(life(3).and(Cost::sacrifice(1, positive_creature))), ..CardDef::BLANK
};

static LIFE_MANA_SOURCE: CardDef = CardDef {
    name: "Cost Test Life Mana Source", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(life(3), add(Mana::G), "Pay 3 life: Add {G}.")], ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_DYNAMIC_BEHOLD: CardDef = CardDef {
    name: "Cost Test Mana And Dynamic Behold", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { behold: Some((1, positive_creature, "positive-power creature")), ..Cost::NONE }), ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_POWER_CHOICE: CardDef = CardDef {
    name: "Cost Test Mana And Power Choice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_choice: &[ExtraCost { text: "Sacrifice a creature with positive power", cost: Cost::sacrifice(1, positive_creature) }],
    ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_OPTIONAL_POWER: CardDef = CardDef {
    name: "Cost Test Mana And Optional Power", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    optional_cost: Some(OptionalCost::new(Cost::sacrifice(1, positive_creature))),
    labels: & ["Without optional cost", "Sacrifice a creature with positive power"], ..CardDef::BLANK
};
static SPELL_WITH_LIFE_AND_OPTIONAL_POWER: CardDef = CardDef {
    name: "Cost Test Life And Optional Power", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three), additional_cost: Some(life(3)),
    optional_cost: Some(OptionalCost::new(Cost::sacrifice(1, positive_creature))),
    labels: &["Without optional cost", "Sacrifice a creature with positive power"], ..CardDef::BLANK
};

static SPELL_WITH_POWER_AND_OPTIONAL_LIFE: CardDef = CardDef {
    name: "Cost Test Power And Optional Life", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], damage_from_paid_power), additional_cost: Some(Cost::sacrifice(1, positive_creature)),
    optional_cost: Some(OptionalCost::new(life(3))), labels: &["Without optional cost", "Pay 3 life"], ..CardDef::BLANK
};

static SPELL_WITH_LIFE_AND_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Life And Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], damage_from_paid_power),
    additional_cost: Some(life(3).and(Cost::sacrifice(1, is_life_star))), ..CardDef::BLANK
};

fn creature_while_one_mana_remains(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.state().player(a.controller).pool.total() == 1
}
static SPELL_WITH_TWO_MANA_AND_MIDPAYMENT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Mana And Midpayment Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, creature_while_one_mana_remains)), ..CardDef::BLANK
};
fn creature_after_one_mana_was_spent(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.history().mana_spent_to_cast(a.source).map(|(_, mana)| mana.0.iter().map(|n| u32::from(*n)).sum::<u32>()).sum::<u32>() == 1
}
static SPELL_WITH_MANA_RECEIPT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Mana Receipt Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, creature_after_one_mana_was_spent)), ..CardDef::BLANK
};
fn dork_when_bear_is_tapped(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Dork" && q.state().battlefield().iter().any(|&other| q.chars(other).name == "Cost Test Bear" && q.state().obj(other).tapped)
}
static SPELL_WITH_CONVOKE_MIDPAYMENT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Convoke Midpayment Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO }, pay_with: PayWith { convoke: true, ..PayWith::NONE },
    spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, dork_when_bear_is_tapped)), ..CardDef::BLANK
};
fn dork_when_graveyard_is_empty(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Dork" && q.state().zone(Zone::graveyard(a.controller)).is_empty()
}
fn dork_when_artifact_is_tapped(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Dork" && q.state().battlefield().iter().any(|&other| q.chars(other).name == "Cost Test Counter And Self Sacrifice" && q.state().obj(other).tapped)
}
static SPELL_WITH_DELVE_MIDPAYMENT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Delve Midpayment Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO }, pay_with: PayWith { delve: true, ..PayWith::NONE },
    spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, dork_when_graveyard_is_empty)), ..CardDef::BLANK
};
static SPELL_WITH_IMPROVISE_MIDPAYMENT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Improvise Midpayment Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO }, pay_with: PayWith { improvise: true, ..PayWith::NONE },
    spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, dork_when_artifact_is_tapped)), ..CardDef::BLANK
};
static SPELL_WITH_WATERBEND_MIDPAYMENT_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Waterbend Midpayment Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::waterbend(1, 0).and(Cost::sacrifice(1, dork_when_bear_is_tapped))), ..CardDef::BLANK
};
fn tap_batch_match(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event { Event::Tapped { permanent } => Fired::about(Target::Object(*permanent)), _ => None }
}
static TAP_BATCH_WATCHER: CardDef = CardDef {
    name: "Cost Test Tap Batch Watcher", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::TAPPED, tap_batch_match), gain_three, "Whenever one or more permanents become tapped, gain 3 life.").once_per_batch()], ..CardDef::BLANK
};
static SPELL_WITH_SNOW_MANA_AND_LIFE: CardDef = CardDef {
    name: "Cost Test Snow Mana And Life", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: mana("{U}{S}"), spell: spell(&[], gain_three), additional_cost: Some(life(3)), ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_LIFE: CardDef = CardDef {
    name: "Cost Test Mana And Life", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    spell: spell(&[], gain_three), additional_cost: Some(life(3)), ..CardDef::BLANK
};

fn energy_power(q: &dyn LayerQuery, id: ObjectId, chars: &mut Chars) {
    chars.power = Some(q.state().player(q.state().obj(id).controller).energy() as i32);
    chars.toughness = Some(1);
}
static ENERGY_STAR: CardDef = CardDef {
    name: "Cost Test Energy Star", types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, energy_power, "Its power is your energy.")], ..CardDef::BLANK
};
static SPELL_WITH_ENERGY_AND_POWER_COST: CardDef = CardDef {
    name: "Cost Test Energy And Power", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three), additional_cost: Some(Cost { energy: 2, ..Cost::sacrifice(1, positive_creature) }), ..CardDef::BLANK
};
fn power_three(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|n| n >= 3)
}
static SPELL_WITH_COUNTER_AND_POWER_COST: CardDef = CardDef {
    name: "Cost Test Counter And Power", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three), additional_cost: Some(Cost {
        remove_counters_among: Some(CountersAmong { count: 1, filter: is_creature, kind: Some(CounterKind::PlusOne) }),
        ..Cost::sacrifice(1, power_three)
    }), ..CardDef::BLANK
};
fn power_two(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|n| n >= 2)
}
static SPELL_WITH_BLIGHT_AND_POWER_COST: CardDef = CardDef {
    name: "Cost Test Blight And Power", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three), additional_cost: Some(Cost { blight: 1, ..Cost::sacrifice(1, power_two) }), ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_GRAVEYARD_EXILE: CardDef = CardDef {
    name: "Cost Test Mana And Graveyard Exile", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { exile_from_graveyard: 1, ..Cost::NONE }), ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_HAND_EXILE: CardDef = CardDef {
    name: "Cost Test Mana And Hand Exile", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { exile_from_hand: Some((1, is_creature)), ..Cost::NONE }), ..CardDef::BLANK
};
static EVIDENCE_CARD: CardDef = CardDef {
    name: "Cost Test Evidence Card", types: TypeSet::of(&[CardType::Creature]), has_cost: true,
    cost: ManaCost { generic: 3, ..ManaCost::ZERO }, power: Some(1), toughness: Some(1), ..CardDef::BLANK
};
static SPELL_WITH_MANA_AND_EVIDENCE: CardDef = CardDef {
    name: "Cost Test Mana And Evidence", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { collect_evidence: 3, ..Cost::NONE }), ..CardDef::BLANK
};
fn target_defined_exile(_: &dyn Query, _: &Asker, targets: &[Target]) -> Cost {
    Cost { exile_from_graveyard: targets.len() as u8, ..Cost::NONE }
}
static SPELL_WITH_LIFE_AND_EXILE_CHOICE: CardDef = CardDef {
    name: "Cost Test Life And Exile Choice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three), additional_cost: Some(life(1)),
    additional_choice: &[ExtraCost { text: "Exile a graveyard card", cost: Cost { exile_from_graveyard: 1, ..Cost::NONE } }],
    ..CardDef::BLANK
};
static SPELL_WITH_DISCARD_AND_EXILE_CHOICE: CardDef = CardDef {
    name: "Cost Test Discard And Exile Choice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three), additional_cost: Some(discard_cards(1)),
    additional_choice: &[ExtraCost { text: "Exile a graveyard card", cost: Cost { exile_from_graveyard: 1, ..Cost::NONE } }],
    ..CardDef::BLANK
};
static SPELL_WITH_TARGET_EXILE_ONLY: CardDef = CardDef {
    name: "Cost Test Target Exile Only", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[cantrip_mechanics::targets::CREATURE], gain_three), cost_of_targets: Some(target_defined_exile), ..CardDef::BLANK
};
static SPELL_WITH_DISCARD_AND_TARGET_EXILE: CardDef = CardDef {
    name: "Cost Test Discard And Target Exile", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[cantrip_mechanics::targets::CREATURE], gain_three),
    additional_cost: Some(discard_cards(1)), cost_of_targets: Some(target_defined_exile), ..CardDef::BLANK
};
static SPELL_WITH_DISCARD_AND_GRAVEYARD_EXILE: CardDef = CardDef {
    name: "Cost Test Discard And Graveyard Exile", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { discard: Discards::one(1, None), exile_from_graveyard: 1, ..Cost::NONE }), ..CardDef::BLANK
};
static SPELL_WITH_LIFE_AND_VARIABLE_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Life And Variable Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three), additional_cost: Some(life(3)),
    optional_cost: Some(OptionalCost::new(Cost { variable: Some(VariableCost::SacrificeOneOrMore(is_creature)), ..Cost::NONE })),
    labels: &["Without optional cost", "Sacrifice one or more creatures"], ..CardDef::BLANK
};
static SPELL_WITH_BEHOLD_AND_DISCARD: CardDef = CardDef {
    name: "Cost Test Behold And Discard", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { behold: Some((1, is_creature, "creature")), discard: Discards::one(1, None), ..Cost::NONE }), ..CardDef::BLANK
};
const ONE_BEAR_DISCARD: Cost = Cost { discard: Discards::one(1, Some(is_bear)), ..Cost::NONE };
const ONE_DORK_DISCARD: Cost = Cost { discard: Discards::one(1, Some(is_dork)), ..Cost::NONE };
static SPELL_WITH_SEVEN_DISCARDS: CardDef = CardDef {
    name: "Cost Test Seven Discards", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sequence(&[ONE_BEAR_DISCARD, ONE_DORK_DISCARD, ONE_BEAR_DISCARD, ONE_DORK_DISCARD, ONE_BEAR_DISCARD, ONE_DORK_DISCARD, ONE_BEAR_DISCARD])), ..CardDef::BLANK
};
static SPELL_WITH_TEAMWORK_AND_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Teamwork And Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, is_bear)),
    optional_cost: Some(cantrip_mechanics::teamwork::optional(2)), labels: &["Without teamwork", "Teamwork 2"], ..CardDef::BLANK
};
static SPELL_WITH_CONVOKE_AND_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Convoke And Sacrifice", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO }, pay_with: PayWith { convoke: true, ..PayWith::NONE },
    spell: spell(&[], gain_three), additional_cost: Some(Cost::sacrifice(1, is_bear)), ..CardDef::BLANK
};
static ABILITY_WITH_LIFE_AND_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Ability Life And Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(life(3).and(Cost::sacrifice(1, is_life_star)), gain_three, "Pay 3 life, sacrifice the creature: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_TAP_AND_SELF_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Tap And Self Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(TAP.and(SACRIFICE_THIS), gain_three, "{T}, Sacrifice this: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_COUNTER_AND_SELF_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Counter And Self Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(put_counters(CounterKind::PlusOne, 1).and(SACRIFICE_THIS), gain_three, "Put a +1/+1 counter on this, sacrifice it: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_VARIABLE_COUNTER_AND_SELF_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Variable Counter And Self Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::Charge)), sacrifice_self: true, ..Cost::NONE }, gain_three, "Remove any number of charge counters from this, sacrifice it: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_LOYALTY_AND_SELF_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Loyalty And Self Sacrifice", types: TypeSet::of(&[CardType::Planeswalker]),
    abilities: &[ability(Cost { loyalty: Some(1), sacrifice_self: true, ..Cost::NONE }, gain_three, "+1, Sacrifice this: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_EXERT_AND_SELF_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Exert And Self Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { exert: true, sacrifice_self: true, ..Cost::NONE }, gain_three, "Exert and sacrifice this: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_UNATTACH_AND_TAP: CardDef = CardDef {
    name: "Cost Test Unattach And Tap", types: TypeSet::of(&[CardType::Artifact]), subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[ability(Cost { unattach_self: true, tap: true, ..Cost::NONE }, gain_three, "Unattach and tap this: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_CRAFT_AND_LIFE: CardDef = CardDef {
    name: "Cost Test Craft And Life", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { life: 3, exile_self: true, materials: Some(&cantrip_mechanics::craft::ARTIFACT), ..Cost::NONE }, gain_three, "Pay 3 life, exile this and an artifact: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_LIFE_AND_LIBRARY_TOP: CardDef = CardDef {
    name: "Cost Test Life And Library Top", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { life: 3, put_self_into_library: Some(LibraryPlace::Top), ..Cost::NONE }, gain_three, "Pay 3 life, put this on top of its owner's library: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_LIFE_AND_RANDOM_LIBRARY: CardDef = CardDef {
    name: "Cost Test Life And Shuffle", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { life: 3, put_self_into_library: Some(LibraryPlace::ShuffledIn), ..Cost::NONE }, gain_three, "Pay 3 life, shuffle this into its owner's library: gain 3 life.")], ..CardDef::BLANK
};
fn power_ten(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|n| n >= 10)
}
static ABILITY_WITH_RANDOM_AND_DYNAMIC_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Shuffle And Dynamic Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::sacrifice(1, power_ten).and(Cost { life: 3, put_self_into_library: Some(LibraryPlace::ShuffledIn), ..Cost::NONE }), gain_three, "Pay 3 life, sacrifice a creature with power 10 or greater, shuffle this into its owner's library: gain 3 life.")], ..CardDef::BLANK
};
static ABILITY_WITH_TAP_AND_SELF_RETURN: CardDef = CardDef {
    name: "Cost Test Tap And Self Return", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(TAP.and(RETURN_THIS), gain_three, "{T}, Return this to its owner's hand: gain 3 life.")], ..CardDef::BLANK
};
static SPELL_WITH_TWO_GRAVEYARD_EXILES: CardDef = CardDef {
    name: "Cost Test Two Graveyard Exiles", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { exile_from_graveyard: 1, ..Cost::NONE }),
    optional_cost: Some(OptionalCost::new(Cost { exile_from_graveyard: 1, ..Cost::NONE })),
    labels: &["Without optional cost", "Exile another card"], ..CardDef::BLANK
};

static SPELL_WITH_HARMONIZE: CardDef = CardDef {
    name: "Cost Test Harmonize", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three),
    ways: &[cantrip_mechanics::harmonize::way(mana("{3}"), "Harmonize {3}")], ..CardDef::BLANK
};

static SPELL_WITH_TAP_AND_SACRIFICE: CardDef = CardDef {
    name: "Cost Test Spell Tap And Sacrifice", types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true, cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, is_bear)),
    optional_cost: Some(OptionalCost::new(Cost::tap_untapped(1, is_bear))),
    labels: &["Without optional cost", "Tap a Bear"], ..CardDef::BLANK
};

static SPELL_WITH_OVERLAPPING_COSTS: CardDef = CardDef {
    name: "Cost Test Spell With Overlapping Costs", types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true, cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, is_creature)),
    optional_cost: Some(OptionalCost::new(Cost::sacrifice(1, is_bear))),
    labels: &["Without optional cost", "Sacrifice a Bear"], ..CardDef::BLANK
};

static SPELL_WITH_TWO_ACTIONS: CardDef = CardDef {
    name: "Cost Test Spell With Two Actions", types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true, cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, is_bear)),
    optional_cost: Some(OptionalCost::new(Cost::return_to_hand(1, is_dork))),
    labels: &["Without optional cost", "Return a Dork"], ..CardDef::BLANK
};

static SPELL_WITH_TWO_COSTS: CardDef = CardDef {
    name: "Cost Test Spell With Two Costs", types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true, cost: ManaCost::ZERO,
    spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sacrifice(1, is_bear)),
    optional_cost: Some(OptionalCost::new(Cost::sacrifice(1, is_dork))),
    labels: &["Without optional cost", "Sacrifice a Dork"],
    ..CardDef::BLANK
};

static TAP_AND_SACRIFICE_COSTS: CardDef = CardDef {
    name: "Cost Test Tap And Sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::sequence(&[Cost::tap_untapped(1, is_bear), Cost::sacrifice(1, is_bear)]), gain_three, "Tap a Bear and sacrifice a Bear: You gain 3 life.")],
    ..CardDef::BLANK
};

static OVERLAPPING_COSTS: CardDef = CardDef {
    name: "Cost Test Overlapping Costs", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::sequence(&[Cost::sacrifice(1, is_creature), Cost::sacrifice(1, is_bear)]), gain_three, "Sacrifice a creature and a Bear: You gain 3 life.")],
    ..CardDef::BLANK
};

fn mana_from_paid_power(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let n = g.mana_ability_cost_objects(0, CostPart::Permanents).iter().map(|&o| u32::try_from(g.chars(o).power.unwrap_or(0).max(0)).unwrap()).sum();
    g.add_mana(asker.controller, Mana::G, n);
    Ok(())
}
fn mana_after_another_sacrifice(g: &mut dyn cantrip_api::Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let other = g.permanents_of(asker.controller).into_iter().find(|&id| is_dork(g, asker, id)).unwrap();
    g.sacrifice(asker.controller, &[other]);
    mana_from_paid_power(g, asker, color)
}
fn add_mana_ability_cost(_: &dyn Query, _: &Asker, _: Costed, _: &[Target]) -> CostDelta { CostDelta::more(1) }
static MANA_WITH_TAXED_DYNAMIC_SACRIFICE: CardDef = CardDef {
    name: "Mana with taxed dynamic sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sacrifice(1, positive_creature), add(Mana::U), "Sacrifice a positive-power creature: Add {U}.").cost_change(add_mana_ability_cost)], ..CardDef::BLANK
};
static MANA_WITH_NESTED_DYNAMIC_SACRIFICE: CardDef = CardDef {
    name: "Mana with nested dynamic sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{G}").and(Cost::sacrifice(1, positive_creature)), add(Mana::U), "Pay {G}, sacrifice a positive-power creature: Add {U}.")], ..CardDef::BLANK
};
static DYNAMIC_COLOR_STAR: CardDef = CardDef {
    name: "Dynamic color star", types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, reverse_life_power, "Its power is 18 minus your life total.")],
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")], ..CardDef::BLANK
};
static MANA_WITH_DYNAMIC_COLOR: CardDef = CardDef {
    name: "Mana with dynamic color", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(life(3).and(Cost::sacrifice(1, positive_creature)), add_any_color(1), "Pay 3 life, sacrifice a positive-power creature: Add mana it could produce.").colors_from_sacrifice()], ..CardDef::BLANK
};
static MANA_WITH_LIFE_AND_DYNAMIC_SACRIFICE: CardDef = CardDef {
    name: "Mana with life and dynamic sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(life(3).and(Cost::sacrifice(1, positive_creature)), add(Mana::G), "Pay 3 life, sacrifice a positive-power creature: Add {G}.")], ..CardDef::BLANK
};
static MANA_WITH_LIFE_AND_SACRIFICE: CardDef = CardDef {
    name: "Mana with life and sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(life(3).and(Cost::sacrifice(1, is_life_star)), add_by(mana_from_paid_power), "Pay 3 life, sacrifice the creature: Add green mana equal to its power.")], ..CardDef::BLANK
};
fn player_mana_draw(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, 1);
    g.draw(asker.controller, 1);
    Ok(())
}
fn player_mana_draw_with_resolution_payment(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    if g.may_pay(asker.controller, asker.source, asker.source, life(1).and(Cost::return_to_hand(1, is_bear)), &PAY)? {
        g.add_mana(asker.controller, Mana::G, 1);
        g.draw(asker.controller, 1);
    }
    Ok(())
}
static PLAYER_MANA_WITH_LIFE_AND_SACRIFICE: CardDef = CardDef {
    name: "Player mana with life and sacrifice",
    delayed: &[mana_ability(life(3).and(Cost::sacrifice(1, is_life_star)), add_by(mana_from_paid_power), "Pay 3 life, sacrifice the creature: Add green mana equal to its power."),
        mana_ability(cost("{G}").and(life(3)).and(Cost::sacrifice(1, is_life_star)), add_by(mana_from_paid_power), "Pay {G} and 3 life, sacrifice the creature: Add green mana equal to its power."),
        mana_ability(life(3).and(Cost::sacrifice(1, is_life_star)), add_by(player_mana_draw), "Pay life and sacrifice: Add {G} and draw."),
        mana_ability(life(3).and(Cost::sacrifice(1, is_life_star)), add_by(player_mana_draw_with_resolution_payment), "Pay life and sacrifice: May pay life, then add {G} and draw."),
        mana_ability(Cost { variable: Some(VariableCost::SacrificeX(is_creature)), ..Cost::NONE }, add(Mana::G), "Sacrifice X creatures: Add {G}."),
        mana_ability(life(3).and(Cost { variable: Some(VariableCost::SacrificeX(positive_creature)), ..Cost::NONE }), add(Mana::G), "Pay 3 life, sacrifice X positive-power creatures: Add {G}."),
        mana_ability(life(3).and(Cost { variable: Some(VariableCost::SacrificeAnyNumber { filter: positive_creature, less_each: 0 }), ..Cost::NONE }), add(Mana::G), "Pay 3 life, sacrifice any number of positive-power creatures: Add {G}.")],
    ..CardDef::BLANK
};
fn add_temporary_player_mana(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, 1);
    let ability = AbilityRef::delayed(g.card_id(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 2);
    g.add_effect(asker.source, asker.source, EffectKind::PlayerManaAbility(ability), Duration::EndOfTurn);
    Ok(())
}
static TEMPORARY_PLAYER_GRANTER: CardDef = CardDef {
    name: "Temporary player granter", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_by(add_temporary_player_mana), "{T}: Add {G} and grant a player mana ability.")], ..CardDef::BLANK
};
static VANISHING_PLAYER_GRANTER: CardDef = CardDef {
    name: "Vanishing player granter", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(SACRIFICE_THIS, add(Mana::G), "Sacrifice this: Add {G}.")], ..CardDef::BLANK
};
static GRANTED_COLOR_MANA: CardDef = CardDef {
    name: "Granted sacrifice color mana",
    delayed: &[mana_ability(Cost { sacrifice_granter: true, ..Cost::NONE }, add_any_color(1), "Sacrifice the granting permanent: Add mana it could produce.").colors_from_sacrifice()],
    ..CardDef::BLANK
};
static SELF_COLOR_MANA: CardDef = CardDef {
    name: "Self sacrifice color mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        mana_ability(SACRIFICE_THIS, add_any_color(1), "Sacrifice this: Add mana it could produce.").colors_from_sacrifice()],
    ..CardDef::BLANK
};
fn is_red_dork(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).name == "Red mana dork" }
static RED_DORK: CardDef = CardDef {
    name: "Red mana dork", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")], ..CardDef::BLANK
};
static TWO_SACRIFICE_COLOR_MANA: CardDef = CardDef {
    name: "Two sacrifice color mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sequence(&[Cost::sacrifice(1, is_dork), Cost::sacrifice(1, is_red_dork)]), add_any_color(1), "Sacrifice two creatures: Add a type the second could produce.").colors_from_sacrifice_component(1)],
    ..CardDef::BLANK
};
static RETURN_THEN_COLOR_MANA: CardDef = CardDef {
    name: "Return then color mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sequence(&[Cost::return_to_hand(1, is_bear), Cost::sacrifice(1, is_dork)]), add_any_color(1), "Return a Bear, sacrifice a Dork: Add mana it could produce.").colors_from_sacrifice()],
    ..CardDef::BLANK
};
static MANA_AFTER_OTHER_SACRIFICE: CardDef = CardDef {
    name: "Mana after another sacrifice", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sacrifice(1, is_bear), add_by(mana_after_another_sacrifice), "Sacrifice a Bear: Sacrifice a Dork, then add green mana equal to the Bear's power.")],
    ..CardDef::BLANK
};

fn mana_from_two_cost_parts(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let a = g.mana_ability_cost_objects(0, CostPart::Permanents);
    let b = g.mana_ability_cost_objects(1, CostPart::Permanents);
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_ne!(a[0], b[0]);
    let n = u32::try_from(g.chars(a[0]).power.unwrap_or(0).max(0)).unwrap();
    let m = u32::try_from(g.chars(b[0]).power.unwrap_or(0).max(0)).unwrap();
    g.add_mana(asker.controller, Mana::G, n);
    g.add_mana(asker.controller, Mana::B, m);
    Ok(())
}
static MANA_WITH_TWO_COSTS: CardDef = CardDef {
    name: "Mana with two distinct costs", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sequence(&[Cost::sacrifice(1, is_bear), Cost::sacrifice(1, is_dork)]), add_by(mana_from_two_cost_parts), "Sacrifice a Bear and a Dork: Add mana for each.")],
    ..CardDef::BLANK
};

fn replace_sacrifice(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.reason == ZoneReason::Sacrifice && z.from.is(ZoneKind::Battlefield))
}
fn exile_sacrifice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::exile(z.to.player); }
}
fn prevent_sacrifice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.replaced = true; }
}
static PREVENT_SACRIFICES: CardDef = CardDef {
    name: "Cost Test Prevent Sacrifices", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, replace_sacrifice, prevent_sacrifice, "It remains instead.")],
    ..CardDef::BLANK
};

fn color_mana(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, 1);
    g.add_effect(asker.source, asker.source, EffectKind::SetColors(ColorSet::of(Color::Red)), Duration::EndOfTurn);
    Ok(())
}
static COLOR_MANA: CardDef = CardDef {
    name: "Cost Test Color Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_by(color_mana), "{T}: Add {G}. This becomes red until end of turn.")], ..CardDef::BLANK
};
fn red_mana_source_exists(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).name == "Cost Test Color Mana" && q.chars(id).colors.has(Color::Red))
}
static STATE_WATCHER: CardDef = CardDef {
    name: "Cost Test State Watcher", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[state_triggered(red_mana_source_exists, gain_three, "When the mana source is red, gain 3 life.")], ..CardDef::BLANK
};
fn observe_after_payment(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let _ = g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY)?;
    let observed = g.state().triggers.iter().any(|trigger| g.chars(trigger.source).name == "Cost Test State Watcher");
    let mana_source = g.permanents_of(item.controller).into_iter().find(|&id| g.chars(id).name == "Cost Test Color Mana").unwrap();
    // End the condition before the state trigger resolves, so it triggers at most once.
    g.add_effect(item.object, mana_source, EffectKind::SetColors(ColorSet::NONE), Duration::EndOfTurn);
    if observed { g.gain_life(item.controller, 7); }
    Ok(())
}
static PAYMENT_BOUNDARY: CardDef = CardDef {
    name: "Cost Test Payment Boundary", has_cost: true, cost: ManaCost::ZERO, types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], observe_after_payment), ..CardDef::BLANK
};

static EXILE_SACRIFICES: CardDef = CardDef {
    name: "Cost Test Exile Sacrifices", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, replace_sacrifice, exile_sacrifice, "Exile it instead.")],
    ..CardDef::BLANK
};

static OUTER_MANA_COST: CardDef = CardDef {
    name: "Cost Test Outer Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}").and(Cost::sacrifice(1, is_bear)), add_by(mana_from_paid_power), "Pay {1}, sacrifice a Bear: Add green mana equal to its power.")],
    ..CardDef::BLANK
};
static INNER_MANA_COST: CardDef = CardDef {
    name: "Cost Test Inner Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sacrifice(1, is_dork), add(Mana::C), "Sacrifice a Dork: Add {C}.")],
    ..CardDef::BLANK
};

static MANA_FROM_PAID: CardDef = CardDef {
    name: "Mana from paid cost", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sacrifice(1, is_creature), add_by(mana_from_paid_power), "Sacrifice a creature: Add green mana equal to its power.")],
    ..CardDef::BLANK
};

const PAY: Question = Question::new("Sacrifice", "pay {cost}?", "", "Pay", "Don't");

fn may_pay_with(g: &mut dyn cantrip_api::Game, item: &StackItem, cost: Cost) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY)? {
        g.gain_life(item.controller, 5);
    }
    Ok(())
}

/// "You may pay {1} and sacrifice a creature. If you do, you gain 5 life."
fn sacrifice_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{1}").and(Cost::sacrifice(1, is_creature)))
}

/// "You may pay {1} and return a tapped creature you control to its owner's hand. If you do, you gain 5 life."
fn return_tapped(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{1}").and(Cost::return_tapped_to_hand(1, is_creature)))
}

/// "You may discard a card and pay 1 energy. If you do, you gain 5 life."
fn discard_and_energy(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, discard_cards(1).and(Cost { energy: 1, ..Cost::NONE }))
}

/// "You may pay {B/P}. If you do, you gain 5 life." - a Phyrexian symbol in a cost paid as a spell resolves, as Ice Cave's
/// payer meets one in a spell's mana cost: paid with {B} or 2 life (CR 107.4f, 118.13b).
fn phyrexian_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{B/P}"))
}

/// "You may pay {2/G} 150 times and {2/U} 150 times. If you do, you gain 5 life." - more monocolored hybrid symbols than a
/// choice of modes holds, which no printed card asks: how each is paid is still announced (CR 107.4e, 118.13b).
fn many_twobrids(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let mut twobrid = [0; 5];
    twobrid[Color::Blue as usize] = 150;
    twobrid[Color::Green as usize] = 150;
    may_pay_with(g, item, Cost::mana(ManaCost { twobrid, ..ManaCost::ZERO }))
}

/// "You may sacrifice 300 creatures. If you do, you gain 5 life." - a count past a byte, which a cumulative upkeep reaches with
/// 300 age counters (CR 702.24a).
fn sacrifice_three_hundred(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost::sacrifice(300, is_creature))
}

/// A loyalty cost, which no resolving spell asks for (CR 606.4): never paid.
fn loyalty_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { loyalty: Some(-1), ..Cost::NONE })
}

fn high_life_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { life: 65_536, ..Cost::NONE })
}

fn impossible_life_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { life: i32::MAX as u32 + 1, ..Cost::NONE })
}

fn resolution_life_then_sacrifice(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, life(3).and(Cost::sacrifice(1, positive_creature)))
}
fn resolution_one_payment(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost::sacrifice(1, is_creature))
}
static RESOLUTION_ONE_PAYMENT_ABILITY: CardDef = CardDef {
    name: "Resolution one payment ability", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::NONE, resolution_one_payment, "You may sacrifice a creature; if you do, gain 5 life.")], ..CardDef::BLANK
};
static RESOLUTION_ONE_PAYMENT: CardDef = CardDef {
    name: "Resolution one payment", types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], resolution_one_payment), ..CardDef::BLANK
};
fn resolution_two_payments(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let cost = Cost::sacrifice(1, is_creature);
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY)?
        && g.may_pay(item.controller, item.object, item.object, cost, &PAY)? {
        g.gain_life(item.controller, 5);
    }
    Ok(())
}
static RESOLUTION_TWO_PAYMENTS: CardDef = CardDef {
    name: "Resolution two payments", types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], resolution_two_payments), ..CardDef::BLANK
};
static RESOLUTION_LIFE_AND_SACRIFICE: CardDef = CardDef {
    name: "Resolution life and sacrifice", types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], resolution_life_then_sacrifice), ..CardDef::BLANK
};
static OFFER: CardDef = CardDef { name: "Cost Test Offer", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], sacrifice_one), ..CardDef::BLANK };
static RECALL: CardDef = CardDef { name: "Cost Test Recall", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], return_tapped), ..CardDef::BLANK };
static TITHE: CardDef = CardDef { name: "Cost Test Tithe", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], discard_and_energy), ..CardDef::BLANK };
static TOLL: CardDef = CardDef { name: "Cost Test Toll", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], phyrexian_cost), ..CardDef::BLANK };
static HOARD: CardDef = CardDef { name: "Cost Test Hoard", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], many_twobrids), ..CardDef::BLANK };
static MASS_OFFER: CardDef = CardDef { name: "Cost Test Mass Offer", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], sacrifice_three_hundred), ..CardDef::BLANK };
/// "Put 300 -1/-1 counters on this creature: You gain 3 life." - a count past a byte (CR 702.24a, 118.3).
static WITHERER: CardDef = CardDef {
    name: "Cost Test Witherer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1000),
    toughness: Some(1000),
    abilities: &[ability(put_counters(CounterKind::MinusOne, 300), gain_three, "Put 300 -1/-1 counters on this creature: You gain 3 life.")],
    ..CardDef::BLANK
};
static VOW: CardDef = CardDef { name: "Cost Test Vow", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], loyalty_cost), ..CardDef::BLANK };
static HIGH_LIFE: CardDef = CardDef { name: "Cost Test High Life", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], high_life_cost), ..CardDef::BLANK };
static IMPOSSIBLE_LIFE: CardDef = CardDef { name: "Cost Test Impossible Life", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], impossible_life_cost), ..CardDef::BLANK };

/// "Whenever a card is put into a hand or a library, you gain 2 life" (CR 603.10a: it looks back for a card all players can
/// see).
fn a_card_into_a_hand_or_library(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, to, .. } if (to.is(ZoneKind::Hand) || to.is(ZoneKind::Library)) && !q.obj(*old).token => Fired::PLAIN,
        _ => None,
    }
}

fn gain_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

static LIBRARIAN: CardDef = CardDef {
    name: "Cost Test Librarian",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, a_card_into_a_hand_or_library), gain_two, "Whenever a card is put into a hand or a library, you gain 2 life.")],
    ..CardDef::BLANK
};

/// An Arcane sorcery for {1}: "You gain 3 life."
static RITE: CardDef = CardDef {
    name: "Cost Test Rite",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    subtypes: SubtypeSet::of(&[Subtype::Arcane]),
    spell: spell(&[], gain_three),
    ..CardDef::BLANK
};

/// "You gain 3 life. Splice onto Arcane {1}."
static GRAFT: CardDef = CardDef {
    name: "Cost Test Graft",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], gain_three),
    splice: Some(ManaCost { generic: 1, ..ManaCost::ZERO }),
    ..CardDef::BLANK
};

/// "{T}: Add {C}. Put each card in your hand on top of your library, and each Librarian you control into your graveyard,
/// at once": a mana ability whose effect moves a card the spell being cast revealed (CR 605.1a, 601.2g).
fn stow(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::C, 1);
    let hand = g.state().zone(Zone::hand(asker.controller)).to_vec();
    let librarians: Vec<ObjectId> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).name == "Cost Test Librarian").collect();
    let moves: Vec<(ObjectId, Zone)> = hand
        .iter()
        .map(|&c| (c, Zone::library(asker.controller)))
        .chain(librarians.iter().map(|&l| (l, Zone::graveyard(asker.controller))))
        .collect();
    g.move_together(&moves, MoveReason::Other);
    Ok(())
}

static STOWER: CardDef = CardDef {
    name: "Cost Test Stower",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C).by(stow), "{T}: Add {C}. Put each card in your hand on top of your library.")],
    ..CardDef::BLANK
};

// The grant's exact linked identity is part of the context for a cost's eligibility and final object choice.
fn is_granting_object(_: &dyn Query, asker: &Asker, object: ObjectId) -> bool {
    !asker.spell && asker.by.is_some_and(|by| by.object == object)
}
static GRANTED_CONTEXT_CHOOSE: CardDef = CardDef {
    name: "Granted choose context probe",
    delayed: &[ability(Cost { choose: Some((is_granting_object, "granting permanent")), ..Cost::NONE }, gain_three, "Choose the granting permanent: Gain 3 life.")],
    ..CardDef::BLANK
};

// A spell-only chooser becomes eligible after its controller pays life; the choice itself does not change state.
fn spell_bear_after_life(q: &dyn Query, asker: &Asker, object: ObjectId) -> bool {
    asker.spell && q.state().player(asker.controller).life < 20 && is_bear(q, asker, object)
}
static CHOOSE_ONLY_SPELL: CardDef = CardDef {
    name: "Choose-only spell probe", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost { choose: Some((is_creature, "creature")), ..Cost::NONE }), ..CardDef::BLANK
};
static LIFE_THEN_CHOOSE_SPELL: CardDef = CardDef {
    name: "Life then choose spell probe", types: TypeSet::of(&[CardType::Sorcery]), has_cost: true,
    cost: ManaCost::ZERO, spell: spell(&[], gain_three),
    additional_cost: Some(Cost::sequence(&[life(1), Cost { choose: Some((spell_bear_after_life, "spell Bear")), ..Cost::NONE }])), ..CardDef::BLANK
};

// Two independently named choices on an Arcane spell exercise declaration order against payment order and splice data.
fn chosen_pair_effect(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = g.cost_objects(item);
    if objects.len() == 2 && g.chars(objects[0]).name == "Cost Test Bear" && g.chars(objects[1]).name == "Cost Test Dork" {
        g.gain_life(item.controller, 10);
    }
    Ok(())
}
static ARCANE_CHOOSE_PAIR: CardDef = CardDef {
    name: "Arcane choose pair probe",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    subtypes: SubtypeSet::of(&[Subtype::Arcane]),
    additional_cost: Some(Cost::sequence(&[
        Cost { choose: Some((is_bear, "Bear")), ..Cost::NONE },
        Cost { choose: Some((is_dork, "Dork")), ..Cost::NONE },
    ])),
    spell: spell(&[], chosen_pair_effect),
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&CHOOSE_ONLY_SPELL, &LIFE_THEN_CHOOSE_SPELL, &GRANTED_CONTEXT_CHOOSE, &ARCANE_CHOOSE_PAIR, &FILLER, &BEAR, &DORK, &SEER, &GAZER, &RETURNER, &OFFER, &RECALL, &TITHE, &VOW, &RECOILER, &LIBRARIAN, &RITE, &GRAFT, &STOWER, &TOLL, &HOARD, &HIGH_LIFE, &IMPOSSIBLE_LIFE, &MASS_OFFER, &WITHERER, &RESOLUTION_LIFE_AND_SACRIFICE, &RESOLUTION_ONE_PAYMENT_ABILITY, &RESOLUTION_ONE_PAYMENT, &RESOLUTION_TWO_PAYMENTS, &TWO_DISTINCT_COSTS, &CHOSEN_DIFFERENT_COSTS, &MULTIKICK_DIFFERENT_COSTS, &MANA_FROM_PAID, &MANA_WITH_LIFE_AND_SACRIFICE, &MANA_WITH_LIFE_AND_DYNAMIC_SACRIFICE, &DYNAMIC_COLOR_STAR, &MANA_WITH_DYNAMIC_COLOR, &MANA_WITH_NESTED_DYNAMIC_SACRIFICE, &MANA_WITH_TAXED_DYNAMIC_SACRIFICE, &PLAYER_MANA_WITH_LIFE_AND_SACRIFICE, &TEMPORARY_PLAYER_GRANTER, &VANISHING_PLAYER_GRANTER, &GRANTED_COLOR_MANA, &SELF_COLOR_MANA, &RED_DORK, &TWO_SACRIFICE_COLOR_MANA, &RETURN_THEN_COLOR_MANA, &OVERLAPPING_COSTS, &MANA_AFTER_OTHER_SACRIFICE, &MANA_WITH_TWO_COSTS, &TAP_AND_SACRIFICE_COSTS, &SPELL_WITH_TWO_COSTS, &SPELL_WITH_TWO_ACTIONS, &OUTER_MANA_COST, &INNER_MANA_COST, &EXILE_SACRIFICES, &SPELL_WITH_OVERLAPPING_COSTS, &SPELL_WITH_TAP_AND_SACRIFICE, &PREVENT_SACRIFICES, &COLOR_MANA, &STATE_WATCHER, &PAYMENT_BOUNDARY, &LIFE_STAR, &SPELL_WITH_LIFE_AND_SACRIFICE, &REVERSE_LIFE_STAR, &SPELL_WITH_LIFE_AND_POWER_COST, &SPELL_WITH_MANA_AND_LIFE, &SPELL_WITH_TWO_MANA_AND_MIDPAYMENT_SACRIFICE, &SPELL_WITH_SNOW_MANA_AND_LIFE, &SPELL_WITH_MANA_RECEIPT_SACRIFICE, &SPELL_WITH_CONVOKE_MIDPAYMENT_SACRIFICE, &SPELL_WITH_DELVE_MIDPAYMENT_SACRIFICE, &SPELL_WITH_IMPROVISE_MIDPAYMENT_SACRIFICE, &SPELL_WITH_WATERBEND_MIDPAYMENT_SACRIFICE, &TAP_BATCH_WATCHER, &SPELL_WITH_HARMONIZE, &SPELL_WITH_LIFE_AND_OPTIONAL_POWER, &LIFE_MANA_SOURCE, &SPELL_WITH_MANA_AND_OPTIONAL_POWER, &SPELL_WITH_MANA_AND_POWER_CHOICE, &SPELL_WITH_MANA_AND_DYNAMIC_BEHOLD, &SPELL_WITH_POWER_AND_OPTIONAL_LIFE, &ENERGY_STAR, &SPELL_WITH_ENERGY_AND_POWER_COST, &SPELL_WITH_COUNTER_AND_POWER_COST, &SPELL_WITH_BLIGHT_AND_POWER_COST, &SPELL_WITH_MANA_AND_GRAVEYARD_EXILE, &SPELL_WITH_MANA_AND_HAND_EXILE, &SPELL_WITH_MANA_AND_EVIDENCE, &SPELL_WITH_DISCARD_AND_GRAVEYARD_EXILE, &SPELL_WITH_DISCARD_AND_TARGET_EXILE, &SPELL_WITH_TARGET_EXILE_ONLY, &SPELL_WITH_DISCARD_AND_EXILE_CHOICE, &SPELL_WITH_LIFE_AND_EXILE_CHOICE, &SPELL_WITH_LIFE_AND_VARIABLE_SACRIFICE, &SPELL_WITH_BEHOLD_AND_DISCARD, &SPELL_WITH_SEVEN_DISCARDS, &SPELL_WITH_TEAMWORK_AND_SACRIFICE, &SPELL_WITH_CONVOKE_AND_SACRIFICE, &ABILITY_WITH_LIFE_AND_SACRIFICE, &ABILITY_WITH_TAP_AND_SELF_SACRIFICE, &ABILITY_WITH_TAP_AND_SELF_RETURN, &ABILITY_WITH_COUNTER_AND_SELF_SACRIFICE, &ABILITY_WITH_VARIABLE_COUNTER_AND_SELF_SACRIFICE, &ABILITY_WITH_LOYALTY_AND_SELF_SACRIFICE, &ABILITY_WITH_EXERT_AND_SELF_SACRIFICE, &ABILITY_WITH_UNATTACH_AND_TAP, &ABILITY_WITH_CRAFT_AND_LIFE, &ABILITY_WITH_LIFE_AND_LIBRARY_TOP, &ABILITY_WITH_LIFE_AND_RANDOM_LIBRARY, &ABILITY_WITH_RANDOM_AND_DYNAMIC_SACRIFICE, &EVIDENCE_CARD, &SPELL_WITH_TWO_GRAVEYARD_EXILES])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    // What is put on the battlefield below has been there since before the turn began: no creature is summoning sick.
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn add_mana(g: &mut Game, n: u32) {
    let source = g.state.battlefield().first().copied().unwrap_or(ObjectId(0));
    g.state.add_mana(YOU, Mana::C, n, source, None, ManaLasts::Step, false);
}

/// Runs to the next question and gives it back.
fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().unwrap_or_else(|| panic!("no question: over {} outcome {:?} agenda {}", g.is_over(), g.state.outcome, g.state.agenda.len()))
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Pay the first currently offered cost operation until the announcement completes.
fn finish_cost_order(g: &mut Game) {
    loop {
        let r = next(g);
        if !matches!(r.kind, RequestKind::ChooseCostPayment { .. }) { return; }
        answer(g, &r, Answer::CostPayment(0));
    }
}

/// The option `YOU` have to activate `source`'s ability, or to cast `source`.
fn option_of(g: &Game, r: &Request, source: ObjectId) -> Option<ActionOption> {
    match &r.kind {
        RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source)).cloned(),
        RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.source == Some(source)).cloned(),
        _ => None,
    }
    .filter(|_| r.player == YOU && !g.is_over())
}

/// Both players pass until the stack is empty, answering nothing else.
fn resolve_all(g: &mut Game) -> Request {
    loop {
        let r = next(g);
        if g.state.stack.is_empty() || !matches!(r.kind, RequestKind::Priority { .. }) {
            return r;
        }
        answer(g, &r, Answer::Pass);
    }
}

#[test]
fn an_activated_ability_beholds_a_permanent_once_its_mana_is_paid_and_not_without_one() {
    let mut g = game();
    let seer = put(&mut g, &SEER, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    // Nothing to behold: not offered (CR 602.2b, 118.3).
    assert!(!priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(seer)));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, seer).expect("the Seer's ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let mut r = next(&mut g);
    if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
        let mana = choices.iter().position(|choice| choice.kind == CostPaymentKind::Mana).expect("mana can be paid first");
        answer(&mut g, &r, Answer::CostPayment(mana));
        r = next(&mut g);
    }
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Behold, .. } => {
            assert_eq!(crate::first_choices(&g, &r), vec![bear]);
            assert_eq!(g.state.player(YOU).pool.total(), 0, "the mana is paid before the behold is chosen");
        }
        other => panic!("expected the behold, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
    // The Bear was only chosen: it is still there (CR 701.4a).
    assert!(g.state.obj(bear).zone.is(ZoneKind::Battlefield));
}

#[test]
fn an_activated_ability_may_behold_its_own_source() {
    let mut g = game();
    let gazer = put(&mut g, &GAZER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, gazer).expect("the Gazer may behold itself");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Behold, .. } => assert_eq!(crate::first_choices(&g, &r), vec![gazer]),
        other => panic!("expected the behold, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![gazer]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
}

/// A card in the hand is revealed to behold it (CR 701.4a).
#[test]
fn an_activated_ability_beholds_a_card_from_the_hand_by_revealing_it() {
    let mut g = game();
    let seer = put(&mut g, &SEER, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let activate = option_of(&g, &r, seer).expect("offered");
    answer(&mut g, &r, Answer::Act(Action::of(&activate)));
    let mut r = next(&mut g);
    if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
        let mana = choices.iter().position(|choice| choice.kind == CostPaymentKind::Mana).expect("mana can be paid first");
        answer(&mut g, &r, Answer::CostPayment(mana));
        r = next(&mut g);
    }
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    assert!(g.log.iter().any(|rec| matches!(&rec.event, Event::Revealed { card, .. } if *card == bear)), "the Bear was revealed");
    // It stays revealed until the ability leaves the stack (CR 701.20a): one of the objects its cost chose.
    let item = *g.state.stack.last().expect("the ability is on the stack");
    assert_eq!(g.state.members(item.members), &[bear]);
    let cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert!(crate::ops::seen_by_all(&cx, bear, false), "the Bear is public while the ability is on the stack");
}

/// The Returner's {1} is paid by tapping the Dork for mana while it is activated (CR 605.3a, 601.2g); the tapped creature
/// it returns is chosen after that (CR 601.2h), so the Dork it just tapped is one.
#[test]
fn a_mana_ability_returns_a_tapped_creature_chosen_once_its_own_mana_is_paid() {
    let mut g = game();
    let returner = put(&mut g, &RETURNER, Zone::BATTLEFIELD);
    // No creature at all: not offered.
    assert!(!priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(returner)));
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, returner).expect("the Dork could be tapped for its mana: offered");
    assert!(option.target_candidates.is_empty(), "the tapped creature is not chosen with the activation");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let tap_dork = option_of(&g, &r, dork).expect("the Dork's mana ability is offered while paying");
    answer(&mut g, &r, Answer::Act(Action::of(&tap_dork)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
        let mana = choices.iter().position(|c| c.kind == CostPaymentKind::Mana).expect("mana can be paid first");
        answer(&mut g, &r, Answer::CostPayment(mana));
        r = next(&mut g);
        if let RequestKind::PayCosts { options, .. } = &r.kind {
            let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
            answer(&mut g, &r, Answer::Act(Action::of(confirm)));
            r = next(&mut g);
        }
    }
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![dork]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert!(g.state.obj(dork).became.is_some_and(|d| g.state.obj(d).zone == Zone::hand(YOU)), "the Dork went back to the hand");
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 3);
}

/// Casts `spell` (free: it has no mana cost) and passes until it resolves, giving back the payment question.
fn cast_to_payment(g: &mut Game, spell: ObjectId) -> Request {
    let r = next(g);
    let cast = option_of(g, &r, spell).expect("the spell is offered");
    answer(g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(g);
    assert!(matches!(r.kind, RequestKind::UnlessPays { .. }), "expected the payment, got {:?}", r.kind);
    r
}

fn pays(r: &Request) -> Option<ActionOption> {
    match &r.kind {
        RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::PayCost).cloned(),
        _ => None,
    }
}

#[test]
fn resolution_life_payment_above_u16_is_exact() {
    let mut g = game();
    g.state.update_player(YOU, |p| p.life = 65_535);
    let spell = put(&mut g, &HIGH_LIFE, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "65,535 life cannot pay 65,536");
    assert_eq!(g.state.player(YOU).life, 65_535);

    let mut g = game();
    g.state.update_player(YOU, |p| p.life = i32::MAX);
    let spell = put(&mut g, &IMPOSSIBLE_LIFE, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "a life cost above i32::MAX cannot be paid");
    assert_eq!(g.state.player(YOU).life, i32::MAX);

    let mut g = game();
    g.state.update_player(YOU, |p| p.life = 65_536);
    let spell = put(&mut g, &HIGH_LIFE, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, spell);
    assert!(matches!(r.kind, RequestKind::UnlessPays { life: 65_536, .. }));
    let pay = pays(&r).expect("65,536 life pays the full cost");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 5, "full cost paid before the effect gains five life");
}

#[test]
fn a_resolving_spell_takes_mana_and_a_sacrificed_creature_chosen_as_it_is_paid() {
    // No mana: only declining.
    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none());
    // No creature: only declining.
    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none());

    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    let r = cast_to_payment(&mut g, offer);
    let pay = pays(&r).expect("the pool and the Bear pay it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let operation = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(operation));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. } => assert_eq!(crate::first_choices(&g, &r), vec![bear]),
        other => panic!("expected the sacrifice, got {other:?}"),
    }
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert!(g.state.zone(Zone::graveyard(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&BEAR)), "the Bear was sacrificed");
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

/// "Return a tapped creature" during a resolution, the Dork untapped: paying is offered once its mana ability has tapped
/// it for the {1} (CR 605.3a), and the Dork is then the tapped creature returned.
#[test]
fn a_resolving_spells_tapped_creature_is_judged_after_the_mana_abilities() {
    let mut g = game();
    let recall = put(&mut g, &RECALL, Zone::hand(YOU));
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = cast_to_payment(&mut g, recall);
    assert!(pays(&r).is_none(), "nothing tapped and no mana yet");
    let tap = option_of(&g, &r, dork).expect("the Dork's mana ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&tap)));
    let r = next(&mut g);
    let pay = pays(&r).expect("the Dork is tapped and its {G} pays the {1}");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let operation = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(operation));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![dork]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert!(g.state.zone(Zone::hand(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&DORK)));
}

/// A discard and energy: offered only with a card and the energy; the card chosen is discarded and the energy paid. A
/// choice given up is undone, and the payer asked again.
#[test]
fn a_resolving_spell_takes_a_discard_and_energy_and_a_choice_given_up_asks_again() {
    let mut g = game();
    let tithe = put(&mut g, &TITHE, Zone::hand(YOU));
    let card = put(&mut g, &BEAR, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, tithe);
    assert!(pays(&r).is_none(), "no energy");

    let mut g = game();
    let tithe = put(&mut g, &TITHE, Zone::hand(YOU));
    let card2 = put(&mut g, &BEAR, Zone::hand(YOU));
    let _ = card;
    crate::ops::change_energy(&mut cx(&mut g), YOU, 2, YOU);
    let r = cast_to_payment(&mut g, tithe);
    let pay = pays(&r).expect("a card and the energy");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let operation = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Discard).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(operation));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Discard, .. }));
    // Given up: asked to pay again.
    answer(&mut g, &r, Answer::Cancel);
    let r = next(&mut g);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::ResolutionPaymentBegan { .. })), "the abandoned payment left no receipt");
    let pay = pays(&r).expect("asked again");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let operation = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Discard).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(operation));
    let r = next(&mut g);
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![card2]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert_eq!(g.state.player(YOU).energy(), 1);
    assert!(g.state.zone(Zone::graveyard(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&BEAR)));
    let payments: Vec<_> = g.events().iter().filter_map(|r| match r.event { Event::ResolutionPaymentBegan { payment, .. } => Some(payment), _ => None }).collect();
    assert_eq!(payments.len(), 1);
    assert_eq!(g.history().resolution_cost_objects(payments[0], 0, CostPart::Discard(0)).collect::<Vec<_>>(), vec![card2]);
}

/// A loyalty cost is no resolution's: never paid, and nothing is asked.
#[test]
fn a_resolving_spell_never_pays_a_part_only_an_activation_has() {
    let mut g = game();
    let vow = put(&mut g, &VOW, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, vow).expect("offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let life = g.state.player(YOU).life;
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }) && g.state.stack.is_empty());
    assert_eq!(g.state.player(YOU).life, life);
}

/// "{T}, Return a tapped creature you control": the parts are paid in any order (CR 601.2h by way of 602.2b), so {T} taps
/// the Recoiler and it is the tapped creature returned - offered with no other creature, and the only one to choose.
#[test]
fn a_cost_with_its_own_tap_returns_its_source_as_the_tapped_creature() {
    let mut g = game();
    let recoiler = put(&mut g, &RECOILER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, recoiler).expect("offered: its own {T} taps it");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![recoiler]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![recoiler]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
    assert!(g.state.zone(Zone::hand(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&RECOILER)), "it went back to the hand");
}

/// A card spliced onto a spell is revealed from the spell's announcement until it leaves the stack (CR 701.20a, 702.47a),
/// before it has become cast (CR 601.2i) as much as after: a mana ability activated while the spell's cost is paid (CR
/// 601.2g) puts the hand into the library as the Librarian dies, and the Librarian, judged as it was just before (CR
/// 603.10a), triggers for the spliced card, one all players could see. Not spliced, every card it moves is hidden.
#[test]
fn a_card_revealed_for_a_spell_being_cast_is_looked_back_on_as_a_mana_ability_moves_it() {
    for spliced in [true, false] {
        let mut g = game();
        let librarian = put(&mut g, &LIBRARIAN, Zone::BATTLEFIELD);
        let stower = put(&mut g, &STOWER, Zone::BATTLEFIELD);
        let rite = put(&mut g, &RITE, Zone::hand(YOU));
        let graft = put(&mut g, &GRAFT, Zone::hand(YOU));
        add_mana(&mut g, if spliced { 1 } else { 0 });
        let r = next(&mut g);
        let cast = option_of(&g, &r, rite).expect("the Rite may be cast");
        answer(&mut g, &r, Answer::Act(Action::of(&cast)));
        let r = next(&mut g);
        assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Splice, .. }), "splice is offered: {:?}", r.kind);
        answer(&mut g, &r, Answer::Cards(if spliced { vec![graft] } else { Vec::new() }));
        let r = next(&mut g);
        let stow = option_of(&g, &r, stower).expect("the Stower's mana ability may pay");
        answer(&mut g, &r, Answer::Act(Action::of(&stow)));
        assert!(g.state.obj(graft).became.is_some_and(|c| g.state.obj(c).zone.is(ZoneKind::Library)), "the Graft is in the library");
        assert!(!g.state.battlefield().contains(&librarian));
        let triggered = g.state.triggers.iter().any(|t| t.source == librarian)
            || g.state.stack.iter().any(|i| i.kind == StackKind::Triggered && i.object == librarian);
        assert_eq!(triggered, spliced, "the Librarian looks back only on a card all players could see");
    }
}

/// A Phyrexian symbol in a cost paid as a spell resolves is paid with its color or 2 life, chosen immediately before the cost
/// is paid (CR 107.4f, 118.13b): with no mana, paying is offered and takes 2 life; with {B} the payer may pay it with the
/// mana instead; with neither {B} nor 2 life to pay, only declining is offered.
#[test]
fn a_phyrexian_symbol_in_a_cost_paid_as_a_spell_resolves_is_paid_with_its_color_or_two_life() {
    // No mana: 2 life.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, toll);
    let pay = pays(&r).expect("2 life pays it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseModes { modes, at_most, .. } => {
            assert_eq!(modes, &vec![Label::PayLife(Color::Black)]);
            assert_eq!(*at_most, 1);
        }
        other => panic!("expected the choice of life for {{B/P}}, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Modes(vec![0]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 20 - 2 + 5, "2 life paid, 5 gained");

    // {B} in the pool: paid with the mana, no life.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    g.state.add_mana(YOU, Mana::B, 1, toll, None, ManaLasts::Step, false);
    let r = cast_to_payment(&mut g, toll);
    let pay = pays(&r).expect("{B} pays it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "how {{B/P}} is paid is asked: {:?}", r.kind);
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 25, "no life paid");
    assert_eq!(g.state.player(YOU).pool.get(Mana::B), 0, "the {{B}} was spent");

    // Neither {B} nor 2 life: only declining.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    g.state.update_player(YOU, |p| p.life = 1);
    let r = cast_to_payment(&mut g, toll);
    assert!(pays(&r).is_none(), "a {{B/P}} with no {{B}} and 1 life can't be paid");
}

/// 300 monocolored hybrid symbols in a cost paid as a spell resolves, more than a choice of modes holds (255): which are
/// paid with {2} is asked a page at a time, 255 then 45, none of them lost to a count cut to a byte (CR 107.4e, 118.13b).
#[test]
fn more_monocolored_hybrid_symbols_than_a_choice_holds_are_asked_a_page_at_a_time() {
    let mut g = game();
    let hoard = put(&mut g, &HOARD, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, hoard).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("which {{2/C}} to pay with {{2}}, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (255, 255));
    assert!(modes.iter().all(|m| matches!(m, Label::PayGeneric(_))));
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("the rest, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (45, 45));
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::UnlessPays { .. }), "then the payment, not {:?}", r.kind);
}

/// A cost of 300 creatures sacrificed is paid in full: not a cost nobody can pay, and not a smaller one.
#[test]
fn a_sacrifice_cost_past_a_byte_is_paid_in_full() {
    let mut g = game();
    let offer = put(&mut g, &MASS_OFFER, Zone::hand(YOU));
    let bears: Vec<ObjectId> = (0..300).map(|_| put(&mut g, &BEAR, Zone::BATTLEFIELD)).collect();
    let r = cast_to_payment(&mut g, offer);
    let pay = pays(&r).expect("300 creatures pay 300 sacrificed");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(bears));
    next(&mut g);
    assert_eq!(g.state.zone(Zone::graveyard(YOU)).iter().filter(|&&c| g.state.obj(c).card == g.cards.id_of(&BEAR)).count(), 300);
    assert_eq!(g.state.player(YOU).life, life + 5);

    // One creature short is not payable.
    let mut g = game();
    let offer = put(&mut g, &MASS_OFFER, Zone::hand(YOU));
    for _ in 0..299 {
        put(&mut g, &BEAR, Zone::BATTLEFIELD);
    }
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none(), "299 creatures cannot sacrifice 300");
}

/// A cost of 300 counters put on the creature is paid in full: 300 -1/-1 counters, as 300 age counters ask of Aboroth.
#[test]
fn a_counters_cost_past_a_byte_puts_every_counter() {
    let mut g = game();
    let witherer = put(&mut g, &WITHERER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, witherer).expect("300 counters can be put on it");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    resolve_all(&mut g);
    assert_eq!(g.state.obj(witherer).counters(CounterKind::MinusOne), 300);
}

#[test]
fn separate_sacrifice_cost_components_keep_their_own_filters() {
    let mut g = game();
    let source = put(&mut g, &TWO_DISTINCT_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    assert!(priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(source)),
        "one Bear and one Dork pay two separate sacrifice components");
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    resolve_all(&mut g);
    assert!(g.state.obj(bear).became.is_some());
    assert!(g.state.obj(dork).became.is_some());
}

#[test]
fn mana_ability_reads_the_exact_permanent_paid_for_its_cost() {
    let mut g = game();
    let source = put(&mut g, &MANA_FROM_PAID, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability is available");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(bear)], ..Action::of(&option) }));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2, "the callback reads the Bear as it was sacrificed");
}

#[test]
fn choosing_an_unpayable_order_reverses_the_activation_and_can_be_retried() {
    let mut g = game();
    let source = put(&mut g, &OVERLAPPING_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the cost is payable");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&bear));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "taking the Bear first leaves the second component unpayable");
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&bear) && g.state.zone(Zone::BATTLEFIELD).contains(&dork));
    let option = option_of(&g, &r, source).expect("the activation can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    resolve_all(&mut g);
    assert!(g.state.obj(bear).became.is_some() && g.state.obj(dork).became.is_some());
}

#[test]
fn a_later_unrelated_sacrifice_does_not_replace_the_mana_cost_object() {
    let mut g = game();
    let source = put(&mut g, &MANA_AFTER_OTHER_SACRIFICE, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability is available");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(bear)], ..Action::of(&option) }));
    next(&mut g);
    assert!(g.state.obj(dork).became.is_some(), "an unrelated permanent left after the paid Bear");
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2);
}

#[test]
fn mana_callback_distinguishes_two_separate_cost_components() {
    let mut g = game();
    let source = put(&mut g, &MANA_WITH_TWO_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(g.state.player(YOU).pool.get(Mana::B), 1);
}

#[test]
fn separate_tap_and_sacrifice_components_can_use_the_same_permanent() {
    let mut g = game();
    let source = put(&mut g, &TAP_AND_SACRIFICE_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the same Bear can pay both different actions");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Tap).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { .. }), "second prompt: {:?}", r.kind);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    resolve_all(&mut g);
    assert!(g.state.obj(bear).became.is_some());
    assert!(g.state.obj(bear).tapped, "the Bear was tapped as it left");
}

#[test]
fn printed_and_optional_spell_costs_record_distinct_objects() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TWO_COSTS, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }));
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    finish_cost_order(&mut g);
    let stack_spell = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::Optional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn printed_sacrifice_and_optional_return_are_separate_costs() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TWO_ACTIONS, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    finish_cost_order(&mut g);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.obj(dork).became.is_some_and(|id| g.state.obj(id).zone == Zone::hand(YOU)));
    let stack_spell = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::Optional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn ability_copies_retain_the_originals_distinct_paid_cost_objects() {
    let mut g = game();
    let source = put(&mut g, &TWO_DISTINCT_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    let first = crate::ops::copy_ability(&mut cx(&mut g), Target::Ability(serial), YOU, false).unwrap().unwrap();
    let second = crate::ops::copy_ability(&mut cx(&mut g), Target::Ability(first), YOU, false).unwrap().unwrap();
    for item in [serial, first, second] {
        assert_eq!(g.history().ability_cost_objects(item, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
        assert_eq!(g.history().ability_cost_objects(item, 1, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
    }
}

#[test]
fn mana_cost_objects_survive_a_nested_sacrificial_mana_activation() {
    let mut g = game();
    let outer = put(&mut g, &OUTER_MANA_COST, Zone::BATTLEFIELD);
    let inner = put(&mut g, &INNER_MANA_COST, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, outer).expect("the outer ability is payable with the inner ability");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(bear)], ..Action::of(&option) }));
    let r = next(&mut g);
    let option = option_of(&g, &r, inner).expect("the inner mana ability is offered during payment");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(dork)], ..Action::of(&option) }));
    loop {
        let r = next(&mut g);
        match &r.kind {
            RequestKind::ChooseCostPayment { choices, .. } => {
                let pick = choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0);
                answer(&mut g, &r, Answer::CostPayment(pick));
            }
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana)).expect("the inner mana pays the cost");
                answer(&mut g, &r, Answer::Act(Action::of(pay)));
            }
            RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. } => answer(&mut g, &r, Answer::Cards(vec![bear])),
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected mana cost question {other:?}"),
        }
    }
    assert!(g.state.obj(bear).became.is_some() && g.state.obj(dork).became.is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2, "the outer callback reads the Bear, not the nested Dork");
}

#[test]
fn mana_cost_object_uses_old_incarnation_when_sacrifice_is_redirected() {
    let mut g = game();
    let source = put(&mut g, &MANA_FROM_PAID, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    put(&mut g, &EXILE_SACRIFICES, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability is available");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(bear)], ..Action::of(&option) }));
    next(&mut g);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::exile(YOU)));
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2, "the paid object's last-known power is retained");
}

#[test]
fn unpayable_later_origin_reverses_the_cast_and_can_be_retried() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_OVERLAPPING_COSTS, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&bear));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the incomplete cast is reversed");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&bear) && g.state.zone(Zone::BATTLEFIELD).contains(&dork));
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
    let option = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let stack_spell = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::Optional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn spell_cost_can_tap_and_sacrifice_one_object_across_origins() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TAP_AND_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Sacrifice")).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the incomplete cast is reversed");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell) && g.state.zone(Zone::BATTLEFIELD).contains(&bear));
    assert!(!g.state.obj(bear).tapped);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
    let option = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Tap")).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert!(g.state.obj(bear).became.is_some(), "the Bear paid the sacrifice cost");
    assert!(g.state.obj(bear).tapped, "the Bear was tapped before it left");
    let stack_spell = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(stack_spell, CostOrigin::Optional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn a_prevented_sacrifice_does_not_attest_a_paid_spell_cost() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TWO_COSTS, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    put(&mut g, &PREVENT_SACRIFICES, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell can be proposed");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![0]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert!(g.state.obj(bear).became.is_none(), "the cost did not move the Bear");
    assert!(g.state.stack.is_empty(), "the cast was taken back");
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })), "an abandoned payment has no paid-object record");
}

#[test]
fn declined_payment_judges_state_changed_by_a_mana_ability_before_effect_continues() {
    let mut g = game();
    let spell = put(&mut g, &PAYMENT_BOUNDARY, Zone::hand(YOU));
    put(&mut g, &STATE_WATCHER, Zone::BATTLEFIELD);
    let mana = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = cast_to_payment(&mut g, spell);
    let option = option_of(&g, &r, mana).expect("the mana ability is offered during payment");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let decline = match &r.kind { RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::DeclineCost).unwrap(), _ => panic!("expected payment") };
    answer(&mut g, &r, Answer::Act(Action::of(decline)));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 30, "the state trigger must already be queued when the effect continues");
}

#[test]
fn successful_payment_judges_state_changed_by_a_mana_ability_before_effect_continues() {
    let mut g = game();
    let spell = put(&mut g, &PAYMENT_BOUNDARY, Zone::hand(YOU));
    put(&mut g, &STATE_WATCHER, Zone::BATTLEFIELD);
    let mana = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = cast_to_payment(&mut g, spell);
    let option = option_of(&g, &r, mana).expect("the mana ability is offered during payment");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let pay = match &r.kind { RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::PayCost).unwrap(), _ => panic!("expected payment") };
    answer(&mut g, &r, Answer::Act(Action::of(pay)));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 30, "the state trigger must already be queued when the effect continues");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "the produced mana paid the cost");
}

#[test]
fn payment_state_trigger_is_restored_on_rollback_and_replay() {
    let mut g = game();
    let spell = put(&mut g, &PAYMENT_BOUNDARY, Zone::hand(YOU));
    put(&mut g, &STATE_WATCHER, Zone::BATTLEFIELD);
    let mana = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = cast_to_payment(&mut g, spell);
    let before = g.checkpoint();
    let state = g.state.clone();
    let log = g.log.clone();
    let option = option_of(&g, &r, mana).expect("the mana ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let decline = match &r.kind { RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::DeclineCost).unwrap(), _ => panic!("expected payment") };
    answer(&mut g, &r, Answer::Act(Action::of(decline)));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 30);
    g.rollback(before);
    assert!(g.state == state && g.log == log, "rollback restores the pending payment and trigger state");
    let r = next(&mut g);
    let option = option_of(&g, &r, mana).expect("the mana ability is offered again");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let pay = match &r.kind { RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::PayCost).unwrap(), _ => panic!("expected payment") };
    answer(&mut g, &r, Answer::Act(Action::of(pay)));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 30, "replay judges the state trigger once");
}

#[test]
fn player_can_pay_life_before_sacrificing_a_life_dependent_creature() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_SACRIFICE, Zone::hand(YOU));
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let choice = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.contains("Pay 3 life")).expect("paying life is offered"),
        other => panic!("expected a choice of payment order, got {other:?}"),
    };
    answer(&mut g, &r, Answer::CostPayment(choice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(PlayerId(1)).life, 12, "the sacrificed creature had power 8 after paying life");
}

#[test]
fn cancelling_after_a_life_payment_restores_cast_and_paid_costs() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_SACRIFICE, Zone::hand(YOU));
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let choice = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.contains("Pay 3 life")).unwrap(),
        other => panic!("expected a choice of payment order, got {other:?}"),
    };
    answer(&mut g, &r, Answer::CostPayment(choice));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cancel);
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 20, "the life payment was taken back");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&star));
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}

#[test]
fn source_counter_can_be_placed_before_sacrificing_it() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_COUNTER_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let counter = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Counters).expect("counter payment is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(counter));
    next(&mut g);
    assert!(g.state.obj(source).became.is_some());
    assert_eq!(g.state.obj(source).counters(CounterKind::PlusOne), 1);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::PutSourceCounters).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn variable_source_counter_can_be_removed_before_sacrificing_it() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_VARIABLE_COUNTER_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), source, CounterKind::Charge, 1, YOU, crate::counters::How::Effect);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    let counter = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Counters).expect("counter payment is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(counter));
    next(&mut g);
    assert!(g.state.obj(source).became.is_some());
    assert_eq!(g.state.obj(source).counters(CounterKind::Charge), 0);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::VariableSourceCounters).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn loyalty_can_be_paid_before_sacrificing_its_source() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_LOYALTY_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), source, CounterKind::Loyalty, 1, YOU, crate::counters::How::Effect);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let counter = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Counters).expect("loyalty payment is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(counter));
    next(&mut g);
    assert!(g.state.obj(source).became.is_some());
    assert_eq!(g.state.obj(source).counters(CounterKind::Loyalty), 2);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::Loyalty).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn source_can_be_exerted_before_sacrificing_it() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_EXERT_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let exert = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Exert ")).expect("exert is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(exert));
    next(&mut g);
    assert!(g.state.obj(source).became.is_some());
    assert_eq!(g.state.obj(source).marks.exerted_by, Some(YOU));
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::ExertSelf).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn source_can_be_unattached_before_it_is_tapped() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_UNATTACH_AND_TAP, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    g.state.update_obj(source, |o| o.attached_to = Some(bear));
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let unattach = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Unattach).expect("unattach is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(unattach));
    next(&mut g);
    assert_eq!(g.state.obj(source).attached_to, None);
    assert!(g.state.obj(source).tapped);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::UnattachSelf).collect::<Vec<_>>(), vec![source]);
    assert!(g.events().iter().any(|r| matches!(r.event, Event::Unattached { attachment, from } if attachment == source && from == bear)));
}

#[test]
fn craft_can_be_paid_before_life() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_CRAFT_AND_LIFE, Zone::BATTLEFIELD);
    let material = put(&mut g, &ABILITY_WITH_COUNTER_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let craft = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Exile).expect("craft is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(craft));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![material]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(g.state.obj(source).became.is_some() && g.state.obj(material).became.is_some());
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::CraftSource).collect::<Vec<_>>(), vec![source]);
    assert_eq!(g.history().ability_cost_objects_of_part(serial, CostPart::CraftMaterial).collect::<Vec<_>>(), vec![material]);
}

#[test]
fn nonrandom_library_cost_can_be_paid_before_life() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_LIFE_AND_LIBRARY_TOP, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let library = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Library).expect("library payment is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(library));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.obj(g.state.obj(source).became.unwrap()).zone, Zone::library(YOU));
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::LibrarySelf).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn random_library_cost_is_paid_after_nonrandom_costs() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_LIFE_AND_RANDOM_LIBRARY, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    next(&mut g);
    let events = g.events();
    let life = events.iter().position(|r| matches!(r.event, Event::LifeChange { player, delta: -3, .. } if player == YOU)).expect("life was paid");
    let shuffle = events.iter().position(|r| matches!(r.event, Event::Shuffled { player } if player == YOU)).expect("the library was shuffled");
    assert!(life < shuffle, "the nonrandom cost precedes the random library cost");
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::LibrarySelf).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn random_library_cost_waits_when_a_nonrandom_payment_becomes_impossible() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_RANDOM_AND_DYNAMIC_SACRIFICE, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let before = g.events().len();
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("sacrificing first can pay the ability");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let life = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => {
            assert!(!choices.iter().any(|c| c.kind == CostPaymentKind::Library), "shuffle cannot precede the other costs");
            choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap()
        }
        other => panic!("expected payment order, got {other:?}"),
    };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&source) && g.state.zone(Zone::BATTLEFIELD).contains(&star));
    assert!(!g.events()[before..].iter().any(|r| matches!(r.event, Event::Shuffled { player } if player == YOU)));
}

#[test]
fn individual_mana_units_can_bracket_another_payment() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TWO_MANA_AND_MIDPAYMENT_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    add_mana(&mut g, 2);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("one mana, sacrifice, one mana can pay the spell");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered after the first mana"), other => panic!("expected payment order after the first mana, got {other:?}") };
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    assert!(g.state.obj(bear).became.is_some());
}

#[test]
fn spent_mana_is_visible_before_later_payment_operations() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_RECEIPT_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    add_mana(&mut g, 2);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("one mana then sacrifice then one mana can pay");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("spent mana is recorded before the sacrifice"), other => panic!("expected payment order after one mana, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let on_stack = g.state.obj(spell).became.expect("spell moved to stack");
    assert_eq!(g.history().mana_spent_to_cast(on_stack).map(|(_, mana)| mana.0.iter().map(|n| u32::from(*n)).sum::<u32>()).sum::<u32>(), 2);
}

#[test]
fn a_convoke_tap_can_precede_another_cost_before_the_rest_of_mana() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_CONVOKE_MIDPAYMENT_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("convoke, sacrifice, and mana can pay");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, mana_abilities: true, .. } = &r.kind {
        let close = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap();
        answer(&mut g, &r, Answer::Act(Action::of(close)));
        r = next(&mut g);
    }
    let tap = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::TapToPay && o.source == Some(bear)).expect("bear can convoke").clone(), other => panic!("expected mana payment, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&tap)));
    let r = next(&mut g);
    let commit = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::CommitHelpers).expect("the convoke group can be committed"), other => panic!("expected helper-group choice, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(commit));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("the dork became eligible"), other => panic!("expected payment order after convoke, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert!(g.state.obj(bear).tapped && g.state.obj(dork).became.is_some());
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    let on_stack = g.state.obj(spell).became.expect("spell moved to stack");
    assert_eq!(g.history().spell_cost_objects(on_stack, CostOrigin::ManaTotal, 0, CostPart::Convoke).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn delve_improvise_and_waterbend_can_precede_another_cost() {
    for (spell_def, action, helper_def, graveyard, paid_part) in [
        (&SPELL_WITH_DELVE_MIDPAYMENT_SACRIFICE, ActionKind::ExileToPay, &FILLER, true, CostPart::Delve),
        (&SPELL_WITH_IMPROVISE_MIDPAYMENT_SACRIFICE, ActionKind::TapToPay, &ABILITY_WITH_COUNTER_AND_SELF_SACRIFICE, false, CostPart::Improvise),
        (&SPELL_WITH_WATERBEND_MIDPAYMENT_SACRIFICE, ActionKind::TapToPay, &BEAR, false, CostPart::Waterbend),
    ] {
        let mut g = game();
        let spell = put(&mut g, spell_def, Zone::hand(YOU));
        let helper = put(&mut g, helper_def, if graveyard { Zone::graveyard(YOU) } else { Zone::BATTLEFIELD });
        let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
        add_mana(&mut g, 1);
        let r = next(&mut g);
        let cast = option_of(&g, &r, spell).expect("helper, sacrifice, then mana can pay");
        answer(&mut g, &r, Answer::Act(Action::of(&cast)));
        let mut r = next(&mut g);
        if let RequestKind::PayCosts { options, mana_abilities: true, .. } = &r.kind {
            let close = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap();
            answer(&mut g, &r, Answer::Act(Action::of(close)));
            r = next(&mut g);
        }
        let selected = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == action && o.source == Some(helper)).expect("helper is available").clone(), other => panic!("expected helper payment, got {other:?}") };
        answer(&mut g, &r, Answer::Act(Action::of(&selected)));
        let r = next(&mut g);
        let commit = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::CommitHelpers).unwrap(), other => panic!("expected helper-group choice, got {other:?}") };
        answer(&mut g, &r, Answer::CostPayment(commit));
        let r = next(&mut g);
        let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("the dork became eligible"), other => panic!("expected cost order after committing helper, got {other:?}") };
        answer(&mut g, &r, Answer::CostPayment(sacrifice));
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Cards(vec![dork]));
        let r = next(&mut g);
        if let RequestKind::PayCosts { options, .. } = &r.kind {
            let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana)).expect("remaining mana can be paid");
            answer(&mut g, &r, Answer::Act(Action::of(pay)));
            next(&mut g);
        }
        assert!(g.state.obj(dork).became.is_some());
        assert_eq!(g.state.player(YOU).pool.total(), 0, "{}", spell_def.name);
        if graveyard { assert!(g.state.obj(helper).became.is_some()); } else { assert!(g.state.obj(helper).tapped); }
        let on_stack = g.state.obj(spell).became.expect("spell moved to stack");
        assert_eq!(g.history().spell_cost_objects(on_stack, CostOrigin::ManaTotal, 0, paid_part).collect::<Vec<_>>(), vec![helper]);
    }
}

#[test]
fn continuing_convoke_keeps_the_selected_taps_in_one_batch() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_CONVOKE_MIDPAYMENT_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let watcher = put(&mut g, &TAP_BATCH_WATCHER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("two creatures can convoke the spell");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, mana_abilities: true, .. } = &r.kind {
        let close = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap();
        answer(&mut g, &r, Answer::Act(Action::of(close)));
        r = next(&mut g);
    }
    let first = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::TapToPay && o.source == Some(bear)).unwrap().clone(), other => panic!("{other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&first)));
    let r = next(&mut g);
    let keep = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap(), other => panic!("{other:?}") };
    answer(&mut g, &r, Answer::CostPayment(keep));
    let r = next(&mut g);
    let second = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::TapToPay && o.source == Some(dork)).unwrap().clone(), other => panic!("{other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&second)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    let taps: Vec<_> = g.events().iter().enumerate().filter_map(|(n, r)| match r.event { Event::Tapped { permanent } if permanent == bear || permanent == dork => Some(n), _ => None }).collect();
    assert_eq!(taps.len(), 2);
    assert_eq!(taps[1], taps[0] + 1, "both tap events were emitted together");
    assert_eq!(g.state.stack.iter().filter(|item| item.object == watcher).count(), 1, "a once-per-batch watcher triggers once");
    let on_stack = g.state.obj(spell).became.expect("spell moved to stack");
    assert_eq!(g.history().spell_cost_objects(on_stack, CostOrigin::ManaTotal, 0, CostPart::Convoke).collect::<Vec<_>>(), vec![bear, dork]);
}

#[test]
fn committed_helper_payments_are_reversed_when_the_cast_is_cancelled() {
    for (spell_def, action, helper_def, graveyard) in [
        (&SPELL_WITH_CONVOKE_MIDPAYMENT_SACRIFICE, ActionKind::TapToPay, &BEAR, false),
        (&SPELL_WITH_DELVE_MIDPAYMENT_SACRIFICE, ActionKind::ExileToPay, &FILLER, true),
    ] {
        let mut g = game();
        let spell = put(&mut g, spell_def, Zone::hand(YOU));
        let helper = put(&mut g, helper_def, if graveyard { Zone::graveyard(YOU) } else { Zone::BATTLEFIELD });
        let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
        add_mana(&mut g, 1);
        let r = next(&mut g);
        let cast = option_of(&g, &r, spell).expect("the spell is offered");
        answer(&mut g, &r, Answer::Act(Action::of(&cast)));
        let mut r = next(&mut g);
        if let RequestKind::PayCosts { options, mana_abilities: true, .. } = &r.kind {
            let close = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap();
            answer(&mut g, &r, Answer::Act(Action::of(close)));
            r = next(&mut g);
        }
        let selected = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == action && o.source == Some(helper)).unwrap().clone(), other => panic!("{other:?}") };
        answer(&mut g, &r, Answer::Act(Action::of(&selected)));
        let r = next(&mut g);
        let commit = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::CommitHelpers).unwrap(), other => panic!("{other:?}") };
        answer(&mut g, &r, Answer::CostPayment(commit));
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Cancel);
        next(&mut g);
        assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
        assert!(g.state.zone(Zone::BATTLEFIELD).contains(&dork));
        assert_eq!(g.state.player(YOU).pool.total(), 1);
        if graveyard { assert!(g.state.zone(Zone::graveyard(YOU)).contains(&helper)); }
        else { assert!(g.state.zone(Zone::BATTLEFIELD).contains(&helper) && !g.state.obj(helper).tapped); }
        assert!(!g.events().iter().any(|r| matches!(r.event, Event::Convoked { creature, .. } if creature == helper)));
        assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { receipt, .. } if g.state.cost_receipts[receipt].origin == Some(CostOrigin::ManaTotal))));
    }
}

#[test]
fn snow_mana_and_life_keep_their_qualities_when_interleaved() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_SNOW_MANA_AND_LIFE, Zone::hand(YOU));
    let source = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    g.state.add_mana(YOU, Mana::U, 2, source, None, ManaLasts::Step, true);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let mana = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(mana));
    let r = next(&mut g);
    let snow = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::AssignMana && o.pay_snow && o.snow).expect("snow mana can pay the snow symbol").clone(), other => panic!("expected mana assignment, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&snow)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected payment order after snow mana, got {other:?}") };
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::PayCosts { options, .. } => {
            let blue = options.iter().find(|o| o.kind == ActionKind::AssignMana && o.snow && !o.pay_snow).expect("remaining snow blue mana pays blue");
            answer(&mut g, &r, Answer::Act(Action::of(blue)));
            next(&mut g);
        }
        RequestKind::Priority { .. } => {}
        other => panic!("unexpected final mana payment {other:?}"),
    }
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(g.state.stack.iter().any(|i| i.object == g.state.obj(spell).became.unwrap()));
}

#[test]
fn added_mana_cost_can_enable_an_outer_cost_object() {
    let mut g = game();
    let outer = put(&mut g, &MANA_WITH_TAXED_DYNAMIC_SACRIFICE, Zone::BATTLEFIELD);
    let source = put(&mut g, &LIFE_MANA_SOURCE, Zone::BATTLEFIELD);
    let star = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, outer).expect("the taxed mana ability may begin");
    let action = Action::of(&option);
    assert!(crate::validate::check(&g, &r, &Answer::Act(action.clone())).is_ok());
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    let nested = option_of(&g, &r, source).expect("the nested life-paying mana ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&nested)));
    let r = next(&mut g);
    let close = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected nested mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&close)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected outer cost order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(crate::first_choices(&g, &r).contains(&star));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert!(g.state.obj(star).became.is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
}

#[test]
fn nested_mana_ability_can_enable_an_outer_cost_object() {
    let mut g = game();
    let outer = put(&mut g, &MANA_WITH_NESTED_DYNAMIC_SACRIFICE, Zone::BATTLEFIELD);
    let source = put(&mut g, &LIFE_MANA_SOURCE, Zone::BATTLEFIELD);
    let star = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, outer).expect("the outer mana ability may begin");
    assert!(option.target_candidates.is_empty());
    let action = Action::of(&option);
    assert!(crate::validate::check(&g, &r, &Answer::Act(action.clone())).is_ok());
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    let nested = option_of(&g, &r, source).expect("the nested life-paying mana ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&nested)));
    let r = next(&mut g);
    let close = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected nested mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&close)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected outer cost order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(crate::first_choices(&g, &r).contains(&star));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert!(g.state.obj(star).became.is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
}

#[test]
fn dynamic_sacrifice_can_name_the_type_the_later_eligible_object_produces() {
    let mut g = game();
    let source = put(&mut g, &MANA_WITH_DYNAMIC_COLOR, Zone::BATTLEFIELD);
    let star = put(&mut g, &DYNAMIC_COLOR_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("mana ability may begin");
    assert!(option.target_candidates.is_empty());
    let mut action = Action::of(&option);
    action.color = Some(Color::Red);
    action.mana = None;
    assert!(crate::validate::check(&g, &r, &Answer::Act(action.clone())).is_ok());
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(crate::first_choices(&g, &r).contains(&star));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert!(g.state.obj(star).became.is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::R), 1);
}

#[test]
fn mana_ability_can_begin_before_its_cost_object_becomes_eligible() {
    let mut g = game();
    let source = put(&mut g, &MANA_WITH_LIFE_AND_DYNAMIC_SACRIFICE, Zone::BATTLEFIELD);
    let star = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability may begin");
    assert!(option.target_candidates.is_empty(), "no creature is eligible until life is paid");
    let action = Action::of(&option);
    assert!(crate::validate::check(&g, &r, &Answer::Act(action.clone())).is_ok(), "no premature cost object is required");
    let mut invalid = action.clone();
    invalid.targets = vec![Target::Object(source)];
    assert!(crate::validate::check(&g, &r, &Answer::Act(invalid)).is_err(), "an explicitly named invalid object is still refused");
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    assert_eq!(g.state.player(YOU).life, 17);
    assert!(crate::first_choices(&g, &r).contains(&star));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert!(g.state.obj(star).became.is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
}

#[test]
fn mana_ability_can_pay_life_before_sacrificing_its_cost_object() {
    let mut g = game();
    let source = put(&mut g, &MANA_WITH_LIFE_AND_SACRIFICE, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the mana ability is offered");
    let mut action = Action::of(&option);
    action.targets = vec![Target::Object(star)];
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).expect("life is offered"), other => panic!("expected mana-ability payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 8, "the callback reads the sacrificed creature after the life payment");
    assert!(g.state.obj(star).became.is_some());
}

#[test]
fn source_can_be_tapped_before_it_is_returned_for_the_same_activation() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_TAP_AND_SELF_RETURN, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let returned = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(returned));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "returning first makes the tap unpayable");
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&source) && !g.state.obj(source).tapped);
    let ability = option_of(&g, &r, source).expect("the activation can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Tap).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    next(&mut g);
    let returned = g.state.obj(source).became.expect("source returned");
    assert_eq!(g.state.obj(returned).zone, Zone::hand(YOU));
    assert!(g.state.obj(source).tapped);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::TapSelf).collect::<Vec<_>>(), vec![source]);
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::ReturnSelf).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn source_can_be_tapped_before_it_is_sacrificed_for_the_same_activation() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_TAP_AND_SELF_SACRIFICE, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "sacrificing first makes the tap unpayable");
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&source) && !g.state.obj(source).tapped);
    let ability = option_of(&g, &r, source).expect("the activation can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Tap).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    next(&mut g);
    assert!(g.state.obj(source).became.is_some());
    assert!(g.state.obj(source).tapped);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::TapSelf).collect::<Vec<_>>(), vec![source]);
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::SelfPermanent).collect::<Vec<_>>(), vec![source]);
}

#[test]
fn ability_controller_can_choose_life_before_sacrifice() {
    let mut g = game();
    let source = put(&mut g, &ABILITY_WITH_LIFE_AND_SACRIFICE, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).expect("life is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    let serial = g.state.stack.last().expect("the activation is on the stack").serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![star]);
    assert_eq!(g.chars(star).power, Some(8), "last-known power reflects life paid before sacrifice");
}

#[test]
fn convoke_can_tap_a_creature_before_another_cost_sacrifices_it() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_CONVOKE_AND_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("convoke then sacrifice can pay the spell");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { .. }));
    answer(&mut g, &r, Answer::Cancel);
    let r = next(&mut g);
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&bear) && !g.state.obj(bear).tapped);
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let mana = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap(), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(mana));
    let r = next(&mut g);
    let convoke = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::TapToPay && o.source == Some(bear)).expect("the Bear may convoke"), other => panic!("expected mana payment, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(convoke)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert!(g.history().convoked(cast).any(|object| object == bear));
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::ManaTotal, 0, CostPart::Convoke).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn teamwork_can_tap_a_creature_before_another_cost_sacrifices_it() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TEAMWORK_AND_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "sacrificing first leaves teamwork unpayable");
    assert!(!g.state.obj(bear).tapped && g.state.zone(Zone::BATTLEFIELD).contains(&bear));
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Tap).expect("teamwork is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::TapForPower { .. }));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::Optional, 0, CostPart::TapCreatures).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert!(g.events().iter().any(|r| matches!(r.event, Event::TappedForTeamwork { creature, spell } if creature == bear && spell == cast)));
}

#[test]
fn static_sequence_keeps_more_than_six_discard_components() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_SEVEN_DISCARDS, Zone::hand(YOU));
    let cards: Vec<_> = (0..7).map(|i| put(&mut g, if i % 2 == 0 { &BEAR } else { &DORK }, Zone::hand(YOU))).collect();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("all seven filtered discard components are payable");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    for &card in &cards {
        let mut r = next(&mut g);
        if matches!(r.kind, RequestKind::ChooseCostPayment { .. }) {
            answer(&mut g, &r, Answer::CostPayment(0));
            r = next(&mut g);
        }
        answer(&mut g, &r, Answer::Cards(vec![card]));
    }
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    for (component, &card) in cards.iter().enumerate() {
        assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, component, CostPart::Discard(0)).collect::<Vec<_>>(), vec![card]);
    }
}

#[test]
fn behold_can_precede_discard_of_the_same_card_and_records_both() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_BEHOLD_AND_DISCARD, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let discard = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Discard).expect("discard is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(discard));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "discarding first makes behold impossible and reverses the cast");
    assert!(g.state.zone(Zone::hand(YOU)).contains(&bear));
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let behold = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Behold).expect("behold is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(behold));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Behold).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Discard(0)).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn variable_sacrifice_can_precede_life_and_records_the_paid_object() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_VARIABLE_SACRIFICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 20);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    let item = g.state.stack.last().expect("the spell was cast");
    assert_eq!(item.paid, 1);
    assert_eq!(g.history().spell_cost_objects(item.object, CostOrigin::Optional, 0, CostPart::Variable).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn unpaid_chosen_additional_cost_reverses_prior_life_payment() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_EXILE_CHOICE, Zone::hand(YOU));
    let life = g.state.player(YOU).life;
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell may begin");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert!(g.state.stack.is_empty());
    assert_eq!(g.state.obj(spell).zone, Zone::hand(YOU));
    assert!(g.state.zone(Zone::graveyard(YOU)).is_empty());
    assert_eq!(g.state.player(YOU).life, life);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}

#[test]
fn discard_can_feed_a_chosen_additional_graveyard_cost() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_DISCARD_AND_EXILE_CHOICE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell may begin");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::ChooseCards { purpose: Purpose::Discard, .. }));
    assert!(crate::first_choices(&g, &r).contains(&bear));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    let discarded = g.state.obj(bear).became.expect("discard created a graveyard object");
    assert!(crate::first_choices(&g, &r).contains(&discarded));
    answer(&mut g, &r, Answer::Cards(vec![discarded]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Discard(0)).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::AdditionalChoice(0), 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![discarded]);
}

#[test]
fn unpaid_target_defined_cost_reverses_the_cast() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TARGET_EXILE_ONLY, Zone::hand(YOU));
    let target = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let life = g.state.player(YOU).life;
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell may begin");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(target)], ..Action::of(&cast) }));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert!(g.state.stack.is_empty());
    assert_eq!(g.state.obj(spell).zone, Zone::hand(YOU));
    assert!(g.state.zone(Zone::graveyard(YOU)).is_empty());
    assert_eq!(g.state.player(YOU).life, life);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}

#[test]
fn discard_can_feed_target_defined_graveyard_cost() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_DISCARD_AND_TARGET_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    let target = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell may begin");
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Object(target)], ..Action::of(&cast) }));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::ChooseCards { purpose: Purpose::Discard, .. }));
    assert!(crate::first_choices(&g, &r).contains(&bear));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    let discarded = g.state.obj(bear).became.expect("discard created a graveyard object");
    assert!(crate::first_choices(&g, &r).contains(&discarded));
    answer(&mut g, &r, Answer::Cards(vec![discarded]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Discard(0)).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::Targets, 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![discarded]);
}

#[test]
fn discard_can_put_a_new_object_in_the_graveyard_for_another_cost() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_DISCARD_AND_GRAVEYARD_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    put(&mut g, &DORK, Zone::graveyard(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let discard = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Discard).expect("discard is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(discard));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    let discarded = g.state.obj(bear).became.expect("the discard made a graveyard object");
    assert!(crate::first_choices(&g, &r).contains(&discarded));
    answer(&mut g, &r, Answer::Cards(vec![discarded]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Discard(0)).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![discarded]);
}

#[test]
fn evidence_can_be_paid_before_mana_and_records_its_object() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_EVIDENCE, Zone::hand(YOU));
    let card = put(&mut g, &EVIDENCE_CARD, Zone::graveyard(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let evidence = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Collect evidence 3")).expect("evidence is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(evidence));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::CollectEvidence { .. }));
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    answer(&mut g, &r, Answer::Cards(vec![card]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Evidence).collect::<Vec<_>>(), vec![card]);
}

#[test]
fn hand_exile_can_be_paid_before_mana_and_records_its_object() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_HAND_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let exile = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Exile).expect("exile is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(exile));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::HandExile).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn cancelling_after_graveyard_exile_restores_the_card_and_receipt() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_GRAVEYARD_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::graveyard(YOU));
    add_mana(&mut g, 1);
    g.state.add_mana(YOU, Mana::G, 1, ObjectId(0), None, ManaLasts::Step, false);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let exile = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Exile).expect("exile is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(exile));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { .. }));
    answer(&mut g, &r, Answer::Cancel);
    next(&mut g);
    assert!(g.state.zone(Zone::graveyard(YOU)).contains(&bear));
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert_eq!(g.state.player(YOU).pool.total(), 2);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}

#[test]
fn distinct_graveyard_exile_costs_record_each_origin() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_TWO_GRAVEYARD_EXILES, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::graveyard(YOU));
    let dork = put(&mut g, &DORK, Zone::graveyard(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let printed = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.contains("additional cost of")).expect("the printed cost is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(printed));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(!crate::first_choices(&g, &r).contains(&bear), "one card cannot pay both exiles");
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::Optional, 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn graveyard_exile_can_be_paid_before_mana_and_records_its_object() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_GRAVEYARD_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::graveyard(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let exile = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Exile).expect("exile is offered"), other => panic!("expected payment order, got {other:?}") };
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    answer(&mut g, &r, Answer::CostPayment(exile));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::exile(YOU)));
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::GraveyardExile).collect::<Vec<_>>(), vec![bear]);
}

#[test]
fn blight_and_sacrifice_order_changes_eligibility_and_retries() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_BLIGHT_AND_POWER_COST, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let blight = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Blight")).expect("blight is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(blight));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the now-impossible sacrifice reverses the cast");
    assert_eq!(g.state.obj(bear).counters(CounterKind::MinusOne), 0);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert!(g.state.obj(bear).became.is_some());
    assert_eq!(g.state.obj(dork).counters(CounterKind::MinusOne), 1);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Blight).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn counter_removal_and_sacrifice_order_changes_eligibility_and_retries() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_COUNTER_AND_POWER_COST, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), bear, CounterKind::PlusOne, 1, YOU, crate::counters::How::Effect);
    crate::counters::put(&mut cx(&mut g), dork, CounterKind::PlusOne, 1, YOU, crate::counters::How::Effect);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let counters = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Counters).expect("counter removal is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(counters));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Counters(vec![(bear, CounterKind::PlusOne, 1)]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the now-impossible sacrifice reverses the cast");
    assert_eq!(g.state.obj(bear).counters(CounterKind::PlusOne), 1);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Counters(vec![(dork, CounterKind::PlusOne, 1)]));
    next(&mut g);
    assert!(g.state.obj(bear).became.is_some());
    assert_eq!(g.state.obj(dork).counters(CounterKind::PlusOne), 0);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::CountersAmong).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn energy_and_sacrifice_order_changes_eligibility_and_retries() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_ENERGY_AND_POWER_COST, Zone::hand(YOU));
    let star = put(&mut g, &ENERGY_STAR, Zone::BATTLEFIELD);
    crate::ops::change_energy(&mut cx(&mut g), YOU, 2, YOU);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let energy = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Energy).expect("energy is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(energy));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the now-impossible sacrifice reverses the cast");
    assert_eq!(g.state.player(YOU).energy(), 2);
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&star));
    let cast = option_of(&g, &r, spell).expect("the spell can be retried");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).expect("sacrifice is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).energy(), 0);
    assert!(g.state.obj(star).became.is_some());
    assert!(g.state.stack.last().is_some_and(|item| item.kind == StackKind::Spell));
}

#[test]
fn caster_can_pay_life_before_spending_mana() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let choice = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Pay 3 life")).expect("life payment is offered"),
        other => panic!("expected a choice before paying mana or life, got {other:?}"),
    };
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    answer(&mut g, &r, Answer::CostPayment(choice));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

#[test]
fn mana_ability_window_is_available_even_when_pool_already_pays() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    let source = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let activation = option_of(&g, &r, source).expect("CR 601.2g permits activating mana abilities before paying even when the pool suffices");
    assert_eq!(g.state.player(YOU).life, 20, "no life has yet been paid");
    assert_eq!(g.state.player(YOU).pool.total(), 1, "the mana has not yet been spent");
    answer(&mut g, &r, Answer::Act(Action::of(&activation)));
    let r = next(&mut g);
    let mana = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Mana).expect("mana is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(mana));
    let r = next(&mut g);
    let spend = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana == Some(Mana::C)).expect("choose the original mana"), other => panic!("expected mana payment, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(spend)));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17, "the additional life cost was paid");
    assert_eq!(g.state.player(YOU).pool.total(), 1, "the newly produced mana remains after one mana is spent");
}

#[test]
fn mana_window_can_close_before_all_mana_abilities_are_used() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    let first = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    let second = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let activate = option_of(&g, &r, first).expect("first mana ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&activate)));
    let r = next(&mut g);
    assert!(option_of(&g, &r, second).is_some(), "the second mana ability is still available before paying");
    assert_eq!(g.state.player(YOU).life, 20);
    assert_eq!(g.state.player(YOU).pool.total(), 1);
    let done = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap(), other => panic!("expected the mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(done)));
    let r = next(&mut g);
    let mana = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Mana).expect("mana is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(mana));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    assert!(!g.state.obj(second).tapped, "the other mana ability was not activated");
}

#[test]
fn harmonize_creature_is_not_tapped_until_after_the_mana_window() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_HARMONIZE, Zone::graveyard(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let source = put(&mut g, &COLOR_MANA, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("harmonize is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::TapForPower { .. }));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    assert!(!g.state.obj(bear).tapped, "the harmonize tap is a cost, paid after mana abilities");
    let activate = option_of(&g, &r, source).expect("the mana ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&activate)));
    let r = next(&mut g);
    let tap = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.objects.contains(&bear)).expect("the harmonize tap is offered"), other => panic!("expected payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(tap));
    next(&mut g);
    assert!(g.state.obj(bear).tapped, "the selected creature paid the harmonize cost");
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

#[test]
fn player_can_sacrifice_a_life_dependent_creature_before_paying_life() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_SACRIFICE, Zone::hand(YOU));
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let choice = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.starts_with("Sacrifice")).expect("sacrificing the creature is offered"),
        other => panic!("expected a choice of payment order, got {other:?}"),
    };
    answer(&mut g, &r, Answer::CostPayment(choice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(PlayerId(1)).life, 10, "the sacrificed creature had power 10 before paying life");
}

#[test]
fn cost_objects_are_chosen_when_paid_after_earlier_costs_change_eligibility() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_POWER_COST, Zone::hand(YOU));
    put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the spell is available");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let choice = match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.label.contains("Pay 3 life")).expect("paying life is offered"),
        other => panic!("payment order must be chosen before the cost object, got {other:?}"),
    };
    answer(&mut g, &r, Answer::CostPayment(choice));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&reverse), "the creature became eligible after life was paid");
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(PlayerId(1)).life, 19);
}

#[test]
fn earlier_life_payment_can_make_a_previously_ineligible_creature_payable() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_POWER_COST, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("paying life first makes the creature eligible to be sacrificed");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    // The sacrifice is not yet payable, so the only available operation pays life.
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(PlayerId(1)).life, 19, "the creature's power was 1 when sacrificed");
}

#[test]
fn life_paid_for_mana_can_enable_a_behold_cost() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_DYNAMIC_BEHOLD, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    let source = put(&mut g, &LIFE_MANA_SOURCE, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::PayCosts { mana_abilities: true, .. }), "expected mana window, got {:?}", r.kind);
    let ability = option_of(&g, &r, source).expect("life-paying mana ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let close = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&close)));
    let r = next(&mut g);
    let behold = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Behold).unwrap(), other => panic!("expected cost order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(behold));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&reverse));
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    next(&mut g);
    let cast = g.state.stack.last().expect("the spell was cast").object;
    assert_eq!(g.history().spell_cost_objects(cast, CostOrigin::PrintedAdditional, 0, CostPart::Behold).collect::<Vec<_>>(), vec![reverse]);
}

#[test]
fn life_paid_for_mana_can_enable_a_chosen_additional_sacrifice() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_POWER_CHOICE, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    let source = put(&mut g, &LIFE_MANA_SOURCE, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("the chosen cost remains available until the mana window");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let close = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&close)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected cost order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&reverse));
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert!(g.state.obj(reverse).became.is_some());
    assert_eq!(g.state.player(YOU).life, 20);
}

#[test]
fn life_paid_for_mana_can_enable_an_optional_sacrifice() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_OPTIONAL_POWER, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    let source = put(&mut g, &LIFE_MANA_SOURCE, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let RequestKind::ChooseModes { eligible, .. } = &r.kind else { panic!("expected optional-cost choice, got {:?}", r.kind) };
    assert!(eligible.contains(&1), "a mana ability can pay life and make the sacrifice payable");
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    let ability = option_of(&g, &r, source).expect("life-paying mana ability is available");
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    let close = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&close)));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected cost order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    assert!(crate::first_choices(&g, &r).contains(&reverse));
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert!(g.state.obj(reverse).became.is_some());
    assert_eq!(g.state.player(YOU).life, 20);
}

#[test]
fn another_cost_origin_can_make_the_optional_sacrifice_payable() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_OPTIONAL_POWER, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let RequestKind::ChooseModes { eligible, .. } = &r.kind else { panic!("expected optional-cost choice") };
    assert!(eligible.contains(&1), "the mandatory life payment can make this optional cost payable");
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(g.state.obj(reverse).became.is_some());
}

#[test]
fn optional_sacrifice_still_unpayable_after_another_cost_reverses_the_cast() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_OPTIONAL_POWER, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }));
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}

#[test]
fn optional_cost_can_make_an_additional_sacrifice_payable() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_POWER_AND_OPTIONAL_LIFE, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("paying the optional life cost first makes the mandatory sacrifice payable");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![reverse]));
    resolve_all(&mut g);
    assert_eq!(g.state.player(PlayerId(1)).life, 19);
}

#[test]
fn declining_the_only_enabling_optional_cost_reverses_the_cast() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_POWER_AND_OPTIONAL_LIFE, Zone::hand(YOU));
    let reverse = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell can be proposed");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes(vec![0]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }));
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell) && g.state.zone(Zone::BATTLEFIELD).contains(&reverse));
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}
#[test]
fn discard_can_enable_a_graveyard_exile_when_the_graveyard_starts_empty() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_DISCARD_AND_GRAVEYARD_EXILE, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("discarding first makes the graveyard exile payable");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { .. }));
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    let discarded = g.state.obj(bear).became.expect("the discard made a graveyard object");
    answer(&mut g, &r, Answer::Cards(vec![discarded]));
    next(&mut g);
    assert!(g.state.stack.last().is_some_and(|item| item.kind == StackKind::Spell));
}

#[test]
fn proposed_cast_with_no_sacrifice_candidate_is_reversed() {
    let mut g = game();
    let spell = put(&mut g, &SPELL_WITH_LIFE_AND_SACRIFICE, Zone::hand(YOU));
    let before_life = g.state.player(YOU).life;
    let r = next(&mut g);
    let option = option_of(&g, &r, spell).expect("the cast may be proposed");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "unpayable cast returns to priority: {:?}", r.kind);
    assert_eq!(g.state.player(YOU).life, before_life);
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.stack.is_empty());
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { .. })));
}


#[test]
fn player_mana_ability_can_pay_life_before_sacrificing_its_cost_object() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.effect_push(ContinuousEffect {
        source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 0)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE,
    });
    g.state.see_statics();
    let r = next(&mut g);
    let option = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).cloned().expect("player mana ability is offered"), other => panic!("expected priority, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).expect("life is offered"), other => panic!("expected player mana-ability payment order, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 8);
    assert!(g.state.obj(star).became.is_some());
}

#[test]
fn color_from_sacrifice_ignores_an_earlier_return_component() {
    let mut g = game();
    let source = put(&mut g, &RETURN_THEN_COLOR_MANA, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the ability is offered");
    assert!(option.color_choices.has(Color::Green));
    let mut action = Action::of(&option);
    action.color = Some(Color::Green);
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    let returned = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(returned));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::hand(YOU)));
    assert!(g.state.obj(dork).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn player_mana_ability_opens_a_nested_mana_window_before_its_costs() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 1)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let player_ability = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&player_ability)));
    let r = next(&mut g);
    let nested = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index == 0).unwrap().clone(), other => panic!("expected nested mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&nested)));
    let r = next(&mut g);
    let confirm = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&confirm)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    let sacrifice = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Sacrifice).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(sacrifice));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 8);
    assert!(g.state.obj(source).tapped);
    assert!(g.state.obj(star).became.is_some());
}

#[test]
fn player_mana_ability_costs_reverse_when_the_enclosing_cast_is_cancelled() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 0)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let player_ability = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&player_ability)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected ability cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    let r = next(&mut g);
    let cancel = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap().clone(), other => panic!("expected enclosing mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&cancel)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 20);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 0);
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&star));
    assert!(g.state.stack.is_empty());
}

#[test]
fn color_from_sacrificing_the_source_uses_its_producible_mana() {
    let mut g = game();
    let source = put(&mut g, &SELF_COLOR_MANA, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index == 1).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    assert!(option.color_choices.has(Color::Green));
    let mut action = Action::of(&option);
    action.color = Some(Color::Green);
    answer(&mut g, &r, Answer::Act(action));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(source).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn irreversible_player_mana_ability_replays_after_enclosing_cast_is_cancelled() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 2)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let library = g.state.zone(Zone::library(YOU)).len();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let player_ability = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&player_ability)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected ability cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    let r = next(&mut g);
    let cancel = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap().clone(), other => panic!("expected enclosing mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&cancel)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1);
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.stack.is_empty());
}

#[test]
fn color_from_sacrificing_the_granter_uses_that_granters_producible_mana() {
    let mut g = game();
    let granter = put(&mut g, &SELF_COLOR_MANA, Zone::BATTLEFIELD);
    let holder = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let stamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source: granter, affected: holder, controller: YOU, timestamp: stamp,
        kind: EffectKind::AddAbility(AbilityRef::delayed(g.cards.id_of(&GRANTED_COLOR_MANA), 0)), duration: Duration::EndOfTurn, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let option = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(holder) && o.kind == ActionKind::ActivateAbility).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    assert!(option.color_choices.has(Color::Green));
    let mut action = Action::of(&option);
    action.color = Some(Color::Green);
    answer(&mut g, &r, Answer::Act(action));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(granter).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&holder));
}

#[test]
fn repeated_player_grants_keep_their_paid_objects_isolated() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let first = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let second = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    for _ in 0..2 {
        g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
            kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 0)),
            duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    }
    g.state.see_statics();
    for (ordinal, star) in [(0, first), (1, second)] {
        let r = next(&mut g);
        let option = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index == PLAYER_MANA_ABILITY + ordinal).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
        answer(&mut g, &r, Answer::Act(Action::of(&option)));
        let r = next(&mut g);
        let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
        answer(&mut g, &r, Answer::CostPayment(life));
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Cards(vec![star]));
    }
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 14);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 15, "the second activation sees only its own paid object and its later power");
    assert!(g.state.obj(first).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.obj(second).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn kept_player_mana_replay_identifies_the_grant_after_an_earlier_grant_returns() {
    let mut g = game();
    let vanishing = put(&mut g, &VANISHING_PLAYER_GRANTER, Zone::BATTLEFIELD);
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    for (granter, index, duration) in [(vanishing, 0, Duration::WhileSourceOnBattlefield), (source, 2, Duration::EndOfTurn)] {
        g.state.effect_push(ContinuousEffect { source: granter, affected: granter, controller: YOU,
            kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), index)),
            duration, timestamp: 1, words: Words::NONE });
    }
    g.state.see_statics();
    let library = g.state.zone(Zone::library(YOU)).len();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let vanish = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(vanishing) && o.index == 0).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&vanish)));
    let r = next(&mut g);
    let kept = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&kept)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    let r = next(&mut g);
    let cancel = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&cancel)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1, "the kept ability drew once");
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.zone(Zone::BATTLEFIELD).contains(&vanishing), "the unrelated reversible ability is undone");
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
}

#[test]
fn kept_player_mana_replays_even_if_the_granting_mana_ability_is_undone() {
    let mut g = game();
    let source = put(&mut g, &TEMPORARY_PLAYER_GRANTER, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    g.state.see_statics();
    let library = g.state.zone(Zone::library(YOU)).len();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let grant = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index == 0).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&grant)));
    let r = next(&mut g);
    let kept = match &r.kind { RequestKind::PayCosts { options, mana_abilities: true, .. } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&kept)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    let r = next(&mut g);
    let cancel = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&cancel)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1);
    assert_eq!(g.state.player(YOU).life, 17);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(!g.state.obj(source).tapped);
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(!g.state.effects.iter().any(|e| matches!(e.kind, EffectKind::PlayerManaAbility(_))));
    assert!(g.state.zone(Zone::hand(YOU)).contains(&spell));
}

#[test]
fn color_from_multiple_sacrifices_uses_the_explicit_component() {
    let mut g = game();
    let source = put(&mut g, &TWO_SACRIFICE_COLOR_MANA, Zone::BATTLEFIELD);
    let green = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let red = put(&mut g, &RED_DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the ability is offered");
    assert!(option.color_choices.has(Color::Red));
    // Green may be announced before the costs are paid, but the second sacrifice controls the outcome.
    let mut wrong = Action::of(&option);
    wrong.color = Some(Color::Green);
    answer(&mut g, &r, Answer::Act(wrong));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![green]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![red]));
    let r = next(&mut g);
    assert!(g.state.obj(green).became.is_none(), "the first sacrifice was reversed");
    assert!(g.state.obj(red).became.is_none(), "the second sacrifice was reversed");
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 0);
    assert_eq!(g.state.player(YOU).pool.get(Mana::R), 0);
    let option = option_of(&g, &r, source).expect("the ability may be tried again");
    let mut action = Action::of(&option);
    action.color = Some(Color::Red);
    answer(&mut g, &r, Answer::Act(action));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCostPayment { .. }));
    answer(&mut g, &r, Answer::CostPayment(0));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![green]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![red]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::R), 1);
    assert!(g.state.obj(green).became.is_some());
    assert!(g.state.obj(red).became.is_some());
}

#[test]
fn chosen_activation_cost_retains_each_distinct_operation_and_receipt() {
    let mut g = game();
    let source = put(&mut g, &CHOSEN_DIFFERENT_COSTS, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, source).expect("the ability can be proposed");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let returned = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(returned));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let serial = g.state.stack.last().unwrap().serial;
    assert_eq!(g.history().ability_cost_objects(serial, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().ability_cost_objects(serial, 1, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    assert!(g.state.obj(dork).became.is_some_and(|id| g.state.obj(id).zone == Zone::hand(YOU)));
}

#[test]
fn multikicker_and_printed_cost_retain_different_operations() {
    let mut g = game();
    let spell = put(&mut g, &MULTIKICK_DIFFERENT_COSTS, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell can be proposed");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    let returned = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(returned));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    let item = g.state.stack.last().expect("the spell was cast");
    assert_eq!(g.history().spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert_eq!(g.history().spell_cost_objects(item.object, CostOrigin::Kicker(0), 0, CostPart::Permanents).collect::<Vec<_>>(), vec![dork]);
}

#[test]
fn resolution_cost_can_pay_life_before_the_sacrifice_it_enables() {
    let mut g = game();
    let spell = put(&mut g, &RESOLUTION_LIFE_AND_SACRIFICE, Zone::hand(YOU));
    let creature = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.see_statics();
    let r = cast_to_payment(&mut g, spell);
    let pay = pays(&r).expect("paying life first makes the creature eligible");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17, "the only currently payable operation was paid first");
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    answer(&mut g, &r, Answer::Cards(vec![creature]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 22);
    assert!(g.state.obj(creature).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    let payment = g.events().iter().find_map(|r| match r.event { Event::ResolutionPaymentBegan { payment, payer, .. } if payer == YOU => Some(payment), _ => None }).expect("the resolution payment has an identity");
    assert_eq!(g.history().resolution_cost_objects(payment, 0, CostPart::Permanents).collect::<Vec<_>>(), vec![creature]);
    assert!(!g.events().iter().any(|r| matches!(r.event, Event::CostObjectPaid { object, .. } if object == creature)), "the object did not pay the spell's casting cost");
}

#[test]
fn repeated_resolution_payments_have_distinct_object_receipts() {
    let mut g = game();
    let spell = put(&mut g, &RESOLUTION_TWO_PAYMENTS, Zone::hand(YOU));
    let first = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let second = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = cast_to_payment(&mut g, spell);
    let serial = g.state.stack.last().expect("the spell is resolving").serial;
    let pay = pays(&r).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![first]));
    let r = next(&mut g);
    let pay = pays(&r).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![second]));
    next(&mut g);
    let payments: Vec<_> = g.events().iter().filter_map(|r| match r.event { Event::ResolutionPaymentBegan { payment, .. } => Some(payment), _ => None }).collect();
    assert_eq!(payments.len(), 2);
    assert_ne!(payments[0], payments[1]);
    assert_eq!(g.history().resolution_payments(serial).collect::<Vec<_>>(), payments);
    assert_eq!(g.history().resolution_cost_objects(payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![first]);
    assert_eq!(g.history().resolution_cost_objects(payments[1], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![second]);
    assert_eq!(g.state.player(YOU).life, 25);
}

#[test]
fn resolution_payment_receipt_survives_replay_after_an_unrelated_mana_ability_is_undone() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let star = put(&mut g, &LIFE_STAR, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &SPELL_WITH_MANA_AND_LIFE, Zone::hand(YOU));
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 3)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let library = g.state.zone(Zone::library(YOU)).len();
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    let reversible = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.source == Some(source) && o.index == 0).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&reversible)));
    let r = next(&mut g);
    let kept = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&kept)));
    let r = next(&mut g);
    let life = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Life).unwrap(), other => panic!("expected cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(life));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    let r = next(&mut g);
    let pay = pays(&r).expect("the resolving mana ability can pay life");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let returned = match &r.kind { RequestKind::ChooseCostPayment { choices, .. } => choices.iter().position(|c| c.kind == CostPaymentKind::Return).unwrap(), other => panic!("expected resolution cost ordering, got {other:?}") };
    answer(&mut g, &r, Answer::CostPayment(returned));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let r = next(&mut g);
    let cancel = match &r.kind { RequestKind::PayCosts { options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap().clone(), other => panic!("expected mana window, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&cancel)));
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.player(YOU).life, 16);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert_eq!(g.state.zone(Zone::library(YOU)).len(), library - 1);
    assert!(!g.state.obj(source).tapped);
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
    let payments: Vec<_> = g.events().iter().filter_map(|r| match r.event { Event::ResolutionPaymentBegan { payment, .. } => Some(payment), _ => None }).collect();
    assert_eq!(payments.len(), 1);
    assert_eq!(g.history().resolution_cost_objects(payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![bear]);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::hand(YOU)));
}

#[test]
fn copied_spell_resolution_payments_belong_to_each_resolving_serial() {
    let mut g = game();
    let spell = put(&mut g, &RESOLUTION_ONE_PAYMENT, Zone::hand(YOU));
    let first = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let second = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    next(&mut g);
    let original = *g.state.stack.last().unwrap();
    let copy = cantrip_api::Game::copy_spell(&mut cx(&mut g), original.object, YOU, false).unwrap().unwrap();
    let copied = *g.state.stack.last().unwrap();
    assert_eq!(copied.object, copy);
    assert_ne!(original.serial, copied.serial);
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    for object in [first, second] {
        let r = resolve_all(&mut g);
        let pay = pays(&r).unwrap_or_else(|| panic!("the resolving instance can pay: {:?}", r.kind));
        answer(&mut g, &r, Answer::Act(Action::of(&pay)));
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Cards(vec![object]));
    }
    next(&mut g);
    let copy_payments: Vec<_> = g.history().resolution_payments(copied.serial).collect();
    let original_payments: Vec<_> = g.history().resolution_payments(original.serial).collect();
    assert_eq!(copy_payments.len(), 1);
    assert_eq!(original_payments.len(), 1);
    assert_ne!(copy_payments, original_payments);
    assert_eq!(g.history().resolution_cost_objects(copy_payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![first]);
    assert_eq!(g.history().resolution_cost_objects(original_payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![second]);
    assert_eq!(g.state.player(YOU).life, 30);
}

#[test]
fn copied_ability_resolution_payments_belong_to_each_resolving_serial() {
    let mut g = game();
    let source = put(&mut g, &RESOLUTION_ONE_PAYMENT_ABILITY, Zone::BATTLEFIELD);
    let first = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let second = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let activate = option_of(&g, &r, source).unwrap();
    answer(&mut g, &r, Answer::Act(Action::of(&activate)));
    next(&mut g);
    let original = g.state.stack.last().unwrap().serial;
    let copied = cantrip_api::Game::copy_ability(&mut cx(&mut g), Target::Ability(original), YOU, false).unwrap().unwrap();
    assert_ne!(original, copied);
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    for object in [first, second] {
        let r = resolve_all(&mut g);
        let pay = pays(&r).expect("the resolving instance can pay");
        answer(&mut g, &r, Answer::Act(Action::of(&pay)));
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Cards(vec![object]));
    }
    next(&mut g);
    let copy_payments: Vec<_> = g.history().resolution_payments(copied).collect();
    let original_payments: Vec<_> = g.history().resolution_payments(original).collect();
    assert_eq!(copy_payments.len(), 1);
    assert_eq!(original_payments.len(), 1);
    assert_ne!(copy_payments, original_payments);
    assert_eq!(g.history().resolution_cost_objects(copy_payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![first]);
    assert_eq!(g.history().resolution_cost_objects(original_payments[0], 0, CostPart::Permanents).collect::<Vec<_>>(), vec![second]);
    assert_eq!(g.state.player(YOU).life, 30);
}

#[test]
fn player_mana_ability_announces_x_for_a_variable_sacrifice_cost() {
    let mut g = game();
    let source = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 4)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let ability = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "X is announced before payment: {:?}", r.kind);
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(bear).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn player_mana_x_can_anticipate_a_filter_changed_by_another_cost() {
    let mut g = game();
    let source = put(&mut g, &SELF_COLOR_MANA, Zone::BATTLEFIELD);
    let star = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 5)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let ability = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, most, .. } if most >= 1), "X can anticipate the life payment: {:?}", r.kind);
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn player_mana_any_number_can_anticipate_a_filter_changed_by_another_cost() {
    let mut g = game();
    let source = put(&mut g, &SELF_COLOR_MANA, Zone::BATTLEFIELD);
    let star = put(&mut g, &REVERSE_LIFE_STAR, Zone::BATTLEFIELD);
    g.state.effect_push(ContinuousEffect { source, affected: source, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(g.cards.id_of(&PLAYER_MANA_WITH_LIFE_AND_SACRIFICE), 6)),
        duration: Duration::EndOfTurn, timestamp: 1, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let ability = match &r.kind { RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source) && o.index >= PLAYER_MANA_ABILITY).unwrap().clone(), other => panic!("expected priority, got {other:?}") };
    answer(&mut g, &r, Answer::Act(Action::of(&ability)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, most, .. } if most >= 1));
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
    answer(&mut g, &r, Answer::Cards(vec![star]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
    assert!(g.state.obj(star).became.is_some_and(|id| g.state.obj(id).zone == Zone::graveyard(YOU)));
}

#[test]
fn spliced_arcane_cost_choices_keep_prefix_order_and_copy_effects() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let spell = put(&mut g, &ARCANE_CHOOSE_PAIR, Zone::hand(YOU));
    let graft = put(&mut g, &GRAFT, Zone::hand(YOU));
    add_mana(&mut g, 2);
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("Arcane probe offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Splice, .. }));
    answer(&mut g, &r, Answer::Cards(vec![graft]));
    let mut selected_dork = false;
    for _ in 0..30 {
        let r = next(&mut g);
        match &r.kind {
            RequestKind::ChooseCostPayment { choices, .. } => {
                // Deliberately pay the second declared choice before the first.
                let index = if !selected_dork {
                    choices.iter().position(|c| c.kind == CostPaymentKind::Choose && c.label.contains("Dork"))
                } else { choices.iter().position(|c| c.kind == CostPaymentKind::Choose) }
                    .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana))
                    .unwrap_or_else(|| panic!("unexpected payment choices: {choices:?}"));
                answer(&mut g, &r, Answer::CostPayment(index));
            }
            RequestKind::ChooseCards { purpose: Purpose::Choose, cards, rule, .. } => {
                let object = if cards.contains(&dork) && rule.listed_card(dork) && !selected_dork { selected_dork = true; dork } else { bear };
                answer(&mut g, &r, Answer::Cards(vec![object]));
            }
            RequestKind::PayCosts { options, .. } => {
                let option = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .unwrap_or_else(|| panic!("unexpected mana choices: {options:?}"));
                answer(&mut g, &r, Answer::Act(Action::of(option)));
            }
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected question: {other:?}"),
        }
    }
    assert!(selected_dork);
    let original = *g.state.stack.last().expect("cast spell on stack");
    assert_eq!(g.state.members(original.members), &[graft, bear, dork]);
    assert_eq!(original.data[0], 1);
    let copy = cantrip_api::Game::copy_spell(&mut cx(&mut g), original.object, YOU, false).unwrap().unwrap();
    let copied = *g.state.stack.last().unwrap();
    assert_eq!(copied.object, copy);
    assert_eq!(g.state.members(copied.members), &[graft, bear, dork]);
    assert_eq!(copied.data[0], 1);
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 46, "both the original and its copy resolve their chosen-object and spliced effects");
}

#[test]
fn granted_choice_uses_the_same_link_for_eligibility_and_payment() {
    let mut g = game();
    let granter = put(&mut g, &FILLER, Zone::BATTLEFIELD);
    let holder = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let without_grant = Asker { instance: None, source: holder, controller: YOU, by: None, spell: false };
    assert!(!is_granting_object(&cx(&mut g), &without_grant, holder), "the same object is ineligible without grant context");
    let stamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source: granter, affected: holder, controller: YOU, timestamp: stamp,
        kind: EffectKind::AddAbility(AbilityRef::delayed(g.cards.id_of(&GRANTED_CONTEXT_CHOOSE), 0)), duration: Duration::EndOfTurn, words: Words::NONE });
    g.state.see_statics();
    let r = next(&mut g);
    let option = option_of(&g, &r, holder).expect("the granted ability is offered with its linked granter");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Choose, cards, rule, .. } => {
            assert!(cards.contains(&holder) && rule.listed_card(holder), "linked ability identity is its holder: {cards:?}");
            assert!(!cards.contains(&granter) || !rule.listed_card(granter));
        }
        other => panic!("expected the granted-cost choice, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![holder]));
    next(&mut g);
    let item = *g.state.stack.last().expect("ability remains on stack");
    assert_eq!(g.state.members(item.members), &[holder]);
    assert_eq!(g.history().ability_cost_objects(item.serial, 0, CostPart::Choose).collect::<Vec<_>>(), vec![holder]);
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 23);
}

#[test]
fn choice_alone_needs_a_candidate_but_life_can_enable_a_spell_choice() {
    let mut empty = game();
    let inert = put(&mut empty, &CHOOSE_ONLY_SPELL, Zone::hand(YOU));
    let r = next(&mut empty);
    assert!(option_of(&empty, &r, inert).is_none(), "a lone choice with no candidate cannot change itself");

    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let spell = put(&mut g, &LIFE_THEN_CHOOSE_SPELL, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("paying life can enable the later spell-only choice");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let mut r = next(&mut g);
    if let RequestKind::ChooseCostPayment { choices, .. } = &r.kind {
        let life = choices.iter().position(|c| c.kind == CostPaymentKind::Life).expect("life is a legal first operation");
        answer(&mut g, &r, Answer::CostPayment(life));
        r = next(&mut g);
    }
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Choose, cards, rule, .. } => assert!(cards.contains(&bear) && rule.listed_card(bear)),
        other => panic!("expected spell-context choice after paying life, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 19);
    let item = *g.state.stack.last().expect("spell was cast");
    assert_eq!(g.history().spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 1, CostPart::Choose).collect::<Vec<_>>(), vec![bear]);
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, 22);
}
